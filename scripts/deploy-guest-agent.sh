#!/bin/bash
# Deploy guest agent to a VM

set -e

VM_ID="$1"
MANAGER_URL="${MANAGER_URL:-http://localhost:8080}"
GUEST_AGENT_PATH="${GUEST_AGENT_PATH:-target/x86_64-unknown-linux-musl/release/guest-agent}"

if [ -z "$VM_ID" ]; then
    echo "Usage: $0 <vm-id>"
    echo ""
    echo "Example: $0 f63d66de-6232-4769-8b03-98b772c1ab1c"
    echo ""
    echo "This script will:"
    echo "1. Check if guest agent binary exists"
    echo "2. Serve the binary via HTTP"
    echo "3. Generate installation script for the VM"
    echo ""
    echo "Then you need to:"
    echo "1. Get the VM's shell (via manager UI)"
    echo "2. Run the installation commands shown below"
    exit 1
fi

if [ ! -f "$GUEST_AGENT_PATH" ]; then
    echo "Error: Guest agent binary not found at $GUEST_AGENT_PATH"
    echo "Please build it first:"
    echo "  cargo build --release --bin guest-agent --target x86_64-unknown-linux-musl"
    exit 1
fi

echo "==> Guest Agent Deployment Script"
echo ""
echo "VM ID: $VM_ID"
echo "Binary: $GUEST_AGENT_PATH"
echo ""

# Get local IP (try to find the right one)
LOCAL_IP=$(ip route get 1.1.1.1 | grep -oP 'src \K\S+')

echo "==> Step 1: Start HTTP Server"
echo "Run this in another terminal:"
echo ""
echo "  cd $(dirname $GUEST_AGENT_PATH)"
echo "  python3 -m http.server 8000"
echo ""
echo "Press Enter when HTTP server is running..."
read

echo ""
echo "==> Step 2: Installation Commands for VM"
echo "Copy and paste these commands in your VM's shell:"
echo ""
echo "---8<--- CUT HERE ---8<---"
cat << 'EOF'
# Download and install guest agent
GATEWAY=$(ip route | grep default | awk '{print $3}')
wget http://$GATEWAY:8000/guest-agent -O /usr/local/bin/guest-agent || \
  curl http://$GATEWAY:8000/guest-agent -o /usr/local/bin/guest-agent
chmod +x /usr/local/bin/guest-agent

# Create OpenRC service
cat > /etc/init.d/guest-agent << 'SVCEOF'
#!/sbin/openrc-run

name="guest-agent"
description="Guest metrics agent"
command="/usr/local/bin/guest-agent"
command_background=true
pidfile="/run/${RC_SVCNAME}.pid"
output_log="/var/log/guest-agent.log"
error_log="/var/log/guest-agent.err"

depend() {
    need net
    after firewall
}
SVCEOF

chmod +x /etc/init.d/guest-agent
rc-update add guest-agent default
rc-service guest-agent start

# Get VM IP and report to manager
MY_IP=$(ip addr show eth0 | grep 'inet ' | awk '{print $2}' | cut -d/ -f1)
echo "Guest agent installed! IP: $MY_IP"
echo "Waiting for agent to start..."
sleep 2

# Test guest agent
curl -s http://localhost:8080/health && echo " - Guest agent is running!"
curl -s http://localhost:8080/metrics | head -5

# Report IP to manager (you'll need to run this from host)
echo ""
echo "To complete setup, run this on the HOST:"
echo "curl -X POST ${MANAGER_URL}/v1/vms/${VM_ID}/guest-ip \\"
echo "  -H 'Content-Type: application/json' \\"
echo "  -d '{\"guest_ip\": \"'\$MY_IP'\"}'"
EOF
echo "---8<--- CUT HERE ---8<---"
echo ""
echo "After running the commands above, the VM will show you a curl command."
echo "Run that curl command on the HOST to register the guest IP."
echo ""
