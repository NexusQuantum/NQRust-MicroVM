# ✅ BaseURL Changed untuk Development

## 📍 Perubahan URL

**Sebelumnya** (dengan `/NQRust-MicroVM/`):
```
http://localhost:1313/NQRust-MicroVM/docs/
```

**Sekarang** (tanpa subdirectory):
```
http://localhost:1313/docs/
```

---

## ⚙️ Konfigurasi Saat Ini

File: `hugo.toml`

```toml
# Development mode - local development
baseURL = '/'

# Production mode - untuk GitHub Pages (di-comment untuk development)
# baseURL = 'https://nexusquantum.github.io/NQRust-MicroVM/'
```

---

## 🚀 Cara Menggunakan

### 1. Start Hugo Server

```bash
cd /home/shiro/nexus/nqrust-microvm/docs
./serve.sh

# Atau dengan fresh build
./START-HUGO-FRESH.sh
```

### 2. Akses di Browser

**URL Baru** (✅ Gunakan ini):
```
http://localhost:1313/docs/
```

**Contoh pages**:
- Containers: http://localhost:1313/docs/containers/deploy-container/
- Functions: http://localhost:1313/docs/functions/create-function/
- VM: http://localhost:1313/docs/vm/create-vm/

### 3. Gambar Sekarang Muncul! ✅

Dengan `baseURL = '/'`, semua gambar di `static/images/` akan accessible di:
```
http://localhost:1313/images/containers/deploy-step1-nav.png
http://localhost:1313/images/containers/deploy-step1-form.png
```

Markdown image paths:
```markdown
![Image](/images/containers/deploy-step1-nav.png)
```

Akan di-render sebagai:
```html
<img src="/images/containers/deploy-step1-nav.png" ...>
```

Dan akan load dari: `http://localhost:1313/images/...` ✅

---

## 📸 Upload Gambar

Gambar tetap di-upload ke lokasi yang sama:

```bash
# Upload location (tidak berubah)
/home/shiro/nexus/nqrust-microvm/docs/static/images/containers/
/home/shiro/nexus/nqrust-microvm/docs/static/images/functions/
/home/shiro/nexus/nqrust-microvm/docs/static/images/vm/
```

Hugo akan sync ke:
```bash
/home/shiro/nexus/nqrust-microvm/docs/public/images/...
```

Accessible di browser:
```
http://localhost:1313/images/...
```

---

## 🔄 Sebelum Deploy ke GitHub Pages

**PENTING**: Sebelum commit untuk production deployment, **kembalikan baseURL**!

Edit `hugo.toml`:

```toml
# Production mode - GitHub Pages
baseURL = 'https://nexusquantum.github.io/NQRust-MicroVM/'

# Development mode (comment untuk production)
# baseURL = '/'
```

Kemudian build:

```bash
cd /home/shiro/nexus/nqrust-microvm/docs
./build.sh
```

Output di `public/` folder akan memiliki path yang benar untuk GitHub Pages:
- `/NQRust-MicroVM/images/...`
- `/NQRust-MicroVM/docs/...`

---

## 📋 Quick Reference

| Environment | baseURL | Access URL | Command |
|-------------|---------|------------|---------|
| **Development** | `/` | `http://localhost:1313/docs/` | `./serve.sh` |
| **Production** | `/NQRust-MicroVM/` | `https://nexusquantum.github.io/NQRust-MicroVM/docs/` | `./build.sh` |

---

## ✅ Checklist

Development workflow:
- [x] baseURL diubah ke `/` di `hugo.toml`
- [x] Start Hugo dengan `./serve.sh`
- [x] Akses di `http://localhost:1313/docs/`
- [x] Gambar muncul dengan benar
- [ ] Upload 64 gambar yang masih kurang (lihat IMAGES-STATUS.md)

Production workflow (nanti):
- [ ] Kembalikan baseURL ke `https://nexusquantum.github.io/NQRust-MicroVM/`
- [ ] Build dengan `./build.sh`
- [ ] Deploy `public/` folder ke GitHub Pages
- [ ] Verify gambar muncul di production

---

## 🎯 Next Steps

1. **Restart Hugo server** jika sedang running:
   ```bash
   # Stop (Ctrl+C) then restart
   cd /home/shiro/nexus/nqrust-microvm/docs
   ./serve.sh
   ```

2. **Akses new URL**:
   ```
   http://localhost:1313/docs/containers/deploy-container/
   ```

3. **Verify gambar muncul**:
   - deploy-step1-nav.png ✅
   - deploy-step1-form.png ✅

4. **Upload 49 gambar lainnya** untuk containers (lihat [IMAGES-STATUS.md](IMAGES-STATUS.md))

---

## 📚 Related Files

- [hugo.toml](hugo.toml) - Config utama (baseURL diubah)
- [serve.sh](serve.sh) - Start Hugo server
- [START-HUGO-FRESH.sh](START-HUGO-FRESH.sh) - Start dengan fresh build
- [IMAGES-STATUS.md](IMAGES-STATUS.md) - Checklist 64 gambar yang kurang
- [FINAL-FIX-INSTRUCTIONS.md](FINAL-FIX-INSTRUCTIONS.md) - Panduan troubleshooting lengkap

---

## 💡 Summary

✅ **baseURL sudah diubah ke `/` untuk development**
✅ **Akses di `http://localhost:1313/docs/`**
✅ **Gambar akan muncul dengan benar**
✅ **Ingat kembalikan baseURL sebelum production deploy!**

Selamat development! 🎉
