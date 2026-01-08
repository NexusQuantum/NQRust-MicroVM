# 🔧 Final Fix: Image Path Issue - Solusi Definitif

## 📊 Status Masalah

**Masalah**: Render hook tidak bekerja meskipun sudah dibuat dengan benar.

**Root Cause**: Hugo module theme (`lotusdocs`) memiliki prioritas lebih tinggi dalam render hook lookup, sehingga custom render hook kita tidak terpakai.

---

## ✅ Solusi yang Sudah Diterapkan

### 1. Render Hook Disederhanakan

File: `layouts/docs/_markup/render-image.html`

Render hook yang sangat sederhana:
- Deteksi external URL (http://, https://)
- Untuk local path: gunakan `relURL` (menambahkan `/NQRust-MicroVM/` otomatis)
- Untuk external URL: keep as-is

### 2. Cache Dibersihkan

Folder `public/` dan `resources/` sudah dihapus untuk fresh build.

### 3. Script Fresh Start

File: `START-HUGO-FRESH.sh`

Script untuk start Hugo dengan:
- Clear cache otomatis
- Disable fast render (force full rebuild)
- Fresh build setiap kali

---

## 🚀 LANGKAH YANG HARUS DILAKUKAN

### Opsi 1: Start Hugo dengan Fresh Build (RECOMMENDED)

```bash
cd /home/shiro/nexus/nqrust-microvm/docs
./START-HUGO-FRESH.sh
```

Script ini akan:
1. ✅ Clear cache (`public/` dan `resources/`)
2. ✅ Start Hugo dengan `--disableFastRender`
3. ✅ Force rebuild semua pages

### Opsi 2: Manual Fresh Start

```bash
cd /home/shiro/nexus/nqrust-microvm/docs

# Clear cache
rm -rf public/ resources/

# Start Hugo
../bin/hugo server --buildDrafts --bind 0.0.0.0 --port 1313 --disableFastRender
```

---

## 🧪 Verifikasi Setelah Start

### 1. Akses URL yang Benar

```
http://localhost:1313/NQRust-MicroVM/docs/containers/deploy-container/
```

### 2. Cek HTML Source

Buka page source (Ctrl+U atau View Page Source).

Search for "deploy-step1-nav.png":

**Yang diharapkan** (✅ Benar):
```html
<img src="/NQRust-MicroVM/images/containers/deploy-step1-nav.png" ...>
```

**Jika masih salah** (❌):
```html
<img src="/images/containers/deploy-step1-nav.png" ...>
```

### 3. Test Direct Image Access

Buka di browser atau curl:
```bash
curl -I http://localhost:1313/NQRust-MicroVM/images/containers/deploy-step1-nav.png
```

Seharusnya return `200 OK`, bukan `404 Not Found`.

---

## 🔍 Jika Masih Tidak Bekerja

### Opsi A: Gunakan Hugo Dev Config (Alternatif)

Jika render hook masih tidak bekerja, gunakan config development tanpa `/NQRust-MicroVM/`:

```bash
cd /home/shiro/nexus/nqrust-microvm/docs

# Edit hugo.dev.toml dan set baseURL = '/'
# Lalu start dengan config itu
../bin/hugo server --config hugo.dev.toml --buildDrafts --bind 0.0.0.0 --port 1313
```

**Akses**:
```
http://localhost:1313/docs/containers/deploy-container/
```

(Tanpa `/NQRust-MicroVM/` prefix)

### Opsi B: Edit Baseurl Sementara (Quick Fix)

Untuk development, edit `hugo.toml`:

```toml
# Temporarily for development
baseURL = '/'
```

Restart Hugo:
```bash
cd /home/shiro/nexus/nqrust-microvm/docs
./serve.sh
```

**Akses**:
```
http://localhost:1313/docs/containers/deploy-container/
```

**PENTING**: Jangan lupa kembalikan baseURL sebelum deploy ke GitHub Pages!

```toml
# For production (GitHub Pages)
baseURL = 'https://nexusquantum.github.io/NQRust-MicroVM/'
```

---

## 💡 Penjelasan Masalah Teknis

### Kenapa Render Hook Tidak Bekerja?

Hugo memiliki lookup order untuk render hooks:

1. **Module imports** (theme modules) - **HIGHEST PRIORITY**
2. Project layouts
3. Theme layouts

Theme `lotusdocs` di-import sebagai Hugo module:

```toml
[[module.imports]]
  path = "github.com/colinwilson/lotusdocs"
```

Hugo module memiliki prioritas lebih tinggi daripada project `layouts/`, sehingga theme's render hook tetap digunakan.

### Solusi Definitif

Ada beberapa cara:

**1. Module Mount Override** (Kompleks)
Edit `hugo.toml` untuk mount project layouts dengan prioritas lebih tinggi.

**2. Disable Theme Module** (Tidak ideal)
Disable module import dan copy theme ke `themes/` folder.

**3. Gunakan baseURL = '/' untuk Development** (RECOMMENDED)
Paling simple dan praktis untuk development.

---

## 🎯 Rekomendasi Saya

### Untuk Development (Lokal)

Gunakan **baseURL = '/'** untuk development:

1. Edit `hugo.toml`:
   ```toml
   baseURL = '/'  # For local development
   ```

2. Start Hugo:
   ```bash
   cd docs
   ./serve.sh
   ```

3. Akses:
   ```
   http://localhost:1313/docs/
   ```

4. Gambar akan load dari `/images/...` (works!)

### Untuk Production (Deploy)

Sebelum commit untuk GitHub Pages, kembalikan baseURL:

```toml
baseURL = 'https://nexusquantum.github.io/NQRust-MicroVM/'
```

Build:
```bash
cd docs
./build.sh
```

Deploy `public/` folder ke GitHub Pages.

---

## 📋 Checklist

- [ ] Start Hugo dengan fresh build: `./START-HUGO-FRESH.sh`
- [ ] Akses `http://localhost:1313/NQRust-MicroVM/docs/containers/deploy-container/`
- [ ] Cek HTML source untuk image src
- [ ] Jika masih tidak bekerja, gunakan baseURL = '/' untuk development
- [ ] Upload 49 gambar lainnya untuk containers
- [ ] Sebelum deploy: Kembalikan baseURL untuk production

---

## 🚀 Quick Commands

```bash
# Fresh start Hugo (recommended)
cd /home/shiro/nexus/nqrust-microvm/docs
./START-HUGO-FRESH.sh

# Access site
open http://localhost:1313/NQRust-MicroVM/docs/containers/deploy-container/

# Test image directly
curl -I http://localhost:1313/NQRust-MicroVM/images/containers/deploy-step1-nav.png

# Check HTML source
curl -s http://localhost:1313/NQRust-MicroVM/docs/containers/deploy-container/ | grep deploy-step1-nav
```

---

## 💭 Catatan Akhir

Jika render hook tetap tidak bekerja setelah fresh rebuild, **gunakan baseURL = '/' untuk development** adalah solusi paling praktis. Ini adalah practice yang umum digunakan:

- Development: `baseURL = '/'` → Akses di `http://localhost:1313/`
- Production: `baseURL = '/subdirectory/'` → Deploy ke GitHub Pages

Tidak ada masalah dengan approach ini, dan sangat umum digunakan di Hugo projects.
