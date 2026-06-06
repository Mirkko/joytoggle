#!/usr/bin/env python3
import json
import os
import subprocess

CONFIG_DIR   = os.path.expanduser('~/.config/joytoggle')
STATE_FILE   = os.path.join(CONFIG_DIR, 'state.json')
SYSTEM_STATE = '/var/lib/joytoggle/state.json'
TOGGLE_SCRIPT = '/usr/lib/joytoggle/toggle_device.py'


def load_state():
    if not os.path.exists(STATE_FILE):
        return {}
    try:
        with open(STATE_FILE) as f:
            return json.load(f)
    except (json.JSONDecodeError, OSError):
        return {}


def save_state(state: dict):
    os.makedirs(CONFIG_DIR, exist_ok=True)
    with open(STATE_FILE, 'w') as f:
        json.dump(state, f, indent=2)
    _write_system_state(state)


def _write_system_state(state: dict):
    """Sync state to /var/lib/joytoggle/ for the boot restore service.

    Tries a direct write first (works when install pre-created the file as
    user-owned). Falls back to pkexec toggle_device.py save-state — same
    polkit action as device toggling, so auth_admin_keep covers it and no
    second password prompt appears on KDE or GNOME.
    """
    try:
        os.makedirs('/var/lib/joytoggle', exist_ok=True)
        with open(SYSTEM_STATE, 'w') as f:
            json.dump(state, f, indent=2)
    except PermissionError:
        json_str = json.dumps(state)
        result = subprocess.run(
            ['pkexec', TOGGLE_SCRIPT, 'save-state', json_str],
            capture_output=True, text=True
        )
        if result.returncode != 0:
            print(f"Warning: could not persist system state: {result.stderr.strip()}")


def load_hidden():
    hidden_file = os.path.join(CONFIG_DIR, 'hidden.json')
    if not os.path.exists(hidden_file):
        return set()
    try:
        with open(hidden_file) as f:
            return set(json.load(f))
    except (json.JSONDecodeError, OSError):
        return set()


def save_hidden(hidden: set):
    hidden_file = os.path.join(CONFIG_DIR, 'hidden.json')
    os.makedirs(CONFIG_DIR, exist_ok=True)
    with open(hidden_file, 'w') as f:
        json.dump(list(hidden), f, indent=2)


def load_shown():
    shown_file = os.path.join(CONFIG_DIR, 'shown.json')
    if not os.path.exists(shown_file):
        return set()
    try:
        with open(shown_file) as f:
            return set(json.load(f))
    except (json.JSONDecodeError, OSError):
        return set()


def save_shown(shown: set):
    shown_file = os.path.join(CONFIG_DIR, 'shown.json')
    os.makedirs(CONFIG_DIR, exist_ok=True)
    with open(shown_file, 'w') as f:
        json.dump(list(shown), f, indent=2)
