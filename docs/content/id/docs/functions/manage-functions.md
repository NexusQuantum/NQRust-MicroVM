+++
title = "Kelola Functions"
description = "Invoke, perbarui, monitor, dan hapus serverless function"
weight = 42
date = 2025-12-18
+++

Pelajari cara mengelola serverless function Anda melalui antarmuka web.

---

## Mengakses Functions

Navigasi ke halaman **Functions** dari sidebar untuk melihat semua function Anda:

![Image: Functions list page](/images/functions/functions-page.png)

Halaman Functions menyediakan:
- Tombol **New Function** - Buat function baru
- Tombol **Playground** - Bereksperimen tanpa membuat function
- Tombol **Refresh** - Perbarui daftar function
- **Search bar** - Temukan function berdasarkan nama atau ID
- **Filter runtime** - Filter berdasarkan bahasa (Python, JavaScript, TypeScript)
- **Filter state** - Filter berdasarkan Ready, Creating, Deploying, Error
- **Tabel function** - Daftar semua function beserta detail dan aksi

---

## Status Function

Function dapat berada dalam beberapa status:

![Image: Function state badges](/images/functions/function-states.png)

| Status | Deskripsi | Aksi Tersedia |
|--------|-----------|---------------|
| **Ready** | Function ter-deploy dan siap | Invoke, Lihat Log, Delete |
| **Creating** | Function sedang dibuat | Tidak ada (tunggu) |
| **Deploying** | microVM sedang disiapkan | Tidak ada (tunggu) |
| **Error** | Function gagal deploy | Lihat Log, Delete |

**Transisi status**:
- Creating → Deploying → Ready
- Status apa pun → Error (jika ada yang gagal)

---

## Filter dan Pencarian Function

### Cari berdasarkan Nama atau ID

Gunakan search bar untuk menemukan function dengan cepat:

![Image: Search bar in Functions page](/images/functions/search-bar.png)

- Ketik nama function (mis., "hello")
- Atau ketik ID function
- Hasil difilter secara instan saat Anda mengetik
- Pencarian tidak case-sensitive

---

### Filter berdasarkan Runtime

Filter berdasarkan bahasa pemrograman:

![Image: Runtime filter dropdown](/images/functions/runtime-filter.png)

**Opsi**:
- **All Languages** - Tampilkan semua function (default)
- **Python** - Hanya function Python
- **JavaScript (Bun)** - Hanya function JavaScript
- **TypeScript (Bun)** - Hanya function TypeScript

---

### Filter berdasarkan Status

Filter berdasarkan status function:

![Image: State filter dropdown](/images/functions/state-filter.png)

**Opsi**:
- **All States** - Tampilkan semua function (default)
- **Ready** - Hanya function yang siap
- **Creating** - Function yang sedang dibuat
- **Deploying** - Function yang sedang di-deploy
- **Error** - Function yang gagal

**Tips**: Gabungkan pencarian dan filter untuk hasil yang tepat.

---

## Informasi Tabel Function

Tabel Functions menampilkan informasi detail:

![Image: Function table with all columns](/images/functions/function-table.png)

### Penjelasan Kolom

1. **Name** - Nama function (klik untuk membuka halaman detail)
2. **Language** - Badge runtime (Python, JavaScript, TypeScript)
3. **State** - Status saat ini dengan badge berwarna
4. **Last Invoked** - Waktu relatif (mis., "2 hours ago") atau "Never"
5. **24h Invocations** - Jumlah invokasi dalam 24 jam terakhir
6. **Guest IP** - Alamat IP dan port microVM function
7. **CPU** - Jumlah vCPU (mis., "1 vCPU")
8. **Memory** - Memory yang dialokasikan dalam MB (mis., "512 MB")
9. **Owner** - Siapa yang membuat function:
   - **"You"** (hijau) - Function Anda
   - **"Other User"** - Function pengguna lain
   - **"System"** - Function yang dibuat sistem
10. **Actions** - Tombol aksi (Invoke, Logs, Delete)

### Paginasi

Jika Anda memiliki lebih dari 10 function:

![Image: Pagination controls](/images/functions/pagination.png)

- **10 function per halaman**
- Klik nomor halaman untuk navigasi
- Gunakan panah Previous/Next

---

## Invoke Function

### Invoke dari Daftar Functions

Untuk menguji function, klik tombol **Invoke** (ikon ▶ Play):

![Image: Invoke button on function row](/images/functions/invoke-button.png)

**Langkah**:
1. Temukan function Anda di tabel
2. Di kolom **Actions**, klik tombol **Invoke**
3. Dialog invoke terbuka

---

### Dialog Invoke

Dialog invoke memungkinkan Anda menguji function dengan input kustom:

![Image: Invoke function dialog](/images/functions/invoke-dialog.png)

#### 1. Masukkan Payload JSON

Tulis atau tempel payload JSON di editor Monaco:

![Image: JSON payload editor in invoke dialog](/images/functions/invoke-payload-editor.png)

**Contoh payload**:
```json
{
  "name": "Alice",
  "age": 30
}
```

```json
{
  "operation": "add",
  "a": 10,
  "b": 5
}
```

#### 2. Validasi JSON

Editor memvalidasi JSON secara real-time:

![Image: JSON valid indicator](/images/functions/json-valid.png)

✅ **"JSON valid"** (hijau) - Siap untuk di-invoke

![Image: JSON invalid error](/images/functions/json-invalid.png)

❌ **"Error: Unexpected token..."** (merah) - Perbaiki JSON terlebih dahulu

**Tips**: Klik tombol **Format JSON** untuk auto-format.

#### 3. Klik Invoke

Klik tombol **Invoke** untuk mengeksekusi:

![Image: Invoke button in dialog](/images/functions/invoke-button-dialog.png)

Function akan mengeksekusi dan menampilkan respons:

![Image: Function response in dialog](/images/functions/invoke-response.png)

#### 4. Lihat Respons

Panel respons menampilkan:

**Body respons**:
```json
{
  "statusCode": 200,
  "headers": {
    "content-type": "application/json"
  },
  "body": "{\"result\": 15}"
}
```

**Aksi respons**:
- **Copy** - Salin respons ke clipboard
- **Clear** - Bersihkan panel respons

### Tutup Dialog

Klik **Cancel** untuk menutup dialog invoke.

**Catatan**: Payload disimpan per-function, sehingga membuka kembali akan menampilkan input terakhir Anda.

---

## Melihat Log Function

Untuk debug atau monitor eksekusi function:

### Akses Log

Klik tombol **Logs** (ikon 📄 FileText) di kolom Actions:

![Image: Logs button on function row](/images/functions/logs-button.png)

Ini membuka halaman Detail Function pada tab Logs.

Lihat [Lihat Log](logs/) untuk informasi logging yang detail.

---

## Memperbarui Functions

Untuk mengubah function yang sudah ada:

### 1. Buka Halaman Detail Function

Klik **nama function** di tabel:

![Image: Function name link](/images/functions/function-name-link.png)

### 2. Edit Function

Editor function terbuka dengan kode saat ini:

![Image: Function editor in update mode](/images/functions/function-editor-update.png)

### 3. Buat Perubahan

Anda dapat memperbarui:
- ✅ **Name** - Ubah nama function
- ✅ **Code** - Edit kode function
- ✅ **Handler** - Ubah entry point
- ✅ **Timeout** - Sesuaikan detik timeout
- ✅ **Memory** - Ubah alokasi memory

**Tidak dapat diubah**:
- ❌ **Runtime** - Tidak dapat diubah setelah pembuatan (hapus dan buat ulang)
- ❌ **vCPU** - Tetap saat pembuatan

### 4. Tes Perubahan

Gunakan bagian **Run Test** untuk menguji perubahan Anda:

![Image: Test section in update mode](/images/functions/test-update.png)

### 5. Simpan Pembaruan

Klik **Save** untuk men-deploy perubahan:

![Image: Save button highlighted](/images/functions/save-update-button.png)

Function akan di-deploy ulang dengan kode baru.

**Catatan**: Function mungkin tidak tersedia sebentar selama pembaruan (2-5 detik).

---

## Menghapus Functions

**⚠️ Peringatan**: Penghapusan bersifat **permanen** dan **tidak dapat dibatalkan**!

### Hapus Function

Dari halaman daftar Functions:

![Image: Delete button on function row](/images/functions/delete-button.png)

**Langkah**:
1. Di kolom **Actions**, klik tombol **Delete** (ikon 🗑️ Trash)
2. Dialog konfirmasi akan muncul:

![Image: Delete confirmation dialog](/images/functions/delete-confirm.png)

3. Klik **Delete** untuk konfirmasi

Function akan dihapus secara permanen.

### Yang Dihapus

Saat Anda menghapus function:

- ✅ **Kode function** - Semua kode dihapus
- ✅ **Konfigurasi** - Pengaturan dihapus
- ✅ **microVM** - VM dihancurkan dan resource dibebaskan
- ✅ **Log** - Log eksekusi dihapus
- ✅ **Riwayat invokasi** - Statistik dibersihkan

**Waktu**: Biasanya instan (< 1 detik)

### Notifikasi Berhasil

Setelah penghapusan, Anda akan melihat pesan berhasil:

![Image: Function deleted success notification](/images/functions/delete-success.png)

**"Function Deleted - [function-name] has been deleted successfully"**

Function akan dihapus dari daftar Functions.

---

## Halaman Detail Function

Klik nama function untuk membuka halaman detail:

![Image: Function detail page overview](/images/functions/function-detail-page.png)

### Tab Overview

Menampilkan informasi function:

![Image: Function overview tab](/images/functions/function-overview-tab.png)

**Informasi yang ditampilkan**:
- Nama dan ID function
- Runtime dan badge status
- vCPU, Memory, Timeout
- Alamat Guest IP
- Tanggal dibuat
- Waktu terakhir di-invoke
- Jumlah invokasi (24j, 7h, 30h)

**Aksi yang tersedia**:
- **Invoke** - Tes function
- **Edit** - Ubah function
- **Delete** - Hapus function

---

### Tab Logs

Lihat log eksekusi dan error:

![Image: Function logs tab](/images/functions/function-logs-tab.png)

Lihat [Lihat Log](logs/) untuk detailnya.

---

### Tab Code (jika tersedia)

Lihat kode function saat ini:

![Image: Function code tab](/images/functions/function-code-tab.png)

**Fitur**:
- Penampil kode baca-saja
- Syntax highlighting
- Salin kode ke clipboard

**Untuk mengedit**: Klik tombol **Edit** atau beralih ke mode edit.

---

## Refresh Daftar Function

Untuk memperbarui daftar function dengan data terbaru:

Klik tombol **Refresh** di header halaman Functions:

![Image: Refresh button](/images/functions/refresh-button.png)

**Kapan perlu di-refresh**:
- Setelah membuat function (untuk melihat status baru)
- Jika status tampak usang
- Untuk memperbarui jumlah invokasi
- Setelah pengguna lain membuat perubahan

**Catatan**: Daftar auto-refresh secara berkala, tetapi Anda dapat refresh manual untuk pembaruan instan.

---

## Monitor Functions

### Metrik Invokasi

Lacak seberapa sering function Anda dipanggil:

![Image: Invocation count columns](/images/functions/invocation-metrics.png)

**Metrik yang ditampilkan**:
- **Last Invoked** - Kapan function terakhir dipanggil
- **24h Invocations** - Panggilan dalam 24 jam terakhir
- **7d Invocations** - Panggilan dalam 7 hari terakhir (jika ditampilkan)
- **30d Invocations** - Panggilan dalam 30 hari terakhir (jika ditampilkan)

**Kasus penggunaan**:
- Identifikasi function yang populer
- Deteksi function yang tidak digunakan (untuk pembersihan)
- Monitor pola traffic
- Lacak adopsi

---

### Kesehatan Function

Monitor status dan error function:

**Function yang sehat**:
- ✅ Status: **Ready** (hijau)
- ✅ Invokasi berhasil
- ✅ Tidak ada error di log

**Function yang tidak sehat**:
- ❌ Status: **Error** (merah)
- ❌ Invokasi gagal
- ❌ Error di log

**Aksi untuk function yang tidak sehat**:
1. Klik **Logs** untuk melihat pesan error
2. Identifikasi masalah (syntax error, timeout, dll.)
3. Klik nama function untuk **Edit**
4. Perbaiki masalah
5. **Save** untuk re-deploy
6. **Invoke** untuk tes

---

## Pemecahan Masalah

### Masalah: Tidak Dapat Invoke Function

**Gejala**:
- Tombol Invoke tidak berfungsi
- Pesan error muncul
- Error timeout

**Solusi**:

1. **Cek status function**:
   - Status harus **"Ready"** untuk di-invoke
   - Jika "Creating" atau "Deploying", tunggu hingga selesai
   - Jika "Error", cek log dan perbaiki masalah

2. **Cek payload JSON**:
   - Harus berupa JSON valid
   - Cek syntax error
   - Klik "Format JSON" untuk verifikasi

3. **Cek jaringan**:
   - Pastikan microVM function online
   - Cek **Guest IP** sudah ditetapkan
   - Buka halaman **Hosts**, pastikan agent online

4. **Coba lagi**:
   - Refresh halaman
   - Coba invoke lagi

---

### Masalah: Function Tertahan di "Deploying"

**Gejala**:
- Status menampilkan "Deploying" lebih dari 1 menit
- Tidak pernah berubah menjadi "Ready"

**Solusi**:

1. **Tunggu lebih lama** - Deploy awal dapat membutuhkan hingga 2 menit
2. **Refresh halaman** - Status mungkin usang
3. **Cek log**:
   - Klik tombol **Logs**
   - Cari error deployment
4. **Cek resource**:
   - Buka halaman **Hosts**
   - Pastikan CPU/memory yang cukup tersedia
5. **Hapus dan buat ulang**:
   - Jika tertahan >5 menit, hapus function
   - Buat yang baru dengan kode yang sama

---

### Masalah: Invokasi Mengembalikan Error

**Gejala**:
- Invoke berhasil tetapi mengembalikan error dalam respons
- `statusCode: 500` atau kode error lainnya

**Solusi**:

1. **Cek body respons**:
   ```json
   {
     "statusCode": 500,
     "body": "{\"error\": \"...\"}"
   }
   ```

2. **Lihat log function**:
   - Klik tombol **Logs**
   - Cari pesan error
   - Identifikasi masalah (mis., variabel tidak terdefinisi)

3. **Masalah umum**:
   - **Field event yang hilang**: Periksa payload memiliki key yang diperlukan
   - **Type error**: Pastikan tipe data sesuai harapan
   - **Timeout**: Tingkatkan timeout atau optimalkan kode
   - **Syntax error**: Tinjau kode untuk typo

4. **Perbaiki dan tes**:
   - Edit function
   - Perbaiki masalah
   - Simpan dan invoke lagi

---

### Masalah: Tidak Dapat Menghapus Function

**Gejala**:
- Tombol Delete tidak merespons
- Pesan error muncul

**Solusi**:

1. **Cek kepemilikan**:
   - Anda hanya dapat menghapus function yang Anda buat
   - Atau jika Anda adalah admin

2. **Coba dari halaman detail**:
   - Klik nama function
   - Coba hapus dari halaman detail

3. **Refresh dan coba lagi**:
   - Refresh halaman
   - Coba delete lagi

4. **Hubungi administrator**:
   - Jika masalah berlanjut

---

## Praktik Terbaik

### Manajemen Function

✅ **Atur function secara logis**:
- Gunakan nama yang deskriptif
- Kelompokkan function terkait dengan prefix
  - `auth-login`, `auth-logout`, `auth-verify`
  - `payment-create`, `payment-verify`, `payment-refund`
- Tambahkan komentar di kode yang menjelaskan tujuan

---

### Monitoring dan Pemeliharaan

✅ **Monitor secara berkala**:
- Cek jumlah invokasi setiap minggu
- Tinjau log error secara teratur
- Hapus function yang tidak digunakan
- Perbarui kode function sesuai kebutuhan

✅ **Bersihkan function yang tidak digunakan**:
- Filter berdasarkan "Last Invoked"
- Hapus function yang tidak digunakan lebih dari 30 hari
- Jaga codebase tetap bersih

---

### Pengujian Sebelum Deploy

✅ **Selalu tes sebelum menyimpan**:
- Gunakan **Run Test** di editor
- Tes dengan beberapa payload
- Verifikasi penanganan error
- Cek edge case

✅ **Tes setelah pembaruan**:
- Setelah menyimpan perubahan, invoke dari daftar
- Verifikasi respons sudah benar
- Cek log untuk error

---

### Optimasi Resource

✅ **Sesuaikan ukuran function**:
- Mulai dengan resource minimum
- Monitor waktu invokasi di log
- Tingkatkan CPU/memory hanya jika diperlukan
- Over-alokasi = biaya lebih tinggi

✅ **Optimalkan timeout**:
- Tetapkan nilai timeout yang realistis
- Terlalu singkat = kegagalan yang tidak perlu
- Terlalu panjang = pemborosan resource pada error

---

## Referensi Cepat

### Ringkasan Aksi Function

| Aksi | Tombol | Deskripsi |
|------|--------|-----------|
| **Invoke** | ▶ Play | Tes function dengan payload kustom |
| **Logs** | 📄 FileText | Lihat log eksekusi dan error |
| **Delete** | 🗑️ Trash | Hapus function secara permanen |
| **Edit** | Klik nama | Buka editor untuk mengubah function |

### Alur Kerja Umum

**Monitoring harian**:
1. Buka halaman Functions
2. Cek status - semua harus "Ready"
3. Tinjau jumlah invokasi
4. Cek log untuk error

**Memperbarui function**:
1. Klik nama function
2. Edit kode
3. Jalankan Test untuk verifikasi
4. Klik Save
5. Invoke dari daftar untuk konfirmasi

**Debug error**:
1. Klik tombol Logs
2. Identifikasi error di log
3. Klik nama function untuk Edit
4. Perbaiki masalah
5. Simpan dan tes

---

## Langkah Selanjutnya

Setelah Anda tahu cara mengelola function:

- **[Lihat Log](logs/)** - Pelajari cara debug dengan log
- **[Playground](playground/)** - Bereksperimen dengan ide baru
- **[Buat Function](create-function/)** - Tinjau langkah pembuatan

---

## Tips Performa

### Optimasi Cold Start

**Cold start** terjadi saat invokasi pertama atau setelah periode idle:

✅ **Minimalkan cold start**:
- Jaga function tetap warm dengan invokasi berkala
- Gunakan cron job untuk invoke setiap 5 menit
- Minimalkan ukuran kode (waktu muat lebih cepat)
- Hindari dependensi yang berat

---

### Optimasi Invokasi Warm

**Invokasi warm** menggunakan kembali microVM yang ada:

✅ **Optimalkan performa warm**:
- Cache operasi yang mahal (koneksi DB, klien API)
- Inisialisasi sekali di scope global
- Gunakan kembali antar invokasi
- Minimalkan overhead per-invokasi

**Contoh** (Python):
```python
# ✅ Baik - Inisialisasi sekali
import requests
session = requests.Session()  # Scope global

def handler(event):
    # Gunakan kembali session antar invokasi
    response = session.get(event["url"])
    return {"statusCode": 200, "body": response.text}

# ❌ Buruk - Inisialisasi setiap kali
def handler(event):
    import requests
    session = requests.Session()  # Dibuat setiap invokasi!
    response = session.get(event["url"])
    return {"statusCode": 200, "body": response.text}
```
