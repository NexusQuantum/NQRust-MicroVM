+++
title = "Serverless Functions"
description = "Deploy dan kelola serverless function dengan auto-scaling"
weight = 40
date = 2025-12-18
+++

Deploy serverless function mirip Lambda dengan isolasi kuat, auto-scaling, dan model eksekusi pay-per-use.

---

## Apa itu Serverless Functions?

Serverless Functions di NQRust-MicroVM adalah **unit komputasi berbasis event yang ringan** yang menjalankan kode Anda sebagai respons terhadap permintaan HTTP. Setiap function berjalan dalam **Firecracker microVM-nya sendiri yang terisolasi**, memberikan keamanan dan isolasi resource yang kuat.

![Image: Function architecture diagram](/images/functions/function-architecture.png)

**Karakteristik Utama**:
- **Eksekusi Terisolasi** - Setiap function berjalan di microVM khusus
- **Berbasis Event** - Dipicu oleh permintaan HTTP dengan payload JSON
- **Auto-scaling** - Function melakukan scaling otomatis berdasarkan permintaan
- **Pay-Per-Use** - Hanya bayar untuk waktu eksekusi aktual
- **Berbagai Runtime** - Dukungan untuk Python, JavaScript (Bun), dan TypeScript (Bun)

---

## Cara Kerja Functions

![Image: Function execution flow](/images/functions/function-flow.png)

1. **Client mengirim permintaan HTTP** dengan payload JSON
2. **Manager merutekan permintaan** ke microVM function
3. **Function mengeksekusi** di lingkungan terisolasi
4. **Respons dikembalikan** ke client
5. **Resource dibebaskan** setelah eksekusi

**Model Eksekusi**:
- Function mengalami **cold-start** saat invokasi pertama (VM dimulai)
- **Warm instance** digunakan kembali untuk permintaan berikutnya (lebih cepat)
- **Shutdown otomatis** setelah idle timeout
- **Eksekusi konkuren** menghasilkan beberapa instans VM

---

## Runtime yang Didukung

### Python 3.11

![Image: Python runtime badge](/images/functions/runtime-python.png)

**Terbaik untuk**:
- Pemrosesan dan analisis data
- Inferensi machine learning
- Integrasi API
- Komputasi ilmiah

**Contoh**:
```python
def handler(event):
    name = event.get("name", "World")
    return {
        "statusCode": 200,
        "headers": {"content-type": "application/json"},
        "body": f'{{"message": "Hello, {name}!"}}',
    }
```

---

### JavaScript (Bun)

![Image: JavaScript runtime badge](/images/functions/runtime-javascript.png)

**Terbaik untuk**:
- Web API dan microservices
- Pemrosesan data real-time
- Transformasi JSON
- Prototyping cepat

**Contoh**:
```javascript
export async function handler(event) {
  const name = event?.name || "World";
  return {
    statusCode: 200,
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ message: `Hello, ${name}!` }),
  };
}
```

---

### TypeScript (Bun)

![Image: TypeScript runtime badge](/images/functions/runtime-typescript.png)

**Terbaik untuk**:
- API yang type-safe
- Aplikasi enterprise
- Logika bisnis yang kompleks
- Codebase yang besar

**Contoh**:
```typescript
interface Event {
  name?: string;
}

export async function handler(event: Event) {
  const name = event?.name || "World";
  return {
    statusCode: 200,
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ message: `Hello, ${name}!` }),
  };
}
```

---

## Siklus Hidup Function

### Status

Function melewati beberapa status:

![Image: Function state diagram](/images/functions/function-states.png)

| Status | Deskripsi | Durasi |
|--------|-----------|--------|
| **Creating** | Function sedang dibuat | 1-2 detik |
| **Deploying** | microVM sedang disiapkan | 2-5 detik |
| **Ready** | Function siap menerima permintaan | - |
| **Error** | Function gagal deploy atau crash | - |

**Status Invokasi** (selama eksekusi):
- **Cold Start** - Invokasi pertama, VM perlu boot (~2-3 detik)
- **Warm** - Invokasi berikutnya, VM sudah berjalan (~50-200ms)
- **Executing** - Kode function sedang berjalan
- **Complete** - Eksekusi selesai, respons dikirim

---

## Kasus Penggunaan

### 1. Endpoint API

Buat HTTP API yang ringan tanpa mengelola server:

**Kasus penggunaan**:
- Endpoint REST API
- Webhook untuk integrasi pihak ketiga
- Pemrosesan formulir
- Endpoint autentikasi

**Contoh**: Proses pengiriman formulir dan kirim notifikasi email

---

### 2. Pemrosesan Data

Proses data sesuai permintaan tanpa infrastruktur khusus:

**Kasus penggunaan**:
- Resize dan optimasi gambar
- Transformasi data CSV/JSON
- Pembuatan laporan
- Pemicu pemrosesan batch

**Contoh**: Resize gambar yang diunggah ke berbagai ukuran

---

### 3. Tugas Terjadwal

Jalankan tugas periodik tanpa cron job:

**Kasus penggunaan**:
- Pembersihan database
- Pembuatan laporan
- Sinkronisasi data
- Health check

**Contoh**: Buat laporan analitik harian

---

### 4. Event Handler

Respons terhadap event dari sistem lain:

**Kasus penggunaan**:
- Handler notifikasi
- Pemroses log audit
- Analitik real-time
- Sistem alert

**Contoh**: Kirim notifikasi Slack saat ambang batas error terlampaui

---

## Keuntungan

### Isolasi Kuat

✅ **Setiap function berjalan di microVM-nya sendiri**
- Isolasi tingkat kernel penuh via Firecracker
- Tidak ada resource yang dibagikan antar function
- Perlindungan dari noisy neighbor
- Multi-tenancy yang aman

### Cold Start Cepat

⚡ **Waktu boot sub-detik**
- Firecracker microVM boot dalam ~150ms
- Total cold start: ~2-3 detik (termasuk inisialisasi runtime)
- Invokasi warm: ~50-200ms
- Lebih cepat dari VM tradisional, sebanding dengan container

### Hemat Biaya

💰 **Bayar hanya untuk yang Anda gunakan**
- Tidak ada biaya untuk waktu idle
- Resource dibebaskan secara otomatis setelah eksekusi
- Utilisasi resource yang efisien
- Overhead lebih rendah dari VM yang selalu menyala

### Pengembangan Mudah

🚀 **Alur kerja pengembangan sederhana**
- Tulis kode di web editor (Monaco)
- Tes langsung di Playground
- Lihat log real-time
- Tidak perlu pipeline deployment yang kompleks

### Auto-scaling

📈 **Scaling sesuai permintaan**
- Nyalakan instans sesuai kebutuhan
- Tangani lonjakan traffic secara otomatis
- Tidak perlu perencanaan kapasitas manual
- Setiap invokasi dapat berjalan secara paralel

---

## Arsitektur

### Model Function-per-VM

Berbeda dengan platform serverless tradisional yang berbagi VM, NQRust-MicroVM menggunakan **satu microVM per instans function**:

![Image: Function-per-VM architecture](/images/functions/function-per-vm.png)

**Serverless Tradisional** (gaya AWS Lambda):
```
┌─────────────────────────────┐
│      Shared Container       │
│  ┌─────┐ ┌─────┐ ┌─────┐  │
│  │ Fn1 │ │ Fn2 │ │ Fn3 │  │ ← Isolasi lebih lemah
│  └─────┘ └─────┘ └─────┘  │
└─────────────────────────────┘
```

**NQRust-MicroVM** (Firecracker):
```
┌──────────┐ ┌──────────┐ ┌──────────┐
│ VM 1     │ │ VM 2     │ │ VM 3     │
│  ┌─────┐ │ │  ┌─────┐ │ │  ┌─────┐ │
│  │ Fn1 │ │ │  │ Fn2 │ │ │  │ Fn3 │ │ ← Isolasi tingkat kernel
│  └─────┘ │ │  └─────┘ │ │  └─────┘ │
└──────────┘ └──────────┘ └──────────┘
```

**Keunggulan**:
- ✅ **Keamanan lebih kuat** - Isolasi tingkat kernel
- ✅ **Tidak ada noisy neighbor** - Resource dedicated
- ✅ **Isolasi crash** - Satu function crash tidak mempengaruhi yang lain
- ✅ **Jaminan resource** - Performa yang dapat diprediksi

---

## Memulai

Siap membuat function pertama Anda? Ikuti panduan berikut:

1. **[Buat Function](create-function/)** - Panduan langkah demi langkah untuk membuat function
2. **[Kelola Functions](manage-functions/)** - Invoke, perbarui, dan hapus function
3. **[Playground](playground/)** - Bereksperimen dengan function tanpa membuatnya
4. **[Lihat Log](logs/)** - Debug dan monitor eksekusi function

---

## Contoh Cepat

Berikut contoh lengkap function kalkulator sederhana:

**Nama Function**: `calculator`
**Runtime**: Python
**Handler**: `handler`

**Kode**:
```python
def handler(event):
    operation = event.get("operation")
    a = float(event.get("a", 0))
    b = float(event.get("b", 0))

    if operation == "add":
        result = a + b
    elif operation == "subtract":
        result = a - b
    elif operation == "multiply":
        result = a * b
    elif operation == "divide":
        result = a / b if b != 0 else "Error: Division by zero"
    else:
        return {
            "statusCode": 400,
            "body": '{"error": "Invalid operation"}',
        }

    return {
        "statusCode": 200,
        "headers": {"content-type": "application/json"},
        "body": f'{{"result": {result}}}',
    }
```

**Invoke** dengan payload:
```json
{
  "operation": "add",
  "a": 10,
  "b": 5
}
```

**Respons**:
```json
{
  "result": 15
}
```

**Selanjutnya**: Pelajari cara [membuat function pertama Anda](create-function/).

---

## Perbandingan dengan VM

| Fitur | Functions | VM |
|-------|-----------|-----|
| **Waktu Startup** | ~2-3 detik (cold), ~50ms (warm) | ~1-2 detik |
| **Isolasi** | microVM per instans function | microVM per VM |
| **Scaling** | Otomatis, per invokasi | Manual |
| **Penagihan** | Per eksekusi | Per jam/selalu menyala |
| **State** | Stateless (ephemeral) | Stateful (persisten) |
| **Terbaik Untuk** | Event-driven, tugas singkat | Layanan yang berjalan lama |
| **Deployment Kode** | Editor bawaan + deploy | Manual (SSH, salin file) |

**Kapan menggunakan Functions**:
- ✅ Beban kerja berbasis event
- ✅ HTTP API dan webhook
- ✅ Tugas singkat (<5 menit)
- ✅ Pola traffic yang tidak dapat diprediksi
- ✅ Prototyping cepat

**Kapan menggunakan VM**:
- ✅ Layanan yang berjalan lama
- ✅ Aplikasi stateful
- ✅ Dependensi yang kompleks
- ✅ Kontrol OS penuh diperlukan
- ✅ Koneksi persisten (database, WebSocket)

---

## Keterbatasan

**Waktu Eksekusi**:
- Timeout maksimum: 300 detik (5 menit)
- Timeout default: 30 detik
- Dapat dikonfigurasi per function

**Resource**:
- vCPU: 1-32 core
- Memory: 128 MB - 3072 MB (3 GB)
- Tidak ada penyimpanan persisten (filesystem ephemeral)

**Jaringan**:
- Akses jaringan keluar tersedia
- Masuk: Hanya permintaan HTTP
- Tidak ada akses SSH langsung (gunakan log untuk debugging)

**Cold Start**:
- Invokasi pertama membutuhkan ~2-3 detik
- Jaga function tetap warm dengan invokasi berkala

---

## Praktik Terbaik

✅ **Jaga function kecil dan terfokus**
- Satu function = satu tanggung jawab
- Pecah logika kompleks menjadi beberapa function
- Cold start lebih cepat dengan kode yang lebih kecil

✅ **Tangani error dengan baik**
- Kembalikan HTTP status code yang tepat
- Sertakan pesan error dalam respons
- Log error untuk debugging

✅ **Optimalkan untuk warm start**
- Gunakan kembali resource yang mahal (koneksi DB)
- Inisialisasi sekali, gunakan kembali antar invokasi
- Jaga scope global seminimal mungkin

✅ **Gunakan timeout yang sesuai**
- Tetapkan nilai timeout yang realistis
- Jangan over-alokasi (biaya lebih tinggi)
- Monitor waktu eksekusi aktual

✅ **Tes di Playground terlebih dahulu**
- Bereksperimen sebelum membuat function
- Validasi payload dan respons
- Iterasi cepat tanpa deployment

---

## Langkah Selanjutnya

- **[Buat Function](create-function/)** - Bangun function pertama Anda
- **[Playground](playground/)** - Coba playground interaktif
- **[Kelola Functions](manage-functions/)** - Pelajari operasi function
