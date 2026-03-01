#!/usr/bin/env bash
# JoyToggle uninstall script

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

info()    { echo -e "${BLUE}==>${NC} $1"; }
success() { echo -e "${GREEN}✓${NC} $1"; }

echo ""
echo "  🕹️  JoyToggle Uninstaller"
echo "  ───────────────────────────"
echo ""

# Re-enable all devices before uninstalling
info "Re-enabling all devices before uninstall..."
STATE_FILE="/var/lib/joytoggle/state.json"
if [ -f "$STATE_FILE" ]; then
    python3 /usr/lib/joytoggle/restore_state.py --enable-all 2>/dev/null || true
fi

info "Stopping and disabling systemd service..."
sudo systemctl stop    joytoggle.service 2>/dev/null || true
sudo systemctl disable joytoggle.service 2>/dev/null || true
sudo rm -f /etc/systemd/system/joytoggle.service
sudo systemctl daemon-reload
success "systemd service removed"

info "Removing app files..."
sudo rm -rf /usr/lib/joytoggle
success "App files removed"

info "Removing polkit policy..."
sudo rm -f /usr/share/polkit-1/actions/org.joytoggle.policy
success "polkit policy removed"

info "Removing app launcher..."
sudo rm -f /usr/share/applications/joytoggle.desktop
success "Launcher removed"

info "Removing system state..."
sudo rm -rf /var/lib/joytoggle
success "System state removed"

echo ""
read -p "  Remove your personal settings (~/.config/joytoggle)? [y/N] " -n 1 -r
echo ""
if [[ $REPLY =~ ^[Yy]$ ]]; then
    rm -rf ~/.config/joytoggle
    success "Personal settings removed"
fi

echo ""
echo -e "  ${GREEN}✓ JoyToggle uninstalled cleanly.${NC}"
echo ""
