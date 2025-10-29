#!/bin/bash
set -e

# Get the project root directory (parent of scripts/)
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "=========================================="
echo "Rebuilding Runtime Images with Guest Agent"
echo "=========================================="
echo ""
echo "Project root: $PROJECT_ROOT"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "❌ ERROR: This script must be run as root (or with sudo)"
    exit 1
fi

# Check if guest-agent binary exists
GUEST_AGENT_PATH="$PROJECT_ROOT/target/x86_64-unknown-linux-musl/release/guest-agent"
if [ ! -f "$GUEST_AGENT_PATH" ]; then
    echo "❌ ERROR: Guest-agent binary not found at $GUEST_AGENT_PATH"
    echo ""
    echo "Please build it first:"
    echo "  cargo build --release --target x86_64-unknown-linux-musl -p guest-agent"
    exit 1
fi

echo "✅ Found guest-agent binary at $GUEST_AGENT_PATH"
echo "   Size: $(du -h "$GUEST_AGENT_PATH" | cut -f1)"
echo ""

# Export the path so child scripts can use it
export GUEST_AGENT_BINARY="$GUEST_AGENT_PATH"

# Change to project root for script execution
cd "$PROJECT_ROOT"

# Rebuild Node runtime
echo "=========================================="
echo "Rebuilding Node.js Runtime"
echo "=========================================="
"$PROJECT_ROOT/scripts/runtime-images/build-node-runtime.sh"

echo ""
echo "=========================================="
echo "Rebuilding Container Runtime"
echo "=========================================="
"$PROJECT_ROOT/scripts/build-container-runtime-v2.sh"

echo ""
echo "=========================================="
echo "✅ All runtime images rebuilt successfully!"
echo "=========================================="
echo ""
echo "Images created:"
ls -lh /srv/images/node-runtime.ext4 /srv/images/container-runtime.ext4 2>/dev/null || echo "Warning: Could not list images"

echo ""
echo "Next steps:"
echo "  1. Migrate to btrfs: sudo $PROJECT_ROOT/scripts/migrate-to-btrfs.sh"
echo "  2. Restart your manager"
echo "  3. Test with a function or container"
