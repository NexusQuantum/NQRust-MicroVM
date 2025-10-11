#!/usr/bin/env bash
set -euo pipefail

# NQRust-MicroVM Setup Script
# This script sets up the development environment for NQRust-MicroVM

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

echo "🚀 Setting up NQRust-MicroVM development environment..."

# Check prerequisites
echo "📋 Checking prerequisites..."
command -v cargo >/dev/null 2>&1 || { echo "❌ Rust/Cargo not found. Please install Rust first."; exit 1; }
command -v docker >/dev/null 2>&1 || { echo "❌ Docker not found. Please install Docker first."; exit 1; }
echo "✅ Prerequisites check passed"

# Download and install Firecracker if not present
if ! command -v firecracker >/dev/null 2>&1 && [ ! -f "$PROJECT_ROOT/firecracker" ]; then
    echo "📥 Downloading Firecracker v1.13.1..."
    TEMP_DIR=$(mktemp -d)
    curl -L https://github.com/firecracker-microvm/firecracker/releases/download/v1.13.1/firecracker-v1.13.1-x86_64.tgz -o "$TEMP_DIR/firecracker.tgz"

    cd "$TEMP_DIR"
    tar -xzf firecracker.tgz

    # Try to install globally, fallback to local
    if sudo cp release-v1.13.1-x86_64/firecracker-v1.13.1-x86_64 /usr/local/bin/firecracker 2>/dev/null && sudo chmod +x /usr/local/bin/firecracker 2>/dev/null; then
        echo "✅ Firecracker installed globally at /usr/local/bin/firecracker"
    else
        echo "⚠️  Could not install globally, installing locally..."
        cp release-v1.13.1-x86_64/firecracker-v1.13.1-x86_64 "$PROJECT_ROOT/firecracker"
        chmod +x "$PROJECT_ROOT/firecracker"
        echo "✅ Firecracker installed locally at $PROJECT_ROOT/firecracker"
        echo "💡 Add '$PROJECT_ROOT' to your PATH or run 'export PATH=\"\$PWD:\$PATH\"' before starting services"
    fi

    cd "$PROJECT_ROOT"
    rm -rf "$TEMP_DIR"
else
    echo "✅ Firecracker already available"
fi

# Fix docker-compose file if needed
echo "🔧 Fixing docker-compose.dev.yml formatting..."
if ! grep -q "  postgres:" infra/docker-compose.dev.yml; then
    cat > infra/docker-compose.dev.yml << 'EOF'
version: "3.8"
services:
  postgres:
    image: postgres:16
    environment:
      POSTGRES_USER: nexus
      POSTGRES_PASSWORD: nexus
      POSTGRES_DB: nexus
    ports: ["5432:5432"]
EOF
    echo "✅ Fixed docker-compose.dev.yml formatting"
fi

# Start PostgreSQL
echo "🐘 Starting PostgreSQL..."
chmod +x scripts/dev-up.sh
./scripts/dev-up.sh

# Set up environment file
if [ ! -f .env ]; then
    echo "📝 Creating .env file..."
    cp .env.example .env
    echo "✅ Created .env file from .env.example"
else
    echo "✅ .env file already exists"
fi

# Create FC runtime directory
FC_RUN_DIR=$(grep FC_RUN_DIR .env | cut -d'=' -f2)
if [ ! -d "$FC_RUN_DIR" ]; then
    echo "📁 Creating Firecracker runtime directory..."
    if sudo mkdir -p "$FC_RUN_DIR" 2>/dev/null && sudo chown "$USER:$USER" "$FC_RUN_DIR" 2>/dev/null; then
        echo "✅ Created $FC_RUN_DIR with proper permissions"
    else
        echo "⚠️  Could not create $FC_RUN_DIR with sudo, trying user directory..."
        USER_FC_DIR="$HOME/fc-runtime"
        mkdir -p "$USER_FC_DIR"
        sed -i "s|^FC_RUN_DIR=.*|FC_RUN_DIR=$USER_FC_DIR|" .env
        echo "✅ Created $USER_FC_DIR and updated .env"
    fi
else
    echo "✅ Firecracker runtime directory already exists"
fi

# Build the project
echo "🔨 Building the project..."
cargo build
echo "✅ Project built successfully"

echo ""
echo "🎉 Setup complete! Next steps:"
echo ""
echo "1. Set up network bridge (requires sudo):"
echo "   sudo ./scripts/fc-bridge-setup.sh fcbr0 <your-network-interface>"
echo "   (Find your interface with: ip link show)"
echo ""
echo "2. Start the services:"
echo "   Terminal 1: cd apps/agent && cargo run"
echo "   Terminal 2: cd apps/manager && cargo run"
echo ""
echo "3. Test with a VM creation (requires kernel/rootfs files):"
echo "   curl -X POST http://127.0.0.1:8080/v1/vms \\"
echo "     -H 'content-type: application/json' \\"
echo "     -d '{\"name\":\"test\",\"vcpu\":1,\"mem_mib\":256,\"kernel_path\":\"/path/to/kernel\",\"rootfs_path\":\"/path/to/rootfs\"}'"
echo ""
echo "📖 See SETUP.md for detailed instructions and troubleshooting."