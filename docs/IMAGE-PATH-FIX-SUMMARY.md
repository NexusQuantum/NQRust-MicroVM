# ✅ Image Path Fix - Summary

## 🎯 Problem Solved

**Issue**: Images tidak muncul di Hugo development server karena baseURL mengandung `/NQRust-MicroVM/` subdirectory.

**Root Cause**: Path gambar di markdown (`/images/...`) tidak include baseURL prefix, jadi mencari di lokasi yang salah.

**Solution**: Hugo **Render Hook** yang otomatis convert image path menggunakan `relURL` function.

---

## 🛠️ What Was Changed

### 1. Created Render Hook
**File**: `layouts/_default/_markup/render-image.html`

This hook automatically processes ALL images in markdown and adds the baseURL prefix.

**How it works:**
- Detects images with absolute path (`/images/...`)
- Uses Hugo's `relURL` function to add baseURL
- Converts `/images/file.png` → `/NQRust-MicroVM/images/file.png`

### 2. Updated serve.sh
**File**: `serve.sh`

Removed reference to `hugo.dev.toml` - now uses default `hugo.toml` with full baseURL.

### 3. Created Documentation
**Files:**
- `RENDER-HOOK-EXPLANATION.md` - Detailed explanation of the render hook
- Updated `IMAGES-STATUS.md` - Corrected access URL

---

## ✅ How to Use

### 1. Start Hugo Server

```bash
cd docs
./serve.sh
```

### 2. Access Site at Correct URL

⭐ **IMPORTANT**: You MUST access the site with `/NQRust-MicroVM/` prefix!

✅ **Correct URL**:
```
http://localhost:1313/NQRust-MicroVM/docs/
```

❌ **Wrong URL** (won't work):
```
http://localhost:1313/docs/
```

### 3. Verify Images Load

Navigate to any page with images:
- Containers: http://localhost:1313/NQRust-MicroVM/docs/containers/deploy-container/
- Functions: http://localhost:1313/NQRust-MicroVM/docs/functions/create-function/
- VM: http://localhost:1313/NQRust-MicroVM/docs/vm/create-vm/

Images should display correctly! ✅

### 4. Check Browser Console

Open browser console (F12):
- Should see NO 404 errors for images
- Image src should be: `/NQRust-MicroVM/images/...`

---

## 🔍 How the Render Hook Works

### Before (Without Render Hook)

**Markdown:**
```markdown
![Deploy form](/images/containers/deploy-step1-form.png)
```

**HTML Output:**
```html
<img src="/images/containers/deploy-step1-form.png" alt="Deploy form">
```

**Browser tries to load:**
```
http://localhost:1313/images/containers/deploy-step1-form.png
```
❌ **404 Error** - Missing `/NQRust-MicroVM/` prefix!

---

### After (With Render Hook)

**Markdown:** (Same - no changes needed!)
```markdown
![Deploy form](/images/containers/deploy-step1-form.png)
```

**HTML Output:** (Automatically converted by render hook)
```html
<img src="/NQRust-MicroVM/images/containers/deploy-step1-form.png"
     alt="Deploy form"
     loading="lazy">
```

**Browser loads from:**
```
http://localhost:1313/NQRust-MicroVM/images/containers/deploy-step1-form.png
```
✅ **Success!** - Correct path with baseURL prefix!

---

## 📋 Key Points

### ✅ What You Get

1. **No markdown changes needed** - Render hook works automatically
2. **Same baseURL in dev and production** - Consistent behavior
3. **Lazy loading** - All images get `loading="lazy"` automatically
4. **Works for all markdown files** - Apply once, works everywhere

### 🎯 What You Need to Remember

1. **Access URL**: Always use `http://localhost:1313/NQRust-MicroVM/docs/`
2. **Image paths**: Keep writing normal markdown (`/images/...`)
3. **No special syntax** - Standard markdown image syntax works

### 📁 File Locations

**Upload images to:**
```
/home/shiro/nexus/nqrust-microvm/docs/static/images/
├── containers/
├── functions/
└── vm/
```

**Images automatically copied to:**
```
/home/shiro/nexus/nqrust-microvm/docs/public/images/
```

**Accessible at:**
```
http://localhost:1313/NQRust-MicroVM/images/...
```

---

## 🧪 Testing

### Test the Fix

1. **Start server:**
   ```bash
   cd docs
   ./serve.sh
   ```

2. **Open correct URL:**
   ```
   http://localhost:1313/NQRust-MicroVM/docs/containers/deploy-container/
   ```

3. **Check existing image:**
   - Look for `deploy-step1-nav.png` in Step 1
   - Should display correctly
   - No 404 in console

4. **Upload a test image:**
   ```bash
   # Create a test image or copy existing one
   cp ~/Downloads/test.png \
      /home/shiro/nexus/nqrust-microvm/docs/static/images/containers/
   ```

5. **Refresh page** - Image should load immediately (Hugo auto-detects)

---

## 📚 Related Documentation

| File | Description |
|------|-------------|
| [RENDER-HOOK-EXPLANATION.md](RENDER-HOOK-EXPLANATION.md) | Detailed explanation of render hook |
| [IMAGES-STATUS.md](IMAGES-STATUS.md) | Image upload status and checklist |
| [README-CONFIG.md](README-CONFIG.md) | Hugo configuration details |
| `layouts/_default/_markup/render-image.html` | The actual render hook code |

---

## 🎉 Next Steps

1. ✅ **Verify the fix works** - Check images load at correct URL
2. 📸 **Upload missing images** - See [IMAGES-STATUS.md](IMAGES-STATUS.md) for checklist
3. 🚀 **Continue documentation** - All image paths will work automatically!

---

## 💡 Bonus Features

The render hook also:
- Adds `loading="lazy"` for better performance
- Preserves alt text and titles
- Works with relative paths too
- Compatible with future Hugo versions

**Everything just works!** 🎉
