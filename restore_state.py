#!/usr/bin/env python3
"""
restore_state.py — called at boot by systemd to re-apply saved device states.
Must run as root.
"""
import sys
import os
import json

sys.path.insert(0, '/usr/lib/joytoggle')
from toggle_device import disable_device, enable_device

STATE_FILE = '/var/lib/joytoggle/state.json'

def get_all_interfaces(usb_sysfs_path):
    if not os.path.exists(usb_sysfs_path):
        return []
    parent        = os.path.dirname(usb_sysfs_path)
    device_prefix = os.path.basename(usb_sysfs_path).split(':')[0]
    try:
        return sorted(e for e in os.listdir(parent) if e.startswith(device_prefix + ':'))
    except OSError:
        return []

def find_usb_path(interface_id):
    """Search sysfs for a given interface ID."""
    base = '/sys/bus/usb/drivers/usbhid'
    candidate = os.path.join(base, interface_id)
    if os.path.exists(candidate):
        return candidate
    # Also check unbound devices
    for root, dirs, _ in os.walk('/sys/devices'):
        for d in dirs:
            if d == interface_id:
                return os.path.join(root, d)
    return None

if __name__ == '__main__':
    if os.geteuid() != 0:
        print("ERROR: must run as root")
        sys.exit(1)

    if not os.path.exists(STATE_FILE):
        print("No state file found, nothing to restore.")
        sys.exit(0)

    with open(STATE_FILE) as f:
        state = json.load(f)

    for iface_id, enabled in state.items():
        if not enabled:
            print(f"Disabling {iface_id}...")
            disable_device(iface_id)
        else:
            print(f"Enabling {iface_id}...")
            enable_device(iface_id)