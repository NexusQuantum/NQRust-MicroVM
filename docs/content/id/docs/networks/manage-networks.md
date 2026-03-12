+++
title = "Kelola Jaringan"
description = "Panduan lengkap untuk melihat, membuat, dan mengelola jaringan virtual melalui antarmuka web"
weight = 61
date = 2025-01-08
+++

Buat dan kelola jaringan virtual untuk VM Anda.

---

## Membuat Jaringan

Klik **+ Create Network** di halaman Networks. Platform secara otomatis menyiapkan bridge, DHCP, dan aturan firewall pada host.

### Kolom umum

**Network Name** *(wajib)* — Nama yang deskriptif, mis. `Dev Network`, `Production`.

**Description** *(opsional)* — Catatan teks bebas mengenai tujuan jaringan.

**Network Type** *(wajib)* — Pilih salah satu dari empat tipe (lihat di bawah).

**Host** *(wajib)* — Host agen yang akan menyiapkan dan mengelola jaringan ini.

---

### NAT

Subnet privat dengan akses internet melalui NAT host. VM menerima alamat DHCP dan mengakses internet melalui host.

![Create Network — NAT type selected](/images/networks/network-create-nat.png)

**Kolom tambahan**:

| Kolom | Keterangan |
|---|---|
| **Subnet CIDR** | mis. `10.0.2.0/24`. Biarkan kosong untuk menggunakan rentang yang disarankan secara otomatis. |
| **VLAN ID** | Tag VLAN 802.1Q opsional (memerlukan trunk port pada uplink). |
| **DHCP Server** | Aktifkan/nonaktifkan. Saat aktif, atur Range Start dan Range End. |

Saat DHCP diaktifkan, platform menampilkan konfigurasi yang ditetapkan secara otomatis (nama bridge, subnet, gateway, rentang DHCP) sebelum Anda mengonfirmasi.

![Create Network — DHCP config preview](/images/networks/network-create-isolated.png)

---

### Isolated

Subnet privat tanpa akses internet. VM hanya dapat berkomunikasi satu sama lain. Ideal untuk beban kerja air-gapped.

Kolom yang sama dengan NAT (Subnet CIDR, VLAN ID, DHCP Server).

---

### Bridged

Akses LAN langsung. NIC fisik terhubung ke bridge sehingga VM mendapatkan alamat nyata di jaringan eksternal Anda.

![Create Network — Bridged type selected](/images/networks/network-create-bridged.png)

**Kolom tambahan**:

| Kolom | Keterangan |
|---|---|
| **Network Interface** | Pilih NIC fisik dari dropdown. |
| **VLAN ID** | Tag VLAN 802.1Q opsional. |

> Jaringan eksternal menangani pemberian IP. Tidak ada CIDR, gateway, atau DHCP yang dikonfigurasi oleh platform untuk jaringan bridged.

---

### VXLAN (Overlay)

Jaringan overlay multi-host. VM pada host yang berbeda berkomunikasi melalui tunnel VXLAN. VNI ditetapkan secara otomatis.

![Create Network — VXLAN type selected](/images/networks/network-create-vxlan.png)

**Kolom tambahan**:

| Kolom | Keterangan |
|---|---|
| **Gateway Host** | Host yang menjalankan DHCP dan NAT untuk overlay. |

> Overlay secara otomatis berkembang ke host lain saat VM dibuat di sana.

---

## Menghapus Jaringan

Klik ikon **tempat sampah** di kolom Actions. Jaringan dengan VM yang terhubung tidak dapat dihapus — hentikan atau pindahkan VM tersebut terlebih dahulu.

Menghapus jaringan akan menghapus entri registry dan membongkar bridge serta konfigurasi DHCP yang disediakan platform pada host.

---

## Pemecahan Masalah

### Tidak dapat menghapus jaringan

Kolom **VMs** menampilkan jumlah > 0. Hentikan atau hapus VM tersebut (atau konfigurasikan ulang untuk menggunakan jaringan yang berbeda), lalu coba hapus kembali.

### Pembuatan jaringan gagal

- Pastikan Host telah dipilih
- Untuk jaringan Bridged, pastikan NIC fisik tersedia di dropdown
- Untuk NAT/Isolated, periksa apakah Subnet CIDR tidak tumpang tindih dengan jaringan yang sudah ada
- Periksa apakah agen host sedang online di halaman Hosts

### VM tidak mendapatkan alamat IP

- Konfirmasi DHCP Server diaktifkan pada jaringan NAT/Isolated
- Verifikasi rentang DHCP berada dalam Subnet CIDR
- Periksa apakah agen host sedang online
- Untuk jaringan Bridged, konfirmasi server DHCP upstream dapat dijangkau

---

## Langkah Berikutnya

- **[Ikhtisar Jaringan](./)** — Penjelasan tipe jaringan
- **[Buat VM](../vm/create-vm/)** — Tetapkan jaringan saat membuat VM
