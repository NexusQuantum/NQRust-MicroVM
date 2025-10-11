#!/bin/bash

echo "=== VM Creation Debug Script ==="
echo "Testing each step of VM creation process"

# Check if required services are running
echo "1. Checking if manager is running..."
curl -s http://localhost:8080/health || echo "❌ Manager not accessible"

echo -e "\n2. Checking if agent is running..."
curl -s http://localhost:9090/health || echo "❌ Agent not accessible"

echo -e "\n3. Checking required directories..."
ls -la /srv/images/ | head -5 || echo "❌ /srv/images not accessible"
ls -la /tmp/claude/fc/ | head -5 || echo "❌ FC run dir not accessible"

echo -e "\n4. Testing VM creation with curl..."
curl -X POST http://localhost:8080/v1/vms \
  -H "Content-Type: application/json" \
  -d '{
    "name": "debug-test-vm-'$(date +%s)'",
    "vcpu": 1,
    "mem_mib": 512,
    "kernel_path": "/srv/images/vmlinux-5.10.186",
    "rootfs_path": "/srv/images/alpine-3.18-minimal.ext4"
  }' \
  --max-time 60 \
  -w "\nResponse time: %{time_total}s\nHTTP status: %{http_code}\nSize: %{size_download} bytes\n" \
  -v 2>&1 | grep -E "(HTTP|time_total|Response|Error|failed)"

echo -e "\n5. Checking for any VMs in database..."
curl -s http://localhost:8080/v1/vms | jq '.' || echo "❌ Could not get VM list"

echo -e "\n=== Debug complete ==="
