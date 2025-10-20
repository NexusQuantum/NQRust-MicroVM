# Firecracker Metrics Debugging Guide

## Current Status

Based on logs from 2025-10-20:

### Working ✅
- **Network RX/TX**: Shows graph init in UI (some data coming through)
- **Disk Write**: Shows spikes in logs: `"disk_write_bytes": Number(10240)`, `Number(1024)`

### Not Working ❌
- **CPU Usage**: Always 0
- **Memory Usage**: Always 0 (Firecracker doesn't expose this)
- **Disk Read**: Needs verification

## Firecracker Metrics Fields (from actual logs)

```json
{
  "utc_timestamp_ms": 1760933283502,
  "api_server": { ... },
  "balloon": { ... },
  "block_rootfs": {
    "activate_fails": 0,
    "cfg_fails": 0,
    "no_avail_buffer": 0,
    "event_fails": 0,
    "execute_fails": 0,
    "invalid_reqs_count": 0,
    "flush_count": 0,
    "queue_event_count": 1,
    "rate_limiter_event_count": 0,
    "update_count": ...,
    "read_bytes": ???,      // Need to verify
    "write_bytes": ???      // Need to verify
  },
  "net_eth0": {
    "rx_bytes_count": 218,
    "tx_bytes_count": 110,
    "rx_packets_count": 3,
    "tx_packets_count": 1,
    ...
  }
}
```

## Next Steps to Debug

1. **Get full metrics JSON**: Save complete output (not truncated at 500 chars)
2. **Check field names**: Verify exact field names for disk metrics
3. **Fix CPU metrics**: Find correct vCPU fields
4. **Document memory limitation**: Firecracker doesn't expose memory usage

## How to Get Full Metrics

SSH into a running VM's host and run:
```bash
# Find VM ID
VM_ID="your-vm-id-here"

# Read one metrics flush
timeout 5 cat /srv/fc/vms/$VM_ID/logs/metrics.json > /tmp/full-metrics.json &
sleep 1
curl -X PUT http://localhost:8080/v1/vms/$VM_ID/flush-metrics
wait

# View full output
cat /tmp/full-metrics.json | jq .
```

This will show ALL available metrics fields so we can map them correctly.
