# Why Run Your App in a MicroVM Instead of a Hypervisor VM?

## The Short Answer

A microVM gives you **everything a traditional VM gives you — but lighter, faster, and cheaper**. Your app doesn't know the difference. It sees a Linux kernel, a filesystem, a network interface. It runs exactly the same. The difference is what happens *underneath*.

## Think of It Like This

|  | Traditional Hypervisor VM | MicroVM |
|---|---|---|
| Analogy | A full house with rooms you never use | A studio apartment — everything you need, nothing you don't |
| Your app cares? | No | No |
| Your wallet cares? | Yes | Yes |

Your app doesn't need a BIOS. It doesn't need 47 emulated hardware devices. It doesn't need a full Ubuntu desktop kernel. But a traditional VM gives it all of that anyway — and you pay for it in RAM, CPU, disk, and boot time.

## The 5 Reasons That Matter to Your Business

### 1. You Fit More Apps on the Same Hardware

A traditional VM running a simple web app typically needs **1–2 GB RAM minimum** just for the OS overhead before your app even starts.

A microVM runs the same app with **128–256 MB**. Same isolation, same security.

**That means:** the server that runs 10 apps in 10 traditional VMs can run **40–50 apps** in microVMs. That's 4–5x more value from hardware you already own.

### 2. Your Apps Start in Seconds, Not Minutes

Traditional VM: power on → BIOS → bootloader → kernel → systemd → services → your app. **30–60 seconds.**

MicroVM: kernel → your app. **Under 5 seconds.**

**That means:** when you deploy an update, reboot after a patch, or recover from a crash — your app is back in seconds. Less downtime, happier users.

### 3. Same Security, Smaller Attack Surface

This is the part people miss. A microVM is **not** a container. It's a real VM with real hardware isolation — its own kernel, its own memory space, completely separated from other VMs.

But unlike a traditional VM that emulates dozens of hardware devices (USB controllers, sound cards, legacy PCI buses), a microVM emulates only **5 devices**. Less emulated hardware = fewer things that can have vulnerabilities.

**That means:** you get VM-level isolation with a **smaller attack surface** than a traditional VM. Your security team will appreciate this.

### 4. Less to Manage, Less to Patch

A traditional VM runs a full OS — systemd, cron, SSH daemon, package manager, logging daemon, dozens of background services. All of those need patching, monitoring, and configuration.

A microVM runs a **minimal kernel and your app**. That's it. Fewer moving parts = fewer things that break, fewer CVEs to chase.

### 5. You Can Still Do Everything You're Used To

| "Can I still..." | Answer |
|---|---|
| SSH into my VM? | Yes — shell access via the platform |
| See CPU/memory usage? | Yes — real-time metrics built in |
| Use my app exactly as before? | Yes — your app sees standard Linux |
| Have my own filesystem? | Yes — each microVM has its own rootfs |
| Get a network IP? | Yes — each VM gets its own IP via DHCP |
| Attach storage? | Yes — drive attachment supported |

Nothing changes from your app's perspective. Everything improves from your infrastructure's perspective.

## Beyond Traditional Apps: Docker Containers & Serverless Functions

MicroVM doesn't just run classic apps. It unlocks two powerful deployment models that are **impractical or impossible** on a traditional hypervisor.

### Docker Containers — But With Real VM Isolation

You know Docker. Your team probably already uses it. The problem? On a traditional hypervisor, you'd spin up a full VM (1–2 GB RAM, 60-second boot), install Docker inside, and then run your container. That's **two layers of overhead** for one app.

With NQRust-MicroVM, you just say: "Run this Docker image." The platform handles everything:

1. Spins up a lightweight microVM with Docker pre-installed (boots in seconds)
2. Pulls your Docker image (or loads it from local cache)
3. Starts your container inside the isolated VM
4. Sets up port forwarding so your app is reachable from the network

**Each container gets its own dedicated microVM.** Unlike a shared Docker host where one bad container can crash everything, here every container is hardware-isolated from every other container.

#### What You Can Do

- **Deploy any Docker image** — `postgres:latest`, `nginx`, `redis`, your own custom images, anything from Docker Hub or a private registry
- **Map ports** — expose container ports to the network, just like you would on any server
- **Mount volumes** — attach persistent storage to your containers
- **Set environment variables** — configure your app without rebuilding the image
- **Run commands** — execute commands inside running containers
- **Monitor in real-time** — CPU, memory, network I/O, logs, all from the dashboard
- **Stream logs live** — real-time log streaming via WebSocket, no SSH needed
- **Use private registries** — authenticate with your own Docker registry

#### Why Not Just Run Docker on a Hypervisor VM?

| | Docker on Hypervisor VM | Docker on MicroVM |
|---|---|---|
| RAM per container | 1–2 GB (VM) + container overhead | 512 MB total (VM + container) |
| Boot to running container | 60s (VM) + 10s (Docker) = **~70s** | **~15s total** |
| Isolation between containers | Shared kernel inside the VM | **Each container in its own VM** |
| One container crashes | Can affect others on same Docker host | **Only that one VM is affected** |
| Containers per host | ~5–10 (limited by heavy VMs) | **~30–50** on same hardware |
| Management | SSH in, run docker commands manually | **Web dashboard, API, one-click deploy** |

**The bottom line:** You get Docker's convenience with VM-level security, at a fraction of the resource cost.

---

### Serverless Functions — Run Code Without Managing Anything

This is for the simplest use case: "I just want to run a piece of code when something happens."

No VM to set up. No Docker image to build. No server to manage. You write a function in **Python, JavaScript, or TypeScript**, paste it into the platform, and it's ready to be called via API.

#### How It Works

1. **You write a function** — a simple script that takes an input and returns an output
2. **The platform creates a microVM** behind the scenes (your users never see it)
3. **You call it via API** — `POST /v1/functions/{id}/invoke` with a JSON payload
4. **You get a response** — the function runs and returns the result in milliseconds
5. **Code updates are instant** — change your code, it's hot-reloaded into the running VM, no restart

#### Real-World Examples

**Webhook handler:**
> "When our payment gateway sends a callback, process it and update our database."
>
> Instead of: spin up a VM, install Node.js, set up a web server, configure nginx, manage SSL...
> With functions: paste 20 lines of JavaScript, done.

**Scheduled data processing:**
> "Every hour, pull data from an external API and generate a report."
>
> Instead of: maintain a dedicated VM running 24/7 for a script that runs 30 seconds per hour.
> With functions: invoke via API on a schedule. Pay for 128 MB RAM instead of 2 GB.

**Internal automation:**
> "When a new employee is added to HR system, create their accounts across all our tools."
>
> Instead of: build and deploy a full microservice.
> With functions: write a Python script, call it from your HR system's webhook.

**Simple API endpoint:**
> "I need a small API that validates input and returns a result."
>
> Instead of: set up a server, deploy an app, manage uptime.
> With functions: write the logic, the platform handles the rest.

#### What You Get

- **Three runtimes** — Python, JavaScript, TypeScript (Bun runtime for JS/TS)
- **Invocation logging** — every call is recorded with input, output, logs, duration, and status
- **Environment variables** — pass configuration to your functions securely
- **Hot reload** — update code without any restart or downtime
- **Full VM isolation** — each function runs in its own microVM, not a shared process

#### Why Not Just Run This on a Hypervisor VM?

| | Script on Hypervisor VM | Serverless Function |
|---|---|---|
| Setup time | Hours (provision VM, install runtime, deploy code) | **Minutes** (paste code, done) |
| RAM usage | 1–2 GB for the VM | **128 MB** |
| Management | You manage the OS, runtime, and deployment | **Platform manages everything** |
| Updating code | SSH in, pull code, restart service | **Paste new code, instant hot-reload** |
| Monitoring | Set up your own logging | **Built-in invocation logs and metrics** |
| Cost per simple task | A full VM running 24/7 | **One tiny microVM** |

---

### Choosing the Right Deployment Model

| I want to... | Use |
|---|---|
| Run a standard Linux application | **MicroVM** |
| Deploy a Docker image (database, web server, etc.) | **Docker Container** |
| Run a small piece of code triggered by an event or API | **Serverless Function** |
| Run Windows or GPU workloads | **Hypervisor VM** |

All three microVM-based options (VM, container, function) give you **hardware-level isolation** at a fraction of the cost of a traditional hypervisor VM. Pick the model that matches your workload — the platform handles the rest.

---

## When Should You Still Use the Hypervisor VM?

MicroVMs aren't for everything. Use traditional hypervisor VMs when you need:

- **Windows workloads** — microVMs run Linux only
- **GPU passthrough** — requires full QEMU/KVM device emulation
- **Massive single VMs** (64 GB+ RAM, 32+ vCPU) — hypervisor VMs handle these better
- **Legacy appliances** that require specific BIOS/UEFI boot or hardware emulation

## The Bottom Line

> You're not replacing your hypervisor. You're making it smarter. Heavy workloads stay on hypervisor VMs. Everything else — your web apps, APIs, microservices, internal tools — runs lighter, faster, and cheaper on microVMs. Same security. Same isolation. Less waste.
