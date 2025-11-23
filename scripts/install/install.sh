#!/usr/bin/env bash
# NQR-MicroVM Quick Installer
# Downloads and runs the Rust TUI installer

set -e

REPO="NexusQuantum/NQRust-MicroVM"
INSTALLER_URL="https://github.com/${REPO}/releases/latest/download/nqr-installer-x86_64-linux-musl"

echo "╔════════════════════════════════════════════════╗"
echo "║  NQR-MicroVM Installer                         ║"
echo "║  Powered by Rust + Ratatui TUI                 ║"
echo "╚════════════════════════════════════════════════╝"
echo ""
echo "Downloading NQR-MicroVM installer..."

# Download installer to /tmp
curl -fsSL "${INSTALLER_URL}" -o /tmp/nqr-installer
chmod +x /tmp/nqr-installer

echo "Starting installer..."
echo ""

# Run installer with all arguments passed through
exec sudo /tmp/nqr-installer install "$@"
