use std::fs;
use std::path::Path;

use joytoggle_core::{DeviceToggler, InterfaceId};

// Bind always goes through usbhid (standard HID USB driver).
const BIND_PATH: &str = "/sys/bus/usb/drivers/usbhid/bind";

// USB interface sysfs root — device driver can be any HID driver.
const USB_DEVICES: &str = "/sys/bus/usb/devices";

pub struct SysfsDeviceToggler;

impl SysfsDeviceToggler {
    // Check driver symlink on the device itself, not in the usbhid dir.
    // Works for usbhid, hid-generic, and any other driver.
    fn driver_unbind_path(iface_id: &InterfaceId) -> std::path::PathBuf {
        Path::new(USB_DEVICES)
            .join(iface_id.as_str())
            .join("driver/unbind")
    }

    fn is_bound(iface_id: &InterfaceId) -> bool {
        Path::new(USB_DEVICES)
            .join(iface_id.as_str())
            .join("driver")
            .exists()
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
        let unbind = Self::driver_unbind_path(iface_id);
        if !unbind.exists() {
            tracing::debug!("already disabled (no driver): {iface_id}");
            return Ok(());
        }
        fs::write(&unbind, iface_id.as_str())?;
        tracing::info!("disabled {iface_id}");
        Ok(())
    }
}
