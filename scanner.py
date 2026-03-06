#!/usr/bin/env python3
import os
import re
import json

CONFIG_DIR   = os.path.expanduser('~/.config/joytoggle')
DEVICES_CACHE = os.path.join(CONFIG_DIR, 'devices_cache.json')

TYPE_RULES = [
    (r'pedal|rudder|torq',                         'Rudder Pedals'),
    (r'throttle|mongoose|throttlem|vmax',           'Throttle'),
    (r'joystick|alpha|constellation|stick|warbrd',  'Joystick'),
    (r'gamepad|xbox|playstation|dualshock|dualsense|logitech f', 'Gamepad'),
    (r'wheel|steering',                             'Steering Wheel'),
]

AUTOHIDE_RULES = [
    r'keyboard',
    r'volume|media|consumer control|system control',
]

IGNORE_RULES = [
    r'mouse',
]

def detect_type(name):
    name_lower = name.lower()
    for pattern, label in TYPE_RULES:
        if re.search(pattern, name_lower):
            return label
    return 'Gamepad'

def should_autohide(name):
    name_lower = name.lower()
    for pattern in AUTOHIDE_RULES:
        if re.search(pattern, name_lower):
            return True
    return False

def should_ignore(name):
    name_lower = name.lower()
    for pattern in IGNORE_RULES:
        if re.search(pattern, name_lower):
            return True
    return False

def get_devices():
    devices = []
    input_dir = '/sys/class/input'

    if not os.path.exists(input_dir):
        return devices

    for entry in sorted(os.listdir(input_dir)):
        if not entry.startswith('event'):
            continue

        event_path = os.path.join(input_dir, entry)
        name_file  = os.path.join(event_path, 'device', 'name')
        if not os.path.exists(name_file):
            continue

        with open(name_file) as f:
            name = f.read().strip()

        if should_ignore(name):
            continue

        caps_file = os.path.join(event_path, 'device', 'capabilities', 'abs')
        if not os.path.exists(caps_file):
            continue
        with open(caps_file) as f:
            if f.read().strip() == '0':
                continue

        device_link = os.path.join(event_path, 'device')
        try:
            real_path = os.path.realpath(device_link)
        except Exception:
            real_path = ''

        usb_path = ''
        for i, part in enumerate(real_path.split('/')):
            if re.match(r'^\d+-[\d.]+:\d+\.\d+$', part):
                usb_path = '/'.join(real_path.split('/')[:i+1])
                break

        vendor_id, product_id = '', ''
        if usb_path:
            for fname, attr in [('idVendor', 'vendor_id'), ('idProduct', 'product_id')]:
                fpath = os.path.join(usb_path, '..', fname)
                if os.path.exists(fpath):
                    with open(fpath) as f:
                        val = f.read().strip()
                    if attr == 'vendor_id':
                        vendor_id = val
                    else:
                        product_id = val

        devices.append({
            'event':      entry,
            'dev_path':   f'/dev/input/{entry}',
            'name':       name,
            'type':       detect_type(name),
            'autohide':   should_autohide(name),
            'usb_path':   usb_path,
            'vendor_id':  vendor_id,
            'product_id': product_id,
            'enabled':    True,
        })

    return devices


def save_devices_cache(devices):
    """Save the current device list so we can show disabled ones after reboot."""
    os.makedirs(CONFIG_DIR, exist_ok=True)
    # Only cache non-autohide devices — no point caching the mouse/keyboard junk
    cacheable = [d for d in devices if not d['autohide']]
    with open(DEVICES_CACHE, 'w') as f:
        json.dump(cacheable, f, indent=2)


def load_devices_cache():
    if not os.path.exists(DEVICES_CACHE):
        return []
    try:
        with open(DEVICES_CACHE) as f:
            return json.load(f)
    except (json.JSONDecodeError, OSError):
        return []


def get_devices_with_cache():
    """
    Returns live devices merged with cached ones.
    Devices that are cached but not live are shown as disabled
    (they've been unbound from the kernel).
    """
    live    = get_devices()
    cached  = load_devices_cache()

    live_usb_paths = {d['usb_path'] for d in live if d['usb_path']}

    # Add cached devices that are no longer visible (i.e. disabled)
    for cached_dev in cached:
        if cached_dev['usb_path'] not in live_usb_paths:
            cached_dev = dict(cached_dev)   # copy
            cached_dev['enabled'] = False
            cached_dev['event']   = cached_dev.get('event', '?')
            cached_dev['dev_path'] = cached_dev.get('dev_path', '?')
            live.append(cached_dev)

    # Save the live (currently plugged in) devices to cache
    if live:
        save_devices_cache([d for d in live if d['usb_path'] in live_usb_paths])

    return live


if __name__ == '__main__':
    print("Scanning for input devices...\n")
    devices = get_devices_with_cache()
    if not devices:
        print("No devices found.")
    else:
        for d in devices:
            status = 'ENABLED' if d['enabled'] else 'DISABLED'
            hidden = ' [AUTO-HIDE]' if d['autohide'] else ''
            print(f"  {d['dev_path']}  |  {d['type']:<16} |  [{status}]  {d['name']}{hidden}")
    print(f"\nTotal: {len(devices)} device(s)")