#!/usr/bin/env python3
"""
toggle_device.py — enable/disable USB HID devices and persist system state.
All subcommands run as root (via pkexec) under one polkit action, so
auth_admin_keep covers both toggling and state saves — no double prompt.

Usage:
  toggle_device.py disable 1-11.4.1:1.0 1-11.4.1:1.1
  toggle_device.py enable  1-11.4.1:1.0 1-11.4.1:1.1
  toggle_device.py save-state '{"1-11.4.1:1.0": false}'
"""

import sys
import os
import re
import json

BIND_PATH   = '/sys/bus/usb/drivers/usbhid/bind'
UNBIND_PATH = '/sys/bus/usb/drivers/usbhid/unbind'
STATE_FILE  = '/var/lib/joytoggle/state.json'

_IFACE_RE = re.compile(r'^\d+-[\d.]+:\d+\.\d+$')


def validate_iface(iface_id):
    if not _IFACE_RE.match(iface_id):
        print(f"ERROR: invalid interface ID: {iface_id!r}")
        sys.exit(1)


def is_bound(iface_id):
    return os.path.exists(f'/sys/bus/usb/drivers/usbhid/{iface_id}')


def disable_device(iface_id):
    validate_iface(iface_id)
    if not is_bound(iface_id):
        print(f"Already disabled {iface_id} (skipping)")
        return
    try:
        with open(UNBIND_PATH, 'w') as f:
            f.write(iface_id)
        print(f"OK: disabled {iface_id}")
    except OSError as e:
        print(f"ERROR: could not disable {iface_id}: {e}")
        sys.exit(1)


def enable_device(iface_id):
    validate_iface(iface_id)
    if is_bound(iface_id):
        print(f"Already enabled {iface_id} (skipping)")
        return
    try:
        with open(BIND_PATH, 'w') as f:
            f.write(iface_id)
        print(f"OK: enabled {iface_id}")
    except OSError as e:
        print(f"ERROR: could not enable {iface_id}: {e}")
        sys.exit(1)


def save_state(json_str):
    try:
        state = json.loads(json_str)
    except json.JSONDecodeError as e:
        print(f"ERROR: invalid JSON: {e}")
        sys.exit(1)
    if not isinstance(state, dict):
        print("ERROR: state must be a JSON object")
        sys.exit(1)
    try:
        os.makedirs('/var/lib/joytoggle', exist_ok=True)
        with open(STATE_FILE, 'w') as f:
            json.dump(state, f, indent=2)
        print("OK: state saved")
    except OSError as e:
        print(f"ERROR: could not save state: {e}")
        sys.exit(1)


if __name__ == '__main__':
    if os.geteuid() != 0:
        print("ERROR: This script must run as root.")
        sys.exit(1)

    if len(sys.argv) < 2:
        print("Usage: toggle_device.py <enable|disable|save-state> <args...>")
        sys.exit(1)

    action = sys.argv[1].lower()

    if action == 'save-state':
        if len(sys.argv) < 3:
            print("Usage: toggle_device.py save-state '<json>'")
            sys.exit(1)
        save_state(sys.argv[2])

    elif action in ('enable', 'disable'):
        if len(sys.argv) < 3:
            print(f"Usage: toggle_device.py {action} <interface_id> [...]")
            sys.exit(1)
        for iface in sys.argv[2:]:
            if action == 'disable':
                disable_device(os.path.basename(iface))
            else:
                enable_device(os.path.basename(iface))

    else:
        print(f"ERROR: unknown action '{action}'")
        sys.exit(1)
