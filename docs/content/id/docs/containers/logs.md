+++
title = "Lihat Log"
description = "Streaming log container dan debugging secara real-time"
weight = 33
date = 2025-12-18
+++

Monitor output container, debug error, dan lacak perilaku aplikasi dengan streaming log secara real-time.

---

## Apa itu Log Container?

Log container menangkap semua output dari Docker container Anda:

**Yang ditangkap**:
- ✅ **stdout** — Output standar (`console.log`, `print`, `echo`)
- ✅ **stderr** — Output error (exception, peringatan)
- ✅ **Docker daemon** — Event siklus hidup container
- ✅ **Log aplikasi** — Output log aplikasi Anda

**Yang tidak ditangkap**:
- ❌ Log yang ditulis ke file di dalam container (kecuali di-mount ke volume)
- ❌ Log historis sebelum streaming dimulai

---

## Mengakses Log Container

1. Buka halaman detail container
2. Klik tab **Logs**

---

## Streaming Log

Log tidak mengalir secara otomatis — Anda harus memulainya secara manual.

Klik **Start Stream** untuk membuka koneksi WebSocket. Tombol berubah menjadi **Stop Stream** setelah terhubung dan log mulai muncul secara real-time.

Aktifkan **Auto-scroll** untuk selalu menampilkan log terbaru saat datang. Gulir ke atas secara manual untuk menjeda auto-scroll dan membaca entri sebelumnya.

Klik **Download** untuk menyimpan semua log yang terlihat saat ini ke file `.txt` — berguna untuk analisis offline, berbagi dengan rekan tim, atau filter menggunakan editor lokal.

---

## Format Entri Log

Setiap entri mengikuti format berikut:

```
[YYYY-MM-DD HH:MM:SS.mmm] [stream] message
```

**Contoh**:
```
[2025-12-18 14:30:45.123] [stdout] Starting Nginx 1.25.3
[2025-12-18 14:30:45.234] [stdout] Listening on port 80
[2025-12-18 14:31:22.456] [stderr] Error: Database connection failed
[2025-12-18 14:31:22.567] [stderr]   at connect (db.js:45:10)
```

- **stdout** — teks normal
- **stderr** — ditampilkan dalam warna merah

Timestamp ditampilkan dalam timezone browser lokal Anda.

---

## Pola Log Umum

### Nginx
```
[stdout] 2025/12/18 14:30:45 [notice] nginx/1.25.3
[stdout] 2025/12/18 14:30:45 [notice] start worker process 29
[stdout] 172.16.0.1 - - [18/Dec/2025:14:32:15 +0000] "GET / HTTP/1.1" 200 615
```

### PostgreSQL
```
[stdout] 2025-12-18 14:30:50 UTC [1] LOG:  starting PostgreSQL 15.3
[stdout] 2025-12-18 14:30:50 UTC [1] LOG:  listening on IPv4 address "0.0.0.0", port 5432
[stdout] 2025-12-18 14:30:50 UTC [1] LOG:  database system is ready to accept connections
```

### Node.js
```
[stdout] Connecting to database...
[stdout] Database connected successfully
[stdout] Listening on port 3000
[stdout] GET /api/users 200 45ms
[stderr] Warning: Deprecated API usage
```

### Error dengan stack trace
```
[stderr] Error: Connection timeout
[stderr]   at Timeout._onTimeout (/app/lib/db.js:123:15)
[stderr]   at listOnTimeout (node:internal/timers:559:17)
[stderr] Application shutting down due to fatal error
```

---

## Debugging dengan Log

### Container dimulai lalu langsung berhenti

1. Buka tab Logs dan klik **Start Stream** sebelum me-restart container
2. Cari entri stderr merah di bagian atas
3. Error startup yang umum:
   ```
   [stderr] Error: POSTGRES_PASSWORD must be set
   [stderr] Error: bind EADDRINUSE 0.0.0.0:3000
   [stderr] Error: Cannot find module 'express'
   ```
4. Perbaiki konfigurasi dan restart

### Menemukan error tertentu di masa lalu

- Klik **Download**, lalu buka file di editor teks dan gunakan `Ctrl+F`
- Atau gunakan pencarian browser (`Ctrl+F`) langsung di penampil log

### Aplikasi lambat

Bandingkan timestamp antar baris log yang terkait untuk menemukan penundaan:

```
[stdout] [14:30:00.000] Database query started
[stdout] [14:30:05.456] Database query completed   ← 5.4 detik!
```

Tambahkan log timing untuk mempersempit masalah:
```javascript
const start = Date.now();
const result = await db.query(...);
console.log(`[PERF] Query completed in ${Date.now() - start}ms`);
```

---

## Pemecahan Masalah

### Streaming tetapi tidak ada log yang muncul

- Aplikasi mungkin menulis log ke file alih-alih stdout — konfigurasi untuk menggunakan stdout/stderr
- Container mungkin baru saja dimulai — tunggu sebentar dan picu beberapa aktivitas
- Level log mungkin terlalu tinggi — set ke `INFO` atau `DEBUG`

### Stream terputus secara tak terduga

1. Klik **Start Stream** lagi untuk menyambung kembali
2. Pastikan container masih dalam status **Running**
3. Periksa gangguan jaringan — refresh halaman dan restart stream
4. Buka DevTools browser → Network → WS untuk melihat error WebSocket

### Terlalu banyak log yang bergulir terlalu cepat

- Nonaktifkan **Auto-scroll** dan gulir ke bagian yang Anda butuhkan
- Klik **Stop Stream**, baca yang ada, lalu lanjutkan
- Unduh log dan filter dengan editor Anda

---

## Praktik Terbaik

**Log ke stdout/stderr** — bukan ke file:
```javascript
// Baik
console.log("User logged in:", userId);
console.error("Login failed:", error);

// Buruk - tidak akan muncul di tab logs
fs.appendFileSync('/var/log/app.log', message);
```

**Sertakan konteks** dalam pesan log:
```javascript
console.error("Payment failed:", { userId, amount, error: err.message });
```

**Gunakan level log** untuk mengontrol verbositas:
```javascript
if (process.env.LOG_LEVEL === 'debug') {
  console.log('Debug: detailed info');
}
```

**Hentikan streaming** setelah selesai debugging untuk mengurangi bandwidth dan penggunaan memory browser.

---

## Referensi Cepat

| Kontrol | Aksi |
|---------|------|
| **Start Stream** | Mulai streaming log via WebSocket |
| **Stop Stream** | Tutup koneksi WebSocket |
| **Auto-scroll** | Aktifkan/nonaktifkan scroll otomatis ke bawah |
| **Download** | Simpan log yang terlihat ke file teks |

---

## Langkah Selanjutnya

- **[Monitor Statistik](stats/)** — Penggunaan resource dan metrik performa
- **[Kelola Containers](manage-containers/)** — Operasi start, stop, restart
- **[Deploy Container](deploy-container/)** — Buat container baru
