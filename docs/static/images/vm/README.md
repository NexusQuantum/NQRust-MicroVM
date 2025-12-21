# Screenshot Requirements for Create a VM Documentation

Upload all screenshots to this folder: `/docs/static/images/vm/`

After uploading, images will be accessible at: `http://localhost:1313/images/vm/filename.png`

---

## 📸 Screenshot List (Based on Actual UI)

The wizard has **6 steps**: Basic Info, Credentials, Machine Config, Boot Source, Network, and Review.

---

### Step 1: Open Wizard

#### 1. vm-create-button.png
**Location**: VMs List Page
**What to capture**:
- Virtual Machines page header
- **"Create VM"** button in top-right corner (highlight with arrow or box)
- Part of VM list table visible below

**Example**:
```
Virtual Machines                    [+ Create VM] ← HIGHLIGHT
────────────────────────────────────────────────
VM Name        Status    IP Address    CPU
web-server-01  Running   192.168.1.10  45%
```

---

### Step 2: Basic Information

#### 2. vm-step1-basic.png
**Location**: Create VM Wizard - Step 1/6
**What to capture**:
- Full wizard dialog showing "Basic Info" step (1 of 6)
- Progress indicators at top (step 1 active)
- Form fields:
  - **Name**: filled with example "my-ubuntu-vm"
  - **Owner**: showing default "developer"
  - **Environment**: dropdown showing "Development"
  - **Description**: filled with example text
- Next button at bottom

**Example**:
```
Create Virtual Machine    [1][2][3][4][5][6]
                          ^^^
Basic Info
────────────────────────────
Name *
[my-ubuntu-vm                      ]

Owner *
[developer                         ]

Environment *
[Development                      ▼]

Description (Optional)
[Ubuntu dev environment            ]
[                                  ]

               [Cancel]  [Next >]
```

#### 3. vm-environment-dropdown.png
**Location**: Basic Info step, Environment dropdown expanded
**What to capture**:
- Environment dropdown in **expanded state**
- Three options visible:
  - Development
  - Staging
  - Production

**Example**:
```
Environment *
[Development                      ▼]
├─ Development         ← SELECTED
├─ Staging
└─ Production
```

---

### Step 3: Credentials

#### 4. vm-step2-credentials.png
**Location**: Create VM Wizard - Step 2/6
**What to capture**:
- Wizard step 2 active
- Form fields:
  - **Username**: showing "root" (default)
  - **Password**: showing password dots (••••)
- Help text visible ("Default: root")
- Previous and Next buttons

**Example**:
```
Create Virtual Machine    [1][2][3][4][5][6]
                             ^^^
Credentials
────────────────────────────
Username *
[root                          ]
Default: root

Password *
[••••••••                      ]

      [< Back]  [Cancel]  [Next >]
```

---

### Step 4: Machine Configuration

#### 5. vm-step3-machine.png
**Location**: Create VM Wizard - Step 3/6
**What to capture**:
- Wizard step 3 active
- **vCPU slider** showing value (e.g., 2)
- **Memory slider** showing value in MiB (e.g., 2048)
- Two checkboxes below:
  - ☐ Enable SMT (unchecked)
  - ☐ Track dirty pages (unchecked)
- Help text "Must be a multiple of 128 MiB"

**Example**:
```
Create Virtual Machine    [1][2][3][4][5][6]
                                ^^^
Machine Config
────────────────────────────
vCPU Count: 2
[───●─────────────────]
1                     32

Memory: 2048 MiB
[────────●────────────]
128                32768
Must be a multiple of 128 MiB

☐ Enable SMT (Simultaneous Multithreading)
☐ Track dirty pages

      [< Back]  [Cancel]  [Next >]
```

#### 6. vm-vcpu-slider.png
**Location**: Machine Config step, vCPU slider close-up
**What to capture**:
- Just the vCPU slider control
- Label showing current value
- Min/max range visible

#### 7. vm-memory-slider.png
**Location**: Machine Config step, Memory slider close-up
**What to capture**:
- Just the Memory slider control
- Label showing current MiB value
- Help text about 128 MiB multiple

#### 8. vm-advanced-options.png
**Location**: Machine Config step, advanced options
**What to capture**:
- The two checkboxes:
  - ☐ Enable SMT
  - ☐ Track dirty pages
- Clear and readable

---

### Step 5: Boot Source

#### 9. vm-step4-boot.png
**Location**: Create VM Wizard - Step 4/6
**What to capture**:
- Wizard step 4 active
- Four form fields:
  - **Kernel Image** dropdown (selected or showing option)
  - **Rootfs Image** dropdown (selected or showing option)
  - **Initrd Path** input (empty, optional)
  - **Boot Arguments** input (empty, optional)
- Help text showing image counts

**Example**:
```
Create Virtual Machine    [1][2][3][4][5][6]
                                   ^^^
Boot Source
────────────────────────────
Kernel Image *
[vmlinux-5.10.fc.bin       ▼]
2 kernel(s) available

Rootfs Image *
[Alpine Linux 3.18         ▼]
5 rootfs image(s) available

Initrd Path (Optional)
[                          ]

Boot Arguments (Optional)
[                          ]

      [< Back]  [Cancel]  [Next >]
```

#### 10. vm-kernel-dropdown.png
**Location**: Boot Source step, Kernel dropdown
**What to capture**:
- Kernel dropdown (can be closed or expanded)
- Show at least one kernel option

#### 11. vm-rootfs-selection.png
**Location**: Boot Source step, Rootfs dropdown **expanded**
**What to capture**:
- Rootfs dropdown in expanded state
- Multiple options visible:
  - Alpine Linux 3.18
  - Ubuntu 22.04
  - Debian 12
  - Others (if available)

**Example**:
```
Rootfs Image *
[Alpine Linux 3.18         ▼]
├─ Alpine Linux 3.18
├─ Ubuntu 22.04           ← Highlight one
├─ Debian 12
└─ Custom Image
```

#### 12. vm-initrd-input.png
**Location**: Boot Source step, Initrd field
**What to capture**:
- Just the Initrd Path input field
- Help text visible

#### 13. vm-bootargs-input.png
**Location**: Boot Source step, Boot Args field
**What to capture**:
- Just the Boot Arguments input field
- Help text visible

---

### Step 6: Network

#### 14. vm-step5-network.png
**Location**: Create VM Wizard - Step 5/6
**What to capture**:
- Wizard step 5 active
- **Enable networking** checkbox (checked by default)
- **Host Device** input showing "tap0"
- **Guest MAC Address** input (empty or with value)
- **Generate** button next to Guest MAC
- Help text "Default: tap0" and "Leave empty for auto-generation"

**Example**:
```
Create Virtual Machine    [1][2][3][4][5][6]
                                      ^^^
Network
────────────────────────────
☑ Enable networking
Default: enabled

Host Device Name
[tap0                      ]
Default: tap0

Guest MAC Address
[                          ] [Generate]
Leave empty for auto-generation

      [< Back]  [Cancel]  [Next >]
```

#### 15. vm-network-enable.png
**Location**: Network step, Enable checkbox
**What to capture**:
- Just the "Enable networking" checkbox
- Checked state visible

#### 16. vm-host-device.png
**Location**: Network step, Host Device field
**What to capture**:
- Host Device input field showing "tap0"
- Help text visible

#### 17. vm-guest-mac.png
**Location**: Network step, Guest MAC field
**What to capture**:
- Guest MAC Address input field
- **Generate** button next to it
- Either empty or with generated MAC (e.g., aa:bb:cc:dd:ee:ff)

---

### Step 7: Review & Create

#### 18. vm-step6-review.png
**Location**: Create VM Wizard - Step 6/6 (Review)
**What to capture**:
- Wizard step 6 active
- Full review page showing all summary cards:
  - Basic Information card
  - Machine Configuration card
  - Boot Source card
  - Network card
- **Create VM** button at bottom (highlighted)

**Example**:
```
Create Virtual Machine    [1][2][3][4][5][6]
                                         ^^^
Review & Create
────────────────────────────
Please review your configuration

┌─ Basic Information ─────────┐
│ Name: my-ubuntu-vm          │
│ Owner: developer            │
│ Environment: Development    │
│ Description: Test VM        │
└─────────────────────────────┘

┌─ Machine Configuration ─────┐
│ vCPU: 2                     │
│ Memory: 2048 MiB            │
│ SMT: Disabled               │
│ Track Dirty Pages: No       │
└─────────────────────────────┘

... (more cards) ...

      [< Back]  [Cancel]  [Create VM] ← HIGHLIGHT
```

#### 19. vm-review-basic.png
**Location**: Review step, Basic Info card
**What to capture**:
- Just the "Basic Information" summary card
- Shows Name, Owner, Environment, Description

#### 20. vm-review-machine.png
**Location**: Review step, Machine Config card
**What to capture**:
- Just the "Machine Configuration" summary card
- Shows vCPU, Memory, SMT, Track Dirty Pages

#### 21. vm-review-boot.png
**Location**: Review step, Boot Source card
**What to capture**:
- Just the "Boot Source" summary card
- Shows Kernel path, Rootfs path

#### 22. vm-review-network.png
**Location**: Review step, Network card
**What to capture**:
- Just the "Network" summary card
- Shows Enabled status, Host Device, Guest MAC

#### 23. vm-create-button-review.png
**Location**: Review step, Create VM button
**What to capture**:
- Bottom of review page
- **Create VM** button highlighted
- Previous and Cancel buttons also visible

---

### VM Creation & Success

#### 24. vm-creating.png
**Location**: After clicking Create VM, loading state
**What to capture**:
- Loading spinner or progress indicator
- Text "Creating VM..." or "Creating Virtual Machine..."
- Optional: Progress steps with checkmarks

**Example**:
```
Creating Virtual Machine...

✓ Allocating resources
✓ Configuring Firecracker
⟳ Attaching kernel and rootfs
  Configuring network
  Starting VM

[Loading spinner]
```

#### 25. vm-created-success.png
**Location**: VM Detail Page or success notification
**What to capture**:
- Success notification/toast (green)
  - Text: "VM created successfully" or similar
- VM detail page visible in background
- Status showing "Running" with green indicator

**Example**:
```
✓ VM "my-ubuntu-vm" created successfully

VM Details: my-ubuntu-vm
───────────────────────────
Status: ● Running
IP Address: 192.168.1.100
Uptime: 00:00:05

[Graphs...]
```

---

### Verification

#### 26. vm-detail-running.png
**Location**: VM Detail Page - Overview Tab
**What to capture**:
- Complete VM detail page
- **Overview** tab active
- Status: Running (green indicator)
- IP address displayed
- CPU and Memory graphs showing activity
- Uptime counter visible

#### 27. vm-console-tab.png
**Location**: VM Detail Page - Console Tab
**What to capture**:
- VM detail page
- **Console** tab highlighted/active
- Terminal area visible with login prompt

**Example**:
```
my-ubuntu-vm
────────────────────────────
Overview | Console | Metrics
         ^^^^^^^^^
         ACTIVE TAB

┌────────────────────────────┐
│ Welcome to Alpine Linux    │
│ alpine login: _            │
└────────────────────────────┘
```

#### 28. vm-console-logged-in.png
**Location**: Console after successful login
**What to capture**:
- Console terminal
- Welcome message
- Shell prompt (alpine:~# or similar)
- Cursor visible

#### 29. vm-network-test.png
**Location**: Console with ping command
**What to capture**:
- Console terminal
- Command: `ping -c 3 google.com`
- Successful output showing:
  - Ping responses (3 packets)
  - 0% packet loss
  - Statistics summary

**Example**:
```
alpine:~# ping -c 3 google.com
PING google.com: 56 data bytes
64 bytes from 142.250.185.46: seq=0 ttl=116 time=10.2 ms
64 bytes from 142.250.185.46: seq=1 ttl=116 time=9.8 ms
64 bytes from 142.250.185.46: seq=2 ttl=116 time=10.1 ms

--- google.com ping statistics ---
3 packets transmitted, 3 received, 0% packet loss
alpine:~# _
```

---

### Quick Start & Troubleshooting

#### 30. template-deploy.png
**Location**: Quick Create from Template dialog
**What to capture**:
- "Quick Create VM from Template" dialog
- Template selection showing template cards with:
  - Template name
  - vCPU and Memory specs
  - Checkmark on selected template
- VM Name input field
- Create VM button

**Example**:
```
Quick Create VM from Template

Select a template:
┌─ Ubuntu 22.04 Base ─────┐
│ ✓ Selected              │
│ 2 vCPU, 2048 MiB       │
└─────────────────────────┘

VM Name
[my-ubuntu-from-template  ]

         [Cancel]  [Create VM]
```

#### 31. troubleshoot-no-images.png
**Location**: Boot Source step with no images
**What to capture**:
- Step 4 Boot Source
- Kernel or Rootfs dropdown showing:
  - "No kernel images available" or
  - "No rootfs images available"
- Empty dropdown

#### 32. troubleshoot-resources.png
**Location**: Error dialog when VM creation fails
**What to capture**:
- Error modal/dialog
- Error icon (⚠️ or ❌)
- Message: "Insufficient resources available" or similar
- Details about required vs available resources
- OK/Close button

**Example**:
```
┌────────────────────────────┐
│  ⚠️  Error Creating VM     │
├────────────────────────────┤
│  Insufficient resources    │
│                            │
│  Required: 2 vCPU, 2048MiB│
│  Available: 1 vCPU, 1024MiB│
│                            │
│           [OK]             │
└────────────────────────────┘
```

---

## 📝 How to Upload Screenshots

1. **Take screenshots** according to descriptions above
2. **Save** with exact filename (e.g., `vm-create-button.png`)
3. **Upload to**:
   ```
   /home/shiro/nexus/nqrust-microvm/docs/static/images/vm/
   ```
4. **Verify** by visiting:
   ```
   http://localhost:1313/images/vm/filename.png
   ```

---

## 🎨 Screenshot Tips

- **Resolution**: 1920x1080 or higher
- **Format**: PNG (preferred for UI) or JPG
- **Cropping**: Focus on relevant area, don't need full screen
- **Highlighting**: Use arrows or colored boxes for important elements
- **Text clarity**: Ensure all text is readable
- **Theme**: Use consistent theme (light mode recommended for docs)
- **Browser**: Use same browser for consistency
- **No sensitive data**: Don't show real IP addresses, passwords, or private info

---

## ✅ Screenshot Checklist

Mark (✓) when uploaded:

### Step 1: Open Wizard
- [ ] 1. vm-create-button.png

### Step 2: Basic Info
- [ ] 2. vm-step1-basic.png
- [ ] 3. vm-environment-dropdown.png

### Step 3: Credentials
- [ ] 4. vm-step2-credentials.png

### Step 4: Machine Config
- [ ] 5. vm-step3-machine.png
- [ ] 6. vm-vcpu-slider.png
- [ ] 7. vm-memory-slider.png
- [ ] 8. vm-advanced-options.png

### Step 5: Boot Source
- [ ] 9. vm-step4-boot.png
- [ ] 10. vm-kernel-dropdown.png
- [ ] 11. vm-rootfs-selection.png
- [ ] 12. vm-initrd-input.png
- [ ] 13. vm-bootargs-input.png

### Step 6: Network
- [ ] 14. vm-step5-network.png
- [ ] 15. vm-network-enable.png
- [ ] 16. vm-host-device.png
- [ ] 17. vm-guest-mac.png

### Step 7: Review
- [ ] 18. vm-step6-review.png
- [ ] 19. vm-review-basic.png
- [ ] 20. vm-review-machine.png
- [ ] 21. vm-review-boot.png
- [ ] 22. vm-review-network.png
- [ ] 23. vm-create-button-review.png

### Creation & Success
- [ ] 24. vm-creating.png
- [ ] 25. vm-created-success.png

### Verification
- [ ] 26. vm-detail-running.png
- [ ] 27. vm-console-tab.png
- [ ] 28. vm-console-logged-in.png
- [ ] 29. vm-network-test.png

### Quick Start & Troubleshooting
- [ ] 30. template-deploy.png
- [ ] 31. troubleshoot-no-images.png
- [ ] 32. troubleshoot-resources.png

**Total**: 32 screenshots

---

## 📄 Reference

- Documentation file: `/docs/content/docs/vm/create-vm.md`
- UI component: `/apps/ui/components/vm/vm-create-wizard.tsx`
- Steps: Basic Info → Credentials → Machine Config → Boot Source → Network → Review
