+++
title = "Host"
description = "Pantau dan kelola host komputasi yang menjalankan layanan agent NQRust-MicroVM"
weight = 100
sort_by = "weight"
template = "section.html"
page_template = "page.html"
+++

# Host

Sebuah **Host** adalah mesin fisik atau virtual yang menjalankan **agent NQRust-MicroVM**, yang memungkinkan Manager untuk membuat dan mengontrol Firecracker microVM di mesin tersebut.

![Image: Hosts page overview](/images/hosts/hosts-page.png)

---

## Apa Itu Host?

Host adalah node komputasi yang menggerakkan platform microVM Anda. Setiap host:

- Menjalankan layanan **nexus-agent**
- Harus memiliki dukungan KVM yang diaktifkan (`/dev/kvm`)
- Mendaftar secara otomatis ke Manager saat startup
- Melaporkan kesehatan melalui heartbeat berkala
- Menyediakan kapasitas untuk menjalankan VM

### Persyaratan Host

| Persyaratan | Detail |
|-------------|---------|
| **OS** | Linux (Ubuntu 22.04+ disarankan) |
| **KVM** | Virtualisasi hardware diaktifkan |
| **Jaringan** | Dapat dijangkau oleh Manager (port default 9090) |
| **Agent** | Layanan nexus-agent terpasang dan berjalan |

---

## Halaman Hosts

Navigasikan ke **Hosts** di sidebar kiri.

![Image: Hosts table with healthy host](/images/hosts/hosts-overview.png)

Halaman Hosts menampilkan:

- **Name / Address** — URL agent (mis. `http://127.0.0.1:19090`)
- **Status** — `healthy`, `unreachable`, atau `degraded`
- **Resources** — vCPU, RAM, dan total/disk yang digunakan
- **Source Count** — Jumlah VM atau container yang berjalan di host ini
- **Last Seen** — Kapan agent terakhir mengirim heartbeat
- **Actions** — Hapus host yang sudah tidak terdaftar

### Badge Status

| Status | Arti |
|--------|---------|
| 🟢 **healthy** | Agent dapat dijangkau dan merespons secara normal |
| 🟡 **degraded** | Agent merespons tetapi melaporkan tekanan sumber daya |
| 🔴 **unreachable** | Tidak ada heartbeat yang diterima dalam jendela waktu yang diharapkan |

---

## Memperbarui Status Host

Klik tombol **Refresh** di kanan atas tabel host untuk segera memeriksa ulang status heartbeat semua host yang terdaftar.

---

## Mendaftarkan Host Baru

Host mendaftar secara otomatis ketika **nexus-agent** dimulai di mesin. Untuk menambahkan host baru:

1. Pasang nexus-agent di mesin target (lihat [Panduan Instalasi](/docs/getting-started/installation/))
2. Konfigurasikan agent dengan alamat Manager:
   ```bash
   MANAGER_BASE=http://<manager-ip>:18080 nexus-agent
   ```
3. Agent akan muncul di tabel Hosts dalam hitungan detik

---

## Menghapus Host

Untuk menghapus host yang sudah tidak terdaftar atau sudah tidak digunakan:

1. Pastikan tidak ada VM yang berjalan di host tersebut
2. Klik ikon **tempat sampah** di kolom Actions
3. Konfirmasi penghapusan

> **Peringatan**: Menghapus host tidak menghentikan layanan agent di mesin. Hentikan layanan `nexus-agent` secara terpisah.

---

## Setup Multi-Host

NQRust-MicroVM mendukung beberapa host secara bersamaan. Manager mendistribusikan beban kerja ke seluruh host yang sehat dan tersedia.

**Contoh setup**:
```
Manager (host-1)  ←→  Agent (host-1)  [lokal]
                  ←→  Agent (host-2)  [remote via jaringan]
                  ←→  Agent (host-3)  [remote via jaringan]
```

Setiap host dikelola secara independen, dengan VM yang terpaku pada host tempat mereka dibuat.

---

## Pemecahan Masalah

### Host menampilkan "unreachable"

1. Periksa apakah layanan agent sedang berjalan di host:
   ```bash
   sudo systemctl status nexus-agent
   ```
2. Verifikasi konektivitas jaringan dari Manager ke port agent 9090
3. Periksa aturan firewall yang memblokir port
4. Lihat log agent: `sudo journalctl -u nexus-agent -f`

### Host tidak muncul setelah agent dimulai

1. Verifikasi `MANAGER_BASE` diatur dengan benar dalam konfigurasi agent
2. Periksa log Manager untuk error pendaftaran
3. Pastikan agent memiliki akses KVM (`ls -la /dev/kvm`)
