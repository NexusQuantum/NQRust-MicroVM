#!/bin/sh
# Init script for the raftblk-vhost microVM smoke test.
#
# This file is placed at /init inside the initramfs that Firecracker
# boots. The kernel runs /init as PID 1 (rdinit=/init in boot_args).
#
# What it does:
#   1. Mount /dev (devtmpfs), /proc, /sys, /tmp (tmpfs).
#   2. Verify /dev/vda exists (vhost-user-blk drive should appear here).
#   3. Build a 4096-byte 0xAB pattern in /tmp.
#   4. Write the pattern to /dev/vda at sector 8 (offset 4096).
#   5. Read 4096 bytes back from sector 8.
#   6. cmp the two; print RAFTBLK-SMOKE-IO-VERIFIED on success.
#   7. Reboot.
#
# Markers the smoke harness greps for:
#   ===== RAFTBLK-SMOKE-INIT-OK =====        guest reached init
#   ===== RAFTBLK-SMOKE-IO-VERIFIED =====    write/read round-trip OK
#   ===== RAFTBLK-SMOKE-IO-MISMATCH =====    bytes differ
#   ===== RAFTBLK-SMOKE-NO-VDA =====         vhost-user-blk never exposed /dev/vda
#   ===== RAFTBLK-SMOKE-DONE =====           init finished
#
# To use this in the smoke runner: extract the FC quickstart initramfs
# (`bsdtar -xf initramfs.cpio`), replace the existing /init with this
# file, then repack (`bsdtar --format=newc -cf initramfs-custom.cpio
# init bin dev proc sys`). Pass the result as INITRD to the smoke
# script.

mount -t devtmpfs devtmpfs /dev
mount -t proc none /proc
mount -t sysfs none /sys
mkdir -p /tmp
mount -t tmpfs tmpfs /tmp
exec 0</dev/console
exec 1>/dev/console
exec 2>/dev/console

echo "===== RAFTBLK-SMOKE-INIT-OK ====="
echo "kernel sees these block devices:"
ls -la /dev/vd* 2>/dev/null || echo "no /dev/vd* present"
echo

if [ -b /dev/vda ]; then
    # Build a 4096-byte recognizable pattern (0xAB repeated). busybox
    # sh's printf supports \xNN; we replicate via concatenation.
    printf '\xab\xab\xab\xab\xab\xab\xab\xab' > /tmp/pat8
    : > /tmp/pat128
    for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16; do cat /tmp/pat8 >> /tmp/pat128; done
    : > /tmp/pat2k
    for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16; do cat /tmp/pat128 >> /tmp/pat2k; done
    cat /tmp/pat2k /tmp/pat2k > /tmp/pat4k

    echo "[smoke] writing 4096 bytes (0xAB) to /dev/vda at sector 8 (offset 4096)"
    dd if=/tmp/pat4k of=/dev/vda bs=4096 count=1 seek=1 conv=fsync 2>&1 | tail -1
    sync

    echo "[smoke] reading 4096 bytes back from /dev/vda at sector 8"
    dd if=/dev/vda of=/tmp/read4k bs=4096 count=1 skip=1 2>&1 | tail -1

    if cmp /tmp/pat4k /tmp/read4k; then
        echo "===== RAFTBLK-SMOKE-IO-VERIFIED ====="
    else
        echo "===== RAFTBLK-SMOKE-IO-MISMATCH ====="
        echo "first 16 bytes of read:"
        od -An -tx1 -N 16 /tmp/read4k
        echo "first 16 bytes of pattern:"
        od -An -tx1 -N 16 /tmp/pat4k
    fi
else
    echo "===== RAFTBLK-SMOKE-NO-VDA ====="
fi

echo "===== RAFTBLK-SMOKE-DONE ====="
sync
sleep 1
reboot -f
