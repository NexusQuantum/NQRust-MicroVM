+++
title = "Access to the VM"
description = "Connect to your VM via web console or SSH"
weight = 42
date = 2025-12-16
+++

Learn how to access your virtual machines using the web console or SSH.

---

## Web Console Access

The web console provides browser-based terminal access to your VMs - no additional software needed.

### Opening the Console

1. Navigate to **Virtual Machines** page
2. Click on your VM name to open details
3. Click the **Console** tab

**[IMAGE: vm-console-tab.png - Console tab in VM detail page]**

The web terminal will load in your browser.

---

### Logging In

#### With SSH Key

If you configured an SSH key during VM creation:

1. The console will show a login prompt
2. Type `root` and press Enter
3. You'll be logged in automatically (key-based auth)

**[IMAGE: console-ssh-key-login.png - Automatic login with SSH key]**

#### With Password

If you set a root password:

1. Type `root` at the login prompt
2. Press Enter
3. Type your password (hidden for security)
4. Press Enter

**[IMAGE: console-password-login.png - Password login prompt]**

---

### Using the Console

Once logged in, you have full terminal access:

**[IMAGE: console-logged-in.png - Active console session]**

**Available features**:
- Full keyboard support
- Copy & paste (Ctrl+Shift+C / Ctrl+Shift+V)
- Terminal resize
- Scrollback history

**Common commands**:

```bash
# Check system information
uname -a
cat /etc/os-release

# View processes
ps aux
top

# Check disk space
df -h

# View network configuration
ip addr
ip route

# Install packages (Alpine)
apk update
apk add curl vim

# Install packages (Ubuntu)
apt update
apt install -y curl vim
```

---

### Console Tips

**Copy Text from Console**:
1. Select text with mouse
2. Press `Ctrl+Shift+C`
3. Paste elsewhere with `Ctrl+V`

**Paste Text to Console**:
1. Copy text from somewhere (`Ctrl+C`)
2. Click in console window
3. Press `Ctrl+Shift+V`

**Clear Screen**:
```bash
clear
# or press Ctrl+L
```

**Exit/Logout**:
```bash
exit
# or press Ctrl+D
```

**Note**: The console stays connected even if you close the browser tab.

---

## SSH Access

For better performance and local tool integration, connect via SSH.

### Prerequisites

- VM must be running
- VM must have an IP address
- SSH key must be configured in the VM
- Your machine must be able to reach the VM's network

---

### Find VM IP Address

1. Open VM detail page
2. Look for **IP Address** in the overview section

**[IMAGE: vm-ip-address.png - IP address shown in VM details]**

Or use the console:

```bash
# In VM console
ip addr show eth0 | grep inet
```

Example IP: `192.168.1.100`

---

### Connect via SSH

#### From Linux/macOS

Open terminal and connect:

```bash
ssh root@192.168.1.100
```

**[IMAGE: ssh-linux-connect.png - SSH connection from terminal]**

#### From Windows

**Option 1: Windows Terminal / PowerShell**

```powershell
ssh root@192.168.1.100
```

**Option 2: PuTTY**

1. Download PuTTY from [putty.org](https://www.putty.org/)
2. Run PuTTY
3. Enter VM IP in "Host Name"
4. Port: 22
5. Click "Open"

**[IMAGE: putty-connection.png - PuTTY configuration window]**

---

### First Connection

On first connect, you'll see a security warning:

```
The authenticity of host '192.168.1.100' can't be established.
ED25519 key fingerprint is SHA256:...
Are you sure you want to continue connecting (yes/no)?
```

Type `yes` and press Enter.

**[IMAGE: ssh-first-connection.png - SSH host key verification]**

You should now be logged in!

```
Welcome to Alpine Linux 3.18
alpine:~#
```

---

### SSH with Custom Port

If you configured a custom SSH port:

```bash
ssh -p 2222 root@192.168.1.100
```

---

### SSH Config for Easy Access

Create an SSH config file for shortcuts:

```bash
# On your local machine
nano ~/.ssh/config
```

Add this configuration:

```
Host my-vm
    HostName 192.168.1.100
    User root
    Port 22
    IdentityFile ~/.ssh/id_ed25519
```

**[IMAGE: ssh-config-file.png - SSH config file example]**

Now connect easily:

```bash
ssh my-vm
```

---

## File Transfer

### Using SCP (Secure Copy)

**Upload file to VM**:

```bash
# From your local machine
scp /path/to/local/file.txt root@192.168.1.100:/root/
```

**[IMAGE: scp-upload.png - SCP upload in progress]**

**Download file from VM**:

```bash
# From your local machine
scp root@192.168.1.100:/root/file.txt /path/to/local/
```

**Upload directory**:

```bash
scp -r /path/to/local/dir root@192.168.1.100:/root/
```

---

### Using SFTP

Start SFTP session:

```bash
sftp root@192.168.1.100
```

**[IMAGE: sftp-session.png - Active SFTP session]**

**SFTP commands**:

```bash
# Upload file
put local-file.txt

# Download file
get remote-file.txt

# Upload directory
put -r local-directory

# List remote files
ls

# List local files
lls

# Change remote directory
cd /var/www

# Change local directory
lcd /home/user/downloads

# Exit SFTP
exit
```

---

### Using WinSCP (Windows)

1. Download WinSCP from [winscp.net](https://winscp.net/)
2. Install and run
3. Enter connection details:
   - Host: VM IP address
   - Port: 22
   - User: root
   - Password: (or use SSH key)
4. Click "Login"

**[IMAGE: winscp-interface.png - WinSCP file transfer interface]**

Drag and drop files between windows!

---

## Troubleshooting

### Issue: Can't Connect to Console

**Problem**: Console shows "Connection failed" or blank screen

**[IMAGE: troubleshoot-console-fail.png - Console connection error]**

**Solutions**:
1. Verify VM is in "Running" state
2. Refresh the browser page
3. Check browser console for JavaScript errors
4. Try different browser (Chrome, Firefox, Edge)
5. Ensure WebSocket connections are allowed
6. Disable browser extensions temporarily

---

### Issue: SSH Connection Refused

**Problem**: `ssh: connect to host 192.168.1.100 port 22: Connection refused`

**Solutions**:
1. Verify VM is running
2. Check VM has IP address
3. Ping the IP: `ping 192.168.1.100`
4. Verify SSH service is running in VM (via console):
   ```bash
   # Alpine
   rc-service sshd status

   # Ubuntu
   systemctl status sshd
   ```
5. Check firewall rules in VM

---

### Issue: SSH Permission Denied

**Problem**: `Permission denied (publickey)`

**Solutions**:
1. Verify SSH key was configured during VM creation
2. Check you're using correct SSH key:
   ```bash
   ssh -v root@192.168.1.100
   ```
3. Try password authentication (if enabled):
   ```bash
   ssh -o PreferredAuthentications=password root@192.168.1.100
   ```
4. Recreate VM with correct SSH key

---

### Issue: Console is Slow/Laggy

**Problem**: Console has noticeable input delay

**Solutions**:
- Use SSH instead of console for better performance
- Check network latency to the server
- Close other browser tabs
- Try console in private/incognito mode

---

### Issue: Can't Paste in Console

**Problem**: Ctrl+V doesn't work in console

**Solution**:
- Use `Ctrl+Shift+V` instead of `Ctrl+V`
- Or right-click and select "Paste"
- Some browsers: try middle mouse button click

---

## Security Best Practices

**SSH Keys**:
- ✅ Use ED25519 keys (modern, secure)
- ✅ Protect private keys with passphrase
- ✅ Never share private keys
- ✅ Use different keys for different environments

**Passwords**:
- ✅ Use strong, unique passwords
- ✅ Consider disabling password auth in production
- ✅ Rotate passwords regularly

**Network**:
- ✅ Use VPN for remote access
- ✅ Restrict SSH access by IP when possible
- ✅ Consider changing default SSH port (22)
- ✅ Enable fail2ban for brute-force protection

**Logging**:
- ✅ Monitor SSH login attempts
- ✅ Review console access logs
- ✅ Set up alerts for suspicious activity

---

## Next Steps

- **[Manage VM](manage-vm/)** - Learn lifecycle operations
- **[Monitoring](monitoring/)** - View VM performance metrics
- **[Backup & Snapshot](backup-snapshot/)** - Protect your VM data
