#!/bin/bash
# Quick script to verify available Firecracker images

set -e

IMAGE_DIR="/srv/images"
echo "================================================"
echo "  Firecracker Image Verification"
echo "================================================"
echo ""

echo "📁 Image Directory: $IMAGE_DIR"
echo ""

echo "=== ✅ WORKING KERNELS ==="
echo ""
find $IMAGE_DIR -maxdepth 1 -name "vmlinux*" -type f -size +0 | while read kernel; do
    size=$(du -h "$kernel" | cut -f1)
    echo "  ✅ $(basename $kernel) - $size"
    file "$kernel" | grep -q "kernel" && echo "     Type: Valid Linux kernel" || echo "     Type: Unknown (check manually)"
done

broken_kernels=$(find $IMAGE_DIR -maxdepth 1 -name "vmlinux*" -type f -size 0 | wc -l)
if [ $broken_kernels -gt 0 ]; then
    echo ""
    echo "=== ❌ BROKEN KERNELS (0 bytes) ==="
    echo ""
    find $IMAGE_DIR -maxdepth 1 -name "vmlinux*" -type f -size 0 | while read kernel; do
        echo "  ❌ $(basename $kernel) - EMPTY FILE"
    done
fi

echo ""
echo "=== ✅ WORKING ROOTFS IMAGES ==="
echo ""
find $IMAGE_DIR -maxdepth 1 -name "*.ext4" -type f -size +0 | while read rootfs; do
    size=$(du -h "$rootfs" | cut -f1)
    echo "  ✅ $(basename $rootfs) - $size"
done

broken_rootfs=$(find $IMAGE_DIR -maxdepth 1 -name "*.ext4" -type f -size 0 | wc -l)
if [ $broken_rootfs -gt 0 ]; then
    echo ""
    echo "=== ❌ BROKEN ROOTFS (0 bytes) ==="
    echo ""
    find $IMAGE_DIR -maxdepth 1 -name "*.ext4" -type f -size 0 | while read rootfs; do
        echo "  ❌ $(basename $rootfs) - EMPTY FILE"
    done
fi

echo ""
echo "=== 🎯 RECOMMENDED COMBINATIONS ==="
echo ""

if [ -f "$IMAGE_DIR/vmlinux-5.10.fc.bin" ] && [ -s "$IMAGE_DIR/vmlinux-5.10.fc.bin" ]; then
    if [ -f "$IMAGE_DIR/busybox-1.36.ext4" ] && [ -s "$IMAGE_DIR/busybox-1.36.ext4" ]; then
        echo "  ⭐ MINIMAL VM (CONFIRMED WORKING):"
        echo "     Kernel:  /srv/images/vmlinux-5.10.fc.bin"
        echo "     Rootfs:  /srv/images/busybox-1.36.ext4"
        echo ""
    fi

    if [ -f "$IMAGE_DIR/alpine-3.18-minimal.ext4" ] && [ -s "$IMAGE_DIR/alpine-3.18-minimal.ext4" ]; then
        echo "  📦 ALPINE VM:"
        echo "     Kernel:  /srv/images/vmlinux-5.10.fc.bin"
        echo "     Rootfs:  /srv/images/alpine-3.18-minimal.ext4"
        echo ""
    fi
fi

echo "=== 📊 SUMMARY ==="
echo ""

working_kernels=$(find $IMAGE_DIR -maxdepth 1 -name "vmlinux*" -type f -size +0 | wc -l)
total_kernels=$(find $IMAGE_DIR -maxdepth 1 -name "vmlinux*" -type f | wc -l)
working_rootfs=$(find $IMAGE_DIR -maxdepth 1 -name "*.ext4" -type f -size +0 | wc -l)
total_rootfs=$(find $IMAGE_DIR -maxdepth 1 -name "*.ext4" -type f | wc -l)

echo "  Kernels: $working_kernels working / $total_kernels total"
echo "  Rootfs:  $working_rootfs working / $total_rootfs total"
echo ""

if [ -f "$IMAGE_DIR/container-runtime.ext4" ] && [ -s "$IMAGE_DIR/container-runtime.ext4" ]; then
    echo "  ✅ Container runtime image available"
else
    echo "  ⚠️  Container runtime image NOT found"
    echo "     To use containers, create: /srv/images/container-runtime.ext4"
    echo "     See: apps/manager/src/features/containers/vm.rs for instructions"
fi

echo ""
echo "================================================"
echo "💡 For detailed information, see AVAILABLE_IMAGES.md"
echo "================================================"
