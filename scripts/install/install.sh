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
if ! curl -fsSL "${INSTALLER_URL}" -o /tmp/nqr-installer 2>/tmp/nqr-installer-download.err; then
    echo "Error: Failed to download installer from ${INSTALLER_URL}"
    echo ""
    echo "This could mean:"
    echo "  1. No release has been published yet"
    echo "  2. Network connectivity issue"
    echo "  3. GitHub is unavailable"
    echo ""
    echo "To build and run the installer from source:"
    echo "  git clone https://github.com/${REPO}.git"
    echo "  cd NQRust-MicroVM"
    echo "  cargo build --release -p nqr-installer"
    echo "  sudo ./target/release/nqr-installer install"
    echo ""
    if [ -f /tmp/nqr-installer-download.err ]; then
        echo "Curl error:"
        cat /tmp/nqr-installer-download.err
        rm /tmp/nqr-installer-download.err
    fi
    exit 1
fi

# Verify the file is not empty and is executable
if [ ! -s /tmp/nqr-installer ]; then
    echo "Error: Downloaded file is empty"
    rm -f /tmp/nqr-installer
    exit 1
fi

chmod +x /tmp/nqr-installer

echo "Starting installer..."
echo ""

# Run installer with all arguments passed through
exec sudo /tmp/nqr-installer install "$@"
