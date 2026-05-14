#!/usr/bin/env bash
# Runs INSIDE the privileged container.
# Sets up samba + agent + exercises every agent SMB route.
set -uo pipefail

PASS=0; FAIL=0
FAILED=()
ok()   { echo "  PASS  $*"; PASS=$((PASS+1)); }
fail() { echo "  FAIL  $*"; FAIL=$((FAIL+1)); FAILED+=("$*"); }
log()  { echo; echo "=== $* ==="; }

# --- Bootstrap dependencies ---
log "Installing samba + cifs-utils"
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq >/dev/null
apt-get install -y -qq --no-install-recommends samba samba-common-bin cifs-utils curl jq smbclient >/tmp/apt.log 2>&1 || {
  cat /tmp/apt.log
  exit 1
}

# --- Samba config ---
log "Configuring Samba"
mkdir -p /srv/test-share
chmod 777 /srv/test-share
cat > /etc/samba/smb.conf <<'EOF'
[global]
   workgroup = WORKGROUP
   server min protocol = SMB2
   server max protocol = SMB3
   map to guest = bad user
   log file = /var/log/samba/log.%m
   log level = 1
   server role = standalone server

[vms]
   path = /srv/test-share
   read only = no
   guest ok = yes
   force user = root
   force group = root
   create mask = 0660
   directory mask = 0770
EOF

useradd -M -s /usr/sbin/nologin vm-admin 2>/dev/null || true
(echo "smb-pass"; echo "smb-pass") | smbpasswd -a -s vm-admin >/dev/null
smbpasswd -e vm-admin >/dev/null

mkdir -p /var/log/samba /run/samba
log "Starting Samba"
smbd -D
sleep 2
smbclient -L //127.0.0.1 -U "vm-admin%smb-pass" 2>&1 | head -10 || true

# --- Start agent ---
log "Starting agent"
mkdir -p /etc/nqrust/storage-creds /var/lib/nqrust/smb /srv/fc/vms
AGENT_BIND=127.0.0.1:9090 MANAGER_BASE=http://127.0.0.1:18080 \
  FC_RUN_DIR=/srv/fc FC_BRIDGE=fcbr0 \
  /test/agent >/tmp/agent.log 2>&1 &
AGENT_PID=$!
for i in {1..15}; do
  if curl -fsS http://127.0.0.1:9090/healthz >/dev/null 2>&1 || \
     curl -fsS http://127.0.0.1:9090/v1/storage/smb/mount -X POST -d '{}' 2>&1 | grep -q "Json"; then
    break
  fi
  sleep 1
done
if ! kill -0 $AGENT_PID 2>/dev/null; then
  echo "agent did not start; log:"; cat /tmp/agent.log; exit 1
fi
ok "agent listening on 127.0.0.1:9090"

api()    { curl -sS -X POST -H 'Content-Type: application/json' "http://127.0.0.1:9090$1" -d "$2"; }
api_w()  { curl -sS -X POST -H 'Content-Type: application/json' "http://127.0.0.1:9090$1" -d "$2" -w "\n%{http_code}"; }

# --- T1: set_credentials → cred file on disk, 0600 ---
log "T1: set_credentials writes cred file mode 0600"
BACKEND_ID="00000000-0000-0000-0000-000000000111"
resp=$(api_w "/v1/storage/smb/set_credentials" \
  "{\"backend_id\":\"$BACKEND_ID\",\"username\":\"vm-admin\",\"password\":\"smb-pass\"}")
code=$(echo "$resp" | tail -1)
if [[ "$code" == "204" ]]; then ok "set_credentials -> 204"; else fail "set_credentials -> $code: $(echo "$resp" | head -1)"; fi

cred_file="/etc/nqrust/storage-creds/$BACKEND_ID.cred"
if [[ -f "$cred_file" ]]; then
  ok "cred file written at $cred_file"
  mode=$(stat -c '%a' "$cred_file")
  [[ "$mode" == "600" ]] && ok "cred file mode = 600" || fail "cred file mode = $mode (want 600)"
  grep -q "username=vm-admin" "$cred_file" && ok "cred file contains username=vm-admin" || fail "cred file missing username line"
  grep -q "password=smb-pass" "$cred_file" && ok "cred file contains password" || fail "cred file missing password line"
else
  fail "cred file NOT written"
fi

# --- T2: mount (authenticated) ---
log "T2: mount authenticated"
resp=$(api_w "/v1/storage/smb/mount" \
  "{\"backend_id\":\"$BACKEND_ID\",\"server\":\"127.0.0.1\",\"share\":\"vms\",\"username\":\"vm-admin\"}")
code=$(echo "$resp" | tail -1)
body=$(echo "$resp" | head -n -1)
if [[ "$code" == "200" ]]; then
  ok "mount -> 200"
  MP=$(echo "$body" | jq -r .mount_point)
  echo "    mount_point: $MP"
  if findmnt --mountpoint "$MP" >/dev/null 2>&1; then
    ok "findmnt sees $MP"
  else
    fail "findmnt did not see $MP"
  fi
else
  fail "mount -> $code: $body"
  echo "  agent log tail:"; tail -20 /tmp/agent.log | sed 's/^/    /'
  MP=""
fi

# --- T3: mount idempotency ---
log "T3: mount is idempotent"
resp=$(api_w "/v1/storage/smb/mount" \
  "{\"backend_id\":\"$BACKEND_ID\",\"server\":\"127.0.0.1\",\"share\":\"vms\",\"username\":\"vm-admin\"}")
code=$(echo "$resp" | tail -1)
[[ "$code" == "200" ]] && ok "second mount -> 200 (no error)" || fail "second mount -> $code"

# --- T4: create_file ---
if [[ -n "$MP" ]]; then
log "T4: create_file (sparse 100MB)"
resp=$(api_w "/v1/storage/smb/create_file" \
  "{\"mount_point\":\"$MP\",\"file\":\"smb-test-1.raw\",\"size_bytes\":104857600}")
code=$(echo "$resp" | tail -1)
[[ "$code" == "204" ]] && ok "create_file -> 204" || fail "create_file -> $code: $(echo "$resp" | head -1)"

if [[ -f "$MP/smb-test-1.raw" ]]; then
  sz=$(stat -c '%s' "$MP/smb-test-1.raw")
  if [[ "$sz" == "104857600" ]]; then
    ok "file logically sized 100MB"
  else
    fail "file size = $sz (want 104857600)"
  fi
else
  fail "file not found on share"
fi
fi

# --- T5: snapshot via copy ---
if [[ -n "$MP" ]]; then
log "T5: snapshot creates a copy"
resp=$(api_w "/v1/storage/smb/snapshot" \
  "{\"mount_point\":\"$MP\",\"source_file\":\"smb-test-1.raw\",\"snap_file\":\"smb-test-1.snap.raw\"}")
code=$(echo "$resp" | tail -1)
[[ "$code" == "204" ]] && ok "snapshot -> 204" || fail "snapshot -> $code: $(echo "$resp" | head -1)"
[[ -f "$MP/smb-test-1.snap.raw" ]] && ok "snap file present" || fail "snap file missing"
fi

# --- T6: clone_from_snapshot ---
if [[ -n "$MP" ]]; then
log "T6: clone_from_snapshot"
resp=$(api_w "/v1/storage/smb/clone_from_snapshot" \
  "{\"mount_point\":\"$MP\",\"snap_file\":\"smb-test-1.snap.raw\",\"file\":\"smb-test-1.clone.raw\"}")
code=$(echo "$resp" | tail -1)
body=$(echo "$resp" | head -n -1)
if [[ "$code" == "200" ]]; then
  ok "clone_from_snapshot -> 200"
  csize=$(echo "$body" | jq -r .size_bytes)
  [[ "$csize" == "104857600" ]] && ok "clone size = 100MB" || fail "clone size = $csize"
else
  fail "clone_from_snapshot -> $code: $body"
fi
fi

# --- T7: clone_from_path (e.g. base image) ---
if [[ -n "$MP" ]]; then
log "T7: clone_from_path"
echo "base-image-data" > /tmp/base-image.raw
resp=$(api_w "/v1/storage/smb/clone_from_path" \
  "{\"source_path\":\"/tmp/base-image.raw\",\"mount_point\":\"$MP\",\"file\":\"smb-from-path.raw\"}")
code=$(echo "$resp" | tail -1)
[[ "$code" == "200" ]] && ok "clone_from_path -> 200" || fail "clone_from_path -> $code: $(echo "$resp" | head -1)"
[[ -f "$MP/smb-from-path.raw" ]] && ok "cloned file present" || fail "cloned file missing"
fi

# --- T8: delete_file ---
if [[ -n "$MP" ]]; then
log "T8: delete_file"
resp=$(api_w "/v1/storage/smb/delete_file" \
  "{\"mount_point\":\"$MP\",\"file\":\"smb-test-1.raw\"}")
code=$(echo "$resp" | tail -1)
[[ "$code" == "204" ]] && ok "delete_file -> 204" || fail "delete_file -> $code"
[[ ! -f "$MP/smb-test-1.raw" ]] && ok "file removed from share" || fail "file still present"

# Idempotency
resp=$(api_w "/v1/storage/smb/delete_file" \
  "{\"mount_point\":\"$MP\",\"file\":\"smb-test-1.raw\"}")
code=$(echo "$resp" | tail -1)
[[ "$code" == "204" ]] && ok "delete_file idempotent (second call -> 204)" || fail "second delete -> $code"
fi

# --- T9: umount ---
if [[ -n "$MP" ]]; then
log "T9: umount"
resp=$(api_w "/v1/storage/smb/umount" "{\"mount_point\":\"$MP\"}")
code=$(echo "$resp" | tail -1)
[[ "$code" == "204" ]] && ok "umount -> 204" || fail "umount -> $code: $(echo "$resp" | head -1)"
if ! findmnt --mountpoint "$MP" >/dev/null 2>&1; then
  ok "mount torn down"
else
  fail "still mounted after umount"
fi
fi

# --- T10: anonymous (guest) mount ---
log "T10: anonymous mount (guest)"
BACKEND_ANON="00000000-0000-0000-0000-000000000222"
resp=$(api_w "/v1/storage/smb/mount" \
  "{\"backend_id\":\"$BACKEND_ANON\",\"server\":\"127.0.0.1\",\"share\":\"vms\"}")
code=$(echo "$resp" | tail -1)
body=$(echo "$resp" | head -n -1)
if [[ "$code" == "200" ]]; then
  ok "anonymous mount -> 200"
  MP_ANON=$(echo "$body" | jq -r .mount_point)
  api "/v1/storage/smb/umount" "{\"mount_point\":\"$MP_ANON\"}" >/dev/null
  ok "anonymous mount torn down"
else
  fail "anonymous mount -> $code: $body"
fi

# --- T11: clear_credentials removes cred file ---
log "T11: clear_credentials"
resp=$(api_w "/v1/storage/smb/clear_credentials" \
  "{\"backend_id\":\"$BACKEND_ID\"}")
code=$(echo "$resp" | tail -1)
[[ "$code" == "204" ]] && ok "clear_credentials -> 204" || fail "clear_credentials -> $code"
[[ ! -f "$cred_file" ]] && ok "cred file removed" || fail "cred file still present after clear"

# --- T12: bad credentials are rejected ---
log "T12: wrong-password mount is rejected"
BACKEND_BAD="00000000-0000-0000-0000-000000000333"
api "/v1/storage/smb/set_credentials" \
  "{\"backend_id\":\"$BACKEND_BAD\",\"username\":\"vm-admin\",\"password\":\"WRONG\"}" >/dev/null
resp=$(api_w "/v1/storage/smb/mount" \
  "{\"backend_id\":\"$BACKEND_BAD\",\"server\":\"127.0.0.1\",\"share\":\"vms\",\"username\":\"vm-admin\"}")
code=$(echo "$resp" | tail -1)
body=$(echo "$resp" | head -n -1)
if [[ "$code" == "500" ]]; then
  ok "bad password -> 500 (mount.cifs error)"
  echo "    surfaced: $(echo "$body" | jq -r '.error' | head -c 200)"
else
  fail "bad password -> $code: $body"
fi

# --- Cleanup ---
kill $AGENT_PID 2>/dev/null || true

echo
echo "================================================================"
echo "  SMB E2E results: $PASS passed, $FAIL failed"
echo "================================================================"
if [[ $FAIL -gt 0 ]]; then
  echo "Failed:"
  printf '  - %s\n' "${FAILED[@]}"
  exit 1
fi
