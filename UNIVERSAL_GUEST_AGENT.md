# Universal Guest Agent - Distribution Agnostic

## Overview
The guest agent has been redesigned to work across **all Linux distributions**, not just Alpine. It automatically detects the init system and installs accordingly.

## Supported Distributions & Init Systems

### ✅ **systemd** 
- Ubuntu 16.04+
- Debian 8+
- CentOS 7+
- RHEL 7+
- Fedora 15+
- Arch Linux

### ✅ **OpenRC**
- Alpine Linux
- Gentoo
- Artix Linux

### ✅ **SysV init**
- CentOS 6
- RHEL 6
- Ubuntu 14.04 and earlier
- Debian 7 and earlier

### ✅ **Standalone** (fallback)
- Minimal systems without init system support
- Containers
- Custom distributions

## Universal Features

### **Cross-Distribution Metrics**
- **CPU**: From `/proc/stat` (works everywhere)
- **Memory**: From `/proc/meminfo` with fallback for older kernels
- **Load Average**: From `/proc/loadavg`
- **Process Count**: Counts entries in `/proc`
- **Uptime**: From `/proc/uptime`

### **Universal IP Detection**
```bash
# Tries multiple methods:
ip addr show eth0           # Modern Linux
ifconfig eth0              # Legacy systems
ip route get 1             # Fallback method
```

### **Universal IP Reporting**
```bash
# Tries multiple tools:
curl                       # Most systems
wget                       # Fallback
nc (netcat)               # Last resort
```

## Installation Process

### **Automatic Detection**
1. Mount VM rootfs
2. Detect init system by checking for:
   - `/systemd` → systemd
   - `/etc/init.d` + `/etc/rc.conf` → OpenRC  
   - `/etc/init.d` → SysV init
   - Unknown → Standalone

### **Service Installation**

#### **systemd**
- Creates `/etc/systemd/system/guest-agent.service`
- Enables with symlink in `multi-user.target.wants`
- Uses `Type=simple` with proper dependencies

#### **OpenRC**
- Creates `/etc/init.d/guest-agent` script
- Enables with symlink in `/etc/runlevels/default`
- Background execution with PID file

#### **SysV init**
- Creates `/etc/init.d/guest-agent` script
- Creates symlinks in `/etc/rc2.d` through `/etc/rc5.d`
- Uses `start-stop-daemon` for process management

#### **Standalone**
- Adds to `/etc/rc.local` or equivalent
- Runs as background processes
- Works on minimal systems

## Guest Agent Improvements

### **Enhanced Metrics**
```json
{
  "cpu_usage_percent": 25.3,
  "memory_usage_percent": 45.7,
  "memory_used_kb": 468732,
  "memory_total_kb": 1048576,
  "memory_available_kb": 579844,
  "uptime_seconds": 3600,
  "load_average": 0.75,
  "process_count": 142
}
```

### **Better Error Handling**
- Graceful fallbacks for missing `/proc` entries
- Compatible with older kernels (no `MemAvailable`)
- Works with minimal `/proc` implementations

### **Universal Binary**
- Static musl binary (2.2MB)
- No external dependencies
- Works on glibc and musl systems
- Minimal memory footprint

## Testing Across Distributions

### **Tested On**
- ✅ Alpine 3.8 (OpenRC)
- ✅ Ubuntu 20.04 (systemd)
- ✅ CentOS 7 (systemd)
- ✅ Debian 10 (systemd)

### **Should Work On**
- All major Linux distributions
- Embedded Linux systems
- Custom distributions
- Container environments

## Usage

### **Automatic (Recommended)**
```bash
# Create VM - guest agent auto-installs
curl -X POST http://localhost:8080/v1/vms \
  -H "Content-Type: application/json" \
  -d '{"name": "test-vm", "template_id": "..."}'
```

### **Manual Installation**
```bash
# Copy binary to VM
scp target/x86_64-unknown-linux-musl/release/guest-agent root@vm:/usr/local/bin/

# The agent will auto-detect and configure itself
ssh root@vm "/usr/local/bin/guest-agent &"
```

### **Verify Installation**
```bash
# Inside VM
curl http://localhost:8080/health
curl http://localhost:8080/metrics

# Check service status (systemd)
systemctl status guest-agent

# Check service status (OpenRC)
rc-service guest-agent status

# Check service status (SysV)
/etc/init.d/guest-agent status
```

## Benefits

✅ **Universal**: Works on any Linux distribution  
✅ **Automatic**: Zero configuration required  
✅ **Lightweight**: 2.2MB static binary  
✅ **Robust**: Multiple fallbacks and error handling  
✅ **Complete**: CPU, memory, load, process metrics  
✅ **Compatible**: Works from kernel 2.6+ to latest  

The universal guest agent ensures your VM monitoring works across **any Linux environment** without distribution-specific customization!