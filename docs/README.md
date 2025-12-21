# NQRust-MicroVM Documentation

Dokumentasi ini menggunakan [Hugo](https://gohugo.io/) dengan [Lotus Docs](https://lotusdocs.dev/) theme.

## Quick Start

### Development Server
Jalankan server development dari direktori docs:
```bash
cd docs
bash serve.sh
```

Atau dengan path absolut:
```bash
bash /home/shiro/nexus/nqrust-microvm/docs/serve.sh
```

Dokumentasi akan tersedia di: http://localhost:1313/

**Catatan**: Script akan otomatis set Go environment yang dibutuhkan Hugo modules.

### Build Production
Build dokumentasi untuk production:
```bash
./build.sh
```

Output akan berada di direktori `public/`.

## Struktur Direktori

```
docs/
├── content/          # Konten dokumentasi (Markdown)
│   ├── _index.md    # Homepage
│   └── docs/        # Halaman dokumentasi
│       ├── getting-started/
│       ├── user-guide/
│       └── ...
├── static/          # File static (images, etc.)
├── hugo.toml        # Konfigurasi Hugo
├── serve.sh         # Script untuk development server
└── build.sh         # Script untuk build production
```

## Menulis Konten

### Front Matter
Setiap halaman menggunakan TOML front matter:

```toml
+++
title = "Judul Halaman"
description = "Deskripsi halaman"
weight = 1
+++

# Konten Markdown
```

### Menambah Halaman Baru
1. Buat file `.md` di dalam `content/docs/`
2. Tambahkan front matter
3. Tulis konten dalam Markdown
4. Server akan auto-reload

## Fitur Lotus Docs

- Dark mode
- Syntax highlighting dengan Prism
- Table of contents otomatis
- Search functionality
- Mobile responsive
- Google Fonts support

## Resources

- [Hugo Documentation](https://gohugo.io/documentation/)
- [Lotus Docs Guide](https://lotusdocs.dev/docs/)
- [Markdown Guide](https://www.markdownguide.org/)
