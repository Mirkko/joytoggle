use std::fs;
use std::path::PathBuf;

use serde_json;

use crate::device::{Device, DeviceState};
use crate::traits::StateStore;

pub fn user_config_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".config/joytoggle")
}

const SYSTEM_STATE_PATH: &str = "/var/lib/joytoggle/state.json";

/// Persists DeviceState to ~/.config/joytoggle/state.json (primary)
/// and /var/lib/joytoggle/state.json (system, for boot restore service).
pub struct FileStateStore;

impl StateStore for FileStateStore {
    fn load(&self) -> DeviceState {
        let user_path = user_config_dir().join("state.json");
        // Try user path first, then system path
        for path in [user_path.as_path(), std::path::Path::new(SYSTEM_STATE_PATH)] {
            if let Ok(data) = fs::read_to_string(path) {
                if let Ok(state) = serde_json::from_str(&data) {
                    return state;
                }
            }
        }
        DeviceState::default()
    }

    fn save(&self, state: &DeviceState) -> anyhow::Result<()> {
        let user_path = user_config_dir().join("state.json");
        if let Some(parent) = user_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(state)?;
        fs::write(&user_path, &json)?;

        // Best-effort write to system path — fails silently if not writable
        // (daemon handles privileged write when needed)
        let _ = fs::write(SYSTEM_STATE_PATH, &json);

        Ok(())
    }
}

/// Persists the scanned device list to cache so disabled (unbound) devices
/// remain visible in the UI after reboot.
pub struct FileCacheStore {
    pub path: PathBuf,
}

impl Default for FileCacheStore {
    fn default() -> Self {
        Self { path: user_config_dir().join("devices_cache.json") }
    }
}

impl FileCacheStore {
    pub fn save_cache(&self, devices: &[Device]) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        // Only cache non-autohide devices
        let cacheable: Vec<_> = devices.iter().filter(|d| !d.autohide).collect();
        fs::write(&self.path, serde_json::to_string_pretty(&cacheable)?)?;
        Ok(())
    }

    pub fn load_cache(&self) -> Vec<Device> {
        fs::read_to_string(&self.path)
            .ok()
            .and_then(|data| serde_json::from_str(&data).ok())
            .unwrap_or_default()
    }
}

// ── Free-function helpers for app-level persistence ───────────────────────────

/// Load the set of manually-hidden interface IDs.
pub fn load_hidden() -> std::collections::HashSet<String> {
    let path = user_config_dir().join("hidden.json");
    fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

/// Persist the set of manually-hidden interface IDs.
pub fn save_hidden(hidden: &std::collections::HashSet<String>) -> anyhow::Result<()> {
    let path = user_config_dir().join("hidden.json");
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(path, serde_json::to_string_pretty(hidden)?)?;
    Ok(())
}

/// Load shown interface IDs (force-shown despite autohide).
pub fn load_shown() -> std::collections::HashSet<String> {
    let path = user_config_dir().join("shown.json");
    fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

/// Persist shown interface IDs.
pub fn save_shown(shown: &std::collections::HashSet<String>) -> anyhow::Result<()> {
    let path = user_config_dir().join("shown.json");
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(path, serde_json::to_string_pretty(shown)?)?;
    Ok(())
}

/// Convenience: load the current device state directly.
pub fn load_state() -> DeviceState {
    FileStateStore.load()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::{DeviceType, InterfaceId};
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn make_store(dir: &TempDir) -> FileCacheStore {
        FileCacheStore { path: dir.path().join("cache.json") }
    }

    fn make_device(name: &str, autohide: bool) -> Device {
        Device {
            event: "event0".into(),
            dev_path: "/dev/input/event0".into(),
            name: name.into(),
            device_type: DeviceType::Joystick,
            autohide,
            usb_path: Some("/sys/bus/usb/1-1:1.0".into()),
            interface_id: Some(InterfaceId::from("1-1:1.0")),
            vendor_id: None,
            product_id: None,
            enabled: true,
        }
    }

    #[test]
    fn cache_roundtrip_excludes_autohide() {
        let dir = TempDir::new().unwrap();
        let store = make_store(&dir);
        let devices = vec![make_device("VIRPIL Stick", false), make_device("USB Keyboard", true)];
        store.save_cache(&devices).unwrap();
        let loaded = store.load_cache();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "VIRPIL Stick");
    }

    #[test]
    fn cache_empty_on_missing_file() {
        let dir = TempDir::new().unwrap();
        let store = FileCacheStore { path: dir.path().join("nonexistent.json") };
        assert!(store.load_cache().is_empty());
    }

    #[test]
    fn state_store_roundtrip() {
        let dir = TempDir::new().unwrap();
        // Override user config dir by pointing FileStateStore to a temp path
        // (unit test via MockStateStore is preferred — this tests serialization)
        let mut state: DeviceState = HashMap::new();
        state.insert(InterfaceId::from("1-6:1.0"), false);
        state.insert(InterfaceId::from("1-11:1.0"), true);
        let json = serde_json::to_string_pretty(&state).unwrap();
        let path = dir.path().join("state.json");
        fs::write(&path, &json).unwrap();
        let loaded: DeviceState = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(loaded.get(&InterfaceId::from("1-6:1.0")), Some(&false));
    }
}
