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

**What's captured**:
- ✅ **stdout** — Standard output (`console.log`, `print`, `echo`)
- ✅ **stderr** — Error output (exceptions, warnings)
- ✅ **Docker daemon** — Container lifecycle events
- ✅ **Application logs** — Your application's log output

**Not captured**:
- ❌ Logs written to files inside the container (unless volume-mounted)
- ❌ Historical logs before streaming started

---

## Accessing Container Logs

1. Open the container detail page
2. Click the **Logs** tab

---

## Streaming Logs

Logs don't stream automatically — you must start them manually.

Click **Start Stream** to open a WebSocket connection. The button changes to **Stop Stream** once connected and logs start appearing in real-time.

Toggle **Auto-scroll** to keep the latest logs in view as they arrive. Scroll up manually to pause auto-scroll and read earlier entries.

Click **Download** to save all currently visible logs to a `.txt` file — useful for offline analysis, sharing with teammates, or grepping locally.

---

## Log Entry Format

Each entry follows this format:

```
[YYYY-MM-DD HH:MM:SS.mmm] [stream] message
```

**Example**:
```
[2025-12-18 14:30:45.123] [stdout] Starting Nginx 1.25.3
[2025-12-18 14:30:45.234] [stdout] Listening on port 80
[2025-12-18 14:31:22.456] [stderr] Error: Database connection failed
[2025-12-18 14:31:22.567] [stderr]   at connect (db.js:45:10)
```

- **stdout** — normal text
- **stderr** — shown in red

Timestamps are displayed in your local browser timezone.

---

## Common Log Patterns

### Nginx
```
[stdout] 2025/12/18 14:30:45 [notice] nginx/1.25.3
[stdout] 2025/12/18 14:30:45 [notice] start worker process 29
[stdout] 172.16.0.1 - - [18/Dec/2025:14:32:15 +0000] "GET / HTTP/1.1" 200 615
```

### PostgreSQL
```
[stdout] 2025-12-18 14:30:50 UTC [1] LOG:  starting PostgreSQL 15.3
[stdout] 2025-12-18 14:30:50 UTC [1] LOG:  listening on IPv4 address "0.0.0.0", port 5432
[stdout] 2025-12-18 14:30:50 UTC [1] LOG:  database system is ready to accept connections
```

### Node.js
```
[stdout] Connecting to database...
[stdout] Database connected successfully
[stdout] Listening on port 3000
[stdout] GET /api/users 200 45ms
[stderr] Warning: Deprecated API usage
```

### Error with stack trace
```
[stderr] Error: Connection timeout
[stderr]   at Timeout._onTimeout (/app/lib/db.js:123:15)
[stderr]   at listOnTimeout (node:internal/timers:559:17)
[stderr] Application shutting down due to fatal error
```

---

## Debugging with Logs

### Container starts then immediately stops

1. Open the Logs tab and click **Start Stream** before restarting the container
2. Look for red stderr entries near the top
3. Common startup errors:
   ```
   [stderr] Error: POSTGRES_PASSWORD must be set
   [stderr] Error: bind EADDRINUSE 0.0.0.0:3000
   [stderr] Error: Cannot find module 'express'
   ```
4. Fix the configuration and restart

### Finding a specific past error

- Click **Download**, then open the file in a text editor and use `Ctrl+F`
- Or use browser search (`Ctrl+F`) directly in the log viewer

### Application is slow

Compare timestamps between related log lines to find the delay:

```
[stdout] [14:30:00.000] Database query started
[stdout] [14:30:05.456] Database query completed   ← 5.4s!
```

Add timing logs to narrow it down further:
```javascript
const start = Date.now();
const result = await db.query(...);
console.log(`[PERF] Query completed in ${Date.now() - start}ms`);
```

---

## Troubleshooting

### Streaming but no logs appear

- The application may log to files instead of stdout — configure it to use stdout/stderr
- The container may have just started — wait a moment and trigger some activity
- Log level may be set too high — set it to `INFO` or `DEBUG`

### Stream disconnects unexpectedly

1. Click **Start Stream** again to reconnect
2. Verify the container is still in **Running** state
3. Check for network interruptions — refresh the page and restart the stream
4. Open browser DevTools → Network → WS to look for WebSocket errors

### Too many logs scrolling too fast

- Disable **Auto-scroll** and scroll to the section you need
- Click **Stop Stream**, read what's there, then resume
- Download the logs and filter with your editor

---

## Best Practices

**Log to stdout/stderr** — not to files:
```javascript
// Good
console.log("User logged in:", userId);
console.error("Login failed:", error);

// Bad - won't appear in logs tab
fs.appendFileSync('/var/log/app.log', message);
```

**Include context** in log messages:
```javascript
console.error("Payment failed:", { userId, amount, error: err.message });
```

**Use log levels** to control verbosity:
```javascript
if (process.env.LOG_LEVEL === 'debug') {
  console.log('Debug: detailed info');
}
```

**Stop streaming** when you're done debugging to reduce bandwidth and browser memory usage.

---

## Quick Reference

| Control | Action |
|---|---|
| **Start Stream** | Begin WebSocket log streaming |
| **Stop Stream** | Close WebSocket connection |
| **Auto-scroll** | Toggle automatic scroll to bottom |
| **Download** | Save visible logs to a text file |

---

## Next Steps

- **[Monitor Stats](stats/)** — Resource usage and performance metrics
- **[Manage Containers](manage-containers/)** — Start, stop, restart operations
- **[Deploy a Container](deploy-container/)** — Create new containers
