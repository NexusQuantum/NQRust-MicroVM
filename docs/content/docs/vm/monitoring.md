+++
title = "Monitoring"
description = "Monitor VM performance and resource usage"
weight = 45
date = 2025-12-16
+++

Learn how to monitor your virtual machines' performance, resource usage, and health.

---

## VM Metrics Overview

The NQRust-MicroVM dashboard provides real-time monitoring for:

- **CPU Usage** - Processor utilization
- **Memory Usage** - RAM consumption
- **Network Traffic** - Incoming/outgoing data
- **Disk I/O** - Read/write operations
- **Uptime** - How long VM has been running

**[IMAGE: vm-metrics-overview.png - VM detail page showing all metrics]**

---

## Viewing VM Metrics

### Access Metrics Dashboard

1. Navigate to **Virtual Machines** page
2. Click on a VM to open details
3. The **Overview** tab shows real-time metrics

**[IMAGE: vm-overview-metrics.png - Overview tab with live metrics]**

### Metrics Tab

For detailed metrics, click the **Metrics** tab:

**[IMAGE: vm-metrics-tab.png - Dedicated metrics tab with detailed graphs]**

This shows:
- Larger graphs for better visibility
- Historical data (last hour, day, week)
- Multiple metrics side-by-side
- Export options

---

## CPU Monitoring

### CPU Usage Graph

Shows percentage of CPU resources used by the VM:

**[IMAGE: cpu-usage-graph.png - CPU utilization over time]**

**Graph details**:
- **Y-axis**: 0% to 100% usage
- **X-axis**: Time (auto-updates)
- **Line color**:
  - 🟢 Green: < 50% (healthy)
  - 🟡 Yellow: 50-80% (moderate)
  - 🔴 Red: > 80% (high)

### Understanding CPU Metrics

**Normal usage** (1 vCPU VM):
```
Idle:         2-5%
Light work:   10-30%
Medium work:  30-60%
Heavy work:   60-100%
```

**What to look for**:
- **Consistent high CPU** (>80%) - May need more vCPUs
- **Spikes** - Normal for batch jobs
- **Always 100%** - VM is CPU-constrained
- **Always near 0%** - VM may be over-provisioned

### CPU Actions

**If CPU is too high**:
1. Go to VM settings
2. Increase vCPU count
3. Restart VM to apply changes

**[IMAGE: vm-increase-cpu.png - CPU adjustment in settings]**

---

## Memory Monitoring

### Memory Usage Graph

Shows RAM consumption over time:

**[IMAGE: memory-usage-graph.png - Memory utilization graph]**

**Graph details**:
- **Y-axis**: Memory in MiB
- **Shows**:
  - Used memory (blue line)
  - Total allocated (dashed line)
  - Available (difference)

### Understanding Memory Metrics

**Example** (2048 MiB VM):
```
Used:      1200 MiB (60%)
Cached:     600 MiB (30%)
Free:       248 MiB (12%)
```

**What to look for**:
- **Consistently high** (>90%) - Need more RAM
- **Swap usage** - Critical: add RAM immediately
- **Memory leaks** - Gradual increase over time
- **Sudden spikes** - Memory-intensive operations

### Memory Warnings

**[IMAGE: memory-warning.png - Warning indicator when memory is high]**

You'll see warnings when:
- ⚠️ Memory > 80% - Consider adding RAM
- 🔴 Memory > 95% - Add RAM urgently
- 🔴 Swap active - System slowing down

### Memory Actions

**If memory is too high**:
1. Stop the VM
2. Go to VM settings
3. Increase memory allocation
4. Start the VM

**Note**: Changing memory requires VM restart

---

## Network Monitoring

### Network Traffic Graph

Shows network activity in/out:

**[IMAGE: network-traffic-graph.png - Network I/O graph with inbound/outbound]**

**Graph shows**:
- 🔵 **Inbound** (download) - Blue line
- 🟢 **Outbound** (upload) - Green line
- **Units**: KB/s or MB/s

### Understanding Network Metrics

**Typical usage**:
```
Web browsing:    50-500 KB/s
File download:   1-10 MB/s
Video streaming: 2-5 MB/s
Database sync:   5-50 MB/s
```

**What to look for**:
- **Unexpected spikes** - Possible attack or data leak
- **Consistently high** - Normal for web servers
- **Zero activity** - Service may be down
- **Asymmetric** - Expected for servers (more outbound)

### Network Details

Click on network graph to see details:

**[IMAGE: network-details.png - Detailed network statistics]**

Details include:
- Total bytes sent/received
- Packets sent/received
- Errors and drops
- Active connections

---

## Disk I/O Monitoring

### Disk Activity Graph

Shows read/write operations:

**[IMAGE: disk-io-graph.png - Disk I/O graph]**

**Graph shows**:
- 🔵 **Read** - Data read from disk
- 🟢 **Write** - Data written to disk
- **Units**: KB/s or MB/s

### Understanding Disk Metrics

**Typical I/O**:
```
Idle:        < 1 MB/s
Light:       1-10 MB/s
Moderate:    10-50 MB/s
Heavy:       50-200 MB/s
```

**What to look for**:
- **High writes** - Database updates, logging
- **High reads** - File serving, queries
- **Sustained high I/O** - May need faster storage
- **Spikes** - Normal for batch operations

---

## System Health

### Health Status Indicator

VM health is shown with color-coded status:

**[IMAGE: vm-health-indicators.png - Different health statuses]**

| Status | Indicator | Meaning |
|--------|-----------|---------|
| **Healthy** | 🟢 Green | All metrics normal |
| **Warning** | 🟡 Yellow | Some metrics elevated |
| **Critical** | 🔴 Red | Immediate attention needed |
| **Unknown** | ⚪ Grey | Metrics unavailable |

### Health Checks

The system monitors:
- ✅ CPU usage < 90%
- ✅ Memory usage < 90%
- ✅ Disk space > 10% free
- ✅ Network responding
- ✅ Services running

### Alerts

When health degrades, you'll see alerts:

**[IMAGE: vm-health-alert.png - Health alert notification]**

Example alerts:
```
⚠️  High Memory Usage (92%)
🔴 Critical: Disk Almost Full (95%)
⚠️  CPU Usage Elevated (85%)
```

Click alert for recommended actions.

---

## Uptime & Performance

### Uptime Display

Shows how long VM has been running:

**[IMAGE: vm-uptime.png - Uptime counter]**

**Format**: `DD days HH:MM:SS`

**Examples**:
```
00:15:30  - 15 minutes
01:23:45  - 1 hour 23 minutes
5d 12:30:00 - 5 days, 12.5 hours
```

### Performance Metrics

**[IMAGE: vm-performance-summary.png - Performance summary panel]**

Summary panel shows:
- **Current CPU**: 45%
- **Current Memory**: 1200 MiB / 2048 MiB
- **Network In**: 150 KB/s
- **Network Out**: 80 KB/s
- **Uptime**: 5 days 12:30:00

---

## Historical Data

### Time Range Selector

View metrics over different periods:

**[IMAGE: metrics-timerange.png - Time range selector]**

Options:
- **Last hour** - Real-time monitoring
- **Last 6 hours** - Recent trends
- **Last 24 hours** - Daily patterns
- **Last 7 days** - Weekly overview
- **Last 30 days** - Monthly analysis

### Viewing Trends

**[IMAGE: metrics-weekly-trend.png - 7-day CPU trend]**

Historical data helps identify:
- Peak usage times
- Resource trends
- Capacity planning needs
- Performance degradation

---

## Resource Optimization

### Identifying Over-Provisioned VMs

Signs VM has too many resources:

**[IMAGE: vm-overprovisioned.png - Low utilization metrics]**

- CPU consistently < 10%
- Memory usage < 30%
- No spikes or peaks
- Uptime with minimal activity

**Action**: Reduce resources to save capacity

### Identifying Under-Provisioned VMs

Signs VM needs more resources:

**[IMAGE: vm-underprovisioned.png - High utilization metrics]**

- CPU consistently > 80%
- Memory usage > 90%
- Frequent swap usage
- Services slow to respond

**Action**: Increase resources for better performance

---

## Monitoring Multiple VMs

### VMs List View

The VMs list shows key metrics for all VMs:

**[IMAGE: vms-list-metrics.png - VMs list with inline metrics]**

For each VM:
- Status indicator
- Current CPU %
- Current Memory usage
- Uptime

**Quick scan** to find problem VMs:
- 🔴 Red indicators = needs attention
- 🟡 Yellow = monitor closely
- 🟢 Green = healthy

### Dashboard Overview

The main dashboard shows aggregate metrics:

**[IMAGE: dashboard-overview.png - Main dashboard with all VMs]**

**Total across all VMs**:
- Total CPU usage
- Total memory usage
- Total network traffic
- Number of VMs by status

---

## Troubleshooting with Metrics

### High CPU Investigation

**Problem**: CPU at 100%

**Steps**:
1. Access VM console/SSH
2. Check processes:
   ```bash
   top
   # Press 'P' to sort by CPU
   ```
3. Identify culprit process
4. Investigate why it's using so much CPU
5. Either:
   - Optimize the application
   - Add more vCPUs
   - Stop unnecessary processes

**[IMAGE: troubleshoot-high-cpu.png - Top command showing processes]**

### Memory Exhaustion

**Problem**: Memory at 100%, VM slow

**Steps**:
1. Check memory usage:
   ```bash
   free -h
   htop
   ```
2. Find memory-hungry processes:
   ```bash
   ps aux --sort=-%mem | head
   ```
3. Actions:
   - Restart memory-leaking services
   - Clear caches
   - Add more RAM
   - Kill unnecessary processes

**[IMAGE: troubleshoot-high-memory.png - Memory usage in htop]**

### Network Issues

**Problem**: High network usage or no activity

**Check connections**:
```bash
# Active connections
netstat -tupln

# Network statistics
ss -s

# Bandwidth by process
nethogs
```

**[IMAGE: troubleshoot-network.png - Network diagnostic commands]**

---

## Best Practices

**Regular Monitoring**:
- ✅ Check VMs daily for anomalies
- ✅ Review weekly trends
- ✅ Set up alerts for critical metrics
- ✅ Document normal baseline

**Resource Planning**:
- ✅ Monitor trends before adding VMs
- ✅ Plan capacity based on peak usage
- ✅ Leave 20% headroom for spikes
- ✅ Regular capacity reviews

**Performance Optimization**:
- ✅ Right-size VMs based on actual usage
- ✅ Scale up slowly, test impact
- ✅ Monitor after changes
- ✅ Document performance baselines

**Alerting** (if available):
- ✅ Set alerts for >80% CPU sustained
- ✅ Alert on >90% memory
- ✅ Alert on disk >90% full
- ✅ Alert on VM state changes

---

## Next Steps

- **[Manage VM](manage-vm/)** - VM lifecycle operations
- **[Backup & Snapshot](backup-snapshot/)** - Protect your VMs
- **[Access VM](access-vm/)** - Connect to investigate issues
