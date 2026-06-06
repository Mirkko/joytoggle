use std::collections::HashMap;
use std::sync::Arc;

use joytoggle_core::{
    Device, DeviceState, DeviceToggler, FileCacheStore, InterfaceId, StateStore, SysfsReader,
};
use zbus::{interface, proxy, Connection, message::Header};
use zbus::zvariant::{OwnedValue, Value};

const POLKIT_ACTION: &str = "org.joytoggle.daemon.manage-device";

#[proxy(
    interface = "org.freedesktop.PolicyKit1.Authority",
    default_service = "org.freedesktop.PolicyKit1",
    default_path = "/org/freedesktop/PolicyKit1/Authority"
)]
trait PolkitAuthority {
    fn check_authorization(
        &self,
        subject: (String, HashMap<String, OwnedValue>),
        action_id: &str,
        details: HashMap<String, String>,
        flags: u32,
        cancellation_id: &str,
    ) -> zbus::Result<(bool, bool, HashMap<String, String>)>;
}

async fn check_polkit_auth(conn: &Connection, sender: &str) -> zbus::fdo::Result<()> {
    let dbus = zbus::fdo::DBusProxy::new(conn).await
        .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

    let bus_name = zbus::names::BusName::try_from(sender)
        .map_err(|_| zbus::fdo::Error::InvalidArgs(format!("invalid sender: {sender:?}")))?;
    let pid = dbus.get_connection_unix_process_id(bus_name).await
        .map_err(|e| zbus::fdo::Error::Failed(format!("could not get caller PID: {e}")))?;

    let mut subject_details: HashMap<String, OwnedValue> = HashMap::new();
    subject_details.insert(
        "pid".into(),
        Value::U32(pid).try_to_owned()
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?,
    );
    subject_details.insert(
        "start-time".into(),
        Value::U64(0).try_to_owned()
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?,
    );

    let polkit = PolkitAuthorityProxy::new(conn).await
        .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

    let (authorized, _, _) = polkit
        .check_authorization(
            ("unix-process".to_string(), subject_details),
            POLKIT_ACTION,
            HashMap::new(),
            1, // AllowUserInteraction
            "",
        )
        .await
        .map_err(|e| zbus::fdo::Error::Failed(format!("polkit check failed: {e}")))?;

    if !authorized {
        return Err(zbus::fdo::Error::AccessDenied(
            "polkit authorization denied".into(),
        ));
    }
    Ok(())
}

pub struct JoyToggleDaemon {
    pub toggler:     Arc<dyn DeviceToggler + Send + Sync>,
    pub state_store: Arc<dyn StateStore + Send + Sync>,
    pub scanner:     Arc<dyn SysfsReader + Send + Sync>,
    pub cache:       Arc<FileCacheStore>,
}

#[interface(name = "org.joytoggle.Daemon1")]
impl JoyToggleDaemon {
    async fn enable_device(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        iface_id: String,
    ) -> zbus::fdo::Result<()> {
        let sender = header.sender()
            .ok_or_else(|| zbus::fdo::Error::Failed("no sender in message header".into()))?;
        check_polkit_auth(conn, sender.as_str()).await?;

        if !InterfaceId::is_valid(&iface_id) {
            return Err(zbus::fdo::Error::InvalidArgs(format!(
                "invalid interface ID: {iface_id:?}"
            )));
        }
        self.toggler
            .enable(&InterfaceId::from(iface_id))
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn disable_device(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        iface_id: String,
    ) -> zbus::fdo::Result<()> {
        let sender = header.sender()
            .ok_or_else(|| zbus::fdo::Error::Failed("no sender in message header".into()))?;
        check_polkit_auth(conn, sender.as_str()).await?;

        if !InterfaceId::is_valid(&iface_id) {
            return Err(zbus::fdo::Error::InvalidArgs(format!(
                "invalid interface ID: {iface_id:?}"
            )));
        }
        self.toggler
            .disable(&InterfaceId::from(iface_id))
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn save_state(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: Header<'_>,
        state_json: String,
    ) -> zbus::fdo::Result<()> {
        let sender = header.sender()
            .ok_or_else(|| zbus::fdo::Error::Failed("no sender in message header".into()))?;
        check_polkit_auth(conn, sender.as_str()).await?;

        let state: DeviceState = serde_json::from_str(&state_json)
            .map_err(|e| zbus::fdo::Error::InvalidArgs(format!("invalid state JSON: {e}")))?;
        self.state_store
            .save(&state)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    /// Returns JSON array of Device — live devices merged with cache and saved state.
    /// Mirrors the Python get_devices_with_cache() logic.
    async fn list_devices(&self) -> zbus::fdo::Result<String> {
        let mut live = self.scanner.read_devices();
        let cached   = self.cache.load_cache();
        let state    = self.state_store.load();

        let live_usb_paths: std::collections::HashSet<String> = live
            .iter()
            .filter_map(|d| d.usb_path.clone())
            .collect();

        // Append cached devices not currently in sysfs (disabled/unbound)
        for mut cd in cached {
            let in_live = cd.usb_path
                .as_ref()
                .map(|p| live_usb_paths.contains(p))
                .unwrap_or(false);
            if !in_live {
                cd.enabled = false;
                live.push(cd);
            }
        }

        // Apply saved state + deduplicate by interface ID
        let mut seen: HashMap<String, ()> = HashMap::new();
        let mut devices: Vec<Device> = Vec::new();
        for mut d in live {
            let key = d.iface_id().as_str().to_owned();
            if seen.contains_key(&key) {
                continue;
            }
            seen.insert(key.clone(), ());
            // Saved state overrides sysfs reading
            if let Some(&enabled) = state.get(&InterfaceId::from(key)) {
                d.enabled = enabled;
            }
            devices.push(d);
        }

        // Persist updated cache (includes disabled devices so they survive reboots)
        let _ = self.cache.save_cache(&devices);

        serde_json::to_string(&devices)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }
}
