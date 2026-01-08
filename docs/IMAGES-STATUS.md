# 📸 Status Gambar Dokumentasi

**Last Updated**: 2026-01-06

---

## 📊 Overview

| Section | Uploaded | Required | Progress | Status |
|---------|----------|----------|----------|--------|
| **Containers** | 1 | 50 | 2% | ❌ **URGENT** |
| **Functions** | 50 | 62 | 81% | 🟡 Almost Done |
| **VM** | 56 | 59 | 95% | ✅ Nearly Complete |
| **TOTAL** | 107 | 171 | 63% | 🟡 In Progress |

---

## ❌ CONTAINERS - 49 Gambar Kurang (PRIORITAS TINGGI)

**Upload ke**: `/home/shiro/nexus/nqrust-microvm/docs/static/images/containers/`

### ✅ Uploaded (1)
- [x] deploy-step1-nav.png

### ❌ Missing (49)

#### Step 1 - Open Deployment Page
- [ ] deploy-step1-form.png

#### Step 2 - Basic Configuration (6)
- [ ] deploy-step2-name.png
- [ ] deploy-step2-image-tabs.png
- [ ] deploy-step2-registry.png
- [ ] deploy-step2-dockerhub.png
- [ ] deploy-step2-upload.png

#### Step 3 - Configure Resources (3)
- [ ] deploy-step3-resources.png
- [ ] deploy-step3-cpu.png
- [ ] deploy-step3-memory.png

#### Step 4 - Port Mappings (5)
- [ ] deploy-step4-ports.png
- [ ] deploy-step4-add-port.png
- [ ] deploy-step4-port-row.png
- [ ] deploy-step4-multiple.png
- [ ] deploy-step4-remove.png

#### Step 5 - Environment Variables (4)
- [ ] deploy-step5-env.png
- [ ] deploy-step5-add-env.png
- [ ] deploy-step5-env-row.png
- [ ] deploy-step5-remove.png

#### Step 6 - Volume Mounts (7)
- [ ] deploy-step6-volumes.png
- [ ] deploy-step6-add-button.png
- [ ] deploy-step6-volume-dialog.png
- [ ] deploy-step6-new-volume.png
- [ ] deploy-step6-existing.png
- [ ] deploy-step6-volume-table.png
- [ ] deploy-step6-remove-volume.png

#### Step 7 - Private Registry Auth (6)
- [ ] deploy-step7-registry.png
- [ ] deploy-step7-enable.png
- [ ] deploy-step7-fields.png
- [ ] deploy-step7-dockerhub.png
- [ ] deploy-step7-github.png
- [ ] deploy-step7-gitlab.png

#### Step 8 - Review and Deploy (2)
- [ ] deploy-step8-review.png
- [ ] deploy-step8-button.png

#### Step 9 - Deployment Progress (6)
- [ ] deploy-step9-progress.png
- [ ] deploy-step9-creating.png
- [ ] deploy-step9-booting.png
- [ ] deploy-step9-initializing.png
- [ ] deploy-step9-running.png
- [ ] deploy-step9-logs.png

#### Complete Examples (5)
- [ ] example-nginx.png
- [ ] example-postgres.png
- [ ] example-redis.png
- [ ] example-nodejs.png
- [ ] example-github.png

#### Troubleshooting (7)
- [ ] troubleshoot-image-not-found.png
- [ ] troubleshoot-exited.png
- [ ] troubleshoot-connection.png
- [ ] troubleshoot-oom.png
- [ ] troubleshoot-volume.png
- [ ] troubleshoot-auth-failed.png
- [ ] troubleshoot-upload.png

**See**: [docs/static/images/containers/README.md](static/images/containers/README.md) for detailed instructions

---

## 🟡 FUNCTIONS - 12 Gambar Kurang

**Upload ke**: `/home/shiro/nexus/nqrust-microvm/docs/static/images/functions/`

### Missing Images (12)

#### Architecture & Concepts (3)
- [ ] function-architecture.png
- [ ] function-flow.png
- [ ] function-per-vm.png

#### Runtime Selection (3)
- [ ] runtime-filter.png
- [ ] runtime-javascript.png
- [ ] runtime-python.png
- [ ] runtime-typescript.png

#### Invocation & Testing (3)
- [ ] invoke-payload-editor.png
- [ ] invocation-metrics.png
- [ ] refresh-button.png

#### Playground Features (2)
- [ ] playground-iteration.png
- [ ] import-from-playground.png

---

## ✅ VM - 3 Gambar Kurang (HAMPIR SELESAI!)

**Upload ke**: `/home/shiro/nexus/nqrust-microvm/docs/static/images/vm/`

### Missing Images (3)

#### Troubleshooting (2)
- [ ] troubleshoot-no-images.png
- [ ] troubleshoot-resources.png

#### VM States (1)
- [ ] vm-stopped-badge.png

---

## 📋 How to Upload Images

### 1. Take Screenshots

Start the UI application:
```bash
cd apps/ui
NEXT_PUBLIC_API_BASE_URL=http://127.0.0.1:18080/v1 pnpm dev
```

Open: http://localhost:3000

### 2. Navigate to the Right Page

- **Containers**: Sidebar → Containers → Deploy Container
- **Functions**: Sidebar → Functions → Create Function
- **VM**: Sidebar → Virtual Machines → Create VM

### 3. Take Screenshots

- Windows: `Win + Shift + S`
- Mac: `Cmd + Shift + 4`
- Linux: Screenshot tool or `Print Screen`

### 4. Save with Correct Filename

Use **exact** filenames from checklists above.

Format: PNG (recommended for UI screenshots)

### 5. Upload to Correct Folder

```bash
# For Containers
cp ~/Downloads/deploy-step1-form.png \
   /home/shiro/nexus/nqrust-microvm/docs/static/images/containers/

# For Functions
cp ~/Downloads/function-architecture.png \
   /home/shiro/nexus/nqrust-microvm/docs/static/images/functions/

# For VM
cp ~/Downloads/troubleshoot-no-images.png \
   /home/shiro/nexus/nqrust-microvm/docs/static/images/vm/
```

### 6. Verify

Hugo will automatically sync to `public/` folder.

If Hugo is running (`hugo serve`), refresh the docs page to see images.

---

## 💡 Screenshot Tips

### Quality
- **Resolution**: Minimum 1280px width
- **Format**: PNG for UI screenshots (better quality)
- **Clarity**: Text must be readable, no blur
- **Zoom**: Browser at 100% zoom level

### Content
- **Crop**: Only relevant area
- **Data**: Use realistic example data
- **Privacy**: Hide sensitive credentials/tokens
- **Highlight**: Add arrows/boxes (optional)

### Consistency
- **Theme**: Same light/dark theme throughout
- **Browser**: Same browser (Chrome recommended)
- **Window**: Consistent window size
- **Zoom**: 100% zoom level

---

## 🔄 Verification

After uploading images, verify they appear:

```bash
# Check if file exists in static
ls -la docs/static/images/containers/deploy-step1-form.png

# Check if synced to public (if Hugo is running)
ls -la docs/public/images/containers/deploy-step1-form.png

# View in browser
# Navigate to the docs page and check image loads
```

---

## 📞 Hugo Documentation Server

To preview documentation with images:

```bash
cd docs

# Use the serve script
./serve.sh
```

**Access URL** (baseURL changed to `/` for development):

✅ **Correct**: http://localhost:1313/docs/

### How Image Paths Work

The project uses a **Hugo render hook** (`layouts/_default/_markup/render-image.html`) that automatically converts image paths to include the baseURL.

**In markdown:**
```markdown
![Image](/images/containers/deploy-step1-nav.png)
```

**Automatically rendered as:**
```html
<img src="/images/containers/deploy-step1-nav.png" ... >
```

This means:
- ✅ Images work in both development and production
- ✅ No need to change markdown files
- ✅ baseURL is handled automatically

Hugo automatically syncs `static/` to `public/` folder.

The log "Syncing /images/containers/xxx.png to /" is **NORMAL** - it means Hugo successfully copied the file.

**See**: [RENDER-HOOK-EXPLANATION.md](RENDER-HOOK-EXPLANATION.md) for detailed explanation of the render hook.

---

## 🎯 Next Steps

### Priority Order:

1. **CONTAINERS** (49 images) - Most urgent, only 2% complete
   - Start with Step 1-3 (basic deployment flow)
   - Then Step 4-7 (advanced features)
   - Finally examples and troubleshooting

2. **FUNCTIONS** (12 images) - 81% complete
   - Architecture diagrams
   - Runtime selection screenshots
   - Playground features

3. **VM** (3 images) - 95% complete
   - Just troubleshooting screenshots

---

## ✅ Completion Checklist

- [ ] **Containers**: 0/49 done
  - [ ] Step 1-3: Basic deployment (0/10)
  - [ ] Step 4-7: Advanced features (0/22)
  - [ ] Step 8-9: Deploy & progress (0/8)
  - [ ] Examples (0/5)
  - [ ] Troubleshooting (0/7)

- [ ] **Functions**: 0/12 done
  - [ ] Architecture (0/3)
  - [ ] Runtimes (0/4)
  - [ ] Invocation (0/3)
  - [ ] Playground (0/2)

- [ ] **VM**: 0/3 done
  - [ ] Troubleshooting (0/2)
  - [ ] States (0/1)

**Target**: 100% (171/171 images)

**Current**: 63% (107/171 images)

**Remaining**: 64 images to upload
