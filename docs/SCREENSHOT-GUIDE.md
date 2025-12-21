# Screenshot Guide untuk Dokumentasi

## 📍 Lokasi Upload

Upload semua gambar ke folder:
```
/home/shiro/nexus/nqrust-microvm/docs/static/images/vm/
```

Setelah upload, gambar akan otomatis accessible di dokumentasi.

---

## 📸 Screenshot yang Diperlukan untuk "Create a VM"

Total: **18 gambar**

Detail lengkap ada di: `/docs/static/images/vm/README.md`

### Quick Checklist:

#### Wizard Steps (9 gambar)
1. ✅ **vm-create-button.png** - Tombol "Create VM" di VMs list page
2. ✅ **vm-step1-basic.png** - Step 1: Basic Info (nama & deskripsi)
3. ✅ **vm-step2-credentials.png** - Step 2: SSH key & password
4. ✅ **vm-step3-machine.png** - Step 3: CPU & memory sliders
5. ✅ **vm-step4-boot.png** - Step 4: Kernel & rootfs dropdown
6. ✅ **vm-rootfs-selection.png** - Rootfs dropdown expanded
7. ✅ **vm-step5-network.png** - Step 5: Network config
8. ✅ **vm-network-bridge.png** - Bridge mode selected
9. ✅ **vm-network-nat.png** - NAT mode selected

#### Creation & Success (3 gambar)
10. ✅ **vm-creating.png** - Loading spinner "Creating VM..."
11. ✅ **vm-created-success.png** - Success notification + VM detail
12. ✅ **vm-detail-running.png** - VM detail page, status Running

#### Console Access (3 gambar)
13. ✅ **vm-console-tab.png** - Console tab highlighted
14. ✅ **vm-console-logged-in.png** - Logged in to console
15. ✅ **vm-network-test.png** - Ping command success

#### Templates & Troubleshooting (3 gambar)
16. ✅ **template-deploy.png** - Deploy from template dialog
17. ✅ **troubleshoot-no-images.png** - Empty dropdown (no images)
18. ✅ **troubleshoot-resources.png** - Error: insufficient resources

---

## 🎯 Workflow Screenshot

### Langkah-langkah:

1. **Buka aplikasi NQRust-MicroVM** di browser
   - URL: http://your-server:3000
   - Login jika perlu

2. **Navigate ke halaman yang sesuai**
   - Contoh: Klik "Virtual Machines" di sidebar

3. **Ambil screenshot**
   - Windows: Win + Shift + S
   - Mac: Cmd + Shift + 4
   - Linux: Screenshot tool atau Print Screen

4. **Crop & annotate** (opsional)
   - Crop ke area yang relevan
   - Tambah arrow/box merah untuk highlight

5. **Save dengan nama yang sesuai**
   - Gunakan nama persis seperti di checklist
   - Format: PNG (lebih baik) atau JPG

6. **Upload ke folder**
   ```bash
   # Copy file ke folder static
   cp ~/Downloads/vm-create-button.png \
      /home/shiro/nexus/nqrust-microvm/docs/static/images/vm/
   ```

7. **Verify di browser**
   - Refresh halaman: http://localhost:1313/docs/vm/create-vm/
   - Gambar seharusnya muncul

---

## 💡 Tips Screenshot

### Kualitas
- ✅ Resolusi tinggi (minimal 1280px width)
- ✅ PNG format untuk UI screenshots
- ✅ Text harus jelas terbaca
- ✅ Tidak blur atau pixelated

### Konten
- ✅ Crop hanya area yang relevan
- ✅ Hapus data sensitif (IP private OK, tapi hapus credentials)
- ✅ Gunakan contoh data yang realistis
- ✅ Highlight element penting (arrow merah, box, dll)

### Konsistensi
- ✅ Gunakan tema yang sama (light/dark)
- ✅ Window size konsisten
- ✅ Browser yang sama (Chrome recommended)
- ✅ Zoom level 100%

---

## 🛠️ Tools Recommended

### Screenshot Tools
- **Windows**: Snipping Tool, ShareX
- **Mac**: Built-in Cmd+Shift+4, CleanShot X
- **Linux**: Flameshot, GNOME Screenshot

### Annotation Tools
- **Windows**: Paint, Paint 3D, Greenshot
- **Mac**: Preview, Skitch
- **Linux**: GIMP, Krita
- **Cross-platform**: Photopea (web-based)

### Browser Extensions
- **Awesome Screenshot** - Full page screenshots
- **Nimbus Screenshot** - Annotate inline
- **FireShot** - Professional screenshots

---

## 🔄 Update Process

Setelah upload gambar baru:

1. **Tidak perlu restart Hugo** - Hugo auto-detect static files
2. **Refresh browser** - Hard refresh (Ctrl+Shift+R)
3. **Check gambar muncul** - Lihat di halaman docs
4. **Update README.md** - Tandai (✓) di checklist

---

## 📋 Progress Tracking

Gunakan file `/docs/static/images/vm/README.md` untuk tracking progress:

```markdown
## ✅ Checklist

- [x] vm-create-button.png        ← DONE
- [x] vm-step1-basic.png          ← DONE
- [ ] vm-step2-credentials.png    ← TODO
- [ ] vm-step3-machine.png        ← TODO
...
```

---

## 🆘 Troubleshooting

### Gambar tidak muncul setelah upload

**Problem**: Image path broken atau 404

**Solutions**:
1. Verify file ada di `/docs/static/images/vm/`
2. Check nama file exact match (case-sensitive)
3. Hard refresh browser (Ctrl+Shift+R)
4. Check Hugo server logs untuk errors
5. Restart Hugo server jika perlu

### Gambar terlalu besar

**Problem**: Page load lambat

**Solutions**:
1. Resize gambar ke max width 1920px
2. Compress dengan tools:
   ```bash
   # Using ImageMagick
   convert input.png -quality 85 -resize 1920x output.png

   # Using online tool
   # TinyPNG.com, Compressor.io
   ```

### Gambar pecah/blur

**Problem**: Resolution terlalu rendah

**Solutions**:
1. Retake screenshot dengan zoom 100%
2. Ensure display scale 100% (not 125% or 150%)
3. Use PNG instead of JPG for UI screenshots

---

## 📞 Need Help?

Jika ada pertanyaan tentang screenshot mana yang perlu diambil, check:

1. **Detail deskripsi**: `/docs/static/images/vm/README.md`
2. **Markdown source**: `/docs/content/docs/vm/create-vm.md`
3. **Live preview**: http://localhost:1313/docs/vm/create-vm/
