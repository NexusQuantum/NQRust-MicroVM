+++
title = "Monitor Stats"
description = "Real-time resource usage and performance metrics"
weight = 34
date = 2025-12-18
+++

Monitor container resource usage in real-time with detailed performance metrics and visualizations.

---

## What are Container Stats?

Container stats provide real-time monitoring of resource usage:

![Image: Container stats overview](/images/containers/stats-overview.png)

**Metrics tracked**:
- ✅ **CPU Usage** - Percentage of allocated CPU used
- ✅ **Memory Usage** - Memory consumption (MB and %)
- ✅ **Network I/O** - Data sent and received
- ✅ **Disk I/O** - Disk read and write operations
- ✅ **Uptime** - How long container has been running

**Use stats to**:
- Identify resource bottlenecks
- Optimize resource allocation
- Monitor application performance
- Detect unusual behavior
- Plan capacity

---

## Accessing Container Stats

### From Container Detail Page

Navigate to the **Stats** tab:

![Image: Stats tab in detail page](/images/containers/stats-access-tab.png)

1. Open container detail page
2. Click **"Stats"** tab
3. View real-time metrics

**Available when**: Container is **Running**

---

### Direct URL

Access stats directly via URL:
```
/containers/{container-id}?tab=stats
```

---

## Stats Page Interface

The stats page displays multiple metric cards:

![Image: Stats page layout](/images/containers/stats-layout.png)

**Sections**:
1. **CPU Usage** - CPU consumption metrics
2. **Memory Usage** - RAM usage metrics
3. **Network** - Network I/O metrics
4. **Disk I/O** - Disk read/write metrics
5. **Container Info** - Uptime and configuration

---

## CPU Usage Metrics

Monitor CPU consumption:

![Image: CPU usage card](/images/containers/stats-cpu-card.png)

### CPU Percentage

**Current usage**:
- Displayed as percentage (e.g., "45%")
- Updated every 5 seconds
- Range: 0% to 100% per allocated vCPU

**Calculation**:
```
CPU % = (CPU time used / Total CPU time available) × 100
```

**Example**:
- Container allocated: 2 vCPU
- Using: 0.9 vCPU
- Display: 45% (0.9 / 2 = 0.45)

---

### CPU Chart

![Image: CPU usage chart](/images/containers/stats-cpu-chart.png)

**Chart features**:
- Line graph showing CPU % over time
- X-axis: Time (last 60 data points)
- Y-axis: CPU percentage (0-100%)
- Auto-updates every 5 seconds
- Smooth line interpolation

**Interpreting the chart**:

**Flat low line** (0-20%):
```
CPU: ▁▁▁▁▁▁▁ (idle)
```
- Container is idle or very light load
- Good: Resource allocation appropriate

**Steady medium line** (20-60%):
```
CPU: ▃▃▃▃▃▃▃ (normal)
```
- Normal operation under load
- Good: Expected behavior

**High usage** (60-90%):
```
CPU: ▆▆▆▆▆▆▆ (high)
```
- Container is working hard
- Monitor: May need more CPU

**Maxed out** (90-100%):
```
CPU: ▇▇▇▇▇▇▇ (maxed)
```
- CPU throttling
- Action: Increase CPU allocation

**Spiky pattern**:
```
CPU: ▁▇▁▁▇▁▇ (spiky)
```
- Bursty workload (requests, cron jobs)
- Normal for event-driven apps

---

### CPU Information Display

![Image: CPU info display](/images/containers/stats-cpu-info.png)

**Shows**:
- Current CPU %
- Allocated vCPUs (e.g., "2 vCPU")
- vCPU count from container configuration

**Example**:
```
CPU Usage: 45%
Allocated: 2 vCPU
Actual usage: 0.9 vCPU (45% of 2)
```

---

## Memory Usage Metrics

Monitor memory (RAM) consumption:

![Image: Memory usage card](/images/containers/stats-memory-card.png)

### Memory Percentage

**Current usage**:
- Displayed as percentage (e.g., "62%")
- Updated every 5 seconds
- Range: 0% to 100%

**Calculation**:
```
Memory % = (Used memory / Allocated memory) × 100
```

**Example**:
- Allocated: 2048 MB (2 GB)
- Used: 1270 MB
- Display: 62% (1270 / 2048)

---

### Memory Chart

![Image: Memory usage chart](/images/containers/stats-memory-chart.png)

**Chart features**:
- Line graph showing memory % over time
- X-axis: Time (last 60 data points)
- Y-axis: Memory percentage (0-100%)
- Auto-updates every 5 seconds

**Interpreting the chart**:

**Low steady line** (10-40%):
```
Memory: ▂▂▂▂▂▂▂ (low)
```
- Normal for lightweight apps
- Room for growth

**Medium steady line** (40-70%):
```
Memory: ▄▄▄▄▄▄▄ (medium)
```
- Normal for databases, caches
- Monitor for increases

**High usage** (70-90%):
```
Memory: ▆▆▆▆▆▆▆ (high)
```
- Approaching limit
- Consider increasing allocation

**Near limit** (90-100%):
```
Memory: ▇▇▇▇▇▇▇ (critical)
```
- Risk of OOM (Out of Memory)
- Action: Increase memory immediately

**Gradual increase**:
```
Memory: ▂▃▄▅▆▆▇ (leak?)
```
- Possible memory leak
- Investigate application code

**Sawtooth pattern**:
```
Memory: ▄▇▃▆▃▇▃ (GC active)
```
- Garbage collection cycles
- Normal for Java, Node.js, Python

---

### Memory Information Display

![Image: Memory info display](/images/containers/stats-memory-info.png)

**Shows**:
- Current memory %
- Used memory in MB
- Allocated memory in MB

**Example**:
```
Memory Usage: 62%
Used: 1270 MB
Allocated: 2048 MB
Available: 778 MB
```

---

## Network Metrics

Monitor network traffic:

![Image: Network metrics card](/images/containers/stats-network-card.png)

### Network Throughput

**Metrics displayed**:
- **Bytes In** - Data received by container
- **Bytes Out** - Data sent from container
- Updated every 5 seconds
- Cumulative since container start

**Example**:
```
Network In:  245 MB
Network Out: 128 MB
```

---

### Network Charts

**Inbound traffic chart**:

![Image: Network in chart](/images/containers/stats-network-in-chart.png)

Shows data received over time.

**Outbound traffic chart**:

![Image: Network out chart](/images/containers/stats-network-out-chart.png)

Shows data sent over time.

**Chart features**:
- Line graphs showing bytes/second
- Auto-scaling Y-axis
- Real-time updates

---

### Interpreting Network Patterns

**Low traffic** (KB/s):
```
Network: ▁▁▁▁▁▁▁ (idle)
```
- No activity or very light traffic
- Normal for idle services

**Steady traffic** (MB/s):
```
Network: ▃▃▃▃▃▃▃ (active)
```
- Consistent request rate
- Normal for web servers, APIs

**High spikes**:
```
Network: ▁▁▇▁▁▇▁ (bursty)
```
- Periodic large transfers
- Normal for batch jobs, backups

**Constant high traffic**:
```
Network: ▇▇▇▇▇▇▇ (saturated)
```
- Heavy network usage
- May indicate:
  - High request volume
  - Large data transfers
  - DDoS attack (if unexpected)

---

## Disk I/O Metrics

Monitor disk read and write operations:

![Image: Disk I/O card](/images/containers/stats-disk-card.png)

### Disk Metrics

**Displayed**:
- **Disk Read** - Data read from disk
- **Disk Write** - Data written to disk
- Cumulative since container start

**Example**:
```
Disk Read:  512 MB
Disk Write: 1.2 GB
```

---

### Disk I/O Charts

**Disk read chart**:

![Image: Disk read chart](/images/containers/stats-disk-read-chart.png)

**Disk write chart**:

![Image: Disk write chart](/images/containers/stats-disk-write-chart.png)

**Chart features**:
- Show bytes read/written over time
- Useful for identifying I/O patterns
- Real-time updates

---

### Interpreting Disk Patterns

**Low I/O**:
```
Disk: ▁▁▁▁▁▁▁ (minimal)
```
- In-memory workload
- Cached reads
- Normal for Redis, Memcached

**Moderate I/O**:
```
Disk: ▃▃▃▃▃▃▃ (normal)
```
- Regular database operations
- Normal for PostgreSQL, MySQL

**High write spikes**:
```
Write: ▁▁▇▁▁▇▁ (batch writes)
```
- Periodic database commits
- Batch processing
- Log rotation

**Constant high I/O**:
```
Disk: ▇▇▇▇▇▇▇ (I/O bound)
```
- Disk bottleneck
- Consider:
  - SSD instead of HDD
  - More memory for caching
  - Query optimization

---

## Container Information

Additional container details:

![Image: Container info section](/images/containers/stats-info-section.png)

### Uptime

**Shows**:
- How long container has been running
- Format: "2h 30m", "5d 12h", "23s"
- Resets on container restart

**Example**:
```
Uptime: 2 days 14 hours 32 minutes
```

---

### Resource Allocation

**Displays**:
- Allocated vCPUs
- Allocated memory (MB)
- Port mappings
- Volume mounts

**Example**:
```
Resources:
  CPU: 2 vCPU
  Memory: 2048 MB

Ports:
  8080:80 (TCP)
  8443:443 (TCP)

Volumes:
  /srv/postgres-data:/var/lib/postgresql/data
```

---

## Auto-Refresh

Stats automatically refresh:

![Image: Auto-refresh indicator](/images/containers/stats-auto-refresh.png)

**Refresh rate**: Every 5 seconds

**What updates**:
- All percentage values
- All charts
- All cumulative totals
- Uptime counter

**Manual refresh**:
- Click "Refresh" in page header
- Or reload page

---

## Performance Analysis

### Identifying CPU Bottlenecks

**Symptoms**:
- CPU at 90-100%
- Response times slow
- Request queue building up

![Image: CPU bottleneck chart](/images/containers/stats-analysis-cpu-bottleneck.png)

**Actions**:
1. **Check chart pattern**:
   - Constant high usage = under-provisioned
   - Spiky usage = bursty workload

2. **Increase CPU allocation**:
   - Stop container
   - Edit → Increase CPU
   - Start container

3. **Optimize application**:
   - Profile code
   - Optimize algorithms
   - Add caching
   - Use async operations

---

### Identifying Memory Leaks

**Symptoms**:
- Memory usage gradually increasing
- Eventually reaches 100%
- Container crashes (OOM killed)

![Image: Memory leak pattern](/images/containers/stats-analysis-memory-leak.png)

**Actions**:
1. **Confirm it's a leak**:
   - Watch chart over hours/days
   - Should see steady increase
   - No corresponding increase in load

2. **Restart container**:
   - Short-term fix
   - Memory resets
   - Leak will return

3. **Fix application**:
   - Profile memory usage
   - Find leak source
   - Fix code
   - Redeploy

**Common leak causes**:
- Unclosed database connections
- Event listeners not removed
- Cached data never expires
- Circular references (JavaScript)

---

### Right-Sizing Resources

**Goal**: Allocate just enough, not too much or too little

**Steps**:
1. **Monitor for 24+ hours**:
   - Let container run under normal load
   - Check stats regularly
   - Note peak usage times

2. **Calculate peak usage**:
   ```
   Peak CPU: 1.4 vCPU (70% of 2 vCPU)
   Peak Memory: 1536 MB (75% of 2048 MB)
   ```

3. **Add headroom**:
   ```
   Target: 60-80% usage at peak

   Current: 70% CPU, 75% memory
   Good: Resources well-sized

   If >90%: Increase allocation
   If <40%: Decrease allocation (save resources)
   ```

4. **Adjust**:
   - Stop container
   - Edit resources
   - Start and monitor again

---

### Network Performance

**Slow response times**:

**Check**:
1. **Network charts**:
   - High inbound = receiving lots of data
   - High outbound = sending lots of data

2. **Compare to expected**:
   - API should have moderate traffic
   - File server should have high traffic
   - Idle service should have low traffic

3. **Investigate unexpected patterns**:
   - Sudden spike = possible DDoS
   - Gradual increase = more users
   - Constant high = normal for service type

**Actions**:
- Add caching to reduce requests
- Optimize payload sizes (compress)
- Use CDN for static content
- Load balance across multiple containers

---

### Disk I/O Performance

**Slow database queries**:

**Check**:
1. **Disk write chart**:
   - High writes = many database commits
   - Spikes = batch operations

2. **Disk read chart**:
   - High reads = cache misses
   - Queries not hitting memory cache

**Actions**:
- Increase database memory (buffer pool)
- Add indexes for frequent queries
- Use SSD for storage volumes
- Optimize queries (EXPLAIN ANALYZE)

---

## Comparing Metrics

### Before and After Optimization

**Example: Optimizing database queries**

**Before**:
```
CPU: 85% (1.7 of 2 vCPU)
Memory: 80% (1638 of 2048 MB)
Disk Read: High (500 MB/min)
Response Time: 850ms average
```

![Image: Before optimization metrics](/images/containers/stats-compare-before.png)

**After** (added indexes, query optimization):
```
CPU: 35% (0.7 of 2 vCPU)
Memory: 80% (1638 of 2048 MB)
Disk Read: Low (50 MB/min)
Response Time: 120ms average
```

![Image: After optimization metrics](/images/containers/stats-compare-after.png)

**Result**:
- ✅ 50% CPU reduction
- ✅ 90% less disk I/O
- ✅ 7x faster responses
- ✅ Can handle more concurrent users

---

## Troubleshooting

### Issue: Stats Not Showing

**Symptoms**:
- Stats tab is empty
- Shows "No VM associated with this container"
- No charts displayed

![Image: No stats available](/images/containers/troubleshoot-no-stats.png)

**Cause**: Container doesn't have an associated VM (shouldn't happen normally)

**Solutions**:
1. **Refresh page**:
   - Click refresh button
   - Or press F5

2. **Check container state**:
   - Ensure container is Running
   - Stats only available for running containers

3. **Restart container**:
   - Stop container
   - Start container
   - Check stats again

4. **Check container VM**:
   - Click "View Container VM"
   - Verify VM exists and is running

---

### Issue: Metrics Not Updating

**Symptoms**:
- Charts frozen
- Numbers not changing
- Last update was minutes ago

![Image: Stale metrics](/images/containers/troubleshoot-stale-metrics.png)

**Solutions**:
1. **Refresh page**:
   - Press F5 or click refresh
   - Metrics should update

2. **Check container running**:
   - If stopped, no new metrics
   - Check Overview tab for state

3. **Check browser**:
   - Browser may throttle background tabs
   - Switch to tab to resume updates

4. **Check network**:
   - Network issue may prevent updates
   - Check browser console for errors

---

### Issue: Memory Always at 100%

**Symptoms**:
- Memory chart shows 100%
- Container doesn't crash
- Application seems to work fine

![Image: Memory at 100%](/images/containers/troubleshoot-memory-100.png)

**Possible causes**:

**1. Application uses all allocated memory (normal)**:
- Some apps (Redis, Memcached) use all available memory
- This is expected behavior
- Not a problem if container is stable

**2. Memory limit too low**:
- Application needs more memory
- Increase allocation to prevent OOM

**3. Monitoring delay**:
- Metrics may lag slightly
- Refresh to get latest data

**Actions**:
1. **Check logs**:
   - Look for OOM errors
   - If no errors, may be normal

2. **Increase memory**:
   - Stop container
   - Edit → Increase memory by 512 MB
   - Start and monitor

3. **Check application type**:
   - Caches should use all memory
   - Apps should have headroom

---

### Issue: CPU Spikes Without Load

**Symptoms**:
- Periodic CPU spikes
- No user activity
- Pattern repeats regularly

![Image: Periodic CPU spikes](/images/containers/troubleshoot-cpu-spikes.png)

**Possible causes**:

**1. Cron jobs / scheduled tasks**:
- Database cleanup
- Index rebuilding
- Cache warming
- Log rotation

**2. Health checks**:
- Load balancer pinging
- Monitoring probes
- Keep-alive checks

**3. Garbage collection**:
- Java GC cycles
- Node.js GC
- Python GC

**Actions**:
1. **Check logs during spike**:
   - View Logs tab
   - Note what happens at spike time
   - Look for scheduled task messages

2. **Verify it's expected**:
   - If cron job, normal behavior
   - If GC, tune GC settings
   - If health check, acceptable

3. **Optimize if needed**:
   - Move heavy tasks to off-peak hours
   - Optimize scheduled job performance
   - Tune GC parameters

---

## Best Practices

### Monitoring Strategy

✅ **Check stats regularly**:
- Daily for production containers
- Weekly for development containers
- After deployments
- When users report slowness

✅ **Establish baselines**:
```
Normal baseline (idle):
  CPU: 5-10%
  Memory: 30-40%
  Network: <1 MB/min

Normal baseline (active):
  CPU: 30-50%
  Memory: 50-70%
  Network: 10-50 MB/min
```

✅ **Set alert thresholds** (mental):
```
Warning:
  CPU >70% for >5 minutes
  Memory >80%
  Disk I/O very high

Critical:
  CPU >90%
  Memory >95%
  OOM errors in logs
```

---

### Resource Optimization

✅ **Start conservative**:
```
Initial allocation:
  Small app:  0.5 vCPU, 512 MB
  Medium app: 1 vCPU, 1024 MB
  Database:   2 vCPU, 2048 MB
```

✅ **Monitor and adjust**:
```
After 24 hours:
  - Check peak usage
  - Add 20% headroom
  - Adjust allocation
```

✅ **Iterate**:
```
Week 1: Deploy with baseline
Week 2: Monitor, adjust if needed
Week 3: Monitor, fine-tune
Week 4: Stable, periodic checks
```

---

### Performance Tuning

✅ **Use stats to guide optimization**:
1. **High CPU**:
   - Profile code
   - Optimize algorithms
   - Add caching
   - Increase CPU allocation

2. **High Memory**:
   - Check for leaks
   - Optimize data structures
   - Tune garbage collection
   - Increase memory allocation

3. **High Network**:
   - Compress responses
   - Cache frequent requests
   - Optimize payloads
   - Use CDN

4. **High Disk I/O**:
   - Add database indexes
   - Increase memory cache
   - Optimize queries
   - Use SSD volumes

---

### Documentation

✅ **Document your baselines**:
```
Container: prod-api
Normal CPU: 30-40%
Normal Memory: 60%
Peak hours: 9am-5pm
Peak CPU: 60-70%
Resource allocation: 2 vCPU, 2048 MB
Last optimized: 2025-12-18
```

✅ **Track changes**:
```
2025-12-01: Initial: 1 vCPU, 1024 MB
2025-12-08: Increased to 2 vCPU (CPU at 95%)
2025-12-15: Increased to 2048 MB (memory leak fixed)
```

---

## Quick Reference

### Metric Update Frequency

| Metric | Update Interval |
|--------|-----------------|
| CPU % | 5 seconds |
| Memory % | 5 seconds |
| Network I/O | 5 seconds |
| Disk I/O | 5 seconds |
| Uptime | 5 seconds |

### Resource Allocation Guidelines

| Service Type | CPU | Memory |
|--------------|-----|--------|
| Static site | 0.5 vCPU | 256-512 MB |
| API server | 1-2 vCPU | 512-1024 MB |
| Database (small) | 2 vCPU | 2048 MB |
| Database (medium) | 2-4 vCPU | 4096-8192 MB |
| Cache (Redis) | 0.5-1 vCPU | 512-2048 MB |
| Message queue | 1-2 vCPU | 1024-2048 MB |

### Healthy Ranges

| Metric | Healthy Range | Warning | Critical |
|--------|---------------|---------|----------|
| CPU | 20-70% | 70-90% | >90% |
| Memory | 30-80% | 80-95% | >95% |
| Network | Varies | N/A | Sustained max |
| Disk I/O | Varies | N/A | Constant high |

---

## Next Steps

- **[View Logs](../logs/)** - Debug issues identified in stats
- **[Manage Containers](../manage-containers/)** - Adjust resources based on stats
- **[Deploy a Container](../deploy-container/)** - Apply lessons learned
- **[Container Overview](../)** - Learn more about containers
