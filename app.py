#!/usr/bin/env python3
import gi
gi.require_version('Gtk', '4.0')
gi.require_version('Adw', '1')

from gi.repository import Gtk, Adw, GLib
import subprocess
import threading
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scanner import get_devices_with_cache
from state   import load_state, save_state, load_hidden, save_hidden

TOGGLE_SCRIPT = '/usr/lib/joytoggle/toggle_device.py'

TYPE_ICONS = {
    'Joystick':       'input-gaming-symbolic',
    'Throttle':       'media-seek-forward-symbolic',
    'Rudder Pedals':  'go-down-symbolic',
    'Gamepad':        'input-gaming-symbolic',
    'Steering Wheel': 'emblem-synchronizing-symbolic',
}


# ── Helpers ───────────────────────────────────────────────────

def get_all_interfaces(usb_sysfs_path):
    """Return all interface IDs for a USB device (handles :1.0, :1.1, etc.)"""
    if not usb_sysfs_path or not os.path.exists(usb_sysfs_path):
        return [os.path.basename(usb_sysfs_path)] if usb_sysfs_path else []
    parent        = os.path.dirname(usb_sysfs_path)
    device_prefix = os.path.basename(usb_sysfs_path).split(':')[0]
    try:
        interfaces = sorted(
            e for e in os.listdir(parent)
            if e.startswith(device_prefix + ':')
        )
    except OSError:
        interfaces = []
    return interfaces or [os.path.basename(usb_sysfs_path)]


def get_iface_id(device):
    return os.path.basename(device['usb_path']) if device['usb_path'] else device['event']


def is_device_enabled(usb_sysfs_path):
    return os.path.exists(os.path.join(usb_sysfs_path, 'driver'))


def toggle_async(interface_ids, enable: bool, on_done):
    """Single pkexec call for all given interfaces. on_done(success) on main thread."""
    if not interface_ids:
        GLib.idle_add(on_done, True)
        return
    action = 'enable' if enable else 'disable'

    def worker():
        try:
            cmd    = ['pkexec', TOGGLE_SCRIPT, action] + interface_ids
            result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
            success = result.returncode == 0
            if not success:
                print(f"pkexec error: {result.stderr}")
        except Exception as e:
            print(f"Exception: {e}")
            success = False
        GLib.idle_add(on_done, success)

    threading.Thread(target=worker, daemon=True).start()


# ── Device row ────────────────────────────────────────────────

class DeviceRow(Adw.ExpanderRow):
    def __init__(self, device, on_toggle_cb, on_hide_cb):
        super().__init__()
        self.device       = device
        self.on_toggle_cb = on_toggle_cb
        self.on_hide_cb   = on_hide_cb
        self._busy        = False

        self.set_title(device['name'])
        self.set_subtitle(device['type'])

        icon = Gtk.Image.new_from_icon_name(
            TYPE_ICONS.get(device['type'], 'input-gaming-symbolic')
        )
        icon.set_pixel_size(20)
        self.add_prefix(icon)

        self.spinner = Gtk.Spinner()
        self.spinner.set_visible(False)
        self.add_suffix(self.spinner)

        self.switch = Gtk.Switch()
        self.switch.set_active(device['enabled'])
        self.switch.set_valign(Gtk.Align.CENTER)
        self.switch.connect('state-set', self._on_switch_toggled)
        self.add_suffix(self.switch)

        # If device is currently unreachable (disabled + not in sysfs) mark it
        if not device['enabled'] and not os.path.exists(device.get('usb_path', '')):
            self.set_subtitle(device['type'] + ' · disconnected from kernel')

        self._add_detail('Device path',        device['dev_path'])
        self._add_detail('USB interface',       os.path.basename(device['usb_path']) if device['usb_path'] else '—')
        self._add_detail('Vendor / Product ID', f"{device['vendor_id']}:{device['product_id']}" if device['vendor_id'] else '—')

        hide_row = Adw.ActionRow()
        hide_row.set_title('Hide this device')
        hide_row.set_subtitle('Move to the Hidden Devices section')
        hide_row.add_suffix(Gtk.Image.new_from_icon_name('view-conceal-symbolic'))
        hide_row.set_activatable(True)
        hide_row.connect('activated', lambda _: self.on_hide_cb(self.device))
        self.add_row(hide_row)

        self._refresh_style()

    def _add_detail(self, label, value):
        row = Adw.ActionRow()
        row.set_title(label)
        row.set_subtitle(value)
        self.add_row(row)

    def _on_switch_toggled(self, switch, state):
        if self._busy:
            return False
        self._set_busy(True)
        self.on_toggle_cb(self.device, state, self._on_toggle_done)
        return True

    def _on_toggle_done(self, success, new_state):
        self._set_busy(False)
        if success:
            self.device['enabled'] = new_state
            self.switch.set_state(new_state)
        else:
            self.switch.set_active(self.device['enabled'])
            self.switch.set_state(self.device['enabled'])
        self._refresh_style()

    def _set_busy(self, busy):
        self._busy = busy
        self.switch.set_sensitive(not busy)
        self.spinner.set_visible(busy)
        self.spinner.set_spinning(busy)

    def _refresh_style(self):
        if self.device['enabled']:
            self.remove_css_class('dim-label')
        else:
            self.add_css_class('dim-label')


# ── Main window ───────────────────────────────────────────────

class JoyToggleWindow(Adw.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app)
        self.set_title('Joystick Manager')
        self.set_default_size(500, -1)
        self.set_resizable(False)

        self.state   = load_state()
        self.hidden  = load_hidden()
        self.devices = []

        self._device_rows = []
        self._hidden_rows = []

        self._build_ui()
        self._load_devices()

    def _build_ui(self):
        toolbar_view = Adw.ToolbarView()
        self.set_content(toolbar_view)

        header = Adw.HeaderBar()
        toolbar_view.add_top_bar(header)

        refresh_btn = Gtk.Button.new_from_icon_name('view-refresh-symbolic')
        refresh_btn.set_tooltip_text('Refresh devices')
        refresh_btn.connect('clicked', lambda _: self._load_devices())
        header.pack_end(refresh_btn)

        scroll = Gtk.ScrolledWindow()
        scroll.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        scroll.set_propagate_natural_height(True)
        toolbar_view.set_content(scroll)

        self.outer_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self.outer_box.set_margin_top(12)
        self.outer_box.set_margin_bottom(24)
        self.outer_box.set_margin_start(16)
        self.outer_box.set_margin_end(16)
        scroll.set_child(self.outer_box)

        self.banner = Adw.Banner()
        self.banner.set_title('All devices disabled')
        self.banner.set_revealed(False)
        self.outer_box.append(self.banner)

        btn_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        btn_box.set_margin_top(12)
        btn_box.set_margin_bottom(12)
        btn_box.set_homogeneous(True)

        disable_btn = Gtk.Button(label='Disable All')
        disable_btn.add_css_class('destructive-action')
        disable_btn.connect('clicked', lambda _: self._set_all(False))

        enable_btn = Gtk.Button(label='Enable All')
        enable_btn.add_css_class('suggested-action')
        enable_btn.connect('clicked', lambda _: self._set_all(True))

        btn_box.append(disable_btn)
        btn_box.append(enable_btn)
        self.outer_box.append(btn_box)

        self.devices_group = Adw.PreferencesGroup()
        self.devices_group.set_title('Sim Controllers')
        self.devices_group.set_description('Toggle to enable or disable devices system-wide')
        self.outer_box.append(self.devices_group)

        self.hidden_group = Adw.PreferencesGroup()
        self.hidden_group.set_title('Hidden Devices')
        self.hidden_group.set_visible(False)
        self.outer_box.append(self.hidden_group)

        self.empty_label = Gtk.Label(
            label='No devices found. Plug in your controllers and hit refresh.'
        )
        self.empty_label.add_css_class('dim-label')
        self.empty_label.set_wrap(True)
        self.empty_label.set_margin_top(32)
        self.empty_label.set_margin_bottom(32)
        self.empty_label.set_visible(False)
        self.outer_box.append(self.empty_label)

    def _clear_groups(self):
        for row in self._device_rows:
            self.devices_group.remove(row)
        for row in self._hidden_rows:
            self.hidden_group.remove(row)
        self._device_rows = []
        self._hidden_rows = []

    def _load_devices(self):
        self._clear_groups()
        self.devices = []

        raw = get_devices_with_cache()

        if not raw:
            self.empty_label.set_visible(True)
            self.devices_group.set_visible(False)
            return

        self.empty_label.set_visible(False)
        self.devices_group.set_visible(True)

        hidden_devices = []

        for d in raw:
            iface_id = get_iface_id(d)

            # Saved state overrides live sysfs reading
            if iface_id in self.state:
                d['enabled'] = self.state[iface_id]
            elif d['usb_path'] and os.path.exists(d['usb_path']):
                d['enabled'] = is_device_enabled(d['usb_path'])
            # else: keep whatever get_devices_with_cache set (False for cached-only)

            self.devices.append(d)

            if d['autohide'] or iface_id in self.hidden:
                hidden_devices.append((iface_id, d))
            else:
                row = DeviceRow(d, self._on_toggle, self._on_hide)
                self.devices_group.add(row)
                self._device_rows.append(row)

        for iface_id, d in hidden_devices:
            row = self._make_hidden_row(d, iface_id)
            self.hidden_group.add(row)
            self._hidden_rows.append(row)

        self.hidden_group.set_visible(bool(hidden_devices))
        self._update_banner()

    def _make_hidden_row(self, device, iface_id):
        row = Adw.ActionRow()
        row.set_title(device['name'])
        row.set_subtitle('Auto-hidden' if device['autohide'] else 'Hidden by user')
        row.add_prefix(Gtk.Image.new_from_icon_name('view-conceal-symbolic'))
        if not device['autohide']:
            btn = Gtk.Button(label='Restore')
            btn.add_css_class('flat')
            btn.set_valign(Gtk.Align.CENTER)
            btn.connect('clicked', lambda _, d=device, i=iface_id: self._on_restore(d, i))
            row.add_suffix(btn)
        return row

    def _on_toggle(self, device, new_state, done_cb):
        iface_id   = get_iface_id(device)
        all_ifaces = get_all_interfaces(device['usb_path']) if device['usb_path'] else []

        def on_done(success):
            if success:
                self.state[iface_id] = new_state
                save_state(self.state)
                self._update_banner()
            done_cb(success, new_state)

        toggle_async(all_ifaces, new_state, on_done)

    def _on_hide(self, device):
        self.hidden.add(get_iface_id(device))
        save_hidden(self.hidden)
        self._load_devices()

    def _on_restore(self, device, iface_id):
        self.hidden.discard(iface_id)
        save_hidden(self.hidden)
        self._load_devices()

    def _set_all(self, enable: bool):
        """Collect ALL interfaces from ALL visible devices — one single pkexec call."""
        visible = [
            d for d in self.devices
            if not d['autohide'] and get_iface_id(d) not in self.hidden
        ]
        if not visible:
            return

        # Gather every interface ID across every device in one flat list
        all_ifaces = []
        for d in visible:
            if d['usb_path']:
                all_ifaces.extend(get_all_interfaces(d['usb_path']))

        if not all_ifaces:
            return

        # Disable the buttons while working
        self.outer_box.set_sensitive(False)

        def on_done(success):
            self.outer_box.set_sensitive(True)
            if success:
                for d in visible:
                    iface_id        = get_iface_id(d)
                    d['enabled']    = enable
                    self.state[iface_id] = enable
                save_state(self.state)
            self._load_devices()

        toggle_async(all_ifaces, enable, on_done)

    def _update_banner(self):
        visible = [
            d for d in self.devices
            if not d['autohide'] and get_iface_id(d) not in self.hidden
        ]
        self.banner.set_revealed(bool(visible) and all(not d['enabled'] for d in visible))


# ── Application ───────────────────────────────────────────────

class JoyToggleApp(Adw.Application):
    def __init__(self):
        super().__init__(application_id='org.joytoggle.app')
        self.connect('activate', self._on_activate)

    def _on_activate(self, app):
        JoyToggleWindow(app).present()


if __name__ == '__main__':
    JoyToggleApp().run(sys.argv)