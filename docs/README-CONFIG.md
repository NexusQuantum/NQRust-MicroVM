# Hugo Configuration Files

## 📁 Config Files

This project uses **TWO Hugo config files** to handle different baseURL requirements:

| File | Purpose | baseURL | When to Use |
|------|---------|---------|-------------|
| `hugo.toml` | **Production** | `/NQRust-MicroVM/` | GitHub Pages deployment |
| `hugo.dev.toml` | **Development** | `/` | Local development |

---

## 🔧 Why Two Configs?

### The Problem

GitHub Pages serves this site at: `https://nexusquantum.github.io/NQRust-MicroVM/`

This requires a **subdirectory** in the URL path (`/NQRust-MicroVM/`).

When you use this baseURL in local development:
- ❌ Images don't load: `/NQRust-MicroVM/images/...` → 404 Error
- ❌ Links don't work: `/NQRust-MicroVM/docs/...` → 404 Error
- ❌ CSS/JS broken: `/NQRust-MicroVM/css/...` → 404 Error

### The Solution

**Use different baseURL for different environments:**

**Development** (`hugo.dev.toml`):
```toml
baseURL = '/'  # No subdirectory
```
- ✅ Images load: `/images/...` → Works!
- ✅ Links work: `/docs/...` → Works!

**Production** (`hugo.toml`):
```toml
baseURL = 'https://nexusquantum.github.io/NQRust-MicroVM/'
```
- ✅ Deployed to GitHub Pages with correct paths

---

## 🚀 Usage

### Local Development

**Option 1: Use the serve script** (Recommended)
```bash
cd docs
./serve.sh
```

This automatically uses `hugo.dev.toml`.

**Option 2: Manual command**
```bash
cd docs
../bin/hugo server --config hugo.dev.toml --buildDrafts --bind 0.0.0.0 --port 1313
```

**Access**: http://localhost:1313/docs/

---

### Production Build

**Option 1: Use the build script** (Recommended)
```bash
cd docs
./build.sh
```

This automatically uses `hugo.toml` (production config).

**Option 2: Manual command**
```bash
cd docs
../bin/hugo --minify
```

Output: `docs/public/` folder ready for deployment to GitHub Pages.

---

## 📋 Config Differences

### What's Different?

| Setting | `hugo.toml` (Production) | `hugo.dev.toml` (Development) |
|---------|--------------------------|-------------------------------|
| `baseURL` | `https://nexusquantum.github.io/NQRust-MicroVM/` | `/` |
| Everything else | Identical | Identical |

### What's the Same?

Everything else is **identical** in both files:
- Site title and description
- Menu configuration
- Module imports
- Markup settings
- Theme configuration

---

## ✅ Verification

### After Starting Dev Server

**Check images load correctly:**

1. Start server:
   ```bash
   cd docs
   ./serve.sh
   ```

2. Open browser: http://localhost:1313/docs/

3. Navigate to any page with images:
   - Containers: http://localhost:1313/docs/containers/deploy-container/
   - Functions: http://localhost:1313/docs/functions/create-function/
   - VM: http://localhost:1313/docs/vm/create-vm/

4. **Verify images display** (no broken image icons)

5. **Check browser console** (F12) - should be no 404 errors for images

---

## 🔍 Troubleshooting

### Images Still Don't Load in Development

**Check you're using the right config:**
```bash
# Look for this in the server output
Using config file: hugo.dev.toml  # ✅ Correct
# or
Using config file: hugo.toml      # ❌ Wrong for development
```

**If using wrong config:**
- Stop server (Ctrl+C)
- Restart with: `./serve.sh`
- Or manually: `../bin/hugo server --config hugo.dev.toml`

---

### Images Don't Load on GitHub Pages

**Check production build uses correct config:**
```bash
cd docs
./build.sh  # Uses hugo.toml automatically
```

**Verify in `public/` folder:**
```bash
# Check that links have /NQRust-MicroVM/ prefix
grep -r 'href="/NQRust-MicroVM/' public/ | head -5
grep -r 'src="/NQRust-MicroVM/images/' public/ | head -5
```

Should show links with `/NQRust-MicroVM/` prefix.

---

### Need to Update Config

**When changing site configuration:**

1. **Update BOTH files** to keep them in sync
2. **Only `baseURL` should be different**
3. Keep all other settings identical

**Example: Adding a new menu item**

Edit **both** files:
- `hugo.toml` (production)
- `hugo.dev.toml` (development)

Add the same menu configuration to both.

---

## 📚 Related Files

| File | Purpose |
|------|---------|
| `hugo.toml` | Production config (GitHub Pages) |
| `hugo.dev.toml` | Development config (local) |
| `serve.sh` | Start dev server (uses `hugo.dev.toml`) |
| `build.sh` | Production build (uses `hugo.toml`) |

---

## 💡 Quick Reference

**Start development server:**
```bash
./serve.sh
# Access: http://localhost:1313/docs/
```

**Build for production:**
```bash
./build.sh
# Output: docs/public/
```

**Update configuration:**
1. Edit both `hugo.toml` and `hugo.dev.toml`
2. Keep them identical except for `baseURL`

**Image paths in markdown:**
```markdown
![Description](/images/vm/screenshot.png)
```
- ✅ Correct (absolute path from site root)
- Works in both development and production
- Hugo automatically adds baseURL prefix
