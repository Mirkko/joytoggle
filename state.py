#!/usr/bin/env python3
"""
state.py — saves and loads device state to ~/.config/joytoggle/state.json
"""

import json
import os

CONFIG_DIR  = os.path.expanduser('~/.config/joytoggle')
STATE_FILE  = os.path.join(CONFIG_DIR, 'state.json')

def load_state():
    """
    Returns a dict of  { interface_id: bool }  e.g. { "1-11.4.1:1.0": False }
    Empty dict if no state file exists yet.
    """
    if not os.path.exists(STATE_FILE):
        return {}
    try:
        with open(STATE_FILE, 'r') as f:
            return json.load(f)
    except (json.JSONDecodeError, OSError):
        return {}

def save_state(state: dict):
    """
    state is a dict of { interface_id: bool }
    """
    os.makedirs(CONFIG_DIR, exist_ok=True)
    with open(STATE_FILE, 'w') as f:
        json.dump(state, f, indent=2)

def load_hidden():
    """
    Returns a set of interface IDs the user has chosen to hide.
    """
    hidden_file = os.path.join(CONFIG_DIR, 'hidden.json')
    if not os.path.exists(hidden_file):
        return set()
    try:
        with open(hidden_file, 'r') as f:
            return set(json.load(f))
    except (json.JSONDecodeError, OSError):
        return set()

def save_hidden(hidden: set):
    hidden_file = os.path.join(CONFIG_DIR, 'hidden.json')
    os.makedirs(CONFIG_DIR, exist_ok=True)
    with open(hidden_file, 'w') as f:
        json.dump(list(hidden), f, indent=2)