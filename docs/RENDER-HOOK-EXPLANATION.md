# Hugo Render Hook for Images

## 🎯 Problem & Solution

### The Problem

With `baseURL = 'https://nexusquantum.github.io/NQRust-MicroVM/'`, image paths in markdown need to include the `/NQRust-MicroVM/` prefix.

**Markdown:**
```markdown
![Image](/images/containers/deploy-step1-nav.png)
```

**What happens without render hook:**
- Path: `/images/containers/deploy-step1-nav.png`
- Resolved to: `http://localhost:1313/images/containers/deploy-step1-nav.png`
- Result: ❌ **404 Error** (missing `/NQRust-MicroVM/` prefix)

**What we need:**
- Path: `/images/containers/deploy-step1-nav.png`
- Should resolve to: `http://localhost:1313/NQRust-MicroVM/images/containers/deploy-step1-nav.png`
- Result: ✅ **Image loads correctly**

---

## ✅ The Solution: Hugo Render Hook

File: `layouts/_default/_markup/render-image.html`

This custom render hook automatically converts image paths using Hugo's `relURL` function, which properly handles the `baseURL`.

### How It Works

**Before** (standard Markdown rendering):
```markdown
![Image](/images/vm/screenshot.png)
```
Renders to:
```html
<img src="/images/vm/screenshot.png" alt="Image">
```
Result: `http://localhost:1313/images/vm/screenshot.png` ❌ 404

**After** (with render hook):
```markdown
![Image](/images/vm/screenshot.png)
```
Renders to:
```html
<img src="/NQRust-MicroVM/images/vm/screenshot.png" alt="Image" loading="lazy">
```
Result: `http://localhost:1313/NQRust-MicroVM/images/vm/screenshot.png` ✅ Success!

---

## 🚀 Usage

### No Changes Needed!

The render hook works **automatically** for all markdown files. You don't need to change anything in your markdown.

**Write markdown normally:**
```markdown
![Container deployment form](/images/containers/deploy-step1-form.png)
![VM creation wizard](/images/vm/create-vm-wizard.png)
![Function editor](/images/functions/code-editor.png)
```

**Hugo automatically converts to:**
```html
<img src="/NQRust-MicroVM/images/containers/deploy-step1-form.png" ... >
<img src="/NQRust-MicroVM/images/vm/create-vm-wizard.png" ... >
<img src="/NQRust-MicroVM/images/functions/code-editor.png" ... >
```

---

## 📋 Features

### 1. Automatic baseURL Handling

The render hook uses Hugo's `relURL` function which:
- ✅ Prepends `baseURL` to all absolute paths (starting with `/`)
- ✅ Works in both development and production
- ✅ Handles subdirectory paths correctly

### 2. Lazy Loading

All images automatically get `loading="lazy"` attribute for better performance.

### 3. Alt Text & Title

Preserves alt text and title from markdown:
```markdown
![Alt text](/images/file.png "Title text")
```

Renders to:
```html
<img src="/NQRust-MicroVM/images/file.png" alt="Alt text" title="Title text" loading="lazy">
```

---

## 🔍 How the Render Hook Works

### Code Breakdown

```html
{{- $src := .Destination -}}        <!-- Get image path from markdown -->
{{- $alt := .Text -}}                <!-- Get alt text -->
{{- $title := .Title -}}             <!-- Get title if present -->

{{- if hasPrefix $src "/" -}}        <!-- Check if path starts with / -->
  <!-- For absolute paths, use relURL to add baseURL -->
  <img src="{{ $src | relURL }}" alt="{{ $alt }}" {{ with $title }}title="{{ . }}"{{ end }} loading="lazy">
{{- else -}}
  <!-- For relative paths, keep as-is -->
  <img src="{{ $src }}" alt="{{ $alt }}" {{ with $title }}title="{{ . }}"{{ end }} loading="lazy">
{{- end -}}
```

### Path Handling

| Markdown Image Path | hasPrefix "/" | Processing | Final HTML src |
|---------------------|---------------|------------|----------------|
| `/images/file.png` | ✅ Yes | Uses `relURL` | `/NQRust-MicroVM/images/file.png` |
| `../images/file.png` | ❌ No | Keep as-is | `../images/file.png` |
| `https://example.com/img.png` | ❌ No | Keep as-is | `https://example.com/img.png` |

---

## ✅ Verification

### Test Image Paths

After setting up the render hook:

1. **Start Hugo server:**
   ```bash
   cd docs
   ./serve.sh
   ```

2. **Access site at:**
   ```
   http://localhost:1313/NQRust-MicroVM/docs/
   ```

   **IMPORTANT**: Notice the URL includes `/NQRust-MicroVM/`!

3. **Check image loads:**
   - Navigate to: http://localhost:1313/NQRust-MicroVM/docs/containers/deploy-container/
   - Images should display correctly
   - No 404 errors in browser console (F12)

4. **Inspect image HTML:**
   - Right-click image → Inspect Element
   - Check `src` attribute includes `/NQRust-MicroVM/` prefix

---

## 🎯 Both Environments Work

The render hook works correctly in both environments:

### Development (Local)
```
baseURL: https://nexusquantum.github.io/NQRust-MicroVM/
Access:  http://localhost:1313/NQRust-MicroVM/docs/
Images:  http://localhost:1313/NQRust-MicroVM/images/file.png
```

### Production (GitHub Pages)
```
baseURL: https://nexusquantum.github.io/NQRust-MicroVM/
Access:  https://nexusquantum.github.io/NQRust-MicroVM/docs/
Images:  https://nexusquantum.github.io/NQRust-MicroVM/images/file.png
```

Both use the **same baseURL**, so images work in both!

---

## 📚 Alternative: Shortcode (Manual)

If you need more control, you can also use the `img` shortcode:

```markdown
{{< img src="/images/containers/deploy.png" alt="Deploy form" >}}
```

But the **render hook is automatic** and works for standard markdown syntax, so you don't need the shortcode.

---

## 🔧 Customization

### Add Custom Classes

Edit `layouts/_default/_markup/render-image.html` to add classes:

```html
<img src="{{ $src | relURL }}"
     alt="{{ $alt }}"
     class="img-fluid rounded"  <!-- Add classes here -->
     loading="lazy">
```

### Add Responsive Images

```html
<picture>
  <source srcset="{{ $src | relURL }}" type="image/webp">
  <img src="{{ $src | relURL }}" alt="{{ $alt }}" loading="lazy">
</picture>
```

---

## 📖 Hugo Documentation

- [Image Render Hooks](https://gohugo.io/templates/render-hooks/)
- [relURL Function](https://gohugo.io/functions/relurl/)
- [absURL Function](https://gohugo.io/functions/absurl/)

---

## ✅ Summary

1. **Render hook automatically handles image paths** - no markdown changes needed
2. **Works with same baseURL in dev and production** - consistent behavior
3. **Access site at** `http://localhost:1313/NQRust-MicroVM/docs/`
4. **Images automatically get baseURL prefix** via `relURL`
5. **Lazy loading enabled** for better performance

**No more 404 errors!** 🎉
