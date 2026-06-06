use zbus_macros::proxy;

/// Generated D-Bus client proxy for org.joytoggle.Daemon1.
/// zbus derives `JoyToggleDaemonProxy` from this trait automatically.
#[proxy(
    interface = "org.joytoggle.Daemon1",
    default_service = "org.joytoggle.Daemon",
    default_path = "/org/joytoggle/Daemon"
)]
pub trait JoyToggleDaemon {
    /// Enable a USB HID device by interface ID (e.g. "1-6:1.0").
    async fn enable_device(&self, iface_id: &str) -> zbus::Result<()>;

    /// Disable a USB HID device by interface ID.
    async fn disable_device(&self, iface_id: &str) -> zbus::Result<()>;

    /// Persist the full device state map as JSON.
    async fn save_state(&self, state_json: &str) -> zbus::Result<()>;

    /// Return JSON array of Device structs — live scan merged with cache + state.
    async fn list_devices(&self) -> zbus::Result<String>;
}
