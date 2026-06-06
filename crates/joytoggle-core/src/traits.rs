use std::collections::HashSet;
use std::sync::Mutex;

use crate::device::{Device, DeviceState, InterfaceId};

pub trait SysfsReader: Send + Sync {
    fn read_devices(&self) -> Vec<Device>;
}

pub trait DeviceToggler: Send + Sync {
    fn enable(&self, iface_id: &InterfaceId) -> anyhow::Result<()>;
    fn disable(&self, iface_id: &InterfaceId) -> anyhow::Result<()>;
}

pub trait StateStore: Send + Sync {
    fn load(&self) -> DeviceState;
    fn save(&self, state: &DeviceState) -> anyhow::Result<()>;
}

// ── Mocks — available to all crates for testing ──────────────────────────────

#[derive(Debug, Default)]
pub struct MockSysfsReader {
    pub devices: Vec<Device>,
}

impl SysfsReader for MockSysfsReader {
    fn read_devices(&self) -> Vec<Device> {
        self.devices.clone()
    }
}

#[derive(Debug)]
pub struct MockDeviceToggler {
    pub enabled: Mutex<HashSet<InterfaceId>>,
}

impl MockDeviceToggler {
    pub fn new(initially_enabled: impl IntoIterator<Item = InterfaceId>) -> Self {
        Self { enabled: Mutex::new(initially_enabled.into_iter().collect()) }
    }

    pub fn is_enabled(&self, iface_id: &InterfaceId) -> bool {
        self.enabled.lock().unwrap().contains(iface_id)
    }
}

impl DeviceToggler for MockDeviceToggler {
    fn enable(&self, iface_id: &InterfaceId) -> anyhow::Result<()> {
        self.enabled.lock().unwrap().insert(iface_id.clone());
        Ok(())
    }

    fn disable(&self, iface_id: &InterfaceId) -> anyhow::Result<()> {
        self.enabled.lock().unwrap().remove(iface_id);
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct MockStateStore {
    pub state: Mutex<DeviceState>,
}

impl MockStateStore {
    pub fn new(initial: DeviceState) -> Self {
        Self { state: Mutex::new(initial) }
    }
}

impl StateStore for MockStateStore {
    fn load(&self) -> DeviceState {
        self.state.lock().unwrap().clone()
    }

    fn save(&self, state: &DeviceState) -> anyhow::Result<()> {
        *self.state.lock().unwrap() = state.clone();
        Ok(())
    }
}
