#!/usr/bin/env python3
"""
toggle_device.py — enable or disable USB HID devices by unbinding/binding driver.
Accepts multiple interface IDs in one call to minimise pkexec password prompts.

Usage:
  python toggle_device.py disable 1-11.4.1:1.0 1-11.4.1:1.1
  python toggle_device.py enable  1-11.4.1:1.0 1-11.4.1:1.1
"""

import sys
import os

BIND_PATH   = '/sys/bus/usb/drivers/usbhid/bind'
UNBIND_PATH = '/sys/bus/usb/drivers/usbhid/unbind'

def get_interface_id(path):
    return os.path.basename(path)

def disable_device(interface_id):
    try:
        with open(UNBIND_PATH, 'w') as f:
            f.write(interface_id)
        print(f"OK: disabled {interface_id}")
    except OSError as e:
        print(f"ERROR: could not disable {interface_id}: {e}")
        sys.exit(1)

def enable_device(interface_id):
    try:
        with open(BIND_PATH, 'w') as f:
            f.write(interface_id)
        print(f"OK: enabled {interface_id}")
    except OSError as e:
        print(f"ERROR: could not enable {interface_id}: {e}")
        sys.exit(1)

def is_currently_enabled(usb_sysfs_path):
    driver_link = os.path.join(usb_sysfs_path, 'driver')
    return os.path.exists(driver_link)

if __name__ == '__main__':
    if os.geteuid() != 0:
        print("ERROR: This script must run as root.")
        sys.exit(1)

    if len(sys.argv) < 3:
        print("Usage: toggle_device.py <enable|disable> <interface_id> [interface_id ...]")
        sys.exit(1)

    action     = sys.argv[1].lower()
    interfaces = [get_interface_id(p) for p in sys.argv[2:]]

    for iface in interfaces:
        if action == 'disable':
            disable_device(iface)
        elif action == 'enable':
            enable_device(iface)
        else:
            print(f"ERROR: unknown action '{action}'")
            sys.exit(1)