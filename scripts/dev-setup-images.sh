#!/usr/bin/env bash
# Development Image Setup Script
# Downloads or builds all necessary images for local development

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
IMAGE_DIR="/srv/images"

# GitHub release URL
REPO="NexusQuantum/NQRust-MicroVM"
RELEASE_URL="https://github.com/${REPO}/releases/latest/download"

echo "╔════════════════════════════════════════════════╗"
echo "║  NQR-MicroVM Development Image Setup           ║"
echo "╚════════════════════════════════════════════════╝"
echo ""

# Create image directory structure
echo "📁 Setting up image directories..."
sudo mkdir -p "$IMAGE_DIR"
sudo mkdir -p "$IMAGE_DIR/functions"
sudo mkdir -p "$IMAGE_DIR/containers"
sudo chown -R $USER:$USER "$IMAGE_DIR"
chmod -R 755 "$IMAGE_DIR"
echo "✅ Image directories created"
echo ""

# Base images to download
declare -A IMAGES=(
    ["vmlinux-5.10.fc.bin"]="Firecracker kernel 5.10"
    ["alpine-3.18-minimal.ext4"]="Alpine Linux minimal"
    ["busybox-1.35.ext4"]="BusyBox"
    ["ubuntu-24.04-minimal.ext4"]="Ubuntu minimal"
    ["python-runtime.ext4"]="Python function runtime"
    ["bun-runtime.ext4"]="JavaScript/TypeScript function runtime"
)

# Function to download image
download_image() {
    local filename="$1"
    local description="$2"
    local dst_path="$IMAGE_DIR/$filename"

    # Skip if already exists
    if [ -f "$dst_path" ]; then
        echo "  ⏭️  $description already exists, skipping"
        return 0
    fi

    echo "  📥 Downloading $description..."
    local url="$RELEASE_URL/$filename"

    if curl -fsSL -o "$dst_path" "$url"; then
        echo "  ✅ $description downloaded"
        return 0
    else
        echo "  ⚠️  Failed to download $filename (might not be released yet)"
        return 1
    fi
}

# Download standard images
echo "📦 Downloading base images..."
for filename in "${!IMAGES[@]}"; do
    download_image "$filename" "${IMAGES[$filename]}"
done
echo ""

# Handle container runtime specially (large, compressed)
echo "🐳 Setting up container runtime..."
if [ -f "$IMAGE_DIR/container-runtime.ext4" ]; then
    echo "  ✅ Container runtime already exists"
elif curl -fsSL -o "$IMAGE_DIR/container-runtime.ext4.gz" "$RELEASE_URL/container-runtime.ext4.gz" 2>/dev/null; then
    echo "  📥 Downloaded container runtime (compressed)"
    echo "  🔓 Decompressing (~2GB, this may take a minute)..."
    gunzip -f "$IMAGE_DIR/container-runtime.ext4.gz"
    echo "  ✅ Container runtime ready"
else
    echo "  ⚠️  Could not download container runtime from releases"
    echo "  💡 Building locally with script (takes ~10-15 minutes)..."
    if [ -f "$SCRIPT_DIR/build-container-runtime-v2.sh" ]; then
        sudo "$SCRIPT_DIR/build-container-runtime-v2.sh"
        echo "  ✅ Container runtime built successfully"
    else
        echo "  ❌ Build script not found. You'll need to build it manually later."
        echo "     Run: sudo ./scripts/build-container-runtime-v2.sh"
    fi
fi
echo ""

# Download kernel if not present
if [ ! -f "$IMAGE_DIR/vmlinux-5.10.fc.bin" ]; then
    echo "📥 Downloading Firecracker kernel..."
    # Try from releases first
    if ! curl -fsSL -o "$IMAGE_DIR/vmlinux-5.10.fc.bin" "$RELEASE_URL/vmlinux-5.10.fc.bin" 2>/dev/null; then
        echo "  ⚠️  Could not download from releases"
        echo "  💡 You can download manually from:"
        echo "     https://github.com/firecracker-microvm/firecracker/releases"
        echo "     Or use an existing kernel binary"
    else
        echo "  ✅ Kernel downloaded"
    fi
fi
echo ""

# Summary
echo "📊 Image Setup Summary"
echo "═══════════════════════════════════════════════"
echo "Image directory: $IMAGE_DIR"
echo ""
echo "Checking downloaded images:"
for filename in "${!IMAGES[@]}"; do
    if [ -f "$IMAGE_DIR/$filename" ]; then
        size=$(du -h "$IMAGE_DIR/$filename" | cut -f1)
        echo "  ✅ $filename ($size)"
    else
        echo "  ❌ $filename (missing)"
    fi
done

if [ -f "$IMAGE_DIR/container-runtime.ext4" ]; then
    size=$(du -h "$IMAGE_DIR/container-runtime.ext4" | cut -f1)
    echo "  ✅ container-runtime.ext4 ($size)"
else
    echo "  ❌ container-runtime.ext4 (missing)"
fi

if [ -f "$IMAGE_DIR/vmlinux-5.10.fc.bin" ]; then
    size=$(du -h "$IMAGE_DIR/vmlinux-5.10.fc.bin" | cut -f1)
    echo "  ✅ vmlinux-5.10.fc.bin ($size)"
else
    echo "  ❌ vmlinux-5.10.fc.bin (missing)"
fi

echo ""
echo "═══════════════════════════════════════════════"
echo ""

# Check for missing critical images
missing_count=0
if [ ! -f "$IMAGE_DIR/container-runtime.ext4" ]; then
    ((missing_count++))
fi
if [ ! -f "$IMAGE_DIR/vmlinux-5.10.fc.bin" ]; then
    ((missing_count++))
fi

if [ $missing_count -eq 0 ]; then
    echo "🎉 All critical images are ready!"
    echo ""
    echo "Next steps:"
    echo "  1. Start manager: cd apps/manager && cargo run"
    echo "  2. Images will be auto-registered on manager startup"
    echo "  3. Create runtime snapshot for warm boot:"
    echo "     RUNTIME_IMAGE_ID=\$(curl -s http://127.0.0.1:18080/v1/images | jq -r '.items[] | select(.name == \"container-runtime\") | .id')"
    echo "     curl -X POST http://127.0.0.1:18080/v1/runtime-snapshots -H 'Content-Type: application/json' -d \"{\\\"runtime_image_id\\\": \\\"\$RUNTIME_IMAGE_ID\\\"}\""
else
    echo "⚠️  $missing_count critical image(s) missing"
    echo ""
    echo "To build missing images manually:"
    echo "  Container runtime: sudo ./scripts/build-container-runtime-v2.sh"
    echo "  Alpine minimal: sudo ./scripts/build-rootfs-debian-minimal.sh"
    echo "  Ubuntu minimal: sudo ./scripts/build-rootfs-ubuntu-minimal.sh"
fi
echo ""
