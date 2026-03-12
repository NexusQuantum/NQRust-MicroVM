+++
title = "Pengguna"
description = "Panduan lengkap untuk manajemen pengguna dan kontrol akses"
weight = 110
date = 2025-01-08
+++

Manajemen Pengguna memungkinkan Anda membuat dan mengelola akun pengguna, menetapkan peran, dan mengontrol akses ke platform Anda. Panduan ini mencakup pembuatan pengguna, kontrol akses berbasis peran (RBAC), dan manajemen akun melalui antarmuka web.

---

## Apa Itu Manajemen Pengguna?

Manajemen Pengguna adalah **sistem pusat untuk mengontrol akses** ke platform Anda. Sistem ini memungkinkan administrator membuat akun pengguna, menetapkan peran, dan mengelola izin untuk semua pengguna.

### Manfaat Utama

**1. Kontrol Akses Berbasis Peran (RBAC)**
- Tiga peran pengguna: Admin, User, dan Viewer
- Kontrol apa yang dapat dilihat dan dilakukan setiap pengguna
- Lindungi operasi sensitif

**2. Administrasi Pengguna Terpusat**
- Lihat semua pengguna dalam satu tempat
- Buat, edit, dan hapus pengguna dengan mudah
- Lacak aktivitas pengguna dan riwayat login

**3. Manajemen Akun Mandiri**
- Pengguna dapat memperbarui profil mereka sendiri
- Fungsionalitas ubah kata sandi
- Upload avatar profil

---

## Peran Pengguna

### Peran Admin

**Akses platform penuh**:
- Buat, edit, dan hapus pengguna
- Akses semua VM, jaringan, volume
- Kelola semua pengaturan sistem
- Lihat semua sumber daya di seluruh platform

**Terbaik untuk**:
- Administrator sistem
- Pemimpin tim
- Manajer IT

---

### Peran User

**Akses operasional standar**:
- Buat dan kelola VM milik sendiri
- Akses sumber daya yang ditetapkan
- Tidak dapat mengelola pengguna lain
- Operasi sehari-hari standar

**Terbaik untuk**:
- Developer
- Anggota tim operasi
- Pengguna platform biasa

---

### Peran Viewer

**Akses hanya baca**:
- Lihat VM, jaringan, volume
- Tidak dapat membuat atau memodifikasi sumber daya
- Tidak ada izin tulis
- Hanya untuk pemantauan dan observasi

**Terbaik untuk**:
- Auditor
- Pemangku kepentingan yang membutuhkan visibilitas
- Anggota tim baru dalam pelatihan

---

## Properti Pengguna

Setiap akun pengguna mencakup:

**Informasi Dasar**:
- **Username** - Pengenal unik untuk login
- **Role** - Tingkat akses (Admin, User, Viewer)
- **Password** - Kredensial autentikasi yang aman

**Informasi Profil**:
- **Avatar** - Foto profil (opsional)
- **Timezone** - Zona waktu pilihan pengguna
- **Theme** - Preferensi mode gelap atau terang

**Pelacakan Aktivitas**:
- **Created At** - Kapan akun dibuat
- **Last Login** - Timestamp login terbaru

---

## Siklus Hidup Pengguna

### 1. Pembuatan Akun

**Cara pengguna dibuat**:
1. Admin navigasi ke halaman Users
2. Klik tombol "Create User"
3. Isi username, kata sandi, dan peran
4. Akun pengguna dibuat
5. Pengguna dapat langsung login

---

### 2. Penggunaan Akun

**Selama penggunaan aktif**:
- Pengguna login dengan kredensial
- Sistem melacak aktivitas login
- Pengguna mengakses sumber daya berdasarkan peran
- Pengguna dapat memperbarui profil mereka sendiri

---

### 3. Manajemen Akun

**Administrasi berkelanjutan**:
- Admin dapat mengedit detail pengguna
- Peran dapat diubah sesuai kebutuhan
- Kata sandi dapat direset
- Akun dapat dihapus saat tidak lagi diperlukan

---

## Mulai Cepat

### 1. Navigasi ke Halaman Users

![Image: Users navigation](/images/users/nav-users.png)

Klik **"Users"** di sidebar untuk mengakses Manajemen Pengguna.

---

### 2. Lihat Daftar Pengguna

Halaman Users menampilkan:
- Jumlah total pengguna
- Jumlah admin
- Tabel pengguna dengan semua akun
- Opsi pencarian dan filter

---

### 3. Buat Pengguna Baru

![Image: Create user](/images/users/create-user-button.png)

1. Klik tombol **"Create User"**
2. Isi detail pengguna:
   - Username (wajib)
   - Password (wajib)
   - Role (Admin, User, atau Viewer)
3. Klik **"Create"**

---

### 4. Kelola Pengguna yang Ada

Untuk setiap pengguna, Anda dapat:
- **Edit** - Perbarui username, kata sandi, atau peran
- **Delete** - Hapus akun pengguna

---

## Kasus Penggunaan Umum

### Orientasi Tim

**Tambahkan anggota tim baru**:
1. Buka halaman Users
2. Buat pengguna dengan peran yang sesuai
3. Bagikan kredensial dengan aman
4. Pengguna login dan mulai bekerja

**Contoh**:
```
Developer Baru:
- Username: john.developer
- Role: User
- Akses: Dapat membuat dan mengelola VM
```

---

### Penyesuaian Peran

**Ubah izin pengguna**:
1. Temukan pengguna di tabel
2. Klik tombol Edit
3. Ubah peran sesuai kebutuhan
4. Simpan perubahan

**Contoh**:
```
Promosi ke Admin:
- Pengguna: jane.ops
- Peran Lama: User
- Peran Baru: Admin
- Hasil: Akses platform penuh
```

---

### Offboarding

**Hapus anggota tim yang pergi**:
1. Temukan pengguna di tabel
2. Klik tombol Delete
3. Konfirmasi penghapusan
4. Akun dihapus

**Penting**: Anda tidak dapat menghapus akun Anda sendiri. Admin lain harus melakukan ini.

---

### Audit Akses

**Tinjau siapa yang memiliki akses**:
1. Buka halaman Users
2. Gunakan filter peran untuk melihat semua Admin
3. Tinjau tanggal login terakhir
4. Identifikasi akun yang tidak aktif

---

## Praktik Keamanan Terbaik

### 1. Gunakan Kata Sandi yang Kuat

**Panduan kata sandi**:
- Minimal 8 karakter
- Campuran huruf, angka, simbol
- Hindari kata-kata umum
- Unik untuk setiap pengguna

---

### 2. Prinsip Hak Akses Minimum

**Berikan akses minimum yang diperlukan**:
- Pengguna baru mulai sebagai Viewer
- Promosikan ke User jika diperlukan
- Admin hanya untuk administrator
- Tinjauan akses berkala

---

### 3. Tinjauan Akses Berkala

**Audit berkala**:
- Tinjau daftar pengguna setiap bulan
- Hapus akun yang tidak aktif
- Verifikasi penugasan peran
- Periksa tanggal login terakhir

---

### 4. Lindungi Akun Admin

**Keamanan akun Admin**:
- Batasi jumlah admin
- Wajib kata sandi yang kuat
- Pantau aktivitas admin
- Hapus akses admin saat tidak diperlukan

---

## Ikhtisar Antarmuka Pengguna

### Header Halaman Users

**Menampilkan**:
- Judul "User Management"
- Jumlah total pengguna
- Jumlah admin
- Ikon visual

---

### Tabel Pengguna

**Kolom**:
- **Username** - Nama login pengguna (menampilkan badge "You" untuk pengguna saat ini)
- **Role** - Badge peran berkode warna
- **Created** - Tanggal pembuatan akun
- **Last Login** - Login terbaru (atau "Never")
- **Actions** - Tombol Edit dan Delete

---

### Filter

**Filter yang tersedia**:
- **Search** - Temukan pengguna berdasarkan username
- **Role** - Filter berdasarkan Admin, User, atau Viewer

---

## Matriks Izin Peran

| Fitur | Admin | User | Viewer |
|---------|-------|------|--------|
| Lihat Pengguna | Ya | Tidak | Tidak |
| Buat Pengguna | Ya | Tidak | Tidak |
| Edit Pengguna | Ya | Tidak | Tidak |
| Hapus Pengguna | Ya | Tidak | Tidak |
| Lihat VM | Ya | Ya | Ya |
| Buat VM | Ya | Ya | Tidak |
| Kelola VM | Ya | Milik sendiri | Tidak |
| Lihat Jaringan | Ya | Ya | Ya |
| Kelola Jaringan | Ya | Ya | Tidak |
| Lihat Volume | Ya | Ya | Ya |
| Kelola Volume | Ya | Ya | Tidak |
| Pengaturan Sistem | Ya | Tidak | Tidak |

---

## Pemecahan Masalah

### Tidak Dapat Login

**Gejala**:
- Login gagal dengan kredensial
- Pesan error muncul

**Kemungkinan penyebab**:
1. Username salah
2. Kata sandi salah
3. Akun dihapus

**Solusi**:
1. Periksa kembali ejaan username
2. Coba reset kata sandi (hubungi admin)
3. Verifikasi akun ada di halaman Users

---

### Tidak Dapat Membuat Pengguna

**Gejala**:
- Pembuatan pengguna gagal
- Notifikasi error muncul

**Kemungkinan penyebab**:
1. Username sudah ada
2. Field wajib kosong
3. Masalah koneksi server

**Solusi**:
1. Coba username yang berbeda
2. Isi semua field yang wajib
3. Muat ulang halaman dan coba lagi

---

### Tidak Dapat Menghapus Pengguna

**Gejala**:
- Tombol Delete dinonaktifkan
- Tidak dapat menghapus pengguna

**Kemungkinan penyebab**:
1. Mencoba menghapus diri sendiri
2. Pengguna memiliki sumber daya terkait

**Solusi**:
1. Minta admin lain untuk menghapus (jika menghapus diri sendiri)
2. Hapus sumber daya pengguna terlebih dahulu

---

### Peran Tidak Berubah

**Gejala**:
- Edit peran tetapi tidak ada perubahan
- Pengguna masih memiliki izin lama

**Kemungkinan penyebab**:
1. Edit tidak tersimpan
2. Halaman belum dimuat ulang

**Solusi**:
1. Pastikan Anda mengklik Save
2. Muat ulang halaman
3. Verifikasi perubahan di tabel pengguna

---

## Praktik Terbaik

### 1. Konvensi Penamaan

**Gunakan username yang konsisten**:
```
Format: firstname.lastname
Contoh:
- john.smith
- jane.doe
- admin.main
```

**Manfaat**:
- Mudah diidentifikasi
- Tampilan profesional
- Konsisten di seluruh platform

---

### 2. Dokumentasikan Akses Pengguna

**Simpan catatan eksternal**:
```
Pengguna: john.developer
Peran: User
Departemen: Engineering
Ditambahkan: 2025-01-08
Tujuan: Akses development VM
```

---

### 3. Pembersihan Berkala

**Pertahankan daftar pengguna yang bersih**:
- Hapus karyawan yang telah pergi
- Nonaktifkan akun yang tidak aktif
- Tinjau penugasan peran setiap kuartal

---

### 4. Redundansi Admin

**Beberapa akun Admin disarankan**:
- Minimal 2 akun Admin
- Jangan bergantung pada satu Admin
- Admin cadangan untuk keadaan darurat

---

## Referensi Cepat

### Aksi Pengguna

| Aksi | Langkah | Siapa yang Dapat Melakukan |
|--------|-------|---------------|
| Buat Pengguna | Halaman Users → Create User → Isi formulir | Hanya Admin |
| Edit Pengguna | Halaman Users → Tombol Edit → Perbarui formulir | Hanya Admin |
| Hapus Pengguna | Halaman Users → Tombol Delete → Konfirmasi | Hanya Admin |
| Ubah Kata Sandi Sendiri | Profile → Change Password | Semua pengguna |
| Perbarui Profil Sendiri | Profile → Edit Profile | Semua pengguna |
| Lihat Pengguna | Navigasi ke halaman Users | Hanya Admin |

---

### Badge Peran

| Peran | Warna Badge | Deskripsi |
|------|-------------|-------------|
| Admin | Merah | Akses platform penuh |
| User | Biru | Akses operasional standar |
| Viewer | Abu-abu | Akses hanya baca |

---

## Langkah Selanjutnya

- **[Kelola Pengguna](manage-users/)** - Panduan detail operasi manajemen pengguna
- **[VM](/docs/vm/)** - Buat dan kelola mesin virtual
- **[Jaringan](/docs/networks/)** - Konfigurasikan pengaturan jaringan
- **[Volume](/docs/volumes/)** - Kelola volume penyimpanan

---

## FAQ

**T: Berapa banyak Admin yang harus saya miliki?**
J: Kami menyarankan minimal 2 akun Admin. Ini memastikan Anda tidak terkunci keluar jika satu Admin tidak tersedia. Namun, batasi akses Admin hanya untuk mereka yang benar-benar membutuhkannya.

**T: Bisakah saya menghapus akun saya sendiri?**
J: Tidak. Untuk alasan keamanan, Anda tidak dapat menghapus akun Anda sendiri. Admin lain harus menghapus akun Anda jika diperlukan.

**T: Apa yang terjadi saat pengguna dihapus?**
J: Akun pengguna dihapus secara permanen. Sumber daya yang mereka buat (VM, dll.) tetap ada di sistem. Pertimbangkan untuk memindahkan sumber daya sebelum menghapus.

**T: Bisakah saya mengubah username?**
J: Ya. Admin dapat mengedit username pengguna mana pun melalui fungsi Edit. Pengguna harus login dengan username baru.

**T: Bagaimana jika saya lupa kata sandi?**
J: Hubungi administrator. Mereka dapat mereset kata sandi Anda melalui fungsi Edit pengguna. Saat ini tidak ada reset kata sandi mandiri.

**T: Bisakah pengguna melihat sumber daya satu sama lain?**
J: Ini bergantung pada konfigurasi platform. Umumnya, Admin dapat melihat semua sumber daya, sementara User melihat sumber daya milik mereka sendiri. Viewer dapat melihat sumber daya tetapi tidak dapat memodifikasinya.
