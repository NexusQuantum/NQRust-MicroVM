# Available Images on Your System

**Generated:** 2025-10-24
**Location:** `/srv/images/`

---

## ✅ **WORKING KERNELS** (Confirmed Non-Zero Size)

### 1. **vmlinux-5.10.fc.bin** ⭐ RECOMMENDED
- **Path:** `/srv/images/vmlinux-5.10.fc.bin`
- **Size:** 21 MB
- **Owner:** shiro
- **Status:** ✅ **WORKING** (This is what you've been using)
- **Notes:** Official Firecracker kernel, battle-tested

### 2. **vmlinux-5.10.186**
- **Path:** `/srv/images/vmlinux-5.10.186`
- **Size:** 12 MB
- **Owner:** root
- **Status:** ✅ Should work (compatible kernel version)

---

## ❌ **BROKEN KERNELS** (Zero Size - Placeholders)

These files exist but are empty (0 bytes):
- ❌ `/srv/images/vmlinux-6.1.0` - **BROKEN**
- ❌ `/srv/images/vmlinux-6.6-rt` - **BROKEN**

---

## ✅ **WORKING ROOTFS IMAGES** (Confirmed Non-Zero Size)

### 1. **busybox-1.36.ext4** ⭐ YOUR CURRENT CHOICE
- **Path:** `/srv/images/busybox-1.36.ext4`
- **Size:** 30 MB
- **Status:** ✅ **WORKING** (You've been using this successfully)
- **Use Case:** Minimal Linux, perfect for testing
- **Contains:** BusyBox utilities, minimal footprint

### 2. **alpine-3.18-minimal.ext4**
- **Path:** `/srv/images/alpine-3.18-minimal.ext4`
- **Size:** 10 MB
- **Status:** ✅ Should work
- **Use Case:** Alpine Linux minimal, good for containers
- **Contains:** Alpine Linux base

### 3. **node-runtime.ext4** (For Functions)
- **Path:** `/srv/images/node-runtime.ext4`
- **Size:** 1.0 GB
- **Status:** ✅ **WORKING** (Used by serverless functions)
- **Use Case:** Node.js runtime for functions module
- **Contains:** Alpine + Node.js + runtime server

### 4. **python-runtime.ext4** (For Functions)
- **Path:** `/srv/images/python-runtime.ext4`
- **Size:** 1.0 GB
- **Status:** ✅ **WORKING** (Used by serverless functions)
- **Use Case:** Python runtime for functions module
- **Contains:** Alpine + Python + runtime server

### 5. **fn-7b0a18d4-aa49-41f2-8939-b9de359ff67d.ext4**
- **Path:** `/srv/images/fn-7b0a18d4-aa49-41f2-8939-b9de359ff67d.ext4`
- **Size:** 1.0 GB
- **Status:** ✅ Function-specific copy (auto-generated)
- **Use Case:** Dedicated to specific function instance
- **Notes:** Created by functions module

---

## ❌ **BROKEN ROOTFS** (Zero Size - Placeholders)

- ❌ `/srv/images/ubuntu-22.04-server.ext4` - **BROKEN** (0 bytes)

---

## 📝 **Recommended Combinations for VMs**

### **Option 1: Minimal Testing (What You've Been Using)** ⭐
```bash
KERNEL:  /srv/images/vmlinux-5.10.fc.bin
ROOTFS:  /srv/images/busybox-1.36.ext4
```
✅ **Status:** CONFIRMED WORKING
🎯 **Use:** Quick tests, minimal VMs, development

### **Option 2: Alpine Minimal**
```bash
KERNEL:  /srv/images/vmlinux-5.10.fc.bin
ROOTFS:  /srv/images/alpine-3.18-minimal.ext4
```
✅ **Status:** Should work
🎯 **Use:** Lightweight production VMs

### **Option 3: Alternative Kernel + BusyBox**
```bash
KERNEL:  /srv/images/vmlinux-5.10.186
ROOTFS:  /srv/images/busybox-1.36.ext4
```
⚠️ **Status:** Untested but should work
🎯 **Use:** Testing alternative kernel

---

## 🚀 **For Container Module**

You'll need to create a **container runtime image** with Docker:

```bash
KERNEL:  /srv/images/vmlinux-5.10.fc.bin
ROOTFS:  /srv/images/container-runtime.ext4  # ❌ TO BE CREATED
```

**How to create `container-runtime.ext4`:**

See the instructions in:
- `apps/manager/src/features/containers/vm.rs` (lines 160-200)
- Build Alpine Linux with Docker daemon installed
- Configure Docker to listen on TCP port 2375

---

## 🔍 **How to Verify an Image Works**

### **Test a Kernel:**
```bash
file /srv/images/vmlinux-5.10.fc.bin
# Should output: Linux kernel x86 boot executable bzImage...
```

### **Test a Rootfs:**
```bash
# Check size
ls -lh /srv/images/busybox-1.36.ext4

# Mount and explore (requires root)
sudo mkdir -p /tmp/test-mount
sudo mount -o loop /srv/images/busybox-1.36.ext4 /tmp/test-mount
ls /tmp/test-mount
sudo umount /tmp/test-mount
```

### **Test with a VM:**
```bash
# Use the manager API to create a VM with specific images
curl -X POST http://localhost:18080/v1/vms \
  -H "Content-Type: application/json" \
  -d '{
    "name": "test-vm",
    "vcpu": 1,
    "mem_mib": 256,
    "kernel_path": "/srv/images/vmlinux-5.10.fc.bin",
    "rootfs_path": "/srv/images/busybox-1.36.ext4"
  }'
```

---

## 📊 **Summary**

| Image Type | Working | Broken | Total |
|------------|---------|--------|-------|
| Kernels    | 2       | 2      | 4     |
| Rootfs     | 5       | 1      | 6     |

**Your Safe Combo:** ✅
- Kernel: `vmlinux-5.10.fc.bin`
- Rootfs: `busybox-1.36.ext4`

This is what you've been using and it's confirmed working!

---

## 🛠️ **What to Do About Broken Images**

### **Option 1: Remove placeholders**
```bash
sudo rm /srv/images/vmlinux-6.1.0
sudo rm /srv/images/vmlinux-6.6-rt
sudo rm /srv/images/ubuntu-22.04-server.ext4
```

### **Option 2: Download/Build proper images**

For Ubuntu 22.04:
```bash
# Download Ubuntu cloud image
wget https://cloud-images.ubuntu.com/releases/22.04/release/ubuntu-22.04-server-cloudimg-amd64.img
sudo mv ubuntu-22.04-server-cloudimg-amd64.img /srv/images/ubuntu-22.04-server.ext4
```

For newer kernels:
```bash
# Download from Firecracker releases
wget https://github.com/firecracker-microvm/firecracker/releases/download/v1.4.0/vmlinux-5.10.186
sudo mv vmlinux-5.10.186 /srv/images/
```

---

## ✨ **Quick Reference**

**For VMs:** Use `vmlinux-5.10.fc.bin` + `busybox-1.36.ext4`
**For Functions:** System auto-uses `node-runtime.ext4` or `python-runtime.ext4`
**For Containers:** Need to create `container-runtime.ext4` (with Docker inside)

**All your current images are in:** `/srv/images/`
**Functions auto-copy to:** `/srv/images/functions/`
