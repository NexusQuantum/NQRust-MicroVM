+++
title = "Instalasi"
description = "Instal NQRust-MicroVM menggunakan installer online atau airgapped"
weight = 2
date = 2025-12-01

[extra]
toc = true
+++

NQRust-MicroVM diinstal melalui `nqr-installer` — sebuah TUI Rust terpandu yang menyediakan segalanya di host Anda: akses KVM, jembatan jaringan, PostgreSQL, layanan systemd, dan konfigurasi platform.

Pilih metode yang sesuai dengan lingkungan Anda.

---

## Persyaratan Sistem

| | Minimum | Direkomendasikan |
|---|---|---|
| **CPU** | x86_64 dengan KVM (Intel VT-x / AMD-V) | — |
| **RAM** | 4 GB | 8 GB+ |
| **Disk** | 20 GB kosong | 50 GB+ |
| **OS** | Ubuntu 22.04, Debian 11 | Ubuntu 24.04 LTS |

**Verifikasi dukungan KVM sebelum menginstal:**
```bash
egrep -c '(vmx|svm)' /proc/cpuinfo   # must be > 0
lsmod | grep kvm                      # must show kvm module
```

---

## Instalasi Online

Untuk host dengan akses internet. Skrip mengunduh binary `nqr-installer` terbaru dari GitHub Releases dan menjalankan TUI.

```bash
curl -fsSL https://github.com/NexusQuantum/NQRust-MicroVM/releases/latest/download/install.sh | sudo bash
```

Installer membuka TUI terpandu yang memandu Anda melalui setiap langkah:

![Layar selamat datang NQR-MicroVM Installer](/images/installer/installer-welcome.png)

**Langkah 1 — Pilih mode instalasi**

Pilih komponen yang akan diinstal. Untuk pengaturan satu host, pilih **Production (Manager + Agent + UI)**.

![Layar pemilihan mode](/images/installer/installer-mode-selection.png)

| Mode | Komponen | Kasus Penggunaan |
|---|---|---|
| **Production** | Manager + Agent + UI | Host tunggal, all-in-one |
| **Development** | Manager + Agent + UI | Build dari source |
| **Manager Only** | Manager | Node control plane |
| **Agent Only** | Agent | Node worker |
| **Minimal** | Manager + Agent | Tanpa web UI |

**Langkah 2 — Konfigurasi jaringan**

Pilih mode bridge dan antarmuka uplink untuk jaringan VM.

![Layar konfigurasi jaringan](/images/installer/installer-network-config.png)

**Langkah 3 — Konfigurasi**

Tinjau dan sesuaikan jalur instalasi serta pengaturan database. Nilai default cocok untuk sebagian besar deployment.

![Layar konfigurasi](/images/installer/installer-configuration.png)

**Langkah 4 — Pemeriksaan awal**

Installer memvalidasi sistem Anda sebelum melakukan perubahan apa pun. Semua pemeriksaan harus lulus untuk melanjutkan.

![Layar pemeriksaan awal](/images/installer/installer-preflight-checks.png)

**Langkah 5 — Instalasi**

Installer menyediakan setiap komponen secara berurutan dan menampilkan log secara langsung.

![Layar progres instalasi](/images/installer/installer-progress.png)

![Progres instalasi selesai](/images/installer/installer-progress-complete.png)

**Langkah 6 — Verifikasi**

Installer memverifikasi setiap komponen dalam kondisi sehat sebelum selesai.

![Layar verifikasi instalasi](/images/installer/installer-verification.png)

**Langkah 7 — Selesai**

Instalasi telah selesai. Installer menampilkan URL akses Anda dan keluar.

![Layar instalasi selesai](/images/installer/installer-complete.png)

---

## Instalasi Airgapped

Untuk host tanpa akses internet. Unduh binary installer di mesin yang terhubung internet dan transfer ke host target Anda.

**Langkah 1 — Unduh di mesin yang terhubung:**
```bash
curl -fsSL -o nqr-installer \
  https://github.com/NexusQuantum/NQRust-MicroVM/releases/latest/download/nqr-installer-x86_64-linux-musl

chmod +x nqr-installer
```

**Langkah 2 — Transfer ke host target:**
```bash
scp nqr-installer user@target-host:/tmp/nqr-installer
```

**Langkah 3 — Jalankan di host target:**
```bash
sudo /tmp/nqr-installer install
```

Installer beroperasi sepenuhnya secara offline — tidak ada unduhan yang terjadi selama proses instalasi itu sendiri.

---

## Mode Instalasi

Ketika diminta, installer menanyakan komponen mana yang akan di-deploy:

| Mode | Kasus Penggunaan |
|---|---|
| **All-in-one** | Host tunggal menjalankan manager, agent, dan UI (default) |
| **Manager only** | Node control plane dalam pengaturan multi-host |
| **Agent only** | Node worker yang bergabung dengan manager yang sudah ada |

Untuk deployment multi-host, jalankan installer dengan **Manager only** di control plane terlebih dahulu, kemudian **Agent only** di setiap worker yang mengarah ke alamat manager.

---

## Yang Terinstal

Setelah berhasil dijalankan:

| Jalur | Konten |
|---|---|
| `/opt/nqrust-microvm/bin/` | Binary `manager`, `agent`, `guest-agent` |
| `/opt/nqrust-microvm/ui/` | Build statis frontend Next.js |
| `/etc/nqrust-microvm/` | File konfigurasi `manager.env`, `agent.env`, `ui.env` |
| `/srv/fc/vms/` | Penyimpanan runtime VM |
| `/srv/images/` | Penyimpanan registri image |
| `/var/log/nqrust-microvm/` | Log layanan |

Layanan dikelola oleh systemd:
```bash
systemctl status nqrust-manager
systemctl status nqrust-agent
```

---

## Setelah Instalasi

Setelah installer selesai, buka web UI di browser Anda. Installer menampilkan URL yang tepat di layar penyelesaian:

| Layanan | URL Default |
|---|---|
| **Web UI** | `http://<host>:3000` |
| **Manager API** | `http://<host>:18080` |
| **API Docs** | `http://<host>:18080/swagger-ui/` |
| **Agent API** | `http://<host>:9090` |

Kredensial default saat login pertama kali:
- **Nama Pengguna:** `root`
- **Kata Sandi:** `root`

Segera ganti kata sandi setelah login pertama melalui **Settings → Account**.

Lanjutkan ke [Quick Start](../quick-start/) untuk membuat VM pertama Anda.

---

## Pemecahan Masalah

### KVM tidak dapat diakses

```bash
ls -l /dev/kvm
# Should show: crw-rw---- 1 root kvm ...

# If your user is not in the kvm group:
sudo usermod -a -G kvm $USER
newgrp kvm
```

### Layanan tidak mau berjalan

```bash
journalctl -u nqrust-manager -n 50
journalctl -u nqrust-agent -n 50
```

### Koneksi database gagal

```bash
# Check PostgreSQL is running
sudo systemctl status postgresql

# Test connection
psql -h localhost -U nexus -d nexus
```

### Instalasi ulang

Untuk menghapus instalasi sebelumnya sebelum menjalankan ulang:
```bash
sudo /opt/nqrust-microvm/scripts/uninstall.sh
```
