# Guest Agent

Lightweight Rust-based guest agent for collecting real CPU and memory metrics from inside Firecracker VMs.

## Features

- **Real Guest Metrics**: Reads actual CPU and memory usage from `/proc/stat` and `/proc/meminfo`
- **Lightweight**: 2.2MB static binary, minimal resource usage
- **HTTP API**: Simple REST endpoint for metrics collection
- **Alpine Linux Compatible**: Static musl binary works out of the box

## Building

Build the static binary for Alpine Linux:

```bash
cargo build --release --bin guest-agent --target x86_64-unknown-linux-musl
```

The binary will be located at: `target/x86_64-unknown-linux-musl/release/guest-agent`

## Installation in VM

### Method 1: Copy via SCP (if VM has network)

```bash
# From host
scp target/x86_64-unknown-linux-musl/release/guest-agent root@<vm-ip>:/usr/local/bin/
```

### Method 2: Base64 Transfer (via terminal)

```bash
# On host - encode binary
base64 target/x86_64-unknown-linux-musl/release/guest-agent > guest-agent.b64

# In VM terminal - decode and install
cat > /tmp/guest-agent.b64 << 'EOF'
<paste base64 content here>
EOF

base64 -d /tmp/guest-agent.b64 > /usr/local/bin/guest-agent
chmod +x /usr/local/bin/guest-agent
rm /tmp/guest-agent.b64
```

### Method 3: HTTP Server (easiest)

```bash
# On host - serve the binary
cd target/x86_64-unknown-linux-musl/release
python3 -m http.server 8000

# In VM - download
wget http://<host-ip>:8000/guest-agent -O /usr/local/bin/guest-agent
chmod +x /usr/local/bin/guest-agent
```

## Running

### Manual Start

```bash
/usr/local/bin/guest-agent
```

The agent will listen on `0.0.0.0:8080`.

### Auto-Start with OpenRC (Alpine Linux)

Create service file `/etc/init.d/guest-agent`:

```bash
#!/sbin/openrc-run

name="guest-agent"
command="/usr/local/bin/guest-agent"
command_background=true
pidfile="/run/${RC_SVCNAME}.pid"
output_log="/var/log/guest-agent.log"
error_log="/var/log/guest-agent.err"

depend() {
    need net
    after firewall
}
```

Enable and start:

```bash
chmod +x /etc/init.d/guest-agent
rc-update add guest-agent default
rc-service guest-agent start
```

## API Endpoints

### GET /metrics

Returns current guest metrics:

```bash
curl http://localhost:8080/metrics
```

Response:

```json
{
  "cpu_usage_percent": 45.2,
  "memory_usage_percent": 62.5,
  "memory_used_kb": 512000,
  "memory_total_kb": 819200,
  "memory_available_kb": 307200,
  "uptime_seconds": 12345
}
```

### GET /health

Health check endpoint:

```bash
curl http://localhost:8080/health
```

Response: `OK`

## Metrics Explanation

- **cpu_usage_percent**: Real CPU usage inside the guest (0-100%)
- **memory_usage_percent**: Memory usage percentage
- **memory_used_kb**: Used memory in kilobytes
- **memory_total_kb**: Total memory in kilobytes
- **memory_available_kb**: Available memory in kilobytes
- **uptime_seconds**: System uptime

## Integration with Manager

The manager will automatically query `http://<vm-ip>:8080/metrics` to get guest metrics and combine them with Firecracker's network/disk metrics.
