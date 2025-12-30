+++
title = "Containers"
description = "Deploy and orchestrate Docker containers with microVM isolation"
weight = 30
date = 2025-12-18
+++

Deploy Docker containers with the strong isolation of Firecracker microVMs - the best of both worlds.

---

## What are Containers?

NQRust-MicroVM Containers combine **Docker's convenience** with **Firecracker's security isolation**:

![Image: Container architecture diagram](/images/containers/architecture-overview.png)

- ✅ **Docker compatibility** - Use any Docker image from Docker Hub or private registries
- ✅ **Strong isolation** - Each container runs in its own Firecracker microVM
- ✅ **Full Docker API** - Compatible with standard Docker commands and tools
- ✅ **Kernel-level security** - Hardware-enforced isolation via KVM virtualization
- ✅ **Fast deployment** - Container-optimized Alpine Linux runtime boots in ~2-3 seconds

---

## Container-per-VM Architecture

Unlike traditional container platforms, NQRust-MicroVM uses a **Container-per-VM** model:

![Image: Container-per-VM diagram](/images/containers/container-per-vm-diagram.png)

**Traditional Containers** (Docker, containerd):
```
┌─────────────────────────────────────┐
│     Host Kernel (Shared)            │
├─────────────────────────────────────┤
│ Container 1 │ Container 2 │ Container 3 │
└─────────────────────────────────────┘
```

**NQRust-MicroVM Containers**:
```
┌──────────────────────────────────────────────┐
│        Host (KVM Hypervisor)                 │
├──────────────┬──────────────┬────────────────┤
│  MicroVM 1   │  MicroVM 2   │  MicroVM 3     │
│ ┌──────────┐ │ ┌──────────┐ │ ┌──────────┐  │
│ │Container │ │ │Container │ │ │Container │  │
│ │  nginx   │ │ │ postgres │ │ │  redis   │  │
│ └──────────┘ │ └──────────┘ │ └──────────┘  │
└──────────────┴──────────────┴────────────────┘
```

**Benefits**:
- **Stronger isolation** - Containers can't escape to host kernel
- **Better multi-tenancy** - Safe isolation between different users' containers
- **Security** - Attack surface reduced to VM boundaries
- **Flexibility** - Each container can have different kernel parameters

---

## How Containers Work

### Deployment Flow

![Image: Container deployment flow](/images/containers/deployment-flow.png)

1. **Select Image**:
   - Choose from local registry (cached images)
   - Pull from Docker Hub (e.g., `nginx:latest`, `postgres:15`)
   - Upload custom tarball (exported with `docker save`)

2. **Configure Container**:
   - Set resource limits (CPU, Memory)
   - Configure port mappings (expose services)
   - Add environment variables
   - Mount volumes for persistent data
   - Optional: Private registry authentication

3. **Deploy**:
   - Manager creates dedicated Firecracker microVM
   - VM boots with Alpine Linux + Docker daemon
   - Docker pulls and starts your container image
   - Container becomes accessible via mapped ports

4. **Access**:
   - Web UI shows container status
   - View real-time logs via WebSocket
   - Monitor resource usage with stats
   - Execute shell commands inside container

---

## Container States

Containers go through several states during their lifecycle:

![Image: Container state diagram](/images/containers/state-diagram.png)

| State | Description | Available Actions |
|-------|-------------|-------------------|
| **Creating** | VM is being created | Wait |
| **Booting** | VM is booting up | Wait |
| **Initializing** | Docker daemon starting | Wait |
| **Running** | Container is active | Stop, Restart, Pause, View Logs, Shell |
| **Stopped** | Container is stopped | Start, Delete |
| **Paused** | Container is paused | Resume, Stop |
| **Error** | Deployment failed | View logs, Delete, Retry |

**Typical deployment timeline**:
- Creating VM: 1-2 seconds
- Booting VM: 2-3 seconds
- Pulling image: 10-60 seconds (depends on image size)
- Starting container: 1-2 seconds
- **Total**: 15-70 seconds for first deployment

---

## Supported Images

NQRust-MicroVM supports **any Docker image** from:

### Docker Hub (Public Registry)

Popular images work out-of-the-box:

![Image: Docker Hub popular images](/images/containers/dockerhub-popular.png)

- **Web servers**: `nginx`, `httpd`, `caddy`
- **Databases**: `postgres`, `mysql`, `mongo`, `redis`, `mariadb`
- **Languages**: `node`, `python`, `golang`, `openjdk`, `ruby`
- **Message queues**: `rabbitmq`, `nats`, `kafka`
- **Caching**: `redis`, `memcached`, `varnish`
- **Monitoring**: `prometheus`, `grafana`

**Example**: Deploy PostgreSQL 15:
```
Image: postgres:15
Environment: POSTGRES_PASSWORD=mypassword
Ports: 5432:5432
```

### Private Registries

Authenticate with private registries:

![Image: Private registry authentication](/images/containers/private-registry-auth.png)

- Docker Hub private repositories
- GitHub Container Registry (ghcr.io)
- GitLab Container Registry
- Azure Container Registry
- Google Container Registry
- Self-hosted registries

**Authentication fields**:
- Username
- Password or access token
- Registry server (e.g., `ghcr.io`, `registry.gitlab.com`)

### Custom Images (Upload)

Upload Docker image tarballs:

![Image: Upload custom image](/images/containers/upload-image.png)

**Export image**:
```bash
docker save -o myimage.tar myimage:latest
```

**Upload via UI**:
1. Select "Upload" tab
2. Choose `.tar` or `.tar.gz` file
3. Deploy container

---

## Use Cases

### Web Application Hosting

Deploy web servers and applications:

![Image: Web app hosting](/images/containers/use-case-web-app.png)

**Example: Nginx static site**:
- Image: `nginx:alpine`
- Ports: `80:80`
- Volume: `/srv/www:/usr/share/nginx/html`
- Resources: 0.5 vCPU, 256 MB

### Database Services

Run database servers with persistent storage:

![Image: Database hosting](/images/containers/use-case-database.png)

**Example: PostgreSQL database**:
- Image: `postgres:15-alpine`
- Ports: `5432:5432`
- Environment: `POSTGRES_PASSWORD=secret`
- Volume: `/srv/pgdata:/var/lib/postgresql/data`
- Resources: 2 vCPU, 2048 MB

### Development Environments

Isolated development environments per project:

![Image: Dev environments](/images/containers/use-case-dev-env.png)

**Example: Node.js app**:
- Image: `node:20-alpine`
- Ports: `3000:3000`
- Volume: `/srv/app:/app` (mount source code)
- Command: `npm run dev`

### Microservices

Deploy and orchestrate microservices:

![Image: Microservices architecture](/images/containers/use-case-microservices.png)

**Example: E-commerce stack**:
- Frontend: `nginx:alpine` (port 80)
- API: `node:20-alpine` (port 3000)
- Database: `postgres:15` (port 5432)
- Cache: `redis:7-alpine` (port 6379)
- Queue: `rabbitmq:3-alpine` (port 5672)

Each service runs in isolated microVM with independent resources.

---

## Key Features

### Resource Management

![Image: Resource configuration](/images/containers/resource-management.png)

**CPU Limits**:
- Range: 0.1 to 16 cores
- Granular control (0.1 increments)
- Dedicated CPU allocation

**Memory Limits**:
- Range: 64 MB to 32 GB
- Prevent memory overuse
- OOM protection

**Example configuration**:
```
Small service:  0.5 vCPU, 512 MB
Medium service: 2 vCPU, 2048 MB
Large service:  4 vCPU, 8192 MB
```

---

### Network Configuration

![Image: Port mapping configuration](/images/containers/port-mapping.png)

**Port Mappings**:
- Map host ports to container ports
- Support TCP and UDP protocols
- Multiple port mappings per container
- Access containers from outside

**Example**:
```
Host:Container  Protocol  Purpose
8080:80         TCP       Web server
5432:5432       TCP       PostgreSQL
6379:6379       TCP       Redis
53:53           UDP       DNS server
```

---

### Volume Mounts

![Image: Volume management](/images/containers/volume-mounts.png)

**Persistent Storage**:
- Mount host directories into containers
- Persist data across container restarts
- Share data between containers
- Read-only or read-write access

**Two types**:
1. **New volumes** - Create new storage for container
2. **Existing volumes** - Attach previously created volumes

**Example**:
```
Database data:  /srv/pgdata:/var/lib/postgresql/data
Application:    /srv/app:/app
Configuration:  /srv/config:/etc/myapp (read-only)
Logs:          /srv/logs:/var/log
```

---

### Environment Variables

![Image: Environment variables](/images/containers/env-vars.png)

Configure containers with environment variables:

**Common uses**:
- Database credentials
- API keys and tokens
- Application configuration
- Feature flags
- Runtime parameters

**Example**:
```
POSTGRES_PASSWORD=mypassword
DATABASE_URL=postgres://user:pass@db:5432/mydb
NODE_ENV=production
API_KEY=abc123xyz
LOG_LEVEL=debug
```

---

### Real-time Logs

![Image: Container logs streaming](/images/containers/logs-streaming.png)

**Log Features**:
- Real-time WebSocket streaming
- Separate stdout/stderr streams
- Auto-scroll to latest logs
- Download logs as text file
- Timestamp for each entry

**Use logs to**:
- Debug application issues
- Monitor application health
- Track requests and errors
- Audit container activity

---

### Resource Monitoring

![Image: Container stats dashboard](/images/containers/stats-dashboard.png)

**Metrics tracked**:
- CPU usage (%)
- Memory usage (MB / %)
- Network I/O (bytes in/out)
- Disk I/O
- Uptime

**Real-time updates**:
- Auto-refresh every 5 seconds
- Charts and graphs
- Historical data

---

## Comparison with VMs

| Feature | Containers | VMs |
|---------|------------|-----|
| **Isolation** | Kernel-level (microVM) | Full virtualization |
| **Boot time** | 2-3 seconds | 5-10 seconds |
| **Image source** | Docker Hub, registries | Custom kernels/rootfs |
| **Use case** | Run existing Docker images | Custom OS/kernel |
| **Deployment** | Pull image, configure, run | Build rootfs, configure kernel |
| **Ecosystem** | Docker ecosystem | Custom microVM ecosystem |
| **Resource overhead** | Low (Alpine + Docker) | Very low (custom kernel) |
| **Flexibility** | Docker-compatible apps | Any Linux distribution |

**When to use Containers**:
- ✅ You have existing Docker images
- ✅ You want Docker Hub ecosystem
- ✅ You need Docker compatibility
- ✅ Quick deployment is important

**When to use VMs**:
- ✅ You need custom kernel/OS
- ✅ You want maximum flexibility
- ✅ You need specific Linux distro
- ✅ You want minimal overhead

---

## Getting Started

### Prerequisites

Before deploying containers:

1. **Container runtime image** must be available:
   - Manager automatically builds it during setup
   - Located at `/srv/images/container-runtime.ext4`
   - Alpine Linux 3.18 + Docker 25.0.5

2. **Network bridge** must be configured:
   - Default: `fcbr0`
   - Setup: `sudo ./scripts/fc-bridge-setup.sh fcbr0 <interface>`

3. **Host with agent** must be registered:
   - At least one host must be online
   - Check: Dashboard → Hosts

### Quick Start

![Image: Quick start flow](/images/containers/quick-start.png)

1. **Go to Containers page**:
   - Navigate to "Containers" in sidebar

2. **Click "Deploy Container"**:
   - Opens deployment form

3. **Configure container**:
   - Name: `my-nginx`
   - Image: `nginx:alpine`
   - Ports: `8080:80`

4. **Deploy**:
   - Click "Deploy Container"
   - Wait for deployment (15-30 seconds)

5. **Access**:
   - Container runs on port 8080
   - View logs, stats, and shell

**Next steps**:
- **[Deploy a Container](deploy-container/)** - Step-by-step deployment guide
- **[Manage Containers](manage-containers/)** - Start, stop, restart, delete
- **[View Logs](logs/)** - Real-time log streaming
- **[Monitor Stats](stats/)** - Resource usage monitoring

---

## Best Practices

### Image Selection

✅ **Use Alpine-based images** when possible:
- Smaller size (nginx:alpine = 40 MB vs nginx:latest = 187 MB)
- Faster pulls and deploys
- Lower resource usage

✅ **Pin specific versions**:
```
Good: postgres:15.3-alpine
Bad:  postgres:latest
```

✅ **Use official images** from Docker Hub:
- Verified publishers
- Security updates
- Good documentation

---

### Resource Allocation

✅ **Start small, scale up**:
```
Initial:   0.5 vCPU, 512 MB
If needed: 1 vCPU, 1024 MB
If needed: 2 vCPU, 2048 MB
```

✅ **Monitor resource usage**:
- Check Stats tab regularly
- Adjust based on actual usage
- Avoid over-provisioning

❌ **Don't over-allocate**:
- Wastes host resources
- Limits number of containers
- Increases costs

---

### Volume Management

✅ **Use volumes for persistent data**:
- Databases: Always use volumes
- Configuration: Mount as read-only
- Logs: Optional (can use Docker logs)

✅ **Organize volumes by purpose**:
```
/srv/container-data/postgres-data    → Database
/srv/container-data/app-uploads      → User files
/srv/container-config/nginx          → Configuration
```

❌ **Don't store data inside container**:
- Data lost on container deletion
- Can't share between containers
- Hard to back up

---

### Security

✅ **Use environment variables for secrets**:
- Don't hardcode passwords in images
- Inject at runtime via env vars
- Rotate credentials regularly

✅ **Limit exposed ports**:
- Only map necessary ports
- Use non-standard host ports (e.g., 8080 instead of 80)
- Consider firewall rules

✅ **Keep images updated**:
- Check for security updates
- Rebuild with latest base images
- Monitor CVE announcements

---

### Monitoring

✅ **Check logs regularly**:
- Identify errors early
- Monitor application health
- Track unusual activity

✅ **Monitor resource usage**:
- Prevent resource exhaustion
- Identify performance issues
- Plan capacity

✅ **Set up alerts** (future feature):
- Container stopped unexpectedly
- High resource usage
- Error rate threshold

---

## Limitations

### Current Limitations

**One container per VM**:
- Can't run multiple containers in one VM
- Each container needs dedicated VM
- Use separate containers for microservices

**No container orchestration**:
- No automatic scaling (yet)
- No service discovery (yet)
- No load balancing (yet)
- Manage containers individually

**No Docker Compose support**:
- Can't deploy multi-container stacks with compose files
- Deploy each service as separate container
- Configure networking manually

**Resource allocation**:
- Resources set at deployment time
- Can't hot-resize CPU/memory (must stop container)
- Edit configuration when stopped

### Future Enhancements

Planned features:
- **Container orchestration** - Auto-scaling, service discovery
- **Docker Compose** - Deploy multi-container applications
- **Container networking** - Virtual networks, service mesh
- **Health checks** - Automatic restart on failure
- **Resource hot-resize** - Adjust CPU/memory without restart
- **Image caching** - Faster deployments with local cache

---

## Troubleshooting

### Container stuck in "Creating" state

**Cause**: VM creation failed or agent unresponsive

**Solution**:
1. Check host status: Dashboard → Hosts
2. Verify agent is running
3. Check agent logs on host
4. Delete and recreate container

---

### Container stuck in "Booting" state

**Cause**: VM failed to boot or network issue

**Solution**:
1. Check network bridge: `ip link show fcbr0`
2. View container VM: Click "View Container VM"
3. Check VM logs
4. Verify container runtime image exists

---

### Can't pull image from Docker Hub

**Cause**: Network issue, rate limit, or invalid image name

**Solution**:
1. Verify image exists on Docker Hub
2. Check correct image name and tag
3. Wait if rate-limited (anonymous: 100 pulls/6h)
4. Use authenticated pull with Docker Hub account

---

### Container exits immediately

**Cause**: Container process crashed or misconfigured

**Solution**:
1. View container logs (Logs tab)
2. Check environment variables
3. Verify image is correct
4. Check for missing dependencies

---

### Port mapping not working

**Cause**: Port conflict, firewall, or networking issue

**Solution**:
1. Verify port not already in use
2. Check host firewall rules
3. Ensure bridge networking configured
4. Try different host port

---

## Quick Reference

### Container Lifecycle

| Action | When Available | Result |
|--------|----------------|--------|
| **Start** | Stopped | Starts container |
| **Stop** | Running | Stops container gracefully |
| **Restart** | Running | Stops then starts container |
| **Pause** | Running | Pauses container execution |
| **Resume** | Paused | Resumes paused container |
| **Delete** | Stopped, Error | Permanently deletes container and VM |

### Common Image Examples

| Image | Ports | Environment | Use Case |
|-------|-------|-------------|----------|
| `nginx:alpine` | 80:80 | - | Static web server |
| `postgres:15-alpine` | 5432:5432 | `POSTGRES_PASSWORD` | Database |
| `redis:7-alpine` | 6379:6379 | - | Cache/Queue |
| `node:20-alpine` | 3000:3000 | `NODE_ENV` | Node.js app |
| `python:3.11-alpine` | 8000:8000 | - | Python app |
| `mongo:7` | 27017:27017 | `MONGO_INITDB_ROOT_PASSWORD` | MongoDB |

### Resource Guidelines

| Service Type | CPU | Memory |
|--------------|-----|--------|
| Static site | 0.5 | 256 MB |
| API server | 1-2 | 512-1024 MB |
| Database | 2-4 | 2048-4096 MB |
| Cache | 0.5-1 | 512-1024 MB |
| Message queue | 1-2 | 1024-2048 MB |

---

## Next Steps

- **[Deploy a Container](deploy-container/)** - Complete deployment walkthrough
- **[Manage Containers](manage-containers/)** - Lifecycle operations
- **[View Logs](logs/)** - Real-time log streaming guide
- **[Monitor Stats](stats/)** - Resource monitoring dashboard
