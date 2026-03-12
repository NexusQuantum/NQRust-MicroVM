+++
title = "Kelola Pengguna"
description = "Panduan lengkap untuk membuat, mengedit, dan mengelola akun pengguna"
weight = 81
date = 2025-01-08
+++

Panduan ini memberikan instruksi langkah demi langkah untuk mengelola akun pengguna melalui antarmuka web. Pelajari cara membuat pengguna baru, mengedit akun yang ada, mengubah peran, dan menghapus pengguna.

---

## Mengakses Manajemen Pengguna

### Navigasi ke Halaman Users

![Image: Users navigation](/images/users/nav-users.png)

Klik **"Users"** di sidebar untuk mengakses halaman Manajemen Pengguna.

**Catatan**: Hanya administrator yang dapat mengakses halaman Users. Jika Anda tidak melihat "Users" di sidebar, Anda tidak memiliki hak akses Admin.

---

### Tata Letak Halaman Users

![Image: Users page layout](/images/users/page-layout-users.png)

Halaman menampilkan:

**Bagian Header**:
- Judul User Management
- Jumlah total pengguna
- Jumlah Admin

**Bagian Tabel**:
- Kotak pencarian untuk menemukan pengguna
- Dropdown filter peran
- Tabel pengguna dengan semua akun
- Paginasi (jika banyak pengguna)

---

## Membuat Pengguna

### Langkah 1: Buka Dialog Buat

![Image: Create user button](/images/users/create-user-button.png)

Klik tombol **"Create User"** di bagian atas halaman.

Dialog Create User terbuka:

![Image: Create user dialog](/images/users/create-user-dialog.png)

---

### Langkah 2: Masukkan Detail Pengguna

![Image: User form fields](/images/users/user-form-fields.png)

Isi field yang wajib:

**Username** (Wajib):
- Harus unik
- Gunakan huruf kecil, angka, titik
- Contoh: `john.smith`, `admin.ops`

**Password** (Wajib):
- Minimal 8 karakter disarankan
- Gunakan kata sandi kuat dengan campuran karakter
- Akan tersembunyi setelah pembuatan

**Role** (Wajib):
- **Admin** - Akses platform penuh
- **User** - Akses operasional standar
- **Viewer** - Akses hanya baca

---

### Langkah 3: Buat Pengguna

Klik tombol **"Create"** untuk membuat pengguna.

**Yang terjadi**:
1. Formulir memvalidasi input
2. Keunikan username diperiksa
3. Akun pengguna dibuat
4. Notifikasi sukses muncul
5. Dialog ditutup secara otomatis
6. Pengguna baru muncul di tabel

**Notifikasi sukses**:
```
User Created
User john.smith has been created successfully
```

---

### Langkah 4: Bagikan Kredensial

Setelah membuat pengguna:

1. Catat username
2. Bagikan kata sandi secara aman kepada pengguna
3. Sarankan untuk mengubah kata sandi saat login pertama kali

**Tips keamanan**: Jangan kirim kredensial melalui email yang tidak terenkripsi. Gunakan saluran pesan yang aman atau komunikasi langsung.

---

### Contoh: Buat Akun Developer

**Skenario**: Tambahkan developer baru ke tim

**Konfigurasi**:
- Username: `alice.developer`
- Password: `SecurePass123!`
- Role: `User`

**Langkah-Langkah**:
1. Klik "Create User"
2. Masukkan username: `alice.developer`
3. Masukkan kata sandi: `SecurePass123!`
4. Pilih peran: "User"
5. Klik "Create"

**Hasil**: Alice sekarang dapat login dan membuat/mengelola VM miliknya.

---

### Contoh: Buat Akun Admin

**Skenario**: Tambahkan administrator sistem baru

**Konfigurasi**:
- Username: `bob.admin`
- Password: `AdminPass456!`
- Role: `Admin`

**Langkah-Langkah**:
1. Klik "Create User"
2. Masukkan username: `bob.admin`
3. Masukkan kata sandi: `AdminPass456!`
4. Pilih peran: "Admin"
5. Klik "Create"

**Hasil**: Bob memiliki akses administratif penuh ke platform.

---

### Contoh: Buat Akun Viewer

**Skenario**: Tambahkan pemangku kepentingan yang membutuhkan akses pemantauan

**Konfigurasi**:
- Username: `carol.viewer`
- Password: `ViewerPass789!`
- Role: `Viewer`

**Langkah-Langkah**:
1. Klik "Create User"
2. Masukkan username: `carol.viewer`
3. Masukkan kata sandi: `ViewerPass789!`
4. Pilih peran: "Viewer"
5. Klik "Create"

**Hasil**: Carol dapat melihat sumber daya tetapi tidak dapat memodifikasi apapun.

---

## Mengedit Pengguna

### Langkah 1: Temukan Pengguna

![Image: User table search](/images/users/search-users.png)

Gunakan kotak pencarian atau gulir tabel untuk menemukan pengguna yang ingin Anda edit.

**Tips pencarian**:
- Ketik sebagian username untuk difilter
- Gunakan filter peran untuk mempersempit hasil
- Pengguna saat ini menampilkan badge "You"

---

### Langkah 2: Buka Dialog Edit

Klik **ikon pensil** di kolom Actions untuk pengguna.

Dialog Edit User terbuka:

![Image: Edit user dialog](/images/users/edit-user-dialog.png)

---

### Langkah 3: Perbarui Detail Pengguna

Anda dapat memperbarui:

**Username**:
- Ubah nama login pengguna
- Harus tetap unik

**Password**:
- Biarkan kosong untuk mempertahankan kata sandi saat ini
- Masukkan kata sandi baru untuk mengubahnya

**Role**:
- Ubah tingkat akses
- Berlaku segera setelah disimpan

---

### Langkah 4: Simpan Perubahan

Klik **"Save"** untuk menerapkan perubahan.

**Notifikasi sukses**:
```
User Updated
User has been updated successfully
```

---

### Contoh: Ubah Peran Pengguna

**Skenario**: Promosikan pengguna ke Admin

**Langkah-Langkah**:
1. Temukan pengguna di tabel
2. Klik Edit (ikon pensil)
3. Ubah Role dari "User" ke "Admin"
4. Klik Save

**Hasil**: Pengguna sekarang memiliki hak akses Admin.

---

### Contoh: Reset Kata Sandi

**Skenario**: Pengguna lupa kata sandi mereka

**Langkah-Langkah**:
1. Temukan pengguna di tabel
2. Klik Edit (ikon pensil)
3. Masukkan kata sandi baru di field Password
4. Klik Save
5. Sampaikan kata sandi baru kepada pengguna

**Hasil**: Pengguna dapat login dengan kata sandi baru.

---

### Contoh: Ubah Username

**Skenario**: Pengguna mengubah nama mereka

**Langkah-Langkah**:
1. Temukan pengguna di tabel
2. Klik Edit (ikon pensil)
3. Perbarui field Username
4. Klik Save
5. Informasikan pengguna tentang username baru mereka

**Hasil**: Pengguna harus login dengan username baru.

---

## Menghapus Pengguna

### Langkah 1: Temukan Pengguna

Temukan pengguna yang ingin Anda hapus di tabel.

**Pemeriksaan penting**:
- Tidak dapat menghapus diri sendiri (tombol delete dinonaktifkan)
- Pastikan pengguna tidak lagi diperlukan
- Pertimbangkan untuk memindahkan sumber daya terlebih dahulu

---

### Langkah 2: Klik Tombol Delete

![Image: Delete button](/images/users/delete-button.png)

Klik ikon **tempat sampah** di kolom Actions.

Dialog konfirmasi muncul:

![Image: Delete confirmation](/images/users/delete-confirm.png)

---

### Langkah 3: Konfirmasi Penghapusan

Tinjau pesan konfirmasi:

```
Delete User?

Are you sure you want to delete john.smith?
This action cannot be undone.

[Cancel]  [Delete]
```

Klik **"Delete"** untuk konfirmasi.

---

### Langkah 4: Pengguna Dihapus

**Notifikasi sukses**:
```
User Deleted
User has been deleted successfully
```

Pengguna menghilang dari tabel dan tidak lagi dapat login.

---

### Tidak Dapat Menghapus Diri Sendiri

![Image: Delete disabled](/images/users/delete-disabled.png)

Tombol delete **dinonaktifkan untuk akun Anda sendiri**.

**Alasan**: Demi keamanan, Anda tidak dapat menghapus akun Admin Anda sendiri. Ini mencegah penguncian yang tidak disengaja.

**Solusi**: Minta Admin lain untuk menghapus akun Anda jika diperlukan.

---

## Pencarian dan Pemfilteran

### Cari Berdasarkan Username

Ketik di kotak pencarian untuk memfilter pengguna:

**Contoh**:
- Ketik `john` untuk menemukan `john.smith`, `john.doe`
- Ketik `admin` untuk menemukan pengguna dengan "admin" dalam username
- Pencarian tidak peka huruf besar/kecil

**Tips**: Pencarian diperbarui secara instan saat Anda mengetik.

---

### Filter Berdasarkan Peran

![Image: Role filter](/images/users/role-filter.png)

Gunakan dropdown peran untuk memfilter:

**Opsi**:
- **All Roles** - Tampilkan semua pengguna
- **Admin** - Tampilkan hanya Admin
- **User** - Tampilkan hanya pengguna standar
- **Viewer** - Tampilkan hanya Viewer

**Kasus penggunaan**:
- Tinjau semua akun Admin
- Temukan pengguna yang perlu penyesuaian peran
- Audit akses peran tertentu

---

### Pemfilteran Gabungan

Anda dapat menggabungkan pencarian dan filter peran:

**Contoh**: Temukan semua akun Admin dengan "john"
1. Pilih "Admin" di filter peran
2. Ketik "john" di kotak pencarian
3. Hasil menampilkan akun Admin yang cocok

---

## Informasi Tabel Pengguna

### Memahami Tabel

**Kolom**:

| Kolom | Deskripsi |
|--------|-------------|
| Username | Nama login pengguna |
| Role | Badge tingkat akses |
| Created | Tanggal pembuatan akun |
| Last Login | Waktu login terbaru |
| Actions | Tombol Edit dan Delete |

---

### Badge Peran

Badge peran berkode warna untuk identifikasi cepat:

| Peran | Warna | Arti |
|------|-------|---------|
| Admin | Merah | Akses penuh |
| User | Biru | Akses standar |
| Viewer | Abu-abu | Hanya baca |

---

### Badge "You"

Akun Anda sendiri menampilkan badge **"You"** di samping username.

**Tujuan**:
- Mudah mengidentifikasi akun Anda
- Pengingat bahwa Anda tidak dapat menghapus diri sendiri
- Referensi cepat untuk login saat ini

---

### Kolom Last Login

Menampilkan kapan pengguna terakhir login:

**Format**:
- "Never" - Pengguna belum pernah login
- "2 hours ago" - Login baru-baru ini (waktu relatif)
- "Jan 8, 2025" - Login lama (format tanggal)

**Kasus penggunaan**:
- Identifikasi akun yang tidak aktif
- Verifikasi pengguna baru telah login
- Audit keamanan

---

## Paginasi

### Navigasi Beberapa Halaman

Saat Anda memiliki banyak pengguna, paginasi muncul:

**Kontrol**:
- **Previous** - Pergi ke halaman sebelumnya
- **Nomor halaman** - Lompat ke halaman tertentu
- **Next** - Pergi ke halaman berikutnya

**Perilaku**:
- 10 pengguna per halaman secara default
- Pemfilteran mengatur ulang ke halaman 1
- Halaman saat ini disorot

---

## Operasi Massal

### Yang Tersedia Saat Ini

Hanya operasi individual:
- Buat satu pengguna sekaligus
- Edit satu pengguna sekaligus
- Hapus satu pengguna sekaligus

### Tips untuk Banyak Pengguna

Untuk menambahkan banyak pengguna:
1. Siapkan daftar pengguna terlebih dahulu
2. Buat pengguna satu per satu
3. Gunakan konvensi penamaan yang konsisten
4. Dokumentasikan akun baru secara eksternal

---

## Tugas Umum

### Tugas: Onboard Tim Baru

**Skenario**: Tambahkan 3 developer baru

**Langkah-Langkah**:
1. Buka halaman Users
2. Buat pengguna pertama:
   - Username: `dev1.name`
   - Peran: User
3. Buat pengguna kedua:
   - Username: `dev2.name`
   - Peran: User
4. Buat pengguna ketiga:
   - Username: `dev3.name`
   - Peran: User
5. Bagikan kredensial dengan aman

---

### Tugas: Audit Akses Admin

**Skenario**: Tinjau siapa yang memiliki hak akses Admin

**Langkah-Langkah**:
1. Buka halaman Users
2. Pilih "Admin" di filter peran
3. Tinjau daftar Admin
4. Periksa tanggal login terakhir
5. Hapus akses Admin yang tidak diperlukan

---

### Tugas: Offboard Karyawan

**Skenario**: Karyawan meninggalkan perusahaan

**Langkah-Langkah**:
1. Buka halaman Users
2. Cari pengguna
3. Catat sumber daya yang mereka miliki
4. Pindahkan/hapus sumber daya mereka jika diperlukan
5. Klik tombol Delete
6. Konfirmasi penghapusan

---

### Tugas: Reset Kata Sandi

**Skenario**: Pengguna terkunci dari akun

**Langkah-Langkah**:
1. Buka halaman Users
2. Temukan pengguna
3. Klik tombol Edit
4. Masukkan kata sandi baru
5. Simpan perubahan
6. Kirim kata sandi baru kepada pengguna dengan aman

---

### Tugas: Ubah Peran

**Skenario**: Promosikan pengguna setelah masa pelatihan

**Langkah-Langkah**:
1. Buka halaman Users
2. Temukan pengguna (saat ini Viewer)
3. Klik tombol Edit
4. Ubah peran ke "User"
5. Simpan perubahan
6. Informasikan pengguna tentang kemampuan baru

---

## Pemecahan Masalah

### Username Sudah Ada

**Gejala**:
- Pembuatan pengguna gagal
- Error: "Username already exists"

**Solusi**:
1. Cari pengguna yang ada dengan nama tersebut
2. Pilih username yang berbeda
3. Pertimbangkan untuk menambahkan angka: `john.smith2`

---

### Tidak Dapat Mengedit Pengguna

**Gejala**:
- Edit tidak tersimpan
- Notifikasi error muncul

**Kemungkinan penyebab**:
1. Konflik username dengan pengguna yang ada
2. Masalah koneksi server
3. Data yang dimasukkan tidak valid

**Solusi**:
1. Coba username yang unik
2. Muat ulang halaman dan coba lagi
3. Periksa semua field valid

---

### Tombol Delete Dinonaktifkan

**Gejala**:
- Tidak dapat mengklik tombol delete
- Tombol tampak abu-abu

**Kemungkinan penyebab**:
1. Mencoba menghapus diri sendiri

**Solusi**:
1. Minta Admin lain untuk menghapus akun
2. Ini adalah fitur keamanan, bukan bug

---

### Pengguna Tidak Muncul

**Gejala**:
- Pengguna yang dibuat tidak ada di tabel
- Pencarian tidak mengembalikan hasil

**Kemungkinan penyebab**:
1. Halaman belum dimuat ulang
2. Filter peran menyembunyikan pengguna
3. Pembuatan sebenarnya gagal

**Solusi**:
1. Muat ulang halaman
2. Atur filter peran ke "All Roles"
3. Hapus kotak pencarian
4. Periksa notifikasi sukses

---

### Perubahan Tidak Tercermin

**Gejala**:
- Edit tersimpan tetapi nilai lama ditampilkan
- Peran tampak tidak berubah

**Kemungkinan penyebab**:
1. Cache browser
2. Halaman belum dimuat ulang

**Solusi**:
1. Muat ulang halaman (F5)
2. Hapus cache browser
3. Logout dan login kembali

---

## Praktik Terbaik

### 1. Penamaan yang Konsisten

**Gunakan format standar**:
```
firstname.lastname
Contoh:
- john.smith
- alice.johnson
- bob.admin
```

**Manfaat**:
- Tampilan profesional
- Mudah mengidentifikasi pengguna
- Mudah diingat

---

### 2. Disiplin Peran

**Ikuti prinsip hak akses minimum**:
- Mulai pengguna baru sebagai Viewer
- Promosikan ke User setelah verifikasi
- Admin hanya untuk administrator
- Tinjauan peran berkala

---

### 3. Manajemen Kata Sandi

**Praktik kata sandi yang aman**:
- Buat kata sandi yang kuat
- Jangan gunakan kata sandi yang sama
- Komunikasikan dengan aman
- Dorong perubahan kata sandi

---

### 4. Dokumentasi

**Simpan catatan**:
- Siapa yang ditambahkan dan kapan
- Penugasan peran dan alasannya
- Tanggal kepergian untuk pengguna yang di-offboard
- Tinjauan akses yang selesai

---

### 5. Audit Berkala

**Tinjauan berkala**:
- Bulanan: Periksa pengguna yang tidak aktif
- Kuartalan: Tinjau penugasan peran
- Tahunan: Audit akses penuh

---

## Referensi Cepat

### Aksi Manajemen Pengguna

| Aksi | Langkah | Catatan |
|--------|-------|-------|
| Buat Pengguna | Tombol Create User → Isi formulir → Create | Hanya Admin |
| Edit Pengguna | Tombol Edit → Perbarui field → Save | Hanya Admin |
| Hapus Pengguna | Tombol Delete → Konfirmasi | Tidak dapat menghapus diri sendiri |
| Cari | Ketik di kotak pencarian | Filter instan |
| Filter Peran | Gunakan dropdown peran | Kombinasikan dengan pencarian |

---

### Pintasan Keyboard

| Aksi | Pintasan |
|--------|----------|
| Fokus pencarian | Klik kotak pencarian |
| Kirim formulir | Enter |
| Batalkan dialog | Esc |
| Muat ulang halaman | F5 |

---

## Langkah Selanjutnya

- **[Ikhtisar Pengguna](./)** - Pelajari tentang peran dan kontrol akses pengguna
- **[VM](/docs/vm/)** - Buat dan kelola mesin virtual
- **[Jaringan](/docs/networks/)** - Konfigurasikan pengaturan jaringan
- **[Volume](/docs/volumes/)** - Kelola volume penyimpanan
