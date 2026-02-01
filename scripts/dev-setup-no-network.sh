#!/usr/bin/env bash
set -euo pipefail

# NQRust-MicroVM Development Setup (Skip Network Configuration)
# This script sets up everything except network bridge configuration

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Variables
DOCKER_SUDO=""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
info() { echo -e "${BLUE}ℹ${NC} $1"; }
success() { echo -e "${GREEN}✅${NC} $1"; }
warn() { echo -e "${YELLOW}⚠️${NC}  $1"; }
error() { echo -e "${RED}❌${NC} $1"; }
section() { echo -e "\n${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n${GREEN}$1${NC}\n${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"; }

# Banner
echo -e "${BLUE}"
cat << "EOF"
╔═══════════════════════════════════════════════╗
║   NQRust-MicroVM Development Setup            ║
║   (Network Configuration Skipped)             ║
╚═══════════════════════════════════════════════╝
EOF
echo -e "${NC}"

warn "Network bridge configuration will be SKIPPED"
info "Make sure you have already configured fcbr0 or will do it manually"
echo ""

# ============================================================================
# 1. PREREQUISITES CHECK
# ============================================================================
section "1️⃣  Checking Prerequisites"

check_command() {
    if command -v "$1" >/dev/null 2>&1; then
        success "$1 found: $(command -v "$1")"
        return 0
    else
        error "$1 not found"
        return 1
    fi
}

MISSING_DEPS=0

# Check Rust
if check_command cargo; then
    RUST_VERSION=$(rustc --version | cut -d' ' -f2)
    info "Rust version: $RUST_VERSION"
else
    error "Please install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    ((MISSING_DEPS++))
fi

# Check Docker
if check_command docker; then
    if docker ps >/dev/null 2>&1; then
        success "Docker daemon is running and accessible"
    else
        error "Docker found but daemon not accessible"

        # Check if user is in docker group
        if groups | grep -q docker; then
            warn "You're in the docker group but the session hasn't updated"
            echo ""
            echo "  Quick fix: Run 'newgrp docker' then re-run this script:"
            echo "    newgrp docker"
            echo "    ./scripts/dev-setup-no-network.sh"
            echo ""
            echo "  Or log out and back in for permanent fix"
            echo ""
        else
            warn "Add your user to the docker group:"
            echo ""
            echo "    sudo usermod -aG docker \$USER"
            echo "    newgrp docker"
            echo ""
            echo "  Then re-run this script"
            echo ""
        fi

        read -p "Try to continue with sudo? [y/N]: " USE_SUDO
        if [ "$USE_SUDO" != "y" ] && [ "$USE_SUDO" != "Y" ]; then
            error "Docker access required. Please fix Docker permissions and try again."
            exit 1
        else
            info "Will attempt to use sudo for Docker commands..."
            DOCKER_SUDO="sudo"
        fi
    fi
else
    error "Please install Docker: https://docs.docker.com/engine/install/"
    ((MISSING_DEPS++))
fi

# Check KVM
if [ -e /dev/kvm ]; then
    success "/dev/kvm exists"
    if [ -r /dev/kvm ] && [ -w /dev/kvm ]; then
        success "You have read/write access to /dev/kvm"
    else
        warn "You don't have access to /dev/kvm. You may need to add your user to the kvm group:"
        warn "  sudo usermod -aG kvm \$USER && newgrp kvm"
    fi
else
    error "/dev/kvm not found. KVM virtualization support is required."
    error "Check if virtualization is enabled in BIOS and load KVM modules:"
    error "  sudo modprobe kvm kvm_intel  # or kvm_amd for AMD"
    ((MISSING_DEPS++))
fi

# Check other tools
check_command curl || ((MISSING_DEPS++))
check_command tar || ((MISSING_DEPS++))
check_command jq || warn "jq not found (optional but recommended): sudo apt install jq"

# Check for musl-gcc (needed for guest-agent release builds)
if command -v musl-gcc >/dev/null 2>&1; then
    success "musl-gcc found (for static guest-agent builds)"
else
    warn "musl-gcc not found - guest-agent release builds will fail"
    info "Install with: sudo apt install musl-tools"
    read -p "Install musl-tools now? [Y/n]: " INSTALL_MUSL
    INSTALL_MUSL=${INSTALL_MUSL:-Y}
    if [ "$INSTALL_MUSL" = "Y" ] || [ "$INSTALL_MUSL" = "y" ]; then
        info "Installing musl-tools..."
        if sudo apt update && sudo apt install -y musl-tools 2>/dev/null; then
            success "musl-tools installed"
        else
            warn "Failed to install musl-tools. You can install it later with: sudo apt install musl-tools"
        fi
    else
        warn "Skipping musl-tools. Debug builds will still work."
    fi
fi

if [ $MISSING_DEPS -gt 0 ]; then
    error "Missing $MISSING_DEPS required dependencies. Please install them and try again."
    exit 1
fi

success "All prerequisites met!"

# ============================================================================
# 2. FIRECRACKER INSTALLATION
# ============================================================================
section "2️⃣  Installing Firecracker v1.13.1"

if command -v firecracker >/dev/null 2>&1; then
    FC_VERSION=$(firecracker --version 2>&1 | head -n1 || echo "unknown")
    success "Firecracker already installed: $FC_VERSION"
elif [ -f "$PROJECT_ROOT/firecracker" ]; then
    success "Firecracker binary found locally at $PROJECT_ROOT/firecracker"
else
    info "Downloading Firecracker v1.13.1..."
    TEMP_DIR=$(mktemp -d)

    curl -fsSL https://github.com/firecracker-microvm/firecracker/releases/download/v1.13.1/firecracker-v1.13.1-x86_64.tgz \
        -o "$TEMP_DIR/firecracker.tgz"

    cd "$TEMP_DIR"
    tar -xzf firecracker.tgz

    # Try to install globally, fallback to local
    if sudo cp release-v1.13.1-x86_64/firecracker-v1.13.1-x86_64 /usr/local/bin/firecracker 2>/dev/null && \
       sudo chmod +x /usr/local/bin/firecracker 2>/dev/null; then
        success "Firecracker installed globally at /usr/local/bin/firecracker"
    else
        warn "Could not install globally, installing locally..."
        cp release-v1.13.1-x86_64/firecracker-v1.13.1-x86_64 "$PROJECT_ROOT/firecracker"
        chmod +x "$PROJECT_ROOT/firecracker"
        success "Firecracker installed locally at $PROJECT_ROOT/firecracker"
        warn "Add '$PROJECT_ROOT' to your PATH or run 'export PATH=\"\$PWD:\$PATH\"' before starting services"
    fi

    cd "$PROJECT_ROOT"
    rm -rf "$TEMP_DIR"
fi

# ============================================================================
# 3. POSTGRESQL SETUP
# ============================================================================
section "3️⃣  Starting PostgreSQL"

info "Starting PostgreSQL container..."

DOCKER_CMD="${DOCKER_SUDO:-} docker"

if $DOCKER_CMD ps 2>/dev/null | grep -q postgres; then
    success "PostgreSQL container already running"
else
    # Make sure docker-compose.dev.yml exists and is properly formatted
    if [ ! -f "infra/docker-compose.dev.yml" ]; then
        info "Creating docker-compose.dev.yml..."
        mkdir -p infra
        cat > infra/docker-compose.dev.yml << 'COMPOSE_EOF'
services:
  postgres:
    image: postgres:16
    environment:
      POSTGRES_USER: nexus
      POSTGRES_PASSWORD: nexus
      POSTGRES_DB: nexus
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data

volumes:
  postgres_data:
COMPOSE_EOF
        success "Created docker-compose.dev.yml"
    fi

    # Start PostgreSQL using docker-compose
    info "Starting PostgreSQL with docker-compose..."
    if [ -n "${DOCKER_SUDO:-}" ]; then
        sudo docker compose -f infra/docker-compose.dev.yml up -d
    else
        docker compose -f infra/docker-compose.dev.yml up -d
    fi

    # Wait for PostgreSQL to be ready
    info "Waiting for PostgreSQL to be ready..."
    sleep 3

    for i in {1..30}; do
        POSTGRES_CONTAINER=$($DOCKER_CMD ps -q -f ancestor=postgres:16)
        if [ -n "$POSTGRES_CONTAINER" ] && $DOCKER_CMD exec "$POSTGRES_CONTAINER" pg_isready -U nexus >/dev/null 2>&1; then
            success "PostgreSQL is ready"
            break
        fi
        if [ $i -eq 30 ]; then
            warn "PostgreSQL might not be ready yet, but continuing..."
        fi
        sleep 1
    done
fi

# ============================================================================
# 4. ENVIRONMENT CONFIGURATION
# ============================================================================
section "4️⃣  Configuring Environment"

if [ ! -f .env ]; then
    info "Creating .env file from .env.example..."
    if [ -f .env.example ]; then
        cp .env.example .env
        success "Created .env file"
    else
        info "Creating default .env file..."
        cat > .env << 'ENV_EOF'
# Manager
DATABASE_URL=postgres://nexus:nexus@localhost:5432/nexus
MANAGER_BIND=127.0.0.1:18080
MANAGER_IMAGE_ROOT=/srv/images
MANAGER_STORAGE_ROOT=/srv/fc/vms
MANAGER_ALLOW_IMAGE_PATHS=true

# Agent
AGENT_BIND=127.0.0.1:9090
MANAGER_BASE=http://127.0.0.1:18080
FC_RUN_DIR=/srv/fc
FC_BRIDGE=fcbr0
ENV_EOF
        success "Created default .env file"
    fi
else
    success ".env file already exists"
fi

# ============================================================================
# 5. DIRECTORY CREATION
# ============================================================================
section "5️⃣  Creating Required Directories"

create_dir() {
    local DIR=$1
    if [ -d "$DIR" ]; then
        success "$DIR already exists"
    else
        if sudo mkdir -p "$DIR" 2>/dev/null && sudo chown "$USER:$USER" "$DIR" 2>/dev/null; then
            success "Created $DIR with proper permissions"
        else
            warn "Could not create $DIR with sudo, creating in home directory..."
            USER_DIR="$HOME/fc-runtime"
            mkdir -p "$USER_DIR"
            mkdir -p "$USER_DIR/vms"
            mkdir -p "$USER_DIR/images"
            success "Created $USER_DIR"
            warn "Update .env file to use $USER_DIR"
        fi
    fi
}

create_dir "/srv/fc"
create_dir "/srv/fc/vms"
create_dir "/srv/images"

# ============================================================================
# 6. BUILD PROJECT
# ============================================================================
section "6️⃣  Building Project"

info "Building Rust workspace (this may take a few minutes)..."
if cargo build --workspace; then
    success "Project built successfully"
else
    error "Build failed"
    exit 1
fi

# Build guest-agent with musl target
info "Building guest-agent with musl target..."
if rustup target list | grep -q "x86_64-unknown-linux-musl (installed)"; then
    success "musl target already installed"
else
    info "Installing musl target..."
    rustup target add x86_64-unknown-linux-musl
fi

if cargo build --release --target x86_64-unknown-linux-musl -p guest-agent; then
    success "Guest-agent built successfully"
else
    warn "Guest-agent build failed (non-critical for basic testing)"
fi

# ============================================================================
# 7. DATABASE MIGRATIONS
# ============================================================================
section "7️⃣  Running Database Migrations"

# Check if sqlx-cli is installed
if command -v sqlx >/dev/null 2>&1; then
    success "SQLx CLI found"
else
    info "Installing SQLx CLI (this may take a few minutes)..."
    if cargo install sqlx-cli --no-default-features --features postgres 2>/dev/null; then
        success "SQLx CLI installed"
    else
        warn "SQLx CLI installation failed (non-critical, migrations run automatically)"
    fi
fi

# Run migrations (manager does this automatically, but we can run them now)
if command -v sqlx >/dev/null 2>&1; then
    info "Running database migrations..."
    cd apps/manager
    if sqlx migrate run 2>/dev/null; then
        success "Migrations completed"
    else
        info "Migrations will run automatically when manager starts"
    fi
    cd "$PROJECT_ROOT"
else
    info "Migrations will run automatically when manager starts"
fi

# ============================================================================
# 8. RUNTIME IMAGES
# ============================================================================
section "8️⃣  Setting Up Runtime Images"

info "Downloading/building runtime images..."
echo ""
read -p "Download runtime images? This will download ~3GB of data [Y/n]: " DOWNLOAD_IMAGES
DOWNLOAD_IMAGES=${DOWNLOAD_IMAGES:-Y}

if [ "$DOWNLOAD_IMAGES" = "Y" ] || [ "$DOWNLOAD_IMAGES" = "y" ]; then
    if [ -f "$SCRIPT_DIR/dev-setup-images.sh" ]; then
        chmod +x "$SCRIPT_DIR/dev-setup-images.sh"
        if "$SCRIPT_DIR/dev-setup-images.sh"; then
            success "Runtime images setup completed"
        else
            warn "Some images may have failed to download. You can manually download them later."
        fi
    else
        warn "Image setup script not found at $SCRIPT_DIR/dev-setup-images.sh"
        info "You can manually download images from: https://github.com/NexusQuantum/NQRust-MicroVM/releases"
    fi
else
    info "Skipping image download. You'll need to set up images manually."
    info "Run: ./scripts/dev-setup-images.sh"
fi

# ============================================================================
# 9. SETUP COMPLETE
# ============================================================================
section "✨ Setup Complete!"

echo ""
success "Development environment is ready!"
echo ""
warn "IMPORTANT: Network bridge configuration was SKIPPED"
echo ""
echo "  Make sure to configure your network bridge (fcbr0) before starting services:"
echo ""
echo "  NAT mode:"
echo "    sudo ./scripts/fc-bridge-setup.sh fcbr0 <interface>"
echo ""
echo "  Bridged mode:"
echo "    sudo ./scripts/fc-bridge-physical.sh fcbr0 <interface>"
echo ""
echo "  Or check if fcbr0 already exists:"
echo "    ip link show fcbr0"
echo ""
echo -e "${BLUE}Next Steps:${NC}"
echo ""
echo "1️⃣  Start the Manager (Terminal 1):"
echo "   cd apps/manager && cargo run"
echo ""
echo "2️⃣  Start the Agent (Terminal 2, requires sudo):"
echo "   sudo -E env \\"
echo "     AGENT_BIND=127.0.0.1:9090 \\"
echo "     MANAGER_BASE=http://127.0.0.1:18080 \\"
echo "     FC_RUN_DIR=/srv/fc \\"
echo "     FC_BRIDGE=fcbr0 \\"
echo "     ./target/debug/agent"
echo ""
echo "3️⃣  Start the Frontend UI (Terminal 3, optional):"
echo "   cd apps/ui && pnpm install"
echo "   cd apps/ui && NEXT_PUBLIC_API_BASE_URL=http://127.0.0.1:18080/v1 pnpm dev"
echo ""
echo -e "${BLUE}Access Points:${NC}"
echo "  • Manager API: http://127.0.0.1:18080"
echo "  • API Docs: http://127.0.0.1:18080/scalar"
echo "  • Frontend UI: http://localhost:3000 (after starting UI)"
echo ""
echo -e "${BLUE}Useful Commands:${NC}"
echo "  • List VMs: curl http://127.0.0.1:18080/v1/vms"
echo "  • List Hosts: curl http://127.0.0.1:18080/v1/hosts"
echo "  • List Images: curl http://127.0.0.1:18080/v1/images"
echo "  • Check PostgreSQL: docker ps | grep postgres"
echo "  • Check Bridge: ip link show fcbr0"
echo ""
echo -e "${YELLOW}Documentation:${NC}"
echo "  • README.md - Installation and setup"
echo "  • RUN.md - Development commands"
echo "  • FEATURES.md - Feature matrix"
echo "  • CLAUDE.md - Claude Code guidance"
echo ""
echo -e "${GREEN}Happy coding! 🚀${NC}"
echo ""
