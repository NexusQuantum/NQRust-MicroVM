#!/usr/bin/env bash
set -euo pipefail

# Create Runtime Snapshot for Fast Container Creation
# This creates a Firecracker snapshot with Docker daemon pre-initialized
# Reduces container creation time from 60-120s to 5-15s

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info() { echo -e "${BLUE}ℹ${NC} $1"; }
success() { echo -e "${GREEN}✅${NC} $1"; }
warn() { echo -e "${YELLOW}⚠️${NC}  $1"; }
error() { echo -e "${RED}❌${NC} $1"; }

# Configuration
MANAGER_URL="${MANAGER_URL:-http://127.0.0.1:18080}"

echo -e "${BLUE}"
cat << "EOF"
╔═══════════════════════════════════════════════╗
║   Runtime Snapshot Creator                    ║
║   Warm Boot for Fast Container Creation       ║
╚═══════════════════════════════════════════════╝
EOF
echo -e "${NC}"

info "This creates a pre-warmed Firecracker snapshot with Docker daemon ready"
info "Benefit: Container creation ~5-15s (instead of 60-120s)"
echo ""

# Check manager is running
info "Checking manager connectivity..."
if ! curl -f -s "$MANAGER_URL/health" >/dev/null 2>&1; then
    error "Manager not accessible at $MANAGER_URL"
    error "Make sure the manager is running: cd apps/manager && cargo run"
    exit 1
fi
success "Manager is running"

# Get list of images
info "Fetching available runtime images..."
IMAGES=$(curl -s "$MANAGER_URL/v1/images" | jq -r '.items[] | select(.kind == "rootfs") | "\(.id) \(.host_path // .name)"' 2>/dev/null || echo "")

if [ -z "$IMAGES" ]; then
    error "No rootfs images found"
    echo ""
    info "You need a container runtime image first. Options:"
    echo "  1. Build one: sudo ./scripts/build-container-runtime-v2.sh"
    echo "  2. Download from releases: ./scripts/dev-setup-images.sh"
    echo "  3. Import manually: curl -X POST $MANAGER_URL/v1/images -d '{\"kind\":\"rootfs\",\"host_path\":\"/srv/images/container-runtime.ext4\"}'"
    exit 1
fi

# Find container runtime image
info "Looking for container runtime image..."
RUNTIME_IMAGE_ID=$(echo "$IMAGES" | grep -i "container-runtime" | head -n1 | awk '{print $1}')

if [ -z "$RUNTIME_IMAGE_ID" ]; then
    warn "No image named 'container-runtime' found"
    echo ""
    echo "Available rootfs images:"
    echo "$IMAGES" | nl
    echo ""
    read -p "Enter the number of the runtime image to use (or 'q' to quit): " CHOICE

    if [ "$CHOICE" = "q" ]; then
        exit 0
    fi

    RUNTIME_IMAGE_ID=$(echo "$IMAGES" | sed -n "${CHOICE}p" | awk '{print $1}')

    if [ -z "$RUNTIME_IMAGE_ID" ]; then
        error "Invalid selection"
        exit 1
    fi
fi

RUNTIME_IMAGE_PATH=$(echo "$IMAGES" | grep "$RUNTIME_IMAGE_ID" | awk '{print $2}')
success "Found runtime image: $RUNTIME_IMAGE_ID"
info "Path: $RUNTIME_IMAGE_PATH"
echo ""

# Check if snapshot already exists
info "Checking for existing snapshots..."
EXISTING_SNAPSHOTS=$(curl -s "$MANAGER_URL/v1/runtime-snapshots" | jq -r ".items[] | select(.runtime_image_id == \"$RUNTIME_IMAGE_ID\")" 2>/dev/null || echo "")

if [ -n "$EXISTING_SNAPSHOTS" ]; then
    EXISTING_ID=$(echo "$EXISTING_SNAPSHOTS" | jq -r '.id' | head -n1)
    EXISTING_STATE=$(echo "$EXISTING_SNAPSHOTS" | jq -r '.state' | head -n1)

    warn "Runtime snapshot already exists: $EXISTING_ID"
    info "State: $EXISTING_STATE"
    echo ""

    if [ "$EXISTING_STATE" = "ready" ]; then
        success "Snapshot is ready! Containers will use warm boot automatically."
        echo ""
        info "Stats:"
        echo "$EXISTING_SNAPSHOTS" | jq -r '"  Success count: \(.success_count)\n  Failure count: \(.failure_count)\n  Last used: \(.last_used_at // "never")"'
        echo ""
        read -p "Rebuild this snapshot? [y/N]: " REBUILD

        if [ "$REBUILD" != "y" ] && [ "$REBUILD" != "Y" ]; then
            info "Keeping existing snapshot. Exiting."
            exit 0
        fi

        info "Rebuilding snapshot..."
        RESPONSE=$(curl -s -X POST "$MANAGER_URL/v1/runtime-snapshots/$EXISTING_ID/rebuild")
        SNAPSHOT_ID="$EXISTING_ID"
    elif [ "$EXISTING_STATE" = "creating" ]; then
        info "Snapshot is currently being created. Monitoring progress..."
        SNAPSHOT_ID="$EXISTING_ID"
    else
        warn "Snapshot state is $EXISTING_STATE. Creating new snapshot..."
        # Continue to create new snapshot
    fi
fi

# Create new snapshot if needed
if [ -z "${SNAPSHOT_ID:-}" ]; then
    info "Creating runtime snapshot..."
    echo ""
    warn "This will take 60-120 seconds (one-time cost)"
    info "The manager will:"
    echo "  1. Create temporary VM with Docker runtime"
    echo "  2. Wait for Docker daemon to start (~60-120s)"
    echo "  3. Take Firecracker snapshot"
    echo "  4. Store snapshot files (~900MB)"
    echo "  5. Cleanup temporary VM"
    echo ""

    RESPONSE=$(curl -s -X POST "$MANAGER_URL/v1/runtime-snapshots" \
        -H "Content-Type: application/json" \
        -d "{\"runtime_image_id\": \"$RUNTIME_IMAGE_ID\"}")

    SNAPSHOT_ID=$(echo "$RESPONSE" | jq -r '.id // empty')

    if [ -z "$SNAPSHOT_ID" ]; then
        error "Failed to create snapshot"
        echo "Response: $RESPONSE"
        exit 1
    fi

    success "Snapshot creation initiated: $SNAPSHOT_ID"
fi

# Monitor snapshot creation
info "Monitoring snapshot creation..."
echo ""

for i in {1..180}; do  # Wait up to 3 minutes
    SNAPSHOT_INFO=$(curl -s "$MANAGER_URL/v1/runtime-snapshots/$SNAPSHOT_ID")
    STATE=$(echo "$SNAPSHOT_INFO" | jq -r '.item.state // .state // empty')

    if [ -z "$STATE" ]; then
        error "Failed to fetch snapshot status"
        exit 1
    fi

    if [ "$STATE" = "ready" ]; then
        echo ""
        success "Runtime snapshot is ready!"
        echo ""

        # Show snapshot details
        info "Snapshot Details:"
        echo "$SNAPSHOT_INFO" | jq -r '.item // . | "  ID: \(.id)\n  Path: \(.snapshot_path)\n  FC Version: \(.fc_version)\n  Created: \(.created_at)"'

        if echo "$SNAPSHOT_INFO" | jq -e '.item.metadata // .metadata' >/dev/null 2>&1; then
            METADATA=$(echo "$SNAPSHOT_INFO" | jq -r '.item.metadata // .metadata')
            SIZE_MB=$(echo "$METADATA" | jq -r '.size_bytes // 0' | awk '{printf "%.1f", $1/1024/1024}')
            info "  Size: ${SIZE_MB}MB"
        fi

        echo ""
        success "🚀 Containers will now use warm boot (5-15s instead of 60-120s)!"
        echo ""
        info "Test it:"
        echo '  curl -X POST '"$MANAGER_URL"'/v1/containers \\'
        echo '    -H "Content-Type: application/json" \\'
        echo '    -d '"'"'{"name": "test-nginx", "image": "nginx:alpine"}'"'"
        echo ""
        info "Check boot method:"
        echo "  curl $MANAGER_URL/v1/containers/{id} | jq .item.boot_method"
        echo "  # Should show: \"warm\""
        echo ""

        exit 0
    elif [ "$STATE" = "unhealthy" ] || [ "$STATE" = "deleted" ]; then
        error "Snapshot creation failed: $STATE"
        exit 1
    fi

    # Show progress
    printf "\r${BLUE}ℹ${NC} State: $STATE... [${i}s elapsed]"
    sleep 1
done

echo ""
error "Timeout waiting for snapshot to be ready"
info "Check status manually: curl $MANAGER_URL/v1/runtime-snapshots/$SNAPSHOT_ID"
exit 1
