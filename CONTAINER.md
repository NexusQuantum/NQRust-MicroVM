# Docker Container Management

**Last Updated:** 2025-10-27

NQRust-MicroVM now supports running Docker containers inside Firecracker microVMs with complete isolation and networking capabilities.

## Overview

The container feature implements a **container-per-VM architecture** where each Docker container runs in its own isolated Firecracker microVM. This provides:

- **Strong Isolation**: Each container runs in a separate kernel and VM
- **Network Integration**: Containers can access external networks via bridge networking
- **Docker API Compatibility**: Full Docker Remote API support
- **Resource Management**: Per-container CPU and memory limits via VM configuration

## Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Manager      │    │   Firecracker    │    │   Docker        │
│   API          │───▶│   VM             │───▶│   Container     │
│                │    │                  │    │                 │
│ - Creates      │    │ - Alpine Linux   │    │ - hello-world   │
│ - Monitors     │    │ - Docker Daemon  │    │ - nginx         │
│ - Manages      │    │ - TCP:2375       │    │ - any image     │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

### Components

1. **Container Runtime Image**: Alpine Linux + Docker daemon + OpenRC init
2. **Manager Service**: Orchestrates container lifecycle via Docker API
3. **Network Bridge**: `fcbr0` provides external network access
4. **Guest Agent**: Reports VM IP and status to manager

## Prerequisites

### System Requirements

- **Bridge Networking**: Must have `fcbr0` bridge configured
- **DHCP Server**: Router or DHCP server for IP assignment
- **Container Runtime Image**: Built and available at `/srv/images/container-runtime.ext4`

### Build Container Runtime Image

```bash
# Build the container runtime image
sudo scripts/build-container-runtime-v2.sh

# Verify image exists
ls -lh /srv/images/container-runtime.ext4
```

The build script creates:
- **Alpine Linux 3.18** base system
- **Docker 25.0.5** with TCP API enabled
- **OpenRC** service management
- **DHCP networking** configuration
- **386MB** optimized image size

## API Reference

### Create Container

```bash
curl -X POST http://localhost:18080/v1/containers \
  -H "Content-Type: application/json" \
  -d '{
    "name": "hello-world",
    "image": "hello-world:latest",
    "args": [],
    "env_vars": {},
    "volumes": [],
    "port_mappings": [],
    "restart_policy": "no"
  }'
```

**Response:**
```json
{
  "id": "e2222978-c5a0-4f49-8a60-2d2f3bfee8e4"
}
```

### Get Container Status

```bash
curl http://localhost:18080/v1/containers/{id}
```

**Response:**
```json
{
  "item": {
    "id": "e2222978-c5a0-4f49-8a60-2d2f3bfee8e4",
    "name": "hello-world",
    "image": "hello-world:latest",
    "args": [],
    "env_vars": {},
    "volumes": [],
    "port_mappings": [],
    "restart_policy": "no",
    "state": "running",
    "container_runtime_id": "vm-7da2dcf3-1d3a-4e3c-92a3-cf59926cd071",
    "container_id": "8dbe98a07e913ccccc39a2304eee7beb54fcf0fee2d0e6bcc833ffc3c3132014",
    "error_message": null,
    "created_at": "2025-10-27T06:32:44.123Z",
    "updated_at": "2025-10-27T06:35:12.456Z"
  }
}
```

### List All Containers

```bash
curl http://localhost:18080/v1/containers
```

### Get Container Logs

```bash
curl http://localhost:18080/v1/containers/{id}/logs
```

### Delete Container

```bash
curl -X DELETE http://localhost:18080/v1/containers/{id}
```

## Container States

| State | Description |
|-------|-------------|
| `creating` | Container record created, VM provisioning started |
| `booting` | VM is starting up |
| `initializing` | VM has IP, waiting for Docker daemon |
| `running` | Container is running and accessible |
| `error` | Failed to create or start container |

## Request Parameters

### Container Creation

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `name` | string | Yes | Container name (unique) |
| `image` | string | Yes | Docker image name (e.g., `nginx:latest`) |
| `args` | array | No | Container command arguments |
| `env_vars` | object | No | Environment variables (`{"KEY": "value"}`) |
| `volumes` | array | No | Volume mappings (future feature) |
| `port_mappings` | array | No | Port mappings (future feature) |
| `restart_policy` | string | No | Restart policy (`no`, `always`, `on-failure`) |

## Network Configuration

### Bridge Setup

Containers use the same bridge networking as VMs:

```bash
# Create bridge (one-time setup)
sudo brctl addbr fcbr0
sudo ip addr add 192.168.18.1/24 dev fcbr0
sudo ip link set fcbr0 up

# Connect to physical network
sudo ip link set eth0 master fcbr0
sudo ip link set eth0 up
```

### IP Assignment

- **DHCP**: Containers automatically get IPs from your router
- **Range**: Typically 192.168.18.x subnet
- **Access**: Full external network access

## Docker API Access

Each container VM exposes Docker API on port 2375:

```bash
# Get VM IP from container
VM_IP=$(curl -s http://localhost:18080/v1/containers/{id} | jq -r '.item.container_runtime_id')
GUEST_IP=$(curl -s http://localhost:18080/v1/vms/${VM_IP#vm-} | jq -r '.item.guest_ip')

# Direct Docker API access
curl http://${GUEST_IP}:2375/_ping
curl http://${GUEST_IP}:2375/version
curl http://${GUEST_IP}:2375/containers/json
```

## Examples

### Hello World Container

```bash
# Create container
curl -X POST http://localhost:18080/v1/containers \
  -H "Content-Type: application/json" \
  -d '{"name": "hello-world", "image": "hello-world:latest"}'

# Wait for provisioning (2-3 minutes)
# Check status
curl http://localhost:18080/v1/containers/{id}

# Get logs
curl http://localhost:18080/v1/containers/{id}/logs
```

### Nginx Web Server

```bash
# Create nginx container
curl -X POST http://localhost:18080/v1/containers \
  -H "Content-Type: application/json" \
  -d '{
    "name": "nginx-server",
    "image": "nginx:alpine",
    "port_mappings": [{"container_port": 80, "host_port": 8080}]
  }'

# Access nginx (once container is running)
curl http://<container-vm-ip>:80
```

### Custom Application

```bash
# Create application container with environment variables
curl -X POST http://localhost:18080/v1/containers \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-app",
    "image": "node:18-alpine",
    "env_vars": {
      "NODE_ENV": "production",
      "PORT": "3000"
    },
    "args": ["node", "app.js"]
  }'
```

## Troubleshooting

### Common Issues

#### Container Stuck in "initializing"

**Cause**: Docker daemon not ready within timeout
**Solution**: Manager waits up to 120 seconds for Docker to start

```bash
# Check VM status
curl http://localhost:18080/v1/vms/{vm-id}

# Test Docker directly
curl http://<vm-ip>:2375/_ping
```

#### Container Fails with "Timeout waiting for VM guest IP"

**Cause**: Network configuration issue
**Solution**: Verify bridge networking and DHCP

```bash
# Check bridge
ip addr show fcbr0

# Check DHCP leases
ip neigh show dev fcbr0

# Verify VM networking
curl http://localhost:18080/v1/vms/{vm-id}
```

#### Docker API Not Responding

**Cause**: Docker daemon not started or misconfigured
**Solution**: Check container runtime image build

```bash
# Rebuild container runtime
sudo scripts/build-container-runtime-v2.sh

# Verify Docker configuration
# (Check /etc/conf.d/docker in the image)
```

### Debug Commands

```bash
# Check container logs
curl http://localhost:18080/v1/containers/{id}/logs

# Check VM details
curl http://localhost:18080/v1/vms/{vm-id}

# Test Docker API directly
curl http://<vm-ip>:2375/version
curl http://<vm-ip>:2375/containers/json

# Check manager logs
sudo journalctl -u nexus-manager --since "5 minutes ago" | grep -i container
```

## Performance Considerations

### Resource Usage

- **Memory**: ~512MB base + container requirements
- **CPU**: 1 vCPU per container (configurable)
- **Disk**: 2.2GB container runtime + container layers
- **Network**: Full bridge network performance

### Scaling

- **Concurrent Containers**: Limited by host resources
- **Startup Time**: 60-90 seconds per container (Docker daemon startup)
- **Isolation**: Full kernel-level isolation between containers

## Security

### Isolation

- **Kernel Isolation**: Each container in separate Firecracker VM
- **Network Isolation**: Bridge networking with optional filtering
- **Process Isolation**: No shared processes between containers
- **Filesystem Isolation**: Separate root filesystem per container

### Best Practices

1. **Use Trusted Images**: Only pull from trusted registries
2. **Monitor Resources**: Track CPU/memory usage per container
3. **Network Security**: Configure firewall rules as needed
4. **Regular Updates**: Keep container runtime image updated

## Future Enhancements

### Planned Features

- **Volume Mounting**: Persistent storage support
- **Port Mapping**: External port access configuration
- **Resource Limits**: CPU/memory constraints per container
- **Container Registry**: Private registry support
- **Auto-scaling**: Dynamic container provisioning
- **Health Checks**: Container health monitoring

### API Extensions

```json
{
  "volumes": [
    {
      "host_path": "/data/container1",
      "container_path": "/app/data",
      "readonly": false
    }
  ],
  "port_mappings": [
    {
      "container_port": 80,
      "host_port": 8080,
      "protocol": "tcp"
    }
  ],
  "resource_limits": {
    "cpu": "0.5",
    "memory": "512m"
  }
}
```

## Integration with Frontend

The container API is fully compatible with the existing NQRust-MicroVM frontend:

- **Container List**: View all containers with status
- **Container Details**: Show logs, metrics, and configuration
- **Container Actions**: Start, stop, restart containers
- **Real-time Updates**: WebSocket integration for live status

## Summary

The Docker container feature provides:

✅ **Complete Isolation** - Firecracker VM per container  
✅ **Full Docker API** - Compatible with Docker tooling  
✅ **Network Integration** - Bridge networking with DHCP  
✅ **Easy Management** - REST API for container lifecycle  
✅ **Production Ready** - Robust error handling and monitoring  

This makes NQRust-MicroVM a powerful platform for running containers with maximum isolation and security.