use std::sync::Arc;

use joytoggle_core::{DeviceState, DeviceToggler, InterfaceId, StateStore};
use zbus::interface;

pub struct JoyToggleDaemon {
    pub toggler: Arc<dyn DeviceToggler + Send + Sync>,
    pub state_store: Arc<dyn StateStore + Send + Sync>,
}

#[interface(name = "org.joytoggle.Daemon1")]
impl JoyToggleDaemon {
    async fn enable_device(&self, iface_id: String) -> zbus::fdo::Result<()> {
        if !InterfaceId::is_valid(&iface_id) {
            return Err(zbus::fdo::Error::InvalidArgs(format!(
                "invalid interface ID: {iface_id:?}"
            )));
        }
        self.toggler
            .enable(&InterfaceId::from(iface_id))
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn disable_device(&self, iface_id: String) -> zbus::fdo::Result<()> {
        if !InterfaceId::is_valid(&iface_id) {
            return Err(zbus::fdo::Error::InvalidArgs(format!(
                "invalid interface ID: {iface_id:?}"
            )));
        }
        self.toggler
            .disable(&InterfaceId::from(iface_id))
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn save_state(&self, state_json: String) -> zbus::fdo::Result<()> {
        let state: DeviceState = serde_json::from_str(&state_json)
            .map_err(|e| zbus::fdo::Error::InvalidArgs(format!("invalid state JSON: {e}")))?;
        self.state_store
            .save(&state)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn list_devices(&self) -> zbus::fdo::Result<String> {
        // Returns the current saved state as JSON for now.
        // Full device scan will be added when integrating LinuxSysfsReader.
        let state = self.state_store.load();
        serde_json::to_string(&state)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }
}
