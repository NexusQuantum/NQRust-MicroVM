# CI/CD Build Optimization Guide

This document explains how to speed up GitHub Actions builds for NQRust-MicroVM.

## Current Optimizations

The workflows have been optimized with:

1. **sccache**: Distributed compilation cache that speeds up incremental builds by 50-80%
2. **lld linker**: Faster linker from LLVM (2-3x faster than GNU ld)
3. **Cargo caching**: Caches dependencies and build artifacts between runs

These optimizations are already in place in:
- [`.github/workflows/ci.yml`](.github/workflows/ci.yml) - CI pipeline
- [`.github/workflows/release.yml`](.github/workflows/release.yml) - Release builds

## Expected Build Times

### GitHub-hosted runners (ubuntu-22.04, 2 vCPU, 7GB RAM)

**Without optimizations:**
- Full clean build: ~25-35 minutes
- Incremental build: ~15-20 minutes

**With current optimizations:**
- First build: ~20-25 minutes (building sccache cache)
- Subsequent builds: ~8-12 minutes (80% cache hit rate)

### Self-hosted runners (example: 8 vCPU, 16GB RAM)

**Expected times:**
- First build: ~8-10 minutes
- Subsequent builds: ~3-5 minutes

## Option 1: Self-Hosted Runners (Recommended, Free)

Run builds on your own hardware for maximum performance and zero cost.

### Pros
- ✅ **Free** (use existing hardware)
- ✅ **Fast** (full control over specs)
- ✅ **Persistent cache** (sccache cache persists between builds)
- ✅ **No usage limits**
- ✅ **Full control** over environment

### Cons
- ❌ **Maintenance** required (keep runner updated)
- ❌ **Security considerations** (runs third-party code from PRs)
- ❌ **Availability** (must be online when builds run)

### Setup Instructions

#### 1. On your build machine (can be dev machine, VPS, or dedicated server)

```bash
# Create directory for runner
mkdir -p ~/actions-runner && cd ~/actions-runner

# Download latest runner (check https://github.com/actions/runner/releases for latest version)
curl -o actions-runner-linux-x64-2.311.0.tar.gz -L \
  https://github.com/actions/runner/releases/download/v2.311.0/actions-runner-linux-x64-2.311.0.tar.gz

# Extract
tar xzf ./actions-runner-linux-x64-2.311.0.tar.gz

# Configure runner
./config.sh --url https://github.com/NexusQuantum/NQRust-MicroVM --token <TOKEN>

# Install as systemd service (runs on boot)
sudo ./svc.sh install
sudo ./svc.sh start
```

#### 2. Get registration token

Navigate to: https://github.com/NexusQuantum/NQRust-MicroVM/settings/actions/runners/new

Or use GitHub CLI:
```bash
gh api --method POST repos/NexusQuantum/NQRust-MicroVM/actions/runners/registration-token
```

#### 3. Update workflows to use self-hosted runner

Edit `.github/workflows/release.yml`:
```yaml
jobs:
  build-binaries:
    runs-on: self-hosted  # Changed from: ubuntu-22.04
```

#### 4. Security best practices for self-hosted runners

**IMPORTANT:** Self-hosted runners have security implications:

- **Never use self-hosted runners on public repositories** unless you control all contributors
- **Use runner groups** to limit which workflows can use runners
- **Run in isolated environment** (VM, container, or dedicated machine)
- **Keep runner updated** regularly
- **Monitor runner logs** for suspicious activity

For public repos, use one of these strategies:
1. **Only run on protected branches** (main, release)
2. **Require approval for first-time contributors**
3. **Use ephemeral runners** (docker-based, destroyed after each job)

#### 5. Ephemeral Docker-based runners (safer for public repos)

Use [myoung34/docker-github-actions-runner](https://github.com/myoung34/docker-github-actions-runner):

```bash
docker run -d \
  --name github-runner \
  --restart unless-stopped \
  -e REPO_URL=https://github.com/NexusQuantum/NQRust-MicroVM \
  -e RUNNER_NAME=docker-runner \
  -e ACCESS_TOKEN=<TOKEN> \
  -e EPHEMERAL=true \
  -v /var/run/docker.sock:/var/run/docker.sock \
  myoung34/github-runner:latest
```

Ephemeral runners are destroyed after each job, providing better isolation.

### Recommended Hardware Specs

| Use Case | CPU | RAM | Disk | Cost |
|----------|-----|-----|------|------|
| **Light** (1-2 builds/day) | 4 cores | 8GB | 50GB SSD | Dev machine |
| **Medium** (5-10 builds/day) | 8 cores | 16GB | 100GB SSD | $10-20/mo VPS |
| **Heavy** (20+ builds/day) | 16 cores | 32GB | 200GB SSD | $40-60/mo VPS |

**VPS Providers:**
- **Hetzner Cloud**: €7-40/mo (excellent price/performance)
- **DigitalOcean**: $24-80/mo (easy setup)
- **Vultr**: $24-96/mo (good reliability)
- **AWS EC2**: $30-100/mo (pay-as-you-go)

## Option 2: BuildJet (Paid, Easy Setup)

Drop-in replacement for GitHub's runners with better specs.

### Pros
- ✅ **Easy setup** (just change `runs-on`)
- ✅ **Fast** (~3x faster than GitHub runners)
- ✅ **Managed** (no maintenance)
- ✅ **Secure** (same isolation as GitHub)

### Cons
- ❌ **Paid** (~$0.008/minute = $10-20/month typical)
- ❌ **Usage limits** (pay per minute)

### Setup

1. **Sign up**: https://buildjet.com/for-github-actions
2. **Connect your repo**
3. **Update workflow**:

```yaml
jobs:
  build-binaries:
    runs-on: buildjet-4vcpu-ubuntu-2204  # 4 vCPU, 8GB RAM
```

**Available runner sizes:**
- `buildjet-2vcpu-ubuntu-2204` - 2 vCPU, 4GB RAM (~$0.004/min)
- `buildjet-4vcpu-ubuntu-2204` - 4 vCPU, 8GB RAM (~$0.008/min)
- `buildjet-8vcpu-ubuntu-2204` - 8 vCPU, 16GB RAM (~$0.016/min)

**Cost estimate** for NQRust-MicroVM:
- ~10 builds/day × 10 min/build × $0.008/min × 30 days = **~$24/month**

## Option 3: Namespace (Paid, Fast)

Similar to BuildJet with different pricing model.

### Pros
- ✅ **Fast** (~5x faster)
- ✅ **Pay per build** (not per minute)
- ✅ **Managed**

### Cons
- ❌ **Paid** (~$0.01-0.02/build)

### Setup

1. **Sign up**: https://namespace.so/
2. **Update workflow**:

```yaml
jobs:
  build-binaries:
    runs-on: namespace-profile-rust
```

## Option 4: Advanced Caching Strategies

Further optimize existing runners with advanced caching.

### sccache with S3 backend

For distributed teams, use S3 to share sccache cache:

```yaml
env:
  SCCACHE_BUCKET: my-rust-cache
  SCCACHE_REGION: us-east-1
  AWS_ACCESS_KEY_ID: ${{ secrets.AWS_ACCESS_KEY_ID }}
  AWS_SECRET_ACCESS_KEY: ${{ secrets.AWS_SECRET_ACCESS_KEY }}

- name: Setup sccache with S3
  uses: mozilla-actions/sccache-action@v0.0.3
```

### Cargo incremental compilation

Enable incremental compilation (trade-off: faster rebuilds, larger cache):

```yaml
env:
  CARGO_INCREMENTAL: 1
```

### Parallel builds

Maximize CPU usage:

```yaml
env:
  CARGO_BUILD_JOBS: 8  # Adjust based on runner CPU count
```

## Performance Comparison

| Method | First Build | Incremental | Cost/Month | Setup Time |
|--------|-------------|-------------|------------|------------|
| **GitHub runners (default)** | 30 min | 15 min | Free | 0 min |
| **GitHub + optimizations** | 22 min | 10 min | Free | 10 min |
| **Self-hosted (8 core)** | 8 min | 3 min | $0-20 | 30 min |
| **BuildJet (4 core)** | 12 min | 5 min | $24 | 5 min |
| **Namespace** | 10 min | 4 min | $20 | 5 min |

## Recommendations

### For individual developers:
1. **Start with current optimizations** (already in place, free)
2. **Add self-hosted runner on dev machine** if you build frequently
3. Cost: **$0/month**

### For small teams:
1. **Use self-hosted runner on shared VPS**
2. Hetzner CCX23: 8 vCPU, 16GB RAM, €18/mo
3. Cost: **~$20/month**

### For larger projects:
1. **BuildJet for pull requests** (managed, secure)
2. **Self-hosted for release builds** (full control)
3. Cost: **~$40/month**

## Current Status

✅ **Already implemented:**
- sccache distributed compilation cache
- lld fast linker
- Cargo dependency caching
- Optimized Rust flags

🚀 **Ready to enable:**
- Self-hosted runners (just uncomment in workflow)
- BuildJet (change `runs-on` line)

## Monitoring Build Performance

### View sccache stats

After each build, check the workflow logs for:

```
Compile requests: 500
Compile requests executed: 100
Cache hits: 400
Cache hit rate: 80%
```

### Track build times

GitHub Actions automatically tracks build duration:
- Navigate to: https://github.com/NexusQuantum/NQRust-MicroVM/actions
- Click on any workflow run
- See duration in top-right corner

### Optimize based on metrics

If you see:
- **Cache hit rate < 50%**: Check if cache is being saved/restored correctly
- **Link time > 30% of build**: lld is working correctly
- **Build time > 15 min**: Consider self-hosted runner or external compute

## Troubleshooting

### sccache not caching

Check logs for:
```bash
sccache: error: No cached compilation found
```

**Fix**: Ensure `SCCACHE_GHA_ENABLED: "true"` is set in workflow env.

### lld linker not being used

Check logs for:
```bash
= note: /usr/bin/ld: ...
```

**Fix**: Ensure `RUSTFLAGS: "-C link-arg=-fuse-ld=lld"` is set and `lld` is installed.

### Self-hosted runner offline

```bash
# Check runner status
cd ~/actions-runner
sudo ./svc.sh status

# Restart runner
sudo ./svc.sh restart

# View logs
journalctl -u actions.runner.* -f
```

### Out of disk space on runner

```bash
# Clean old build artifacts
cargo clean

# Clean Docker images (if using)
docker system prune -af

# Check disk usage
df -h
du -sh ~/actions-runner
```

## Further Optimizations

### Future possibilities:
1. **Cross-compilation** (build on x86_64 for multiple targets)
2. **Distributed builds** (distcc for multi-machine compilation)
3. **Pre-built Docker images** (cache entire build environment)
4. **ARM64 runners** (AWS Graviton, better price/performance)

## Resources

- [GitHub Actions self-hosted runners](https://docs.github.com/en/actions/hosting-your-own-runners)
- [sccache documentation](https://github.com/mozilla/sccache)
- [lld linker](https://lld.llvm.org/)
- [Rust build optimization](https://doc.rust-lang.org/cargo/reference/profiles.html)
- [BuildJet for GitHub Actions](https://buildjet.com/for-github-actions)

## Support

For questions or issues with CI/CD optimization:
- Open an issue: https://github.com/NexusQuantum/NQRust-MicroVM/issues
- Check workflow runs: https://github.com/NexusQuantum/NQRust-MicroVM/actions
