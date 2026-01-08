# Container Documentation Images

## Status: 1/50 images uploaded ✅ (2%)

Total gambar yang dibutuhkan untuk dokumentasi container deployment.

---

## ✅ Uploaded (1)
- [x] deploy-step1-nav.png

---

## ❌ Missing Images (49)

### Step 1 - Open Deployment (1 missing)
- [ ] deploy-step1-form.png

### Step 2 - Basic Configuration (6 missing)
- [ ] deploy-step2-name.png
- [ ] deploy-step2-image-tabs.png
- [ ] deploy-step2-registry.png
- [ ] deploy-step2-dockerhub.png
- [ ] deploy-step2-upload.png

### Step 3 - Resources (3 missing)
- [ ] deploy-step3-resources.png
- [ ] deploy-step3-cpu.png
- [ ] deploy-step3-memory.png

### Step 4 - Port Mappings (5 missing)
- [ ] deploy-step4-ports.png
- [ ] deploy-step4-add-port.png
- [ ] deploy-step4-port-row.png
- [ ] deploy-step4-multiple.png
- [ ] deploy-step4-remove.png

### Step 5 - Environment Variables (4 missing)
- [ ] deploy-step5-env.png
- [ ] deploy-step5-add-env.png
- [ ] deploy-step5-env-row.png
- [ ] deploy-step5-remove.png

### Step 6 - Volume Mounts (7 missing)
- [ ] deploy-step6-volumes.png
- [ ] deploy-step6-add-button.png
- [ ] deploy-step6-volume-dialog.png
- [ ] deploy-step6-new-volume.png
- [ ] deploy-step6-existing.png
- [ ] deploy-step6-volume-table.png
- [ ] deploy-step6-remove-volume.png

### Step 7 - Private Registry (6 missing)
- [ ] deploy-step7-registry.png
- [ ] deploy-step7-enable.png
- [ ] deploy-step7-fields.png
- [ ] deploy-step7-dockerhub.png
- [ ] deploy-step7-github.png
- [ ] deploy-step7-gitlab.png

### Step 8 - Review and Deploy (2 missing)
- [ ] deploy-step8-review.png
- [ ] deploy-step8-button.png

### Step 9 - Deployment Progress (6 missing)
- [ ] deploy-step9-progress.png
- [ ] deploy-step9-creating.png
- [ ] deploy-step9-booting.png
- [ ] deploy-step9-initializing.png
- [ ] deploy-step9-running.png
- [ ] deploy-step9-logs.png

### Examples (5 missing)
- [ ] example-nginx.png
- [ ] example-postgres.png
- [ ] example-redis.png
- [ ] example-nodejs.png
- [ ] example-github.png

### Troubleshooting (6 missing)
- [ ] troubleshoot-image-not-found.png
- [ ] troubleshoot-exited.png
- [ ] troubleshoot-connection.png
- [ ] troubleshoot-oom.png
- [ ] troubleshoot-volume.png
- [ ] troubleshoot-auth-failed.png
- [ ] troubleshoot-upload.png

---

## 📸 Screenshot Instructions

### How to Take Screenshots:

1. **Start the UI application**
   ```bash
   cd apps/ui
   NEXT_PUBLIC_API_BASE_URL=http://127.0.0.1:18080/v1 pnpm dev
   ```
   Open http://localhost:3000

2. **Navigate to Containers page**
   - Click "Containers" in sidebar
   - Click "Deploy Container" button

3. **Take screenshots for each step**
   - Follow the wizard flow
   - Capture each UI state mentioned above

4. **Save screenshots**
   - Use exact filenames from checklist above
   - Save to: `/home/shiro/nexus/nqrust-microvm/docs/static/images/containers/`

5. **Verify in documentation**
   - Hugo will auto-sync to `public/` folder
   - Refresh docs page to see images

---

## 💡 Tips

### Quality
- Resolution: Minimal 1280px width
- Format: PNG (recommended for UI)
- Clear, readable text
- No blur or pixelation

### Content
- Crop to relevant area only
- Hide sensitive data (credentials, tokens)
- Use realistic example data
- Highlight important elements (optional)

### Consistency
- Same theme (light/dark) throughout
- Same browser (Chrome recommended)
- Zoom level: 100%
- Window size: consistent

---

## 🔍 Current Location

All images should be placed in:
```
/home/shiro/nexus/nqrust-microvm/docs/static/images/containers/
```

Hugo will automatically copy them to:
```
/home/shiro/nexus/nqrust-microvm/docs/public/images/containers/
```

And they will be accessible in markdown as:
```markdown
![Description](/images/containers/filename.png)
```

---

## Progress Tracking

Update this checklist as you upload images:
- Change `[ ]` to `[x]` when uploaded
- Update count in header (e.g., "5/50 images")
