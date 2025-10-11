#!/usr/bin/env bash
set -euo pipefail

# Script to install Firecracker globally
echo "🔥 Installing Firecracker v1.13.1 globally..."

# Check if already installed
if command -v firecracker >/dev/null 2>&1; then
    echo "✅ Firecracker is already installed at: $(which firecracker)"
    firecracker --version
    exit 0
fi

# Check if we have the binary locally
LOCAL_BINARY="/tmp/claude/release-v1.13.1-x86_64/firecracker-v1.13.1-x86_64"
if [ ! -f "$LOCAL_BINARY" ]; then
    echo "📥 Downloading Firecracker v1.13.1..."
    mkdir -p /tmp/claude
    curl -L https://github.com/firecracker-microvm/firecracker/releases/download/v1.13.1/firecracker-v1.13.1-x86_64.tgz -o /tmp/claude/firecracker.tgz

    cd /tmp/claude
    tar -xzf firecracker.tgz
    echo "✅ Downloaded and extracted Firecracker"
fi

# Install globally with sudo
echo "🔐 Installing Firecracker to /usr/local/bin/ (requires sudo)..."
echo "You will be prompted for your password:"

sudo cp "$LOCAL_BINARY" /usr/local/bin/firecracker
sudo chmod +x /usr/local/bin/firecracker

# Verify installation
echo "✅ Firecracker installed successfully!"
echo "📍 Location: $(which firecracker)"
echo "🔍 Version: $(firecracker --version)"

# Also copy jailer if needed
JAILER_BINARY="/tmp/claude/release-v1.13.1-x86_64/jailer-v1.13.1-x86_64"
if [ -f "$JAILER_BINARY" ]; then
    echo "🛡️  Installing Jailer (Firecracker's jail utility)..."
    sudo cp "$JAILER_BINARY" /usr/local/bin/jailer
    sudo chmod +x /usr/local/bin/jailer
    echo "✅ Jailer installed at: $(which jailer)"
fi

echo ""
echo "🎉 Firecracker is now globally installed!"
echo "💡 You can now run 'firecracker' from anywhere in your system."
echo "🧹 You can now remove the local firecracker binary if present:"
echo "   rm -f ./firecracker"