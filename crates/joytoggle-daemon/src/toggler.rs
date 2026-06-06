use std::fs;
use std::path::Path;

use joytoggle_core::{DeviceToggler, InterfaceId};

const BIND_PATH: &str = "/sys/bus/usb/drivers/usbhid/bind";
const UNBIND_PATH: &str = "/sys/bus/usb/drivers/usbhid/unbind";
const USBHID_DIR: &str = "/sys/bus/usb/drivers/usbhid";

pub struct SysfsDeviceToggler;

impl SysfsDeviceToggler {
    fn is_bound(iface_id: &InterfaceId) -> bool {
        Path::new(USBHID_DIR).join(iface_id.as_str()).exists()
    }
}

impl DeviceToggler for SysfsDeviceToggler {
    fn enable(&self, iface_id: &InterfaceId) -> anyhow::Result<()> {
        if Self::is_bound(iface_id) {
            tracing::debug!("already enabled: {iface_id}");
            return Ok(());
        }
        fs::write(BIND_PATH, iface_id.as_str())?;
        tracing::info!("enabled {iface_id}");
        Ok(())
    }

    fn disable(&self, iface_id: &InterfaceId) -> anyhow::Result<()> {
        if !Self::is_bound(iface_id) {
            tracing::debug!("already disabled: {iface_id}");
            return Ok(());
        }
        fs::write(UNBIND_PATH, iface_id.as_str())?;
        tracing::info!("disabled {iface_id}");
        Ok(())
    }
}
