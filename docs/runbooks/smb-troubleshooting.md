# SMB (CIFS) Backend Troubleshooting

This runbook covers the `smb` storage backend (added 2026-05-10). See
`docs/superpowers/plans/2026-05-10-smb-backend.md` for the design and
`apps/manager/src/features/storage/backends/smb.rs` plus
`apps/agent/src/features/storage/smb.rs` for implementation.

## Architecture recap

Manager talks to agent over HTTP at `/v1/storage/smb/*`. Agent runs as
root, owns the SMB mount lifecycle (`mount.cifs`), and stores
per-backend credentials at `/etc/nqrust/storage-creds/<backend-id>.cred`
(mode 0600). Manager never persists the password — UI input is sent to
the agent and written to the cred file via `POST /set_credentials`.

Each `smb` backend mounts one share at `/var/lib/nqrust/smb/<server>:<share>`.
Per-VM rootfs files live as `<mount-point>/smb-<volume-uuid>.raw`.

## Common failure modes

### Add Backend fails: "mount.cifs failed: exit 13"
`mount.cifs` exit 13 means **wrong username / password / domain**. Check:
- Username spelled correctly (case-sensitive on some Samba configs)
- Password actually populated (UI password field; check the toast detail)
- Domain field if the share requires AD authentication
- The user actually has permissions on the share (test with `smbclient -U user //server/share` from any Linux box)

### "mount.cifs failed: exit 32"
The share doesn't exist on the server. Verify with `smbclient -L //server -U user`.

### "mount.cifs: bad option"
Server doesn't support the requested SMB version. Try:
- Set "SMB version" advanced field to a different value (`3` or `2.1` are good fallbacks for older servers)
- Or leave at `default` and let the kernel negotiate

### "Permission denied writing to share"
The file permissions on the SMB share don't match what root-on-VM-host can write. Two fixes:
- (Recommended) On the SMB server, give the user write access to the share
- (Workaround) In the SMB backend's advanced `Extra mount options`, add `uid=0,gid=0,file_mode=0660,dir_mode=0770` — forces all files to be owned by root inside the mount

### Edit-in-place: password unchanged after save
The "Rotate SMB password" field in the edit dialog is **blank by default** to preserve the existing credential. To rotate, you must explicitly type the new password. Saving with a blank field keeps the current cred file intact.

### Anonymous shares not mounting
- Leave both username AND password fields blank in the UI form
- Verify on the server side that the share allows guest access (`guest ok = yes` in `smb.conf`)
- Some servers require `username=guest` rather than just `-o guest` — use the Extra mount options field to add `username=guest` if needed

### Mount probe times out during Add Backend
The 30-second probe timeout was exceeded. Most likely causes:
- Server is not reachable on TCP/445 from the agent host (firewall?)
- DNS resolution slow / failed for the server hostname (use IP address instead)
- Server is doing a slow auth (Active Directory lookup, replication lag)

Check from the agent host:
```
nc -vz <server> 445
mount.cifs //<server>/<share> /mnt/test -o user=<u>,password=<p> -v
```

### Credential file went missing
Manifests as `ensure_mounted` errors with "credentials file missing for backend X". Causes:
- Operator manually deleted `/etc/nqrust/storage-creds/<backend-id>.cred`
- Agent restart on a non-persistent root filesystem

Recover: open the Edit dialog on the backend, type the password into the "Rotate SMB password" field, save. The manager re-sends the credential to the agent.

### Two managers fighting (HA, not currently supported)
Not yet a problem — this codebase has one manager. If you scale out, postgres advisory locks around credential delivery are the mitigation.

## Reference commands

```
# Inspect agent-side mount
sudo findmnt --mountpoint /var/lib/nqrust/smb/<server>:<share>

# Verify credential file
sudo ls -la /etc/nqrust/storage-creds/
sudo cat /etc/nqrust/storage-creds/<backend-id>.cred  # only as root

# Server-side share inspection
smbclient -L //<server> -U <user>
smbclient //<server>/<share> -U <user> -c 'ls'
```
