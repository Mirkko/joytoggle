#!/usr/bin/env python3
import json
import os
import subprocess

CONFIG_DIR  = os.path.expanduser('~/.config/joytoggle')
STATE_FILE  = os.path.join(CONFIG_DIR, 'state.json')
SYSTEM_STATE = '/var/lib/joytoggle/state.json'

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
    # Also write to system location for the boot service
    _write_system_state(state)

def _write_system_state(state: dict):
    """Write state to /var/lib/joytoggle/ via pkexec helper."""
    try:
        os.makedirs('/var/lib/joytoggle', exist_ok=True)
        with open(SYSTEM_STATE, 'w') as f:
            json.dump(state, f, indent=2)
    except PermissionError:
        # Fall back to pkexec if we can't write directly
        import tempfile, shutil
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as tmp:
            json.dump(state, tmp)
            tmp_path = tmp.name
        subprocess.run(['pkexec', 'cp', tmp_path, SYSTEM_STATE], capture_output=True)
        os.unlink(tmp_path)

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