#!/usr/bin/env bash
# Set up an in-VM Samba target for the v0.4.0 SMB integration test.
# Idempotent — safe to run multiple times.
set -euo pipefail

SHARE_DIR="${SHARE_DIR:-/var/lib/test-smb/share}"
SHARE_NAME="${SHARE_NAME:-vms}"
SMB_USER="${SMB_USER:-vm-admin}"
SMB_PASS="${SMB_PASS:-smb-pass}"

# Install samba if missing
if ! command -v smbd >/dev/null 2>&1; then
  DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    samba samba-common-bin cifs-utils
fi

mkdir -p "$SHARE_DIR"
chmod 777 "$SHARE_DIR"

# Minimal Samba config — single share, root squash off, guest allowed.
cat > /etc/samba/smb.conf <<EOF
[global]
   workgroup = WORKGROUP
   server min protocol = SMB2
   server max protocol = SMB3
   server signing = auto
   map to guest = bad user
   log file = /var/log/samba/log.%m
   log level = 1

[$SHARE_NAME]
   path = $SHARE_DIR
   read only = no
   guest ok = yes
   force user = root
   force group = root
   create mask = 0660
   directory mask = 0770
EOF

# Ensure the smb user exists in system AND in Samba's user DB
if ! id -u "$SMB_USER" >/dev/null 2>&1; then
  useradd -M -s /usr/sbin/nologin "$SMB_USER"
fi

# (Re)set samba password idempotently
(echo "$SMB_PASS"; echo "$SMB_PASS") | smbpasswd -a -s "$SMB_USER" >/dev/null
smbpasswd -e "$SMB_USER" >/dev/null

systemctl restart smbd nmbd

# Smoke test: list the share via smbclient
sleep 2
smbclient -L //127.0.0.1 -U "${SMB_USER}%${SMB_PASS}" 2>&1 | head -20 || true

echo "Samba target ready: //127.0.0.1/$SHARE_NAME (user=$SMB_USER, pass=$SMB_PASS, anonymous=yes)"
