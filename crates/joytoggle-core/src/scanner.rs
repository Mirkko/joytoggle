use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::device::{Device, DeviceType, InterfaceId};
use crate::traits::SysfsReader;

const INPUT_DIR: &str = "/sys/class/input";

fn detect_type(name: &str) -> DeviceType {
    let n = name.to_lowercase();
    if n.contains("pedal") || n.contains("rudder") || n.contains("torq") {
        DeviceType::RudderPedals
    } else if n.contains("throttle") || n.contains("mongoose") || n.contains("vmax") {
        DeviceType::Throttle
    } else if n.contains("joystick")
        || n.contains("alpha")
        || n.contains("constellation")
        || n.contains("stick")
        || n.contains("warbrd")
    {
        DeviceType::Joystick
    } else if n.contains("gamepad")
        || n.contains("xbox")
        || n.contains("playstation")
        || n.contains("dualshock")
        || n.contains("dualsense")
        || n.contains("logitech f")
    {
        DeviceType::Gamepad
    } else if n.contains("wheel") || n.contains("steering") {
        DeviceType::SteeringWheel
    } else {
        DeviceType::Gamepad
    }
}

fn should_autohide(name: &str) -> bool {
    let n = name.to_lowercase();
    n.contains("keyboard")
        || n.contains("volume")
        || n.contains("media")
        || n.contains("consumer control")
        || n.contains("system control")
}

fn should_ignore(name: &str) -> bool {
    name.to_lowercase().contains("mouse")
}

/// Checks if a path component looks like a USB interface ID: "1-11.4.1:1.0"
fn is_usb_interface(s: &str) -> bool {
    InterfaceId::is_valid(s)
}

/// Walk the real sysfs path to find the USB interface component.
fn find_usb_path(real_path: &Path) -> Option<PathBuf> {
    let components: Vec<_> = real_path.components().collect();
    for (i, comp) in components.iter().enumerate() {
        let s = comp.as_os_str().to_string_lossy();
        if is_usb_interface(&s) {
            let path: PathBuf = components[..=i].iter().collect();
            return Some(path);
        }
    }
    None
}

fn read_id_file(dir: &Path, filename: &str) -> Option<String> {
    fs::read_to_string(dir.join(filename))
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

/// Real sysfs implementation of SysfsReader.
pub struct LinuxSysfsReader;

impl SysfsReader for LinuxSysfsReader {
    fn read_devices(&self) -> Vec<Device> {
        let mut devices = Vec::new();
        // Deduplicate by USB interface ID (multiple event nodes can share one interface)
        let mut seen: HashMap<String, ()> = HashMap::new();

        let Ok(entries) = fs::read_dir(INPUT_DIR) else {
            return devices;
        };

        let mut sorted: Vec<_> = entries.flatten().collect();
        sorted.sort_by_key(|e| e.file_name());

        for entry in sorted {
            let entry_path = entry.path();
            let event_name = entry.file_name().to_string_lossy().into_owned();
            if !event_name.starts_with("event") {
                continue;
            }

            // Read device name
            let name_file = entry_path.join("device/name");
            let Ok(raw_name) = fs::read_to_string(&name_file) else {
                continue;
            };
            let name = raw_name.trim();

            if should_ignore(name) {
                continue;
            }

            // Must have EV_ABS capabilities (joystick/gamepad criterion)
            let abs_file = entry_path.join("device/capabilities/abs");
            let abs_val = fs::read_to_string(&abs_file)
                .unwrap_or_default()
                .trim()
                .to_owned();
            if abs_val.is_empty() || abs_val == "0" {
                continue;
            }

            // Resolve real sysfs path
            let real_path = match fs::canonicalize(entry_path.join("device")) {
                Ok(p) => p,
                Err(_) => continue,
            };

            let usb_path = find_usb_path(&real_path);

            // Build InterfaceId from USB path component if available
            let interface_id = usb_path.as_ref().and_then(|p| {
                p.file_name()
                    .map(|n| InterfaceId::from(n.to_string_lossy().as_ref()))
            });

            let dedup_key = interface_id
                .as_ref()
                .map(|i| i.as_str().to_owned())
                .unwrap_or_else(|| event_name.clone());

            if seen.contains_key(&dedup_key) {
                continue;
            }
            seen.insert(dedup_key, ());

            // Read vendor/product IDs from USB device parent
            let (vendor_id, product_id) = usb_path
                .as_ref()
                .and_then(|p| p.parent())
                .map(|parent| {
                    (
                        read_id_file(parent, "idVendor"),
                        read_id_file(parent, "idProduct"),
                    )
                })
                .unwrap_or((None, None));

            // Check if device is bound to usbhid driver
            let enabled = usb_path
                .as_ref()
                .map(|p| p.join("driver").exists())
                .unwrap_or(true);

            devices.push(Device {
                event: event_name.clone(),
                dev_path: format!("/dev/input/{event_name}"),
                name: name.to_owned(),
                device_type: detect_type(name),
                autohide: should_autohide(name),
                usb_path: usb_path.map(|p| p.to_string_lossy().into_owned()),
                interface_id,
                vendor_id,
                product_id,
                enabled,
            });
        }

        devices
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_types() {
        assert_eq!(
            detect_type("VIRPIL VPC Rudder Pedals"),
            DeviceType::RudderPedals
        );
        assert_eq!(detect_type("VIRPIL VMAX Throttle"), DeviceType::Throttle);
        assert_eq!(detect_type("Constellation ALPHA-L"), DeviceType::Joystick);
        assert_eq!(detect_type("Xbox Controller"), DeviceType::Gamepad);
        assert_eq!(detect_type("Steering Wheel"), DeviceType::SteeringWheel);
        assert_eq!(detect_type("Some Unknown Device"), DeviceType::Gamepad);
    }

    #[test]
    fn autohide_rules() {
        assert!(should_autohide("GMMK Pro ISO Consumer Control"));
        assert!(should_autohide("USB Keyboard"));
        assert!(!should_autohide("VIRPIL WarBRD Stick"));
    }

    #[test]
    fn ignore_rules() {
        assert!(should_ignore("Logitech Mouse"));
        assert!(!should_ignore("Logitech G Pro (gamepad mode)"));
    }

    #[test]
    fn usb_path_detection() {
        let p = Path::new("/sys/devices/pci0000:00/0000:00:14.0/usb1/1-11/1-11.4/1-11.4.1/1-11.4.1:1.0/0003:3344:4002.0005/input/input23");
        let found = find_usb_path(p);
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.file_name().unwrap(), "1-11.4.1:1.0");
    }
}
