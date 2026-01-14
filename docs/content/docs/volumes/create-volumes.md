+++
title = "Create Volumes"
description = "Add new storage volumes to the registry and prepare them for VM attachment"
weight = 72
date = 2025-01-13
+++

Learn how to create new volumes and import existing storage into the registry for use with your VMs.

---

## Volume Creation Methods

The registry supports three creation methods:

![Image: Creation methods](/images/volumes/creation-methods.png)

| Method | Use Case | Best For |
|--------|----------|----------|
| **Create New** | Empty volume on server | New storage needs |
| **Import Existing** | Register existing file | Pre-staged volumes |
| **Auto-Registration** | VM rootfs volumes | Automatic tracking |

---

## Create New Volume

Create a new empty volume on the server.

### When to Use

**Best for**:
- New data storage needs
- Database volumes
- Application storage
- Log storage
- Scratch space

**Requirements**:
- Server has available disk space
- You know required size
- Admin has enabled volume creation

---

### Step 1: Open Create Dialog

![Image: Create button](/images/volumes/create-button.png)

1. Go to Volumes page
2. Click **"Create Volume"** button
3. Create dialog appears

---

### Step 2: Enter Volume Name

![Image: Volume name](/images/volumes/create-name.png)

Provide a descriptive name:

**Good names**:
```
✅ postgres-data-prod
✅ webapp-uploads-staging
✅ logs-archive-2025-01
✅ scratch-processing-temp
```

**Avoid**:
```
❌ volume1
❌ data
❌ disk
❌ storage
```

**Naming tips**:
- Include application name
- Include environment (prod/staging/dev)
- Add purpose (data/logs/backup)
- Use hyphens, not spaces

---

### Step 3: Select Volume Type

![Image: Volume type](/images/volumes/create-type.png)

Choose volume type:
- **Data** - For application data storage
- **Scratch** - For temporary storage

**Type affects**:
- How the volume is categorized
- Default retention policy
- Backup inclusion

---

### Step 4: Select Format

![Image: Storage format](/images/volumes/create-format.png)

Choose storage format:

**ext4** (recommended):
- Best performance
- Native Linux filesystem
- Full feature support
- Most compatible

**qcow2**:
- Compressed storage
- Smaller file size
- Snapshot support
- Slightly slower

**raw**:
- Simple format
- Fast I/O
- No overhead
- Fixed size

---

### Step 5: Enter Volume Size

![Image: Volume size](/images/volumes/create-size.png)

Specify volume size:

**Size guidelines**:
```
Database volumes: 50-500 GB
Application data: 10-100 GB
Log storage: 20-200 GB
Scratch space: 10-50 GB
User uploads: 50-1000 GB
```

**Size format**:
- Enter number only
- Select unit: GB or TB
- Minimum: 1 GB
- Maximum: Depends on server capacity

**Planning tips**:
- Estimate current needs
- Add growth buffer (20-50%)
- Consider backup space
- Check server capacity first

---

### Step 6: Create Volume

![Image: Create progress](/images/volumes/create-progress.png)

Click **"Create"** button:

**What happens**:
1. Server validates request
2. Allocates disk space
3. Creates volume file
4. Formats filesystem (if ext4)
5. Registers in database
6. Success notification appears

**Creation time**:
- Small (10 GB): 10-30 seconds
- Medium (100 GB): 1-3 minutes
- Large (500 GB): 5-15 minutes

---

### Creation Tips

**Plan ahead**:
- Determine size requirements
- Choose appropriate format
- Consider performance needs

**Start small**:
- Create smaller volumes first
- Expand if needed (contact admin)
- Avoid wasting space

**Document usage**:
- Keep notes on purpose
- Track which app uses it
- Plan retention policy

---

## Import Existing Volume

Register volumes that already exist on the server.

### When to Use

**Best for**:
- Pre-staged volumes
- Migrated storage
- Manually created volumes
- Backup restored volumes

**Requirements**:
- Volume file exists on server
- You know the exact file path
- File has correct permissions

---

### Step 1: Open Import Dialog

![Image: Import button](/images/volumes/import-button.png)

1. Click **"Import Volume"** button
2. Import dialog opens

---

### Step 2: Enter File Path

![Image: Path input](/images/volumes/import-path.png)

Enter the full server path:

**Path examples**:
```
/srv/volumes/postgres-data.ext4
/mnt/storage/webapp-uploads.qcow2
/backup/restored-data.ext4
/tmp/migration/old-volume.raw
```

**Path requirements**:
- Must be absolute path (starts with /)
- File must exist at that location
- Server must have read/write access
- Correct permissions required

**Getting the path**:
- Ask your system administrator
- Check migration documentation
- Use file transfer logs
- Verify file exists first

---

### Step 3: Enter Volume Name

![Image: Import name](/images/volumes/import-name.png)

Provide a name for the registry:

**Note**: This is the display name, not the filename

**Example**:
```
File path: /srv/volumes/postgres_data_production.ext4
Volume name: postgres-data-prod
```

---

### Step 4: Select Type and Format

Choose volume type (Data/Scratch) and format (ext4/qcow2/raw):

**Auto-detection**:
- System tries to detect from filename
- Verify the selection is correct
- Change if needed

---

### Step 5: Import Volume

![Image: Import success](/images/volumes/import-success.png)

Click **"Import"** button:

**What happens**:
1. Server verifies path exists
2. Checks file permissions
3. Detects file size
4. Registers in database
5. Volume available immediately

**Speed**: Nearly instant (no file copy)

---

### Import Tips

**Verify path first**:
- Double-check spelling
- Include full path
- Use forward slashes (/)

**Common path errors**:
```
❌ volumes/data.ext4  (missing /)
❌ C:\volumes\file.ext4  (Windows path)
❌ ~/volumes/file.ext4  (tilde not expanded)
✅ /srv/volumes/data.ext4  (correct)
```

**Work with administrator**:
- Ask them to copy file to standard location
- Get exact path from them
- Confirm permissions are correct

---

## Automatic Volume Registration

Rootfs volumes are automatically registered when creating VMs.

### How It Works

**When creating a VM**:

![Image: VM creation with rootfs](/images/volumes/vm-with-rootfs.png)

**Step 1**: You create VM with rootfs image
**Step 2**: System automatically registers rootfs as volume
**Step 3**: Volume appears in Volumes page
**Step 4**: Shows 1 VM attached

**Example**:
```
Create VM with:
- Rootfs: ubuntu-22.04-base

Result:
- Volume "ubuntu-22.04-base" auto-registered
- Type: Rootfs
- Attached VMs: 1
- Visible in Volumes page
```

---

### Benefits

**No manual work**:
- Rootfs automatically tracked
- No need to manually register
- Central visibility
- Attachment tracking

**Use cases**:
- Track which VMs use which rootfs
- Identify unused rootfs images
- Plan rootfs cleanup
- Audit VM storage

---

## After Creation

### Verify Volume

![Image: Created volume](/images/volumes/verify-created.png)

**Success indicators**:
- Green success notification
- Volume appears in table
- Correct size shown
- Type and format correct

**Check**:
- Name is correct
- Size matches request
- Format is correct
- Type is correct
- Status is "Available"

---

### Attach to VM

After creating, attach volume to VM:

![Image: Attach volume](/images/volumes/attach-new.png)

**Steps**:
1. Find created volume in table
2. Click "Attach" button
3. Select target VM
4. Choose attachment mode
5. Confirm attachment

**See**: [Manage Volumes](manage-volumes/) for attachment guide

---

### Mount Inside VM

After attaching, mount the volume:

**Linux VMs**:
```bash
# List block devices
lsblk

# Create mount point
sudo mkdir -p /mnt/data

# Mount volume (usually /dev/vdb for second volume)
sudo mount /dev/vdb /mnt/data

# Verify mount
df -h /mnt/data
```

**Make mount permanent** (add to /etc/fstab):
```bash
# Get UUID
sudo blkid /dev/vdb

# Edit fstab
sudo nano /etc/fstab

# Add line:
UUID=your-uuid-here /mnt/data ext4 defaults 0 2
```

---

## Volume Templates

### Database Volume Template

**PostgreSQL data volume**:
```
Name: postgres-data-{env}-{date}
Type: Data
Format: ext4
Size: 100 GB
Purpose: PostgreSQL database storage
Mount: /var/lib/postgresql/data
```

---

### Web Application Volume

**Upload storage volume**:
```
Name: webapp-uploads-{env}
Type: Data
Format: ext4
Size: 50 GB
Purpose: User uploaded files
Mount: /var/www/uploads
```

---

### Log Archive Volume

**Log storage volume**:
```
Name: logs-archive-{year}-{month}
Type: Data
Format: ext4
Size: 20 GB
Purpose: Application and system logs
Mount: /var/log/archive
```

---

### Scratch Volume

**Temporary processing**:
```
Name: scratch-{purpose}-{id}
Type: Scratch
Format: raw
Size: 10 GB
Purpose: Temporary computation
Mount: /mnt/scratch
```

---

## Common Issues

### Issue: Creation Fails

**Symptoms**:
- Create operation fails
- Error notification appears

**Possible causes**:
1. Insufficient disk space on server
2. Size too large
3. Name already exists
4. Permission denied

**Solution**:
1. Check server has available space
2. Try smaller size
3. Use unique name
4. Contact administrator for permissions

---

### Issue: Path Not Found

**Symptoms**:
- Import fails with "File not found"
- Path validation error

**Possible causes**:
1. Typo in path
2. File doesn't exist
3. Incorrect permissions
4. Wrong server

**Solution**:
1. Verify exact path with administrator
2. Check file exists: `ls -lh /path/to/file`
3. Confirm read/write permissions
4. Use absolute path (starts with /)

---

### Issue: Format Detection Wrong

**Symptoms**:
- System detects wrong format
- Volume not usable

**Solution**:
1. Manually select correct format
2. Verify file type: `file /path/to/volume`
3. Check file extension
4. Contact administrator if unsure

---

## Best Practices

### Choose Right Size

**Sizing guidelines**:
```
Small (1-10 GB):
- Log rotation
- Cache storage
- Temporary files

Medium (10-100 GB):
- Application data
- User content
- Development workspace

Large (100-500 GB):
- Database storage
- Media files
- Archive storage

Very Large (500+ GB):
- Big data
- Video storage
- Long-term archives
```

---

### Naming Conventions

**Standard pattern**: `{app}-{purpose}-{env}-{date}`

**Examples**:
```
postgres-data-prod
webapp-uploads-staging
logs-archive-2025-01
scratch-ml-training-01
redis-cache-dev
nginx-static-prod
```

---

### Format Selection

**Choose format based on needs**:

**ext4 when**:
- Performance is priority
- Full Linux features needed
- Standard use case

**qcow2 when**:
- Space is limited
- Snapshots are needed
- Compression helps

**raw when**:
- Maximum performance needed
- Simplicity preferred
- Direct disk access

---

### Document Volumes

**Keep external notes**:
```
Volume: postgres-data-prod
Size: 200 GB
Format: ext4
Purpose: Production PostgreSQL database
Attached: database-server-01
Mount: /var/lib/postgresql/data
Owner: database-team@company.com
Created: 2025-01-13
Backup: Daily at 02:00 UTC
Retention: 30 days
```

---

## Quick Reference

### Creation Methods

| Method | Speed | Size Limit | Requires |
|--------|-------|------------|----------|
| Create New | Medium | Server limit | Disk space |
| Import | Instant | Unlimited | Existing file |
| Auto-Registration | Instant | N/A | VM creation |

### Format Comparison

| Format | Speed | Features | Compression | Best For |
|--------|-------|----------|-------------|----------|
| ext4 | Fast | Full | No | General use |
| qcow2 | Medium | Snapshots | Yes | Space saving |
| raw | Fast | None | No | Performance |

### Size Guidelines

| Use Case | Recommended Size |
|----------|------------------|
| Database | 50-500 GB |
| Application data | 10-100 GB |
| User uploads | 50-1000 GB |
| Logs | 20-200 GB |
| Scratch | 10-50 GB |
| Cache | 5-20 GB |

---

## Next Steps

- **[Browse Volumes](browse-volumes/)** - Find created volumes
- **[Manage Volumes](manage-volumes/)** - Attach volumes to VMs
- **[Volumes Overview](../)** - Learn about volume types
- **[Create VM](/docs/vm/create-vm/)** - Create VMs with volumes
