+++
title = "Jaringan"
description = "Panduan lengkap tentang jaringan virtual untuk konektivitas VM melalui antarmuka web"
weight = 80
date = 2025-01-08
+++

Jaringan menyediakan konektivitas untuk VM Anda. Platform secara otomatis menyiapkan bridge, DHCP, dan aturan firewall pada host saat Anda membuat jaringan.

---

## Tipe Jaringan

### NAT
Subnet privat dengan akses internet melalui NAT host. VM mendapatkan alamat DHCP dan dapat mengakses internet melalui host. Paling cocok untuk sebagian besar beban kerja di mana VM memerlukan akses internet keluar tanpa dapat dijangkau langsung di LAN.

### Isolated
Subnet privat tanpa akses internet. VM hanya dapat berkomunikasi satu sama lain. Ideal untuk beban kerja air-gapped, layanan internal yang aman, dan lingkungan yang tidak boleh mengakses internet.

### Bridged
Akses LAN langsung. NIC fisik terhubung ke bridge, memberikan VM alamat langsung di jaringan eksternal Anda. Jaringan eksternal menangani pemberian IP — tidak ada CIDR, gateway, atau DHCP yang dikonfigurasi oleh platform untuk jaringan bridged.

### VXLAN (Overlay)
Jaringan overlay multi-host. VM pada host yang berbeda berkomunikasi melalui tunnel VXLAN. Host gateway menjalankan DHCP dan NAT untuk overlay tersebut. VNI ditetapkan secara otomatis dan overlay secara otomatis berkembang ke host lain saat VM dibuat.

---

## Halaman Jaringan

Navigasi ke **Networks** di sidebar untuk melihat semua jaringan.

![Networks list page](/images/networks/networks-list.png)

Tabel menampilkan:

| Kolom | Keterangan |
|---|---|
| **Name** | Nama jaringan (ikon kunci = jaringan default/terproteksi) |
| **Type** | NAT, Isolated, Bridged, atau VXLAN |
| **VLAN/VNI** | Tag VLAN atau VXLAN VNI (— jika tidak diatur) |
| **Status** | Active (hijau) atau Inactive |
| **Bridge** | Antarmuka bridge Linux pada host (mis. `fcbr0`) |
| **CIDR** | Rentang subnet (— untuk jaringan Bridged) |
| **Host** | URL agen yang mengelola jaringan ini |
| **VMs** | Jumlah VM yang saat ini berada di jaringan ini |
| **Created** | Waktu pembuatan relatif |
| **Actions** | Edit (pensil) dan Delete (tempat sampah) |

---

## Kasus Penggunaan Umum

### Pemisahan lingkungan
```
NAT network — Development (10.0.1.0/24)
NAT network — Staging    (10.0.2.0/24)
NAT network — Production (10.0.3.0/24)
```

### Beban kerja air-gapped
```
Isolated network — Internal services with no internet access
```

### Akses LAN langsung
```
Bridged network — VMs get real IP addresses on your office/datacenter LAN
```

### Konektivitas VM multi-host
```
VXLAN (Overlay) — VMs on Host A and Host B communicate as if on the same LAN
```

---

## Langkah Berikutnya

- **[Kelola Jaringan](manage-networks/)** — Buat, edit, dan hapus jaringan
