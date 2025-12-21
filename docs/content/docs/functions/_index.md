+++
title = "Serverless Functions"
description = "Deploy and manage serverless functions with automatic scaling"
weight = 40
date = 2025-12-18
+++

Deploy Lambda-like serverless functions with strong isolation, automatic scaling, and pay-per-use execution model.

---

## What are Serverless Functions?

Serverless Functions in NQRust-MicroVM are **lightweight, event-driven compute units** that run your code in response to HTTP requests. Each function runs in its own **isolated Firecracker microVM**, providing strong security and resource isolation.

![Image: Function architecture diagram](/images/functions/function-architecture.png)

**Key Characteristics**:
- **Isolated Execution** - Each function runs in a dedicated microVM
- **Event-Driven** - Triggered by HTTP requests with JSON payloads
- **Automatic Scaling** - Functions scale automatically based on demand
- **Pay-Per-Use** - Only pay for actual execution time
- **Multiple Runtimes** - Support for Python, JavaScript (Bun), and TypeScript (Bun)

---

## How Functions Work

![Image: Function execution flow](/images/functions/function-flow.png)

1. **Client sends HTTP request** with JSON payload
2. **Manager routes request** to the function's microVM
3. **Function executes** in isolated environment
4. **Response returned** to client
5. **Resources freed** after execution

**Execution Model**:
- Functions are **cold-started** on first invocation (VM spins up)
- **Warm instances** are reused for subsequent requests (faster)
- **Automatic shutdown** after idle timeout
- **Concurrent executions** spawn multiple VM instances

---

## Supported Runtimes

### Python 3.11

![Image: Python runtime badge](/images/functions/runtime-python.png)

**Best for**:
- Data processing and analysis
- Machine learning inference
- API integrations
- Scientific computing

**Example**:
```python
def handler(event):
    name = event.get("name", "World")
    return {
        "statusCode": 200,
        "headers": {"content-type": "application/json"},
        "body": f'{{"message": "Hello, {name}!"}}',
    }
```

---

### JavaScript (Bun)

![Image: JavaScript runtime badge](/images/functions/runtime-javascript.png)

**Best for**:
- Web APIs and microservices
- Real-time data processing
- JSON transformations
- Quick prototyping

**Example**:
```javascript
export async function handler(event) {
  const name = event?.name || "World";
  return {
    statusCode: 200,
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ message: `Hello, ${name}!` }),
  };
}
```

---

### TypeScript (Bun)

![Image: TypeScript runtime badge](/images/functions/runtime-typescript.png)

**Best for**:
- Type-safe APIs
- Enterprise applications
- Complex business logic
- Large codebases

**Example**:
```typescript
interface Event {
  name?: string;
}

export async function handler(event: Event) {
  const name = event?.name || "World";
  return {
    statusCode: 200,
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ message: `Hello, ${name}!` }),
  };
}
```

---

## Function Lifecycle

### States

Functions progress through several states:

![Image: Function state diagram](/images/functions/function-states.png)

| State | Description | Duration |
|-------|-------------|----------|
| **Creating** | Function is being created | 1-2 seconds |
| **Deploying** | microVM is being provisioned | 2-5 seconds |
| **Ready** | Function is ready to receive requests | - |
| **Error** | Function failed to deploy or crashed | - |

**Invocation States** (during execution):
- **Cold Start** - First invocation, VM needs to boot (~2-3 seconds)
- **Warm** - Subsequent invocations, VM already running (~50-200ms)
- **Executing** - Function code is running
- **Complete** - Execution finished, response sent

---

## Use Cases

### 1. API Endpoints

Create lightweight HTTP APIs without managing servers:

**Use cases**:
- REST API endpoints
- Webhooks for third-party integrations
- Form processing
- Authentication endpoints

**Example**: Process form submissions and send email notifications

---

### 2. Data Processing

Process data on-demand without dedicated infrastructure:

**Use cases**:
- Image resizing and optimization
- CSV/JSON data transformations
- Report generation
- Batch processing triggers

**Example**: Resize uploaded images to multiple sizes

---

### 3. Scheduled Tasks

Run periodic tasks without cron jobs:

**Use cases**:
- Database cleanup
- Report generation
- Data synchronization
- Health checks

**Example**: Generate daily analytics reports

---

### 4. Event Handlers

React to events from other systems:

**Use cases**:
- Notification handlers
- Audit log processors
- Real-time analytics
- Alert systems

**Example**: Send Slack notification when error threshold exceeded

---

## Benefits

### Strong Isolation

✅ **Each function runs in its own microVM**
- Complete kernel-level isolation via Firecracker
- No shared resources between functions
- Protection from noisy neighbors
- Secure multi-tenancy

### Fast Cold Starts

⚡ **Sub-second boot times**
- Firecracker microVMs boot in ~150ms
- Total cold start: ~2-3 seconds (including runtime init)
- Warm invocations: ~50-200ms
- Faster than traditional VMs, comparable to containers

### Cost Efficient

💰 **Pay only for what you use**
- No charges for idle time
- Resources automatically released after execution
- Efficient resource utilization
- Lower overhead than always-on VMs

### Easy Development

🚀 **Simple development workflow**
- Write code in web editor (Monaco)
- Test instantly in Playground
- View real-time logs
- No complex deployment pipelines

### Automatic Scaling

📈 **Scales with demand**
- Spin up instances as needed
- Handle traffic spikes automatically
- No manual capacity planning
- Each invocation can run in parallel

---

## Architecture

### Function-per-VM Model

Unlike traditional serverless platforms that share VMs, NQRust-MicroVM uses **one microVM per function instance**:

![Image: Function-per-VM architecture](/images/functions/function-per-vm.png)

**Traditional Serverless** (AWS Lambda-style):
```
┌─────────────────────────────┐
│      Shared Container       │
│  ┌─────┐ ┌─────┐ ┌─────┐  │
│  │ Fn1 │ │ Fn2 │ │ Fn3 │  │ ← Weaker isolation
│  └─────┘ └─────┘ └─────┘  │
└─────────────────────────────┘
```

**NQRust-MicroVM** (Firecracker):
```
┌──────────┐ ┌──────────┐ ┌──────────┐
│ VM 1     │ │ VM 2     │ │ VM 3     │
│  ┌─────┐ │ │  ┌─────┐ │ │  ┌─────┐ │
│  │ Fn1 │ │ │  │ Fn2 │ │ │  │ Fn3 │ │ ← Kernel-level isolation
│  └─────┘ │ │  └─────┘ │ │  └─────┘ │
└──────────┘ └──────────┘ └──────────┘
```

**Advantages**:
- ✅ **Stronger security** - Kernel-level isolation
- ✅ **No noisy neighbors** - Dedicated resources
- ✅ **Crash isolation** - One function crash doesn't affect others
- ✅ **Resource guarantees** - Predictable performance

---

## Getting Started

Ready to create your first function? Follow these guides:

1. **[Create a Function](create-function/)** - Step-by-step guide to creating functions
2. **[Manage Functions](manage-functions/)** - Invoke, update, and delete functions
3. **[Playground](playground/)** - Experiment with functions without creating them
4. **[View Logs](logs/)** - Debug and monitor function executions

---

## Quick Example

Here's a complete example of a simple calculator function:

**Function Name**: `calculator`
**Runtime**: Python
**Handler**: `handler`

**Code**:
```python
def handler(event):
    operation = event.get("operation")
    a = float(event.get("a", 0))
    b = float(event.get("b", 0))

    if operation == "add":
        result = a + b
    elif operation == "subtract":
        result = a - b
    elif operation == "multiply":
        result = a * b
    elif operation == "divide":
        result = a / b if b != 0 else "Error: Division by zero"
    else:
        return {
            "statusCode": 400,
            "body": '{"error": "Invalid operation"}',
        }

    return {
        "statusCode": 200,
        "headers": {"content-type": "application/json"},
        "body": f'{{"result": {result}}}',
    }
```

**Invoke** with payload:
```json
{
  "operation": "add",
  "a": 10,
  "b": 5
}
```

**Response**:
```json
{
  "result": 15
}
```

**Next**: Learn how to [create your first function](create-function/).

---

## Comparison with VMs

| Feature | Functions | VMs |
|---------|-----------|-----|
| **Startup Time** | ~2-3 seconds (cold), ~50ms (warm) | ~1-2 seconds |
| **Isolation** | microVM per function instance | microVM per VM |
| **Scaling** | Automatic, per invocation | Manual |
| **Billing** | Per execution | Per hour/always-on |
| **State** | Stateless (ephemeral) | Stateful (persistent) |
| **Best For** | Event-driven, short tasks | Long-running services |
| **Code Deployment** | Built-in editor + deploy | Manual (SSH, copy files) |

**When to use Functions**:
- ✅ Event-driven workloads
- ✅ HTTP APIs and webhooks
- ✅ Short-running tasks (<5 minutes)
- ✅ Unpredictable traffic patterns
- ✅ Quick prototyping

**When to use VMs**:
- ✅ Long-running services
- ✅ Stateful applications
- ✅ Complex dependencies
- ✅ Full OS control needed
- ✅ Persistent connections (databases, WebSocket)

---

## Limitations

**Execution Time**:
- Maximum timeout: 300 seconds (5 minutes)
- Default timeout: 30 seconds
- Configurable per function

**Resources**:
- vCPU: 1-32 cores
- Memory: 128 MB - 3072 MB (3 GB)
- No persistent storage (ephemeral filesystem)

**Networking**:
- Outbound network access available
- Inbound: HTTP requests only
- No direct SSH access (use logs for debugging)

**Cold Start**:
- First invocation takes ~2-3 seconds
- Keep functions warm with periodic invocations

---

## Best Practices

✅ **Keep functions small and focused**
- One function = one responsibility
- Break complex logic into multiple functions
- Faster cold starts with smaller code

✅ **Handle errors gracefully**
- Return proper HTTP status codes
- Include error messages in response
- Log errors for debugging

✅ **Optimize for warm starts**
- Reuse expensive resources (DB connections)
- Initialize once, reuse across invocations
- Keep global scope minimal

✅ **Use appropriate timeouts**
- Set realistic timeout values
- Don't over-allocate (costs more)
- Monitor actual execution times

✅ **Test in Playground first**
- Experiment before creating functions
- Validate payloads and responses
- Iterate quickly without deployments

---

## Next Steps

- **[Create a Function](create-function/)** - Build your first function
- **[Playground](playground/)** - Try the interactive playground
- **[Manage Functions](manage-functions/)** - Learn function operations
