use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum DeviceType {
    Joystick,
    Throttle,
    RudderPedals,
    Gamepad,
    SteeringWheel,
    Unknown,
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            DeviceType::Joystick => "Joystick",
            DeviceType::Throttle => "Throttle",
            DeviceType::RudderPedals => "Rudder Pedals",
            DeviceType::Gamepad => "Gamepad",
            DeviceType::SteeringWheel => "Steering Wheel",
            DeviceType::Unknown => "Unknown",
        };
        write!(f, "{s}")
    }
}

/// Newtype for a validated USB interface ID, e.g. "1-11.4.1:1.0".
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct InterfaceId(String);

impl InterfaceId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns true if `s` matches the sysfs USB interface ID format.
    pub fn is_valid(s: &str) -> bool {
        let mut parts = s.splitn(2, ':');
        let bus_port = match parts.next() {
            Some(p) => p,
            None => return false,
        };
        let config_iface = match parts.next() {
            Some(p) => p,
            None => return false,
        };
        let bp_ok = bus_port
            .split_once('-')
            .map(|(bus, port)| {
                !bus.is_empty()
                    && !port.is_empty()
                    && bus.chars().all(|c| c.is_ascii_digit())
                    && port.chars().all(|c| c.is_ascii_digit() || c == '.')
            })
            .unwrap_or(false);
        let ci_ok = config_iface
            .split_once('.')
            .map(|(a, b)| {
                !a.is_empty()
                    && !b.is_empty()
                    && a.chars().all(|c| c.is_ascii_digit())
                    && b.chars().all(|c| c.is_ascii_digit())
            })
            .unwrap_or(false);
        bp_ok && ci_ok
    }
}

impl fmt::Display for InterfaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for InterfaceId {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for InterfaceId {
    fn from(s: String) -> Self {
        InterfaceId(s)
    }
}

impl From<&str> for InterfaceId {
    fn from(s: &str) -> Self {
        InterfaceId(s.to_owned())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Device {
    pub event: String,
    pub dev_path: String,
    pub name: String,
    pub device_type: DeviceType,
    pub autohide: bool,
    pub usb_path: Option<String>,
    pub interface_id: Option<InterfaceId>,
    pub vendor_id: Option<String>,
    pub product_id: Option<String>,
    pub enabled: bool,
}

impl Device {
    /// The key used in state maps — USB interface if known, event node otherwise.
    pub fn iface_id(&self) -> InterfaceId {
        self.interface_id
            .clone()
            .unwrap_or_else(|| InterfaceId::from(self.event.as_str()))
    }
}

/// Per-device enabled/disabled state, keyed by interface ID.
pub type DeviceState = HashMap<InterfaceId, bool>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_interface_ids() {
        assert!(InterfaceId::is_valid("1-11.4.1:1.0"));
        assert!(InterfaceId::is_valid("1-6:1.0"));
        assert!(InterfaceId::is_valid("2-1.2.3:2.1"));
    }

    #[test]
    fn invalid_interface_ids() {
        assert!(!InterfaceId::is_valid("not-an-id"));
        assert!(!InterfaceId::is_valid("1-11.4.1"));
        assert!(!InterfaceId::is_valid(":1.0"));
        assert!(!InterfaceId::is_valid(""));
        assert!(!InterfaceId::is_valid("../../etc/passwd:1.0"));
    }

    #[test]
    fn device_type_display() {
        assert_eq!(DeviceType::RudderPedals.to_string(), "Rudder Pedals");
        assert_eq!(DeviceType::SteeringWheel.to_string(), "Steering Wheel");
        assert_eq!(DeviceType::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn device_iface_id_fallback() {
        let d = Device {
            event: "event3".into(),
            dev_path: "/dev/input/event3".into(),
            name: "Test Device".into(),
            device_type: DeviceType::Joystick,
            autohide: false,
            usb_path: None,
            interface_id: None,
            vendor_id: None,
            product_id: None,
            enabled: true,
        };
        assert_eq!(d.iface_id().as_str(), "event3");
    }
}
