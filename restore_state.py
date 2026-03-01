#!/usr/bin/env python3
"""
restore_state.py — called at boot by systemd to re-apply saved device states.
Must run as root.
"""
import sys
import os
import json
import time

BIND_PATH   = '/sys/bus/usb/drivers/usbhid/bind'
UNBIND_PATH = '/sys/bus/usb/drivers/usbhid/unbind'
STATE_FILE  = '/var/lib/joytoggle/state.json'

def is_bound(iface_id):
    """Check if interface is already bound to usbhid."""
    return os.path.exists(f'/sys/bus/usb/drivers/usbhid/{iface_id}')

def disable_device(iface_id):
    if not is_bound(iface_id):
        print(f"Already disabled {iface_id} (skipping)")
        return
    try:
        with open(UNBIND_PATH, 'w') as f:
            f.write(iface_id)
        print(f"Disabled {iface_id}")
    except OSError as e:
        print(f"Could not disable {iface_id}: {e} (skipping)")

def enable_device(iface_id):
    if is_bound(iface_id):
        print(f"Already enabled {iface_id} (skipping)")
        return
    try:
        with open(BIND_PATH, 'w') as f:
            f.write(iface_id)
        print(f"Enabled {iface_id}")
    except OSError as e:
        print(f"Could not enable {iface_id}: {e} (skipping)")

if __name__ == '__main__':
    if os.geteuid() != 0:
        print("ERROR: must run as root")
        sys.exit(1)

    if not os.path.exists(STATE_FILE):
        print("No state file found, nothing to restore.")
        sys.exit(0)

    time.sleep(2)

    with open(STATE_FILE) as f:
        state = json.load(f)

    for iface_id, enabled in state.items():
        if not enabled:
            disable_device(iface_id)
        else:
            enable_device(iface_id)

    sys.exit(0)