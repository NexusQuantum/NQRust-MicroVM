# 🔧 Fix: Gambar Tidak Muncul - SOLVED

## ✅ Masalah Teridentifikasi

**Root Cause**: Theme `lotusdocs` memiliki render hook sendiri yang **tidak menggunakan baseURL** untuk image paths.

Theme render hook di:
```
themes/lotusdocs/layouts/docs/_markup/render-image.html
```

Render hook ini menggunakan `.Destination` tanpa `relURL`, sehingga path gambar tidak include `/NQRust-MicroVM/` prefix.

---

## ✅ Solusi yang Diterapkan

Saya telah membuat **custom render hook** yang override theme's render hook:

**File**: `layouts/docs/_markup/render-image.html`

Render hook ini:
- ✅ Menggunakan `relURL` untuk semua image paths
- ✅ Menambahkan baseURL prefix otomatis (`/NQRust-MicroVM/`)
- ✅ Tetap support fitur theme (SVG, figure dengan caption, dll)

---

## 🚀 Cara Mengaktifkan Fix

### 1. **RESTART Hugo Server** (PENTING!)

Hugo perlu di-restart untuk mendeteksi render hook yang baru dibuat.

**Stop server** (di terminal yang menjalankan Hugo):
```
Ctrl + C
```

**Start server lagi**:
```bash
cd /home/shiro/nexus/nqrust-microvm/docs
./serve.sh
```

### 2. **Clear Browser Cache**

Setelah Hugo restart:
- Hard refresh browser: `Ctrl + Shift + R` (Linux/Windows) atau `Cmd + Shift + R` (Mac)
- Atau clear browser cache

### 3. **Akses URL yang Benar**

Buka di browser:
```
http://localhost:1313/NQRust-MicroVM/docs/containers/deploy-container/
```

**PENTING**: URL HARUS include `/NQRust-MicroVM/` !

---

## 🧪 Verifikasi Fix Berhasil

### Test 1: Cek HTML Source

1. Buka page: http://localhost:1313/NQRust-MicroVM/docs/containers/deploy-container/
2. View page source (Ctrl+U atau klik kanan → View Page Source)
3. Search for "img src"

**Sebelum fix** (❌ Salah):
```html
<img src="/images/containers/deploy-step1-nav.png" ...>
```

**Setelah fix** (✅ Benar):
```html
<img src="/NQRust-MicroVM/images/containers/deploy-step1-nav.png" ...>
```

### Test 2: Cek Browser Console

1. Buka Developer Tools (F12)
2. Lihat tab Console
3. Tidak boleh ada error 404 untuk gambar

**Sebelum fix**:
```
GET http://localhost:1313/images/containers/deploy-step1-nav.png 404 (Not Found)
```

**Setelah fix**:
```
(No 404 errors)
```

### Test 3: Visual Check

Kedua gambar seharusnya muncul:
- ✅ `deploy-step1-nav.png` (Step 1 navigation)
- ✅ `deploy-step1-form.png` (Step 1 form)

---

## 📋 Status Files

### Gambar yang Ada di Folder Static

```bash
/home/shiro/nexus/nqrust-microvm/docs/static/images/containers/
├── deploy-step1-nav.png (274 KB) ✅
├── deploy-step1-form.png (119 KB) ✅
└── README.md
```

### Render Hooks (Priority Order)

Hugo menggunakan render hook dengan prioritas dari yang paling spesifik:

1. ✅ **AKTIF**: `layouts/docs/_markup/render-image.html` (Custom - baru dibuat)
2. ❌ **OVERRIDE**: `themes/lotusdocs/layouts/docs/_markup/render-image.html` (Theme default)
3. ❌ **TIDAK DIPAKAI**: `layouts/_default/_markup/render-image.html` (Generic)

---

## 🔍 Troubleshooting

### Gambar Masih Tidak Muncul Setelah Restart

**Check 1: Hugo Server Log**

Lihat output Hugo server saat startup:
```
Syncing /images/containers/deploy-step1-nav.png to /
Syncing /images/containers/deploy-step1-form.png to /
```

Jika tidak ada "Syncing..." messages, coba:
```bash
cd docs
rm -rf public/  # Clear public folder
./serve.sh      # Rebuild dari scratch
```

**Check 2: Render Hook Aktif**

Verify file exist dan tidak ada typo:
```bash
ls -la /home/shiro/nexus/nqrust-microvm/docs/layouts/docs/_markup/render-image.html
```

Seharusnya ada (1480 bytes).

**Check 3: Path URL di Browser**

PASTIKAN URL include `/NQRust-MicroVM/`:
- ✅ Benar: `http://localhost:1313/NQRust-MicroVM/docs/...`
- ❌ Salah: `http://localhost:1313/docs/...`

**Check 4: File Gambar Ada**

```bash
ls -lh /home/shiro/nexus/nqrust-microvm/docs/static/images/containers/
```

Kedua file harus ada dan tidak 0 bytes.

---

## 🎯 Upload Gambar Selanjutnya

Setelah fix ini, upload gambar baru akan langsung bekerja:

1. **Copy gambar** ke folder static:
   ```bash
   cp ~/Downloads/deploy-step2-name.png \
      /home/shiro/nexus/nqrust-microvm/docs/static/images/containers/
   ```

2. **Hugo auto-detect** (tidak perlu restart untuk gambar baru)

3. **Refresh browser** (Ctrl+R atau F5)

4. **Gambar muncul** dengan path yang benar secara otomatis!

---

## 📚 Penjelasan Teknis

### Kenapa Perlu Override Theme Render Hook?

Hugo mencari render hooks dengan prioritas:
1. Project-level layouts (lebih prioritas)
2. Theme layouts (default)

Theme `lotusdocs` punya render hook di `themes/.../layouts/docs/_markup/`, yang lebih spesifik daripada `layouts/_default/_markup/`.

Jadi kita perlu buat di path yang sama atau lebih spesifik:
```
layouts/docs/_markup/render-image.html
```

### Apa yang Dilakukan Render Hook?

Render hook mengubah image markdown jadi HTML dengan baseURL:

**Markdown** (tidak berubah):
```markdown
![Image](/images/containers/file.png)
```

**Diproses oleh render hook** menjadi:
```html
<img src="/NQRust-MicroVM/images/containers/file.png"
     alt="Image" loading="lazy">
```

Function `relURL` menambahkan baseURL (`/NQRust-MicroVM/`) otomatis.

---

## ✅ Summary

1. ✅ **Custom render hook dibuat** di `layouts/docs/_markup/render-image.html`
2. ✅ **Override theme render hook** yang tidak pakai baseURL
3. 🔄 **Restart Hugo server** untuk apply changes
4. ✅ **Gambar akan muncul** dengan path yang benar

**Next**: Restart Hugo server dan verify gambar muncul!
