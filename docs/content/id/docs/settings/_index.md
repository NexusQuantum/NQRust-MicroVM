+++
title = "Pengaturan"
description = "Konfigurasikan akun, tampilan, nilai default, dan opsi seluruh platform"
weight = 120
sort_by = "weight"
template = "section.html"
page_template = "page.html"
+++

# Pengaturan

Halaman **Settings** memungkinkan Anda mengonfigurasi profil akun, tampilan platform, nilai default sistem, logging, dan lisensi. Klik **Settings** di sidebar kiri untuk membukanya.

![Image: Settings page overview](/images/settings/settings-overview.png)

Pengaturan diorganisasi dalam enam tab:

| Tab | Yang dapat dikonfigurasi |
|-----|----------------------|
| **Account** | Profil, kata sandi, avatar |
| **Appearance** | Tema, zona waktu, bahasa |
| **Logging** | Log aktivitas, ekspor |
| **Defaults** | Ukuran sumber daya VM default |
| **System** | Info platform dan statistik database |
| **License** | Aktivasi dan status lisensi perangkat lunak |

---

## Tab Account

![Image: Account tab](/images/settings/settings-account.png)

Kelola profil pengguna Anda:

- **Display Name / Avatar** — Upload foto profil atau ubah nama tampilan Anda
- **Change Password** — Masukkan kata sandi saat ini dan yang baru untuk memperbaruinya
- **Profile Information** — Perbarui username dan detail akun lainnya

### Mengubah Kata Sandi

1. Buka **Settings → Account**
2. Gulir ke bagian **Change Password**
3. Masukkan **kata sandi saat ini**
4. Masukkan dan konfirmasi **kata sandi baru**
5. Klik **Save**

---

## Tab Appearance

![Image: Appearance/Preferences tab](/images/settings/settings-preferences.png)

Sesuaikan tampilan dan nuansa:

- **Theme** — Pilih Dark, Light, atau default Sistem
- **Timezone** — Atur zona waktu lokal Anda untuk timestamp
- **Date Format** — Beralih antara format tanggal regional
- **Language** — Preferensi bahasa antarmuka

Perubahan diterapkan segera tanpa memuat ulang halaman.

---

## Tab Logging

![Image: Logging/Audit tab](/images/settings/settings-logging.png)

Lihat log aktivitas platform, yang mencatat peristiwa sistem seperti:

- Operasi siklus hidup VM (buat, mulai, hentikan, hapus)
- Login pengguna dan peristiwa autentikasi
- Deployment container
- Aktivasi lisensi

Gunakan tombol **Export** untuk mengunduh log sebagai CSV untuk tujuan kepatuhan atau peninjauan.

---

## Tab Defaults

![Image: Defaults tab](/images/settings/settings-defaults.png)

Konfigurasikan nilai default yang diisi otomatis saat membuat VM baru:

- **vCPUs** — Jumlah CPU default untuk VM baru
- **Memory** — Alokasi RAM default (MB)
- **Boot Arguments** — Argumen boot kernel default
- **Image Selection** — Preferensi image kernel dan rootfs

Ini adalah preferensi tingkat pengguna — setiap pengguna dapat mengatur defaultnya sendiri.

---

## Tab System

![Image: System tab](/images/settings/settings-system.png)

Lihat status platform dan informasi teknis:

- **Manager Version** — Versi build saat ini
- **Database** — Status koneksi PostgreSQL dan versi migrasi
- **Uptime** — Berapa lama layanan Manager telah berjalan
- **Registered Hosts** — Jumlah host komputasi aktif
- **API Endpoint** — URL API Manager untuk konfigurasi klien

Tab ini hanya baca dan berguna untuk dukungan dan diagnostik.

---

## Tab License

![Image: License tab showing active license](/images/settings/settings-license.png)

Kelola lisensi perangkat lunak Anda:

### Status Lisensi

Tab License menampilkan status aktivasi saat ini:

- 🟢 **Active** — Lisensi valid dan diaktifkan
- 🟡 **Grace Period** — Lisensi kedaluwarsa; waktu terbatas untuk mengaktifkan kembali
- 🔴 **Unlicensed** — Tidak ada lisensi valid; dibatasi ke halaman setup

### Melihat Detail Lisensi

Saat diaktifkan, Anda akan melihat:
- **Product** — Nama produk yang dilisensikan
- **Customer** — Nama organisasi Anda
- **License Key** — Kunci yang disembunyikan (mis. `DGRG-****-****-T4BW`)
- **Expires** — Tanggal kedaluwarsa lisensi

### Mengaktifkan Lisensi

Jika lisensi Anda belum diaktifkan:

1. Buka **Settings → License**
2. Klik **Update License Key**
3. Masukkan kunci lisensi Anda dalam format `XXXX-XXXX-XXXX-XXXX`
4. Klik **Activate**

Untuk aktivasi offline, gunakan opsi **Upload License File** untuk mengunggah file `.lic` yang disediakan oleh Nexus Quantum.

### EULA

Tab License juga menampilkan status penerimaan EULA Anda dan tautan ke Perjanjian Lisensi Pengguna Akhir lengkap. Klik **View EULA** untuk membuka perjanjian lengkap.
