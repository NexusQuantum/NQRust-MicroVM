# On-Premises Lambda Setup Guide

This guide explains how to set up and use the NQRust Lambda (Serverless Functions) feature.

## Architecture Overview

**Each function runs in its own dedicated Firecracker MicroVM:**

```
Function "hello-world"
  → MicroVM (1 vCPU, 256MB RAM)
     → Runtime Server (Node.js/Python)
        → Listens on port 3000
        → Executes function code on demand
     → Guest Agent (reports metrics)
     → IP: 10.0.0.42 (from bridge)

Invocation Flow:
  POST /v1/functions/{id}/invoke
    ↓
  Manager → HTTP POST to http://10.0.0.42:3000/invoke
    ↓
  Runtime Server executes code
    ↓
  Returns: {status, response, logs, duration_ms}
```

## Prerequisites

### 1. Build Runtime Images

You need to create custom rootfs images with:
- Alpine Linux base (lightweight)
- Node.js or Python installed
- Runtime server (`server.js` or `server.py`)
- Init system to auto-start runtime server

#### Node.js Runtime Image

```bash
# 1. Start with Alpine base
wget https://dl-cdn.alpinelinux.org/alpine/v3.18/releases/x86_64/alpine-minirootfs-3.18.4-x86_64.tar.gz

# 2. Create rootfs
mkdir -p node-runtime
cd node-runtime
sudo tar xzf ../alpine-minirootfs-3.18.4-x86_64.tar.gz

# 3. Install Node.js
sudo chroot . /bin/sh << 'EOF'
apk add --no-cache nodejs npm openrc
rc-update add devfs boot
rc-update add procfs boot
rc-update add sysfs boot
EOF

# 4. Copy runtime server
sudo mkdir -p function
sudo cp ../apps/function-runtime/node/server.js usr/local/bin/runtime-server
sudo chmod +x usr/local/bin/runtime-server

# 5. Create init service
sudo tee etc/init.d/runtime-server << 'EOF'
#!/sbin/openrc-run

name="runtime-server"
command="/usr/local/bin/runtime-server"
command_background=true
pidfile="/run/runtime-server.pid"

depend() {
    need net
}
EOF

sudo chmod +x etc/init.d/runtime-server
sudo chroot . rc-update add runtime-server default

# 6. Create ext4 filesystem
sudo mkfs.ext4 -d . ../node-runtime.ext4 1G
cd ..

# 7. Move to images directory
sudo mv node-runtime.ext4 /srv/images/
```

#### Python Runtime Image

```bash
# Similar process but install Python instead
mkdir -p python-runtime
cd python-runtime
sudo tar xzf ../alpine-minirootfs-3.18.4-x86_64.tar.gz

sudo chroot . /bin/sh << 'EOF'
apk add --no-cache python3 openrc
rc-update add devfs boot
rc-update add procfs boot
rc-update add sysfs boot
EOF

sudo mkdir -p function
sudo cp ../apps/function-runtime/python/server.py usr/local/bin/runtime-server
sudo chmod +x usr/local/bin/runtime-server

# Create init service (same as Node.js)
sudo tee etc/init.d/runtime-server << 'EOF'
#!/sbin/openrc-run

name="runtime-server"
command="/usr/local/bin/runtime-server"
command_background=true
pidfile="/run/runtime-server.pid"

depend() {
    need net
}
EOF

sudo chmod +x etc/init.d/runtime-server
sudo chroot . rc-update add runtime-server default

sudo mkfs.ext4 -d . ../python-runtime.ext4 1G
cd ..
sudo mv python-runtime.ext4 /srv/images/
```

### 2. Kernel Image

```bash
# Download or build a compatible Linux kernel
wget https://s3.amazonaws.com/spec.ccfc.min/img/quickstart_guide/x86_64/kernels/vmlinux.bin \
  -O /srv/images/vmlinux-5.10
```

### 3. Configure Image Paths

Edit `apps/manager/src/features/functions/vm.rs` if your images are in different locations:

```rust
fn get_runtime_image_paths(runtime: &str) -> Result<(String, String)> {
    let kernel = "/srv/images/vmlinux-5.10".to_string();

    let rootfs = match runtime {
        "node" => "/srv/images/node-runtime.ext4",
        "python" => "/srv/images/python-runtime.ext4",
        // ...
    };

    Ok((kernel, rootfs.to_string()))
}
```

## Usage

### 1. Create a Function

```bash
curl -X POST http://localhost:18080/v1/functions \
  -H "Content-Type: application/json" \
  -d '{
    "name": "hello-world",
    "runtime": "node",
    "handler": "handler",
    "code": "async function handler(event) { return { message: \"Hello \" + event.name }; }",
    "timeout_seconds": 30,
    "memory_mb": 256,
    "vcpu": 1
  }'
```

Response:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000"
}
```

### 2. Check Function Status

```bash
curl http://localhost:18080/v1/functions/550e8400-e29b-41d4-a716-446655440000
```

Response:
```json
{
  "item": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "hello-world",
    "runtime": "node",
    "state": "ready",
    "vm_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
    "guest_ip": "10.0.0.42",
    "port": 3000,
    "vcpu": 1,
    "memory_mb": 256,
    ...
  }
}
```

**States:**
- `creating` - VM is being spawned
- `ready` - Function is ready to invoke
- `error` - VM creation failed

### 3. Invoke Function

```bash
curl -X POST http://localhost:18080/v1/functions/550e8400-e29b-41d4-a716-446655440000/invoke \
  -H "Content-Type: application/json" \
  -d '{
    "event": {
      "name": "World"
    }
  }'
```

Response:
```json
{
  "request_id": "123e4567-e89b-12d3-a456-426614174000",
  "status": "success",
  "duration_ms": 45,
  "response": {
    "message": "Hello World"
  },
  "logs": [
    "[Runtime] Invoking function with event: {\"name\":\"World\"}"
  ]
}
```

### 4. View Invocation Logs

```bash
curl http://localhost:18080/v1/functions/550e8400-e29b-41d4-a716-446655440000/logs
```

Response:
```json
{
  "items": [
    {
      "id": "...",
      "function_id": "550e8400-e29b-41d4-a716-446655440000",
      "status": "success",
      "duration_ms": 45,
      "memory_used_mb": null,
      "request_id": "123e4567-e89b-12d3-a456-426614174000",
      "event": {"name": "World"},
      "response": {"message": "Hello World"},
      "logs": ["..."],
      "invoked_at": "2025-10-21T10:30:00Z"
    }
  ]
}
```

### 5. Monitor Function VM

Since each function has a VM, you can see it in the VMs list:

```bash
curl http://localhost:18080/v1/vms
```

The function VM will be named like `fn-hello-world-550e8400`.

You can also see real-time metrics:
```bash
# Connect to WebSocket for metrics
ws://localhost:18080/v1/vms/{vm_id}/metrics/ws
```

### 6. Delete Function

```bash
curl -X DELETE http://localhost:18080/v1/functions/550e8400-e29b-41d4-a716-446655440000
```

This will:
1. Stop and delete the function's VM
2. Delete the function record
3. Cascade delete all invocation logs

## Example Functions

### Node.js - HTTP Request

```javascript
const https = require('https');

async function handler(event) {
  return new Promise((resolve, reject) => {
    https.get(event.url, (res) => {
      let data = '';
      res.on('data', (chunk) => data += chunk);
      res.on('end', () => resolve({ status: res.statusCode, data }));
    }).on('error', reject);
  });
}
```

### Python - Data Processing

```python
def handler(event):
    numbers = event.get('numbers', [])
    return {
        'sum': sum(numbers),
        'avg': sum(numbers) / len(numbers) if numbers else 0,
        'max': max(numbers) if numbers else None,
        'min': min(numbers) if numbers else None
    }
```

## Monitoring

### Function-Specific Metrics

Each function VM reports:
- **CPU Usage** - via guest agent
- **Memory Usage** - via guest agent
- **Network I/O** - via Firecracker
- **Disk I/O** - via Firecracker

### Invocation Metrics

Stored in database:
- Total invocations
- Success/error/timeout counts
- Average duration
- P50/P95/P99 latency (query from logs)

## Troubleshooting

### Function stuck in "creating" state

Check manager logs:
```bash
journalctl -u manager -f
```

Look for errors like:
- `Failed to create VM: ...`
- `Runtime image not found: /srv/images/node-runtime.ext4`

### Function returns "Function VM has no IP yet"

The guest agent hasn't reported IP yet. Wait a few seconds for the VM to boot.

Or check the VM manually:
```bash
curl http://localhost:18080/v1/vms/{vm_id}
```

### Invocation times out

- Check if runtime server is running in VM
- Try accessing it directly: `curl http://{guest_ip}:3000/health`
- Increase timeout_seconds when creating function

### Runtime server not starting

Connect to VM shell:
```bash
# Get shell credentials
curl http://localhost:18080/v1/vms/{vm_id}/shell

# Use WebSocket terminal or SSH
# Check service status:
rc-service runtime-server status
# Check logs:
cat /var/log/runtime-server.log
```

## Performance Tips

1. **Reuse functions** - Don't create/delete frequently, keep functions alive
2. **Right-size VMs** - Use 1 vCPU + 128-256MB for most functions
3. **Monitor cold starts** - First invocation is slower (VM boot time)
4. **Use connection pooling** - If function makes HTTP requests, reuse connections

## Next Steps

- **Hot reload** - Implement code updates without VM restart
- **Scaling** - Multiple VMs per function for parallel invocations
- **Snapshots** - Fast function VM cloning for cold starts
- **Build system** - Automated runtime image building
- **Package management** - Support npm/pip dependencies

## API Reference

See full API documentation at: `http://localhost:18080/docs`

All endpoints:
- `POST /v1/functions` - Create function
- `GET /v1/functions` - List functions
- `GET /v1/functions/{id}` - Get function
- `PUT /v1/functions/{id}` - Update function
- `DELETE /v1/functions/{id}` - Delete function
- `POST /v1/functions/{id}/invoke` - Invoke function
- `GET /v1/functions/{id}/logs` - Get logs
