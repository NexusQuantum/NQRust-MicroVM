+++
title = "View Logs"
description = "Real-time container log streaming and debugging"
weight = 33
date = 2025-12-18
+++

Monitor container output, debug errors, and track application behavior with real-time log streaming.

---

## What are Container Logs?

Container logs capture all output from your Docker container:

![Image: Container logs overview](/images/containers/logs-overview.png)

**What's captured**:
- ✅ **stdout** - Standard output (console.log, print, echo)
- ✅ **stderr** - Error output (exceptions, warnings)
- ✅ **Docker daemon** - Container lifecycle events
- ✅ **Application logs** - Your application's log output

**Not captured**:
- ❌ Logs written to files inside container (unless volume-mounted)
- ❌ Logs before streaming started (historical logs limited)

---

## Accessing Container Logs

### From Container Table

Click the **Logs** icon (📄) in the Actions column:

![Image: Logs button in container table](/images/containers/logs-access-table.png)

Opens the container detail page on the Logs tab.

---

### From Container Detail Page

Navigate to the **Logs** tab:

![Image: Logs tab in detail page](/images/containers/logs-access-detail.png)

1. Open container detail page
2. Click **"Logs"** tab

---

### Direct URL

Access logs directly via URL:
```
/containers/{container-id}?tab=logs
```

---

## Logs Page Interface

The logs page has three main sections:

![Image: Logs page layout](/images/containers/logs-layout.png)

1. **Card Header** - Title and controls
2. **Control Bar** - Stream controls, auto-scroll, download
3. **Log Viewer** - Scrollable log output area

---

## Start Log Streaming

Logs don't stream automatically. You must start streaming:

### Initial State

When you first open the Logs tab:

<!-- ![Image: Logs initial state](/images/containers/logs-initial-state.png) -->

You'll see:
```
Click 'Start Stream' to begin streaming logs
```

---

### Start Streaming

Click the **"Start Stream"** button:

![Image: Start stream button](/images/containers/logs-start-button.png)

**What happens**:
1. Button changes to "Connecting..." with spinner
2. WebSocket connection established
3. Button changes to "Stop Stream"
4. Logs start appearing in real-time

![Image: Streaming active state](/images/containers/logs-streaming-active.png)

---

### Connection States

<!-- **Connecting**:
```
[Connecting...] (spinner icon)
```
![Image: Connecting state](/images/containers/logs-connecting.png) -->

**Connected and Streaming**:
```
[Stop Stream] (square icon)
```
![Image: Stop stream button](/images/containers/logs-stop-button.png)

**Waiting for Logs**:
```
Waiting for logs...
```
![Image: Waiting for logs](/images/containers/logs-waiting.png)

---

## Log Entry Format

Each log entry shows:

![Image: Log entry format](/images/containers/logs-entry-format.png)

**Components**:

1. **Timestamp** - When log was generated
   ```
   [2025-12-18 14:30:45.123]
   ```
   - Format: YYYY-MM-DD HH:MM:SS.mmm
   - Your local timezone

2. **Stream** - stdout or stderr
   ```
   [stdout]  (blue)
   [stderr]  (blue)
   ```

3. **Message** - The actual log content
   ```
   Container started successfully
   ```

**Color coding**:
- **stdout** - Normal text color (black/white)
- **stderr** - Red text color

---

### Example Log Entries

**Normal output (stdout)**:
```
[2025-12-18 14:30:45.123] [stdout] Starting Nginx 1.25.3
[2025-12-18 14:30:45.234] [stdout] Listening on port 80
[2025-12-18 14:30:45.345] [stdout] Server ready to accept connections
```

![Image: Stdout log examples](/images/containers/logs-stdout-example.png)

---

**Error output (stderr)**:
```
[2025-12-18 14:31:22.456] [stderr] Error: Database connection failed
[2025-12-18 14:31:22.567] [stderr]   at connect (db.js:45:10)
[2025-12-18 14:31:22.678] [stderr]   at startup (index.js:12:5)
```

![Image: Stderr log examples](/images/containers/logs-stderr-example.png)

---

## Log Streaming Controls

### Auto-Scroll

Keep latest logs in view automatically:

![Image: Auto-scroll toggle](/images/containers/logs-autoscroll.png)

**How it works**:
- ☑ **Enabled** - Page scrolls to bottom as new logs arrive
- ☐ **Disabled** - Stays at current scroll position

**Toggle**:
- Click the switch next to "Auto-scroll" label
- Or manually scroll to bottom to enable
- Scroll up to disable

**Use cases**:
- **Enabled**: Monitor live logs, watch real-time output
- **Disabled**: Read specific log section, review errors

---

### Stop Streaming

Stop the log stream:

![Image: Stop stream button](/images/containers/logs-stop-stream.png)

Click **"Stop Stream"** button:
- WebSocket connection closes
- No new logs appear
- Existing logs remain visible
- Can start streaming again anytime

**Use cases**:
- Reduce bandwidth usage
- Read logs without new entries
- Prevent log overflow

---

### Download Logs

Save logs to a text file:

![Image: Download logs button](/images/containers/logs-download.png)

Click **"Download"** button:
- Downloads `container-{id}-logs.txt`
- Contains all currently visible logs
- Format: `[timestamp] [stream] message`
- One line per log entry

**Example downloaded file**:
```
[2025-12-18 14:30:45.123] [stdout] Starting Nginx 1.25.3
[2025-12-18 14:30:45.234] [stdout] Listening on port 80
[2025-12-18 14:31:22.456] [stderr] Error: Database connection failed
```

**Use cases**:
- Offline analysis
- Share with team
- Archive for later reference
- Parse with scripts (grep, awk, etc.)

**Button state**:
- **Enabled** - When logs exist
- **Disabled** - When no logs (empty)

---

## Log Viewer Area

The scrollable log display:

![Image: Log viewer area](/images/containers/logs-viewer.png)

**Features**:
- Monospace font for readability
- Dark background (light theme) or darker background (dark theme)
- Maximum height: 600px
- Vertical scrollbar when logs exceed height
- Auto-scroll to bottom (when enabled)

---

### Empty State

When no logs are available:

![Image: Empty logs state](/images/containers/logs-empty.png)

**Messages**:

**Not streaming**:
```
Click 'Start Stream' to begin streaming logs
```

**Streaming but no logs**:
```
Waiting for logs...
```

**Possible reasons for no logs**:
- Container just started (hasn't logged anything yet)
- Application doesn't log to stdout/stderr
- Logs written to files instead of console

---

## Common Log Patterns

### Web Server Logs (Nginx)

![Image: Nginx logs example](/images/containers/logs-pattern-nginx.png)

```
[2025-12-18 14:30:45.000] [stdout] 2025/12/18 14:30:45 [notice] 1#1: using the "epoll" event method
[2025-12-18 14:30:45.001] [stdout] 2025/12/18 14:30:45 [notice] 1#1: nginx/1.25.3
[2025-12-18 14:30:45.002] [stdout] 2025/12/18 14:30:45 [notice] 1#1: start worker processes
[2025-12-18 14:30:45.003] [stdout] 2025/12/18 14:30:45 [notice] 1#1: start worker process 29
[2025-12-18 14:32:15.123] [stdout] 172.16.0.1 - - [18/Dec/2025:14:32:15 +0000] "GET / HTTP/1.1" 200 615
```

**Pattern**:
- Startup notices
- Worker process info
- Access logs (IP, method, path, status)

---

### Database Logs (PostgreSQL)

![Image: PostgreSQL logs example](/images/containers/logs-pattern-postgres.png)

```
[2025-12-18 14:30:50.000] [stdout] PostgreSQL Database directory appears to contain a database; Skipping initialization
[2025-12-18 14:30:50.123] [stdout] 2025-12-18 14:30:50.123 UTC [1] LOG:  starting PostgreSQL 15.3
[2025-12-18 14:30:50.234] [stdout] 2025-12-18 14:30:50.234 UTC [1] LOG:  listening on IPv4 address "0.0.0.0", port 5432
[2025-12-18 14:30:50.345] [stdout] 2025-12-18 14:30:50.345 UTC [1] LOG:  database system is ready to accept connections
[2025-12-18 14:32:30.000] [stdout] 2025-12-18 14:32:30.000 UTC [45] LOG:  connection received: host=172.16.0.5 port=54321
```

**Pattern**:
- Initialization status
- Server startup
- Listening addresses
- Connection events

---

### Application Logs (Node.js)

![Image: Node.js logs example](/images/containers/logs-pattern-nodejs.png)

```
[2025-12-18 14:30:55.000] [stdout] Server starting...
[2025-12-18 14:30:55.123] [stdout] Environment: production
[2025-12-18 14:30:55.234] [stdout] Connecting to database...
[2025-12-18 14:30:55.456] [stdout] Database connected successfully
[2025-12-18 14:30:55.567] [stdout] Listening on port 3000
[2025-12-18 14:32:45.000] [stdout] GET /api/users 200 45ms
[2025-12-18 14:32:50.123] [stderr] Warning: Deprecated API usage
```

**Pattern**:
- Application lifecycle
- Environment info
- Database connections
- HTTP request logs
- Warnings and errors

---

### Error with Stack Trace

![Image: Error stack trace](/images/containers/logs-pattern-error.png)

```
[2025-12-18 14:35:12.000] [stderr] Error: Connection timeout
[2025-12-18 14:35:12.001] [stderr]   at Timeout._onTimeout (/app/lib/db.js:123:15)
[2025-12-18 14:35:12.002] [stderr]   at listOnTimeout (node:internal/timers:559:17)
[2025-12-18 14:35:12.003] [stderr]   at processTimers (node:internal/timers:502:7)
[2025-12-18 14:35:12.100] [stderr] Application shutting down due to fatal error
```

**Pattern**:
- Error message
- Stack trace (file, line, function)
- Shutdown messages

---

## Debugging with Logs

### Finding Startup Errors

**Scenario**: Container starts then immediately stops

**Steps**:
1. **Start streaming quickly**:
   - Open Logs tab
   - Click "Start Stream" immediately
   - Watch for startup messages

2. **Look for errors**:
   - Scroll to top of logs
   - Find red stderr entries
   - Identify error message

3. **Common startup errors**:
   ```
   Missing env var:
   [stderr] Error: POSTGRES_PASSWORD must be set

   Port already in use:
   [stderr] Error: bind EADDRINUSE 0.0.0.0:3000

   Missing dependency:
   [stderr] Error: Cannot find module 'express'
   ```

4. **Fix and retry**:
   - Note the error
   - Fix configuration (env vars, ports, etc.)
   - Restart container
   - Watch logs again

---

### Debugging Application Errors

**Scenario**: Application runs but has errors

**Steps**:
1. **Enable streaming**:
   - Start log stream
   - Reproduce the error
   - Watch logs for error messages

2. **Identify error pattern**:
   ```
   Request-specific error:
   [stdout] GET /api/users 500 123ms
   [stderr] Error: Database query failed

   Periodic error:
   [stderr] Error: Connection lost
   (appears every few minutes)
   ```

3. **Analyze stack trace**:
   - Find file and line number
   - Understand error cause
   - Check related code

4. **Add debug logging**:
   - Modify application code
   - Add more console.log/print statements
   - Rebuild and redeploy container
   - Watch for debug output

---

### Monitoring for Warnings

**Scenario**: Application works but shows warnings

**Steps**:
1. **Stream logs continuously**:
   - Enable auto-scroll
   - Watch for yellow/warning messages

2. **Common warnings**:
   ```
   Deprecated API:
   [stderr] Warning: crypto.createCipher is deprecated

   Resource limits:
   [stderr] Warning: Memory usage high (450/512 MB)

   Configuration:
   [stdout] Warning: Using default config, consider setting API_KEY
   ```

3. **Assess severity**:
   - Some warnings are informational
   - Others indicate future problems
   - Prioritize based on impact

4. **Address warnings**:
   - Update deprecated code
   - Adjust resource limits
   - Fix configuration

---

### Performance Debugging

**Scenario**: Application is slow

**Steps**:
1. **Enable timestamps**:
   - Logs already have timestamps
   - Compare log timestamps to find delays

2. **Look for slow operations**:
   ```
   [stdout] [2025-12-18 14:30:00.000] Database query started
   [stdout] [2025-12-18 14:30:05.456] Database query completed
   (5.4 seconds delay!)
   ```

3. **Identify bottlenecks**:
   - Database queries
   - External API calls
   - File I/O operations
   - CPU-intensive tasks

4. **Add performance logging**:
   ```javascript
   console.log(`[PERF] Query started at ${Date.now()}`);
   const result = await db.query(...);
   console.log(`[PERF] Query completed in ${Date.now() - start}ms`);
   ```

5. **Optimize**:
   - Add indexes to database
   - Cache frequent queries
   - Optimize algorithms
   - Increase resources (CPU/memory)

---

## Troubleshooting

### Issue: No Logs Appearing

**Symptoms**:
- Started streaming
- Shows "Waiting for logs..."
- Container is running
- No logs appear

![Image: No logs issue](/images/containers/troubleshoot-no-logs.png)

**Possible causes**:

**1. Application logs to files, not stdout**
- Some apps write logs to /var/log instead of console
- Solution: Configure app to log to stdout/stderr

**2. Container just started**
- App hasn't logged anything yet
- Solution: Wait a few seconds, trigger app activity

**3. Logging disabled**
- App has logging disabled
- Solution: Enable logging in app configuration

**4. Wrong log level**
- App only logs errors, not info
- Solution: Set log level to DEBUG or INFO

---

### Issue: Logs Stream Disconnects

**Symptoms**:
- Logs streaming
- Suddenly stops
- Button shows "Start Stream" again

![Image: Stream disconnected](/images/containers/troubleshoot-disconnected.png)

**Solutions**:
1. **Click "Start Stream" again**:
   - Reconnects WebSocket
   - Resumes streaming

2. **Check container state**:
   - Verify container still running
   - Check Overview tab
   - If stopped, logs won't stream

3. **Check network**:
   - Network interruption can close WebSocket
   - Refresh page
   - Start streaming again

4. **Check browser console**:
   - Open browser DevTools (F12)
   - Look for WebSocket errors
   - Report if persistent issue

---

### Issue: Too Many Logs, Can't Read

**Symptoms**:
- Logs scrolling too fast
- Can't read specific entries
- Auto-scroll makes it worse

![Image: Too many logs](/images/containers/troubleshoot-too-many.png)

**Solutions**:
1. **Disable auto-scroll**:
   - Click auto-scroll toggle OFF
   - Scroll to section you want to read
   - Read at your own pace

2. **Stop streaming temporarily**:
   - Click "Stop Stream"
   - Read existing logs
   - Start streaming again when ready

3. **Download logs**:
   - Click "Download"
   - Open in text editor
   - Search/filter with editor tools

4. **Reduce application logging**:
   - Lower log level (e.g., WARN instead of DEBUG)
   - Remove verbose debug statements
   - Log only important events

---

### Issue: Can't Find Specific Error

**Symptoms**:
- Error occurred earlier
- Logs scrolled past
- Can't scroll back far enough

![Image: Can't find error](/images/containers/troubleshoot-cant-find.png)

**Solutions**:
1. **Download logs**:
   - Click "Download" button
   - Open in text editor
   - Use Ctrl+F (Cmd+F) to search

2. **Use browser search**:
   - Ctrl+F (Cmd+F) in logs viewer
   - Search for error keywords
   - Browser highlights matches

3. **Restart container and watch**:
   - Stop container
   - Clear logs mentally (start fresh)
   - Start container
   - Start streaming immediately
   - Watch for error from beginning

4. **Check Events tab**:
   - Some errors appear in Events
   - Go to Events tab
   - Look for error events

---

### Issue: Timestamps Don't Match My Timezone

**Symptoms**:
- Timestamps shown in wrong timezone
- Off by several hours

**Explanation**:
- Timestamps are in your **local browser timezone**
- Container may run in UTC
- Browser converts to your timezone

**Verification**:
- Check your system timezone
- Timestamps should match your local time
- If wrong, check browser/system settings

**Not an issue**: Just a timezone display difference

---

## Best Practices

### Logging Strategy

✅ **Log to stdout/stderr**:
```javascript
// Good - logs appear in container logs
console.log("User logged in:", userId);
console.error("Login failed:", error);

// Bad - logs go to file, not visible
fs.appendFileSync('/var/log/app.log', message);
```

✅ **Use structured logging**:
```javascript
// Good - easy to parse
console.log(JSON.stringify({
  level: 'info',
  message: 'User login',
  userId: 123,
  timestamp: new Date().toISOString()
}));

// Bad - hard to parse
console.log("User 123 logged in at " + new Date());
```

✅ **Include context**:
```python
# Good - includes request ID for tracing
print(f"[req-{request_id}] Processing payment for user {user_id}")

# Bad - no context
print("Processing payment")
```

---

### Monitoring

✅ **Stream logs during deployment**:
- Start streaming before deploying
- Watch for startup errors
- Verify successful startup

✅ **Regular log checks**:
- Check logs daily for production containers
- Look for errors and warnings
- Identify patterns

✅ **Download logs for analysis**:
- Download logs weekly
- Analyze trends
- Archive for compliance

---

### Performance

✅ **Control log verbosity**:
```javascript
// Use log levels
const LOG_LEVEL = process.env.LOG_LEVEL || 'info';

if (LOG_LEVEL === 'debug') {
  console.log('Debug: Detailed info');
}
```

❌ **Don't log excessively**:
```javascript
// Bad - logs every iteration
for (let i = 0; i < 10000; i++) {
  console.log(`Processing item ${i}`);
}

// Good - log summary
console.log(`Processing ${items.length} items`);
console.log(`Completed processing`);
```

✅ **Stop streaming when not needed**:
- Click "Stop Stream" after debugging
- Reduces bandwidth and browser memory
- Start again when needed

---

### Debugging

✅ **Add temporary debug logs**:
```python
# Add for debugging
print(f"DEBUG: variable value = {value}")
print(f"DEBUG: entering function X")

# Remove after debugging
```

✅ **Use log prefixes**:
```javascript
console.log("[DB] Connecting to database");
console.log("[AUTH] Validating token");
console.log("[API] Calling external service");
```

✅ **Log errors with context**:
```javascript
try {
  await processPayment();
} catch (error) {
  console.error("Payment processing failed:", {
    userId: user.id,
    amount: payment.amount,
    error: error.message,
    stack: error.stack
  });
}
```

---

## Quick Reference

### Log Stream Controls

| Control | Action |
|---------|--------|
| **Start Stream** | Begin WebSocket log streaming |
| **Stop Stream** | Close WebSocket connection |
| **Auto-scroll** | Toggle automatic scroll to bottom |
| **Download** | Save logs to text file |

### Log Entry Format

```
[YYYY-MM-DD HH:MM:SS.mmm] [stream] message
```

### Stream Colors

| Stream | Color | Meaning |
|--------|-------|---------|
| stdout | Normal | Standard output |
| stderr | Red | Error output |

### Common Commands to Check

**Check if container logs to stdout**:
```bash
# Test locally
docker logs <container-id>

# Should show same output as app logs
```

**Test WebSocket connection** (browser console):
```javascript
// Check WebSocket in browser DevTools → Network → WS
```

---

## Next Steps

- **[Monitor Stats](stats/)** - Resource usage and performance metrics
- **[Manage Containers](manage-containers/)** - Start, stop, restart operations
- **[Deploy a Container](deploy-container/)** - Create new containers
- **[Container Overview](./)** - Learn more about containers
