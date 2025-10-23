# Lambda Functions API Documentation

This document describes the Lambda/Serverless Functions API for the NQRust MicroVM platform.

## Overview

The Lambda Functions feature allows you to deploy and run serverless functions in dedicated Firecracker MicroVMs. Each function runs in its own isolated VM with a runtime-specific environment (Node.js or Python).

**Base URL:** `http://localhost:18080/v1/functions`

## Features

- ✅ **Multiple Runtimes:** Node.js and Python support
- ✅ **Isolated Execution:** Each function runs in its own MicroVM
- ✅ **Hot Reload:** Update function code without recreating VMs
- ✅ **Fast Invocation:** ~2ms cold start, sub-millisecond warm invocations
- ✅ **Status Tracking:** Monitor deployment progress with real-time status updates

---

## Function Lifecycle States

Functions progress through these states during creation and deployment:

| State | Description |
|-------|-------------|
| `creating` | Function record created, VM is being provisioned |
| `booting` | VM is booting up |
| `deploying` | Code is being injected into the VM |
| `ready` | Function is ready to invoke |
| `error` | Something went wrong during deployment |

**Typical deployment time:** ~60-90 seconds from creation to ready state.

---

## API Endpoints

### 1. Create Function

Deploy a new serverless function.

**Endpoint:** `POST /v1/functions`

**Request Body:**
```json
{
  "name": "my-function",
  "runtime": "node",  // "node" or "python"
  "handler": "handler",
  "code": "async function handler(event) { return { message: 'Hello!', input: event }; }",
  "vcpu": 1,
  "memory_mb": 256
}
```

**Response:**
```json
{
  "id": "f35fc685-c85e-4f25-93ca-13b32b2de5d8"
}
```

**cURL Example:**
```bash
curl -X POST http://localhost:18080/v1/functions \
  -H "Content-Type: application/json" \
  -d '{
    "name": "hello-nodejs",
    "runtime": "node",
    "handler": "handler",
    "code": "async function handler(event) { return { message: \"Hello from Node.js!\", timestamp: new Date().toISOString(), input: event }; }",
    "vcpu": 1,
    "memory_mb": 256
  }'
```

---

### 2. Get Function Details

Retrieve information about a specific function, including its current state.

**Endpoint:** `GET /v1/functions/{id}`

**Response:**
```json
{
  "item": {
    "id": "f35fc685-c85e-4f25-93ca-13b32b2de5d8",
    "name": "hello-nodejs",
    "runtime": "node",
    "code": "async function handler(event) { ... }",
    "handler": "handler",
    "timeout_seconds": 30,
    "memory_mb": 256,
    "vcpu": 1,
    "vm_id": "5eb44cb5-b760-4248-a597-cb5c12d844e6",
    "guest_ip": "192.168.18.170",
    "port": 3000,
    "state": "ready",
    "created_at": "2025-10-22T08:25:59.898638Z",
    "updated_at": "2025-10-22T08:48:04.716471Z",
    "last_invoked_at": "2025-10-22T08:44:11.204098Z"
  }
}
```

**cURL Example:**
```bash
curl http://localhost:18080/v1/functions/f35fc685-c85e-4f25-93ca-13b32b2de5d8
```

**Status Polling:**
After creating a function, poll this endpoint to check when `state` becomes `"ready"`:

```bash
# Check every 5 seconds until ready
while true; do
  STATE=$(curl -s http://localhost:18080/v1/functions/$FUNCTION_ID | jq -r '.item.state')
  echo "Current state: $STATE"
  if [ "$STATE" = "ready" ]; then
    echo "Function is ready!"
    break
  fi
  sleep 5
done
```

---

### 3. List All Functions

Get a list of all deployed functions.

**Endpoint:** `GET /v1/functions`

**Response:**
```json
{
  "items": [
    {
      "id": "f35fc685-c85e-4f25-93ca-13b32b2de5d8",
      "name": "hello-nodejs",
      "runtime": "node",
      "state": "ready",
      ...
    },
    {
      "id": "9824a1ac-442c-4bcc-8f24-c82e3602406b",
      "name": "hello-python",
      "runtime": "python",
      "state": "deploying",
      ...
    }
  ]
}
```

**cURL Example:**
```bash
curl http://localhost:18080/v1/functions
```

---

### 4. Update Function

Update function code or configuration. The code will be hot-reloaded in the running VM without downtime.

**Endpoint:** `PUT /v1/functions/{id}`

**Request Body (all fields optional):**
```json
{
  "name": "updated-name",
  "code": "async function handler(event) { return { message: 'Updated!', version: 2 }; }",
  "handler": "handler",
  "timeout_seconds": 60,
  "memory_mb": 512
}
```

**Response:**
```json
{
  "item": {
    "id": "f35fc685-c85e-4f25-93ca-13b32b2de5d8",
    "name": "updated-name",
    "code": "async function handler(event) { ... }",
    "state": "ready",
    ...
  }
}
```

**cURL Example:**
```bash
curl -X PUT http://localhost:18080/v1/functions/$FUNCTION_ID \
  -H "Content-Type: application/json" \
  -d '{
    "code": "async function handler(event) { return { message: \"UPDATED CODE!\", version: 2, input: event }; }"
  }'
```

**⚠️ Note:** Code updates happen in the background. The function remains available during the update (zero downtime).

---

### 5. Invoke Function

Execute a function with the provided event data.

**Endpoint:** `POST /v1/functions/{id}/invoke`

**Request Body:**
```json
{
  "event": {
    "name": "World",
    "value": 123,
    "any": "custom data"
  }
}
```

**Response:**
```json
{
  "request_id": "ba5b5277-3cfd-4059-bcd0-12b8dd49bed0",
  "status": "success",
  "duration_ms": 2,
  "response": {
    "message": "UPDATED CODE!",
    "timestamp": "2025-10-22T08:48:25.797Z",
    "version": 2,
    "input": {
      "name": "World",
      "value": 123
    }
  },
  "logs": []
}
```

**cURL Example:**
```bash
curl -X POST http://localhost:18080/v1/functions/$FUNCTION_ID/invoke \
  -H "Content-Type: application/json" \
  -d '{"event": {"name": "World", "test": 123}}'
```

**Performance:**
- **First invocation:** ~2-9ms (cold start)
- **Subsequent invocations:** ~2ms (warm)

---

### 6. Delete Function

Delete a function and its associated VM.

**Endpoint:** `DELETE /v1/functions/{id}`

**Response:**
```json
{
  "message": "Function deleted"
}
```

**cURL Example:**
```bash
curl -X DELETE http://localhost:18080/v1/functions/$FUNCTION_ID
```

---

### 7. Get Function Invocation Logs

Retrieve invocation history for a function.

**Endpoint:** `GET /v1/functions/{id}/logs`

**Query Parameters:**
- `limit` (optional): Number of logs to return (default: 100)
- `offset` (optional): Pagination offset (default: 0)

**Response:**
```json
{
  "items": [
    {
      "id": "inv-123",
      "function_id": "f35fc685-c85e-4f25-93ca-13b32b2de5d8",
      "status": "success",
      "duration_ms": 2,
      "request_id": "ba5b5277-3cfd-4059-bcd0-12b8dd49bed0",
      "invoked_at": "2025-10-22T08:48:25.797Z"
    }
  ]
}
```

**cURL Example:**
```bash
curl "http://localhost:18080/v1/functions/$FUNCTION_ID/logs?limit=50"
```

---

## Runtime-Specific Code Examples

### Node.js Function

```javascript
async function handler(event) {
  // Access event data
  const { name, value } = event;

  // Perform async operations
  const result = await someAsyncOperation();

  // Return response (must be JSON-serializable)
  return {
    message: `Hello, ${name}!`,
    result: result,
    timestamp: new Date().toISOString()
  };
}
```

**Requirements:**
- Must export an async function
- Must return a JSON-serializable object
- Can use Node.js built-in modules
- Event is passed as the first argument

### Python Function

```python
def handler(event):
    # Access event data
    name = event.get('name')
    value = event.get('value')

    # Return response (must be dict)
    return {
        'message': f'Hello, {name}!',
        'value': value * 2,
        'timestamp': str(datetime.now())
    }
```

**Requirements:**
- Must define a `handler` function
- Must return a dictionary
- Can use Python standard library
- Event is passed as the first argument (dict)

---

## Frontend Integration Guide

### Complete Workflow Example

```javascript
// 1. Create a function
const createResponse = await fetch('http://localhost:18080/v1/functions', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    name: 'my-function',
    runtime: 'node',
    handler: 'handler',
    code: 'async function handler(event) { return { message: "Hello!", input: event }; }',
    vcpu: 1,
    memory_mb: 256
  })
});

const { id } = await createResponse.json();
console.log('Function created:', id);

// 2. Poll for ready state
let state = 'creating';
while (state !== 'ready' && state !== 'error') {
  await new Promise(resolve => setTimeout(resolve, 5000)); // Wait 5s

  const statusResponse = await fetch(`http://localhost:18080/v1/functions/${id}`);
  const { item } = await statusResponse.json();
  state = item.state;

  console.log('Deployment status:', state);
}

if (state === 'error') {
  console.error('Function deployment failed');
  return;
}

// 3. Invoke the function
const invokeResponse = await fetch(`http://localhost:18080/v1/functions/${id}/invoke`, {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    event: { name: 'World', value: 123 }
  })
});

const result = await invokeResponse.json();
console.log('Function result:', result.response);

// 4. Update function code (optional)
const updateResponse = await fetch(`http://localhost:18080/v1/functions/${id}`, {
  method: 'PUT',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    code: 'async function handler(event) { return { message: "Updated!", version: 2 }; }'
  })
});

// Wait a few seconds for code reload
await new Promise(resolve => setTimeout(resolve, 3000));

// 5. Invoke again with updated code
const invokeResponse2 = await fetch(`http://localhost:18080/v1/functions/${id}/invoke`, {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ event: {} })
});

const result2 = await invokeResponse2.json();
console.log('Updated function result:', result2.response); // { message: "Updated!", version: 2 }
```

---

## UI/UX Recommendations

### Function List Page
- Show function name, runtime, state, and last invoked time
- Color-code states:
  - `creating`/`booting`/`deploying`: Yellow/Orange (in progress)
  - `ready`: Green (success)
  - `error`: Red (failure)
- Add "Invoke" button for ready functions
- Add "Logs" button to view invocation history

### Function Create/Edit Page
- Code editor with syntax highlighting for JavaScript/Python
- Runtime selector (Node.js / Python)
- Resource configuration (vCPU, memory)
- "Test Invoke" button to quickly test after deployment
- Real-time deployment status indicator

### Function Detail Page
- Show current state with auto-refresh during deployment
- Display function metadata (created_at, updated_at, last_invoked_at)
- Show VM details (vm_id, guest_ip)
- Code editor for inline updates
- Invocation history table
- "Invoke" form to test with custom event data

---

## Error Handling

All endpoints return standard HTTP status codes:

- `200 OK`: Success
- `400 Bad Request`: Invalid request body/parameters
- `404 Not Found`: Function doesn't exist
- `500 Internal Server Error`: Server error

**Error Response Format:**
```json
{
  "error": "Error message description"
}
```

---

## Performance Characteristics

| Metric | Value |
|--------|-------|
| Function deployment time | 60-90 seconds |
| Cold start (first invocation) | 2-9ms |
| Warm invocation | ~2ms |
| Concurrent invocations per function | Unlimited (single VM handles concurrency) |
| Memory per function VM | Configurable (default: 256MB) |
| CPU per function VM | Configurable (default: 1 vCPU) |

---

## Limitations

- **Runtime support:** Currently Node.js v18.20.1 and Python 3.11.12
- **Code size:** No enforced limit, but keep it reasonable
- **Timeout:** Configurable, default 30 seconds
- **Network:** Functions can make outbound HTTP requests
- **File system:** Ephemeral, resets on VM restart

---

## Support

For issues or questions, contact the backend team or file an issue in the repository.
