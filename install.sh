#!/usr/bin/env bash
# JoyToggle install script
# Supports Arch, Ubuntu/Debian, Fedora

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info()    { echo -e "${BLUE}==>${NC} $1"; }
success() { echo -e "${GREEN}✓${NC} $1"; }
warn()    { echo -e "${YELLOW}!${NC} $1"; }
error()   { echo -e "${RED}✗${NC} $1"; exit 1; }

echo ""
echo "  🕹️  JoyToggle Installer"
echo "  ─────────────────────────"
echo ""

# ── Check we're not running as root ───────────────────────────
if [ "$EUID" -eq 0 ]; then
    error "Do not run this script as root. It will ask for sudo when needed."
fi

# ── Detect distro and install dependencies ────────────────────
info "Checking dependencies..."

install_deps_arch() {
    sudo pacman -S --needed --noconfirm python python-gobject gtk4 libadwaita
}

install_deps_debian() {
    sudo apt-get update -qq
    sudo apt-get install -y python3 python3-gi gir1.2-gtk-4.0 gir1.2-adw-1 libadwaita-1-0
}

install_deps_fedora() {
    sudo dnf install -y python3 python3-gobject gtk4 libadwaita
}

if command -v pacman &>/dev/null; then
    info "Detected Arch Linux — installing dependencies..."
    install_deps_arch
elif command -v apt-get &>/dev/null; then
    info "Detected Debian/Ubuntu — installing dependencies..."
    install_deps_debian
elif command -v dnf &>/dev/null; then
    info "Detected Fedora — installing dependencies..."
    install_deps_fedora
else
    warn "Could not detect package manager. Make sure these are installed:"
    warn "  python3, python-gobject, gtk4, libadwaita, polkit"
fi

success "Dependencies ready"

# ── Check python3 is available ────────────────────────────────
PYTHON=$(command -v python3 || command -v python || error "Python 3 not found")
info "Using Python: $PYTHON"

# ── Install app files ─────────────────────────────────────────
info "Installing app files to /usr/lib/joytoggle..."
sudo mkdir -p /usr/lib/joytoggle
sudo cp app.py          /usr/lib/joytoggle/app.py
sudo cp scanner.py      /usr/lib/joytoggle/scanner.py
sudo cp state.py        /usr/lib/joytoggle/state.py
sudo cp toggle_device.py /usr/lib/joytoggle/toggle_device.py
sudo cp restore_state.py /usr/lib/joytoggle/restore_state.py
sudo chmod +x /usr/lib/joytoggle/toggle_device.py
sudo chmod +x /usr/lib/joytoggle/restore_state.py
success "App files installed"

# ── Create system state directory ─────────────────────────────
info "Creating system state directory..."
sudo mkdir -p /var/lib/joytoggle
sudo chown $USER:$USER /var/lib/joytoggle
success "State directory ready"

# ── Install polkit policy ─────────────────────────────────────
info "Installing polkit policy..."
sudo mkdir -p /usr/share/polkit-1/actions
sudo cp org.joytoggle.policy /usr/share/polkit-1/actions/org.joytoggle.policy
success "polkit policy installed"

# ── Install systemd service ───────────────────────────────────
info "Installing systemd boot service..."
sudo bash -c "cat > /etc/systemd/system/joytoggle.service" << 'SYSTEMD'
[Unit]
Description=JoyToggle — restore joystick device states
After=systemd-udev-settle.service
Wants=systemd-udev-settle.service

[Service]
Type=oneshot
ExecStart=/usr/bin/python3 /usr/lib/joytoggle/restore_state.py
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
SYSTEMD

sudo systemctl daemon-reload
sudo systemctl enable joytoggle.service
success "systemd service installed and enabled"

# ── Install .desktop launcher ─────────────────────────────────
info "Installing app launcher..."
sudo bash -c "cat > /usr/share/applications/joytoggle.desktop" << DESKTOP
[Desktop Entry]
Name=JoyToggle
Comment=Enable or disable joystick and sim controller devices
Exec=/usr/bin/python3 /usr/lib/joytoggle/app.py
Icon=input-gaming
Terminal=false
Type=Application
Categories=Settings;HardwareSettings;
Keywords=joystick;gamepad;controller;sim;virpil;hotas;
StartupNotify=true
DESKTOP

success "App launcher installed"

# ── Done ──────────────────────────────────────────────────────
echo ""
echo -e "  ${GREEN}✓ Installation complete!${NC}"
echo ""
echo "  Launch from your app launcher (search 'JoyToggle')"
echo "  Or run:  python3 /usr/lib/joytoggle/app.py"
echo ""
