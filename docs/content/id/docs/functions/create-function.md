+++
title = "Buat Function"
description = "Panduan langkah demi langkah untuk membuat serverless function"
weight = 41
date = 2025-12-18
+++

Pelajari cara membuat serverless function menggunakan antarmuka web dengan editor kode bawaan dan kemampuan pengujian.

---

## Prasyarat

Sebelum membuat function, pastikan:

- ✅ Anda memiliki akses ke dashboard NQRust-MicroVM
- ✅ Setidaknya satu host/agent online
- ✅ Resource yang cukup tersedia (minimum: 1 vCPU, 512 MB RAM per function)

---

## Langkah 1: Buka Halaman Pembuatan Function

1. Klik **Functions** di sidebar kiri
2. Klik tombol **New Function** di sudut kanan atas

![Image: New Function button highlighted](/images/functions/new-function-button.png)

Editor function akan terbuka dengan editor kode Monaco.

---

## Langkah 2: Konfigurasi Dasar

### Nama Function (Diperlukan)

Masukkan nama unik dan deskriptif untuk function Anda:

![Image: Function name input field](/images/functions/function-name-input.png)

- Harus antara 1-50 karakter
- Gunakan nama deskriptif yang menunjukkan tujuan
- Contoh: `image-resizer`, `email-sender`, `data-processor`

**Tips**: Gunakan kebab-case untuk nama function (mis., `process-payment`, `send-notification`)

---

### Runtime (Diperlukan)

Pilih bahasa pemrograman untuk function Anda:

![Image: Runtime dropdown selection](/images/functions/runtime-dropdown.png)

**Runtime yang tersedia**:

| Runtime | Versi | Terbaik Untuk |
|---------|-------|--------------|
| **Python** | 3.11 | Pemrosesan data, ML, API |
| **JavaScript (Bun)** | Terbaru | Web API, pemrosesan JSON |
| **TypeScript (Bun)** | Terbaru | Aplikasi yang type-safe |

**Default**: TypeScript

**Catatan performa**: Semua runtime memiliki waktu cold start yang serupa (~2-3 detik)

---

### Nama Handler (Diperlukan)

Tentukan nama function entry point:

![Image: Handler name input](/images/functions/handler-input.png)

- **Default**: `handler`
- Harus sesuai dengan nama function dalam kode Anda
- Nama umum: `handler`, `main`, `lambda_handler`

**Contoh**:
```python
# Jika handler = "handler"
def handler(event):
    return {"statusCode": 200}
```

```typescript
// Jika handler = "handler"
export async function handler(event) {
  return { statusCode: 200 };
}
```

---

## Langkah 3: Tulis Kode Function

Gunakan editor kode Monaco bawaan untuk menulis function Anda:

![Image: Monaco code editor with function code](/images/functions/code-editor.png)

Editor menyediakan:
- ✅ Syntax highlighting
- ✅ Auto-completion
- ✅ Deteksi error
- ✅ Format kode
- ✅ Pengeditan multi-baris

### Template Kode Default

Saat Anda memilih runtime, kode default disediakan:

#### Template Python

```python
# index.py  (Python 3.11)
def handler(event):
    try:
        a = float(event.get("key1"))
        b = float(event.get("key2"))
    except Exception:
        return {
            "statusCode": 400,
            "headers": {"content-type": "application/json"},
            "body": '{"error":"key1 and key2 must be numbers"}',
        }

    return {
        "statusCode": 200,
        "headers": {"content-type": "application/json"},
        "body": '{"result": %s}' % (a + b),
    }
```

#### Template JavaScript

```javascript
// index.js (JavaScript)
export async function handler(event) {
  const a = Number(event?.key1);
  const b = Number(event?.key2);

  if (!Number.isFinite(a) || !Number.isFinite(b)) {
    return {
      statusCode: 400,
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ error: "key1 and key2 must be numbers" }),
    };
  }

  return {
    statusCode: 200,
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ result: a + b }),
  };
}
```

#### Template TypeScript

```typescript
// index.ts (TypeScript)
interface Event {
  key1?: number | string;
  key2?: number | string;
}

export async function handler(event: Event) {
  const a = Number(event?.key1);
  const b = Number(event?.key2);

  if (!Number.isFinite(a) || !Number.isFinite(b)) {
    return {
      statusCode: 400,
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ error: "key1 and key2 must be numbers" }),
    };
  }

  return {
    statusCode: 200,
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ result: a + b }),
  };
}
```

### Tanda Tangan Function

Semua function menerima parameter `event` dengan payload permintaan:

**Python**:
```python
def handler(event):
    # event adalah dict dengan payload JSON
    name = event.get("name")
    return {"statusCode": 200, "body": "..."}
```

**JavaScript/TypeScript**:
```typescript
export async function handler(event) {
  // event adalah objek dengan payload JSON
  const name = event?.name;
  return { statusCode: 200, body: "..." };
}
```

### Format Respons

Function harus mengembalikan objek dengan:

```javascript
{
  "statusCode": 200,                          // HTTP status code
  "headers": {                                // Header opsional
    "content-type": "application/json"
  },
  "body": "{\"message\": \"Hello World\"}"   // Body respons (string)
}
```

**Penting**: Field `body` harus berupa **string**, bukan objek. Gunakan `JSON.stringify()` atau format string.

---

## Langkah 4: Konfigurasi Resource

Konfigurasi CPU, memory, dan timeout untuk function Anda:

![Image: Resource configuration sliders](/images/functions/resource-config.png)

### vCPU (Virtual CPU)

Pilih core CPU (1-32):

![Image: vCPU slider](/images/functions/vcpu-slider.png)

| vCPU | Terbaik Untuk | Contoh |
|------|--------------|--------|
| 1 | API sederhana, transformasi data | Parser JSON, webhook handler |
| 2 | Pemrosesan menengah, operasi I/O | Resize gambar, pemrosesan CSV |
| 4+ | Tugas intensif CPU | Encoding video, inferensi ML |

**Default**: 1 vCPU

**Tips**: Mulai dengan 1 vCPU dan tingkatkan jika ada masalah performa.

---

### Memory (MB)

Alokasikan memory (128-3072 MB):

![Image: Memory slider](/images/functions/memory-slider.png)

| Memory | Terbaik Untuk | Contoh |
|--------|--------------|--------|
| 128 MB | Function minimal, logika sederhana | Hello World, kalkulator |
| 512 MB | API standar, pemrosesan data | REST API, transformer JSON |
| 1024 MB (1 GB) | Dataset besar, operasi kompleks | Pemrosesan gambar, pembuatan laporan |
| 2048 MB (2 GB+) | Model ML, pemrosesan video | Inferensi ML, resize video |

**Default**: 512 MB

**Penting**: Lebih banyak memory = biaya per invokasi lebih tinggi.

---

### Timeout (detik)

Tetapkan waktu eksekusi maksimum (1-300 detik):

![Image: Timeout input](/images/functions/timeout-input.png)

| Timeout | Kasus Penggunaan |
|---------|-----------------|
| 1-10 dtk | API cepat, pemrosesan sederhana |
| 30 dtk | Function standar (default) |
| 60-120 dtk | Pemrosesan kompleks, panggilan API eksternal |
| 300 dtk (5 mnt) | Tugas yang berjalan lama, pemrosesan batch |

**Default**: 30 detik

**Catatan**: Function otomatis dihentikan jika melampaui timeout.

---

## Langkah 5: Tes Function Anda

### 1. Tulis Event Tes

Masukkan payload JSON di editor event tes:

![Image: Test event JSON editor](/images/functions/test-event-editor.png)

**Contoh payload**:

```json
{
  "key1": 10,
  "key2": 5
}
```

```json
{
  "name": "Alice",
  "age": 30,
  "city": "Jakarta"
}
```

### 2. Jalankan Tes

Klik tombol **Run Test**:

![Image: Run Test button](/images/functions/run-test-button.png)

Function akan dieksekusi secara lokal dan menampilkan hasil:

![Image: Test output showing response](/images/functions/test-output.png)

**Output Tes Menampilkan**:
- ✅ **Respons** - Nilai kembalian function
- ✅ **Log** - Output konsol dan error
- ✅ **Waktu Eksekusi** - Berapa lama proses berlangsung
- ✅ **Status** - Berhasil atau error

### 3. Verifikasi Output

Cek output tes:

```json
{
  "statusCode": 200,
  "headers": {
    "content-type": "application/json"
  },
  "body": "{\"result\": 15}"
}
```

**Jika ada error**:
- Tinjau pesan error di log
- Perbaiki kode Anda
- Jalankan tes lagi

**Tips**: Tes dengan beberapa payload untuk mencakup berbagai skenario.

---

## Langkah 6: Deploy Function

Saat function Anda berfungsi dengan benar, deploy:

### Klik Save/Deploy

![Image: Save button highlighted](/images/functions/save-button.png)

Klik tombol **Save** di sudut kanan atas.

### Proses Deployment

Sistem akan:

1. ✓ Validasi kode dan konfigurasi function
2. ✓ Buat microVM untuk function
3. ✓ Deploy runtime dan dependensi
4. ✓ Jalankan layanan function
5. ✓ Tandai function sebagai "Ready"

**Waktu**: Biasanya selesai dalam **2-5 detik**

### Notifikasi Berhasil

Anda akan melihat pesan berhasil:

![Image: Function created successfully](/images/functions/function-created-success.png)

**"Function created - [function-name] successfully created."**

Anda akan diarahkan ke halaman daftar Functions.

---

## Langkah 7: Verifikasi Deployment

Setelah deployment, verifikasi function siap:

![Image: Function in Ready state](/images/functions/function-ready-state.png)

### Cek Tabel Function

Di halaman daftar Functions, temukan function Anda:

- **State** harus **"Ready"** (badge hijau)
- **Language** menampilkan runtime yang dipilih
- **Guest IP** menampilkan alamat IP function
- **Owner** menampilkan "You"

### Tes Invoke

Klik tombol **Invoke** (ikon ▶ Play) untuk tes:

![Image: Invoke button on function row](/images/functions/invoke-button.png)

Masukkan payload tes dan klik **Invoke**:

![Image: Invoke dialog with payload](/images/functions/invoke-dialog.png)

Verifikasi respons sesuai harapan Anda.

---

## Contoh Lengkap: Hello World API

Mari buat function lengkap dari awal:

### Konfigurasi

- **Name**: `hello-api`
- **Runtime**: TypeScript
- **Handler**: `handler`
- **vCPU**: 1
- **Memory**: 512 MB
- **Timeout**: 30 dtk

### Kode

```typescript
interface Event {
  name?: string;
  language?: string;
}

export async function handler(event: Event) {
  const name = event?.name || "World";
  const language = event?.language || "en";

  const greetings: Record<string, string> = {
    en: "Hello",
    es: "Hola",
    fr: "Bonjour",
    id: "Halo",
    ja: "こんにちは",
  };

  const greeting = greetings[language] || greetings.en;

  return {
    statusCode: 200,
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      message: `${greeting}, ${name}!`,
      timestamp: new Date().toISOString(),
    }),
  };
}
```

### Event Tes

```json
{
  "name": "Alice",
  "language": "id"
}
```

### Respons yang Diharapkan

```json
{
  "statusCode": 200,
  "headers": {
    "content-type": "application/json"
  },
  "body": "{\"message\":\"Halo, Alice!\",\"timestamp\":\"2025-12-18T10:30:00.000Z\"}"
}
```

**Coba**: Buat function ini dan tes dengan berbagai bahasa!

---

## Import dari Playground

Jika Anda sudah bereksperimen di Playground, Anda dapat mengimpor pekerjaan Anda:

![Image: Import from Playground button](/images/functions/import-from-playground.png)

**Langkah**:
1. Kerjakan function di **Playground** terlebih dahulu
2. Klik **"Save to Functions"** di Playground
3. Halaman New Function terbuka dengan kode Anda sudah terisi
4. Konfigurasi nama dan resource
5. Klik Save untuk deploy

**Manfaat**: Iterasi cepat di Playground, lalu deploy saat siap.

---

## Pemecahan Masalah

### Masalah: Function Tertahan di "Creating"

**Gejala**:
- Status menampilkan "Creating" lebih dari 30 detik
- Tidak pernah berubah menjadi "Ready"

**Solusi**:
1. **Refresh halaman** - Terkadang UI perlu diperbarui
2. **Cek resource host**:
   - Buka halaman **Hosts**
   - Pastikan agent online
   - Cek CPU dan memory yang tersedia
3. **Hapus dan buat ulang**:
   - Hapus function yang tertahan
   - Buat yang baru

---

### Masalah: Tes Gagal dengan Error

**Gejala**:
- Output tes menampilkan pesan error
- Function tidak mengembalikan respons yang diharapkan

**Solusi**:

1. **Cek log error** di output tes:
   ```
   Error: name is undefined
   ```

2. **Error umum**:
   - **Syntax error**: Perbaiki sintaks kode
   - **Variabel tidak terdefinisi**: Periksa nama variabel
   - **JSON parse error**: Pastikan `body` adalah string
   - **Timeout**: Kurangi waktu pemrosesan atau tingkatkan timeout

3. **Tips debug**:
   - Tambahkan pernyataan `console.log()` (Python: `print()`)
   - Tes dengan payload sederhana terlebih dahulu
   - Pastikan struktur event sesuai kode Anda

**Contoh perbaikan**:
```typescript
// ❌ Salah - body adalah objek
return {
  statusCode: 200,
  body: { message: "Hi" }  // ERROR!
};

// ✅ Benar - body adalah string
return {
  statusCode: 200,
  body: JSON.stringify({ message: "Hi" })
};
```

---

### Masalah: Tidak Dapat Menyimpan Function

**Gejala**:
- Tombol Save dinonaktifkan
- Error validasi ditampilkan

**Solusi**:

1. **Periksa kolom yang diperlukan**:
   - ✅ Nama terisi
   - ✅ Runtime dipilih
   - ✅ Handler terisi
   - ✅ Kode tidak kosong

2. **Perbaiki error validasi**:
   - Teks merah menampilkan apa yang salah
   - Perbaiki setiap error sebelum menyimpan

3. **Kode harus valid**:
   - Tidak ada syntax error
   - Function handler ada
   - Struktur yang benar

---

## Praktik Terbaik

### Organisasi Kode

✅ **Jaga function terfokus**:
```typescript
// ✅ Baik - Tanggung jawab tunggal
export async function handler(event) {
  return processPayment(event);
}

// ❌ Buruk - Terlalu banyak tanggung jawab
export async function handler(event) {
  // 100 baris logika yang bercampur...
}
```

---

### Penanganan Error

✅ **Selalu tangani error**:
```python
def handler(event):
    try:
        # Logika Anda di sini
        result = process_data(event)
        return {
            "statusCode": 200,
            "body": json.dumps({"result": result})
        }
    except ValueError as e:
        return {
            "statusCode": 400,
            "body": json.dumps({"error": str(e)})
        }
    except Exception as e:
        return {
            "statusCode": 500,
            "body": json.dumps({"error": "Internal server error"})
        }
```

---

### Alokasi Resource

✅ **Sesuaikan ukuran function Anda**:
- Mulai dengan resource minimum (1 vCPU, 512 MB)
- Monitor waktu invokasi di log
- Tingkatkan hanya jika diperlukan
- Jangan over-alokasi (biaya lebih tinggi)

---

### Pengujian

✅ **Tes secara menyeluruh sebelum deploy**:
- Tes dengan input valid
- Tes dengan input tidak valid
- Tes edge case
- Tes skenario error

---

## Langkah Selanjutnya

Setelah membuat function:

- **[Kelola Functions](manage-functions/)** - Invoke, perbarui, dan hapus function
- **[Lihat Log](logs/)** - Debug dan monitor eksekusi function
- **[Playground](playground/)** - Bereksperimen dengan ide baru

---

## Referensi Cepat

### Struktur Function

**Python**:
```python
def handler(event):
    # Proses event (dict)
    return {
        "statusCode": 200,
        "headers": {"content-type": "application/json"},
        "body": json.dumps({"result": "..."})
    }
```

**TypeScript**:
```typescript
export async function handler(event) {
  // Proses event (objek)
  return {
    statusCode: 200,
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ result: "..." }),
  };
}
```

### Batas Resource

| Resource | Minimum | Maksimum | Default |
|----------|---------|---------|---------|
| vCPU | 1 | 32 | 1 |
| Memory | 128 MB | 3072 MB | 512 MB |
| Timeout | 1 dtk | 300 dtk | 30 dtk |

### HTTP Status Code Umum

| Kode | Arti | Kapan Digunakan |
|------|------|----------------|
| 200 | OK | Respons berhasil |
| 400 | Bad Request | Input tidak valid |
| 404 | Not Found | Resource tidak ditemukan |
| 500 | Internal Server Error | Error tak terduga |
