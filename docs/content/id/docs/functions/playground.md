+++
title = "Playground"
description = "Bereksperimen dengan function tanpa membuatnya"
weight = 43
date = 2025-12-18
+++

Playground adalah lingkungan interaktif tempat Anda dapat bereksperimen dengan kode function dan mengujinya secara instan **tanpa membuat function**. Sempurna untuk belajar, prototyping, dan iterasi cepat.

---

## Apa itu Playground?

Playground adalah **sandbox kode** yang memungkinkan Anda:

- ✅ Tulis dan uji kode function secara instan
- ✅ Bereksperimen dengan berbagai runtime (Python, JavaScript, TypeScript)
- ✅ Tes dengan payload JSON kustom
- ✅ Lihat hasil secara langsung
- ✅ Simpan kode yang berfungsi untuk membuat function nanti

![Image: Playground interface overview](/images/functions/playground-overview.png)

**Keuntungan**:
- 🚀 **Tidak perlu deployment** - Tes kode tanpa membuat function
- ⚡ **Umpan balik instan** - Lihat hasil dalam hitungan detik
- 🔄 **Iterasi cepat** - Ubah kode dan uji kembali secara langsung
- 💡 **Belajar sambil melakukan** - Bereksperimen dengan contoh
- 💾 **Simpan saat siap** - Konversi ke function dengan satu klik

---

## Mengakses Playground

### Dari Halaman Functions

Klik tombol **Playground** di halaman Functions:

![Image: Playground button on Functions page](/images/functions/playground-button.png)

**Lokasi**: Header halaman Functions, di sebelah tombol "New Function"

---

## Antarmuka Playground

Playground memiliki tiga bagian utama:

![Image: Playground sections labeled](/images/functions/playground-sections.png)

1. **Configuration** - Pilih runtime
2. **Code Editor** - Tulis kode function
3. **Test Panel** - Input event dan lihat hasil

---

## Langkah 1: Pilih Runtime

Pilih bahasa pemrograman:

![Image: Runtime selector in Playground](/images/functions/playground-runtime-selector.png)

**Runtime yang tersedia**:
- **Python** (Python 3.11)
- **JavaScript** (Bun)
- **TypeScript** (Bun) - Default

**Yang terjadi saat Anda mengganti runtime**:
- Editor kode diperbarui dengan template default untuk bahasa yang dipilih
- Kode sebelumnya diganti (tidak disimpan otomatis)
- Event tes tetap sama

**Tips**: Mulai dengan TypeScript untuk type safety, atau Python untuk kesederhanaan.

---

## Langkah 2: Tulis Kode Function

Gunakan editor kode Monaco untuk menulis function Anda:

![Image: Code editor in Playground](/images/functions/playground-code-editor.png)

### Fitur Editor

Editor menyediakan:
- ✅ **Syntax highlighting** - Kode berwarna-warni
- ✅ **Auto-completion** - Saran IntelliSense
- ✅ **Deteksi error** - Pengecekan sintaks real-time
- ✅ **Pengeditan multi-kursor** - Edit beberapa baris sekaligus
- ✅ **Code folding** - Perluas/ciutkan blok kode

### Template Default

Setiap runtime memiliki function kalkulator default:

**Python**:
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

**TypeScript**:
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

### Persyaratan Function

Function Anda harus:
- Ekspor atau definisikan function `handler`
- Menerima satu parameter: `event` (dict/objek)
- Mengembalikan objek dengan `statusCode` dan `body`

**Struktur yang valid**:
```typescript
export async function handler(event) {
  // Logika Anda di sini
  return {
    statusCode: 200,
    body: JSON.stringify({ message: "Success" }),
  };
}
```

---

## Langkah 3: Konfigurasi Event Tes

Masukkan payload JSON untuk menguji function Anda:

![Image: Test event editor in Playground](/images/functions/playground-test-event.png)

### Editor Event Tes

Panel event tes memiliki:
- **Editor JSON** dengan syntax highlighting
- **Validasi real-time** (menampilkan apakah JSON valid)
- **Payload default** yang sesuai dengan template function

**Event tes default**:
```json
{
  "key1": 10,
  "key2": 5
}
```

### Event Tes Kustom

Ganti default dengan JSON Anda sendiri:

**Contoh 1 - Registrasi pengguna**:
```json
{
  "username": "alice",
  "email": "alice@example.com",
  "age": 30
}
```

**Contoh 2 - Pemrosesan gambar**:
```json
{
  "imageUrl": "https://example.com/photo.jpg",
  "width": 800,
  "height": 600,
  "format": "jpeg"
}
```

**Contoh 3 - Transformasi data**:
```json
{
  "data": [
    {"name": "Alice", "score": 95},
    {"name": "Bob", "score": 87}
  ],
  "sortBy": "score"
}
```

**Tips**: Tes dengan beberapa payload untuk memverifikasi berbagai skenario.

---

## Langkah 4: Jalankan Tes

Klik tombol **Run** untuk mengeksekusi function Anda:

![Image: Run button in Playground](/images/functions/playground-run-button.png)

### Yang Terjadi

Saat Anda klik Run:

1. Kode dikirim ke lingkungan tes backend
2. Function mengeksekusi dengan event tes Anda
3. Hasil ditampilkan di panel Output
4. Log ditampilkan di bawah output

**Waktu**: Biasanya selesai dalam **1-2 detik**

![Image: Running indicator](/images/functions/playground-running.png)

---

## Langkah 5: Lihat Hasil

Setelah eksekusi, hasil muncul di panel Output:

![Image: Output panel with results](/images/functions/playground-output.png)

### Panel Output

Menampilkan nilai kembalian function:

**Respons berhasil**:
```json
{
  "statusCode": 200,
  "headers": {
    "content-type": "application/json"
  },
  "body": "{\"result\": 15}"
}
```

**Respons error**:
```json
{
  "statusCode": 400,
  "body": "{\"error\": \"key1 and key2 must be numbers\"}"
}
```

---


## Langkah 6: Iterasi dan Tingkatkan

Playground sempurna untuk iterasi cepat:

### Alur Kerja Iterasi

1. **Ubah kode** di editor
2. **Klik Run** untuk tes
3. **Lihat hasil** dan log
4. **Ulangi** hingga berfungsi

![Image: Iteration workflow diagram](/images/functions/playground-iteration.png)

**Contoh iterasi**:

**Iterasi 1** - Function dasar:
```typescript
export async function handler(event) {
  return {
    statusCode: 200,
    body: JSON.stringify({ message: "Hello" }),
  };
}
```

**Tes**: ✅ Berfungsi

**Iterasi 2** - Tambah parameter:
```typescript
export async function handler(event) {
  const name = event?.name || "World";
  return {
    statusCode: 200,
    body: JSON.stringify({ message: `Hello, ${name}` }),
  };
}
```

**Tes**: ✅ Berfungsi

**Iterasi 3** - Tambah validasi:
```typescript
export async function handler(event) {
  if (!event?.name) {
    return {
      statusCode: 400,
      body: JSON.stringify({ error: "name is required" }),
    };
  }

  return {
    statusCode: 200,
    body: JSON.stringify({ message: `Hello, ${event.name}` }),
  };
}
```

**Tes**: ✅ Berfungsi dengan validasi

---

## Simpan ke Functions

Saat kode Anda berfungsi dengan sempurna, simpan sebagai function:

### Klik "Save to Functions"

![Image: Save to Functions button](/images/functions/save-to-functions-button.png)

**Lokasi**: Sudut kanan atas Playground

### Yang Terjadi

1. Kode, runtime, dan event tes saat ini disimpan
2. Anda diarahkan ke halaman **New Function**
3. Formulir terisi otomatis dengan kode Playground Anda
4. Selesaikan setup (nama, resource, dll.)
5. Klik Save untuk deploy

**Manfaat**: Lewati penulisan kode ulang - cukup konfigurasi dan deploy!

---

## Navigasi

### Kembali ke Functions

Klik panah **Back** untuk kembali ke daftar Functions:

![Image: Back button in Playground](/images/functions/playground-back-button.png)

**Catatan**: Kode Playground Anda **tidak disimpan** saat Anda berpindah halaman (kecuali Anda klik "Save to Functions").

---

## Kasus Penggunaan

### 1. Belajar Serverless Functions

**Sempurna untuk pemula**:
- Coba template default
- Ubah kode dan lihat apa yang terjadi
- Bereksperimen dengan runtime berbeda
- Pelajari struktur function

**Contoh**: Ubah kalkulator untuk melakukan perkalian alih-alih penjumlahan.

---

### 2. Prototyping Ide Baru

**Pengembangan cepat**:
- Tes algoritma sebelum deploy
- Validasi transformasi data
- Bereksperimen dengan API eksternal
- Prototype logika bisnis

**Contoh**: Tes logika transformasi JSON sebelum membuat function produksi.

---

### 3. Pengujian Cuplikan Kode

**Validasi cepat**:
- Tes pola regex
- Validasi parsing data
- Cek penanganan error
- Verifikasi edge case

**Contoh**: Tes apakah parsing tanggal Anda berfungsi dengan berbagai format.

---

### 4. Membandingkan Runtime

**Perbandingan performa**:
- Tulis logika yang sama di Python dan TypeScript
- Tes waktu eksekusi
- Bandingkan kompleksitas kode
- Pilih runtime terbaik untuk kasus penggunaan Anda

**Contoh**: Bandingkan performa parsing JSON antar runtime.

---

### 5. Debug Function yang Ada

**Isolasi masalah**:
- Salin kode function ke Playground
- Tes dengan payload tertentu
- Tambahkan log debug
- Perbaiki masalah, lalu perbarui function

**Contoh**: Debug mengapa function gagal dengan input tertentu.

---

## Contoh Alur Kerja

### Contoh 1: Buat API Sapaan

**Tujuan**: Function yang menyapa pengguna dalam berbagai bahasa

**Langkah 1**: Buka Playground, pilih TypeScript

**Langkah 2**: Tulis kode:
```typescript
interface Event {
  name?: string;
  language?: string;
}

export async function handler(event: Event) {
  const name = event?.name || "Friend";
  const lang = event?.language || "en";

  const greetings: Record<string, string> = {
    en: "Hello",
    es: "Hola",
    fr: "Bonjour",
    de: "Hallo",
    ja: "こんにちは",
  };

  const greeting = greetings[lang] || greetings.en;

  return {
    statusCode: 200,
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      message: `${greeting}, ${name}!`,
      language: lang,
    }),
  };
}
```

**Langkah 3**: Tes dengan event berbeda:

```json
{"name": "Alice", "language": "es"}
```
→ Respons: `"Hola, Alice!"`

```json
{"name": "Bob", "language": "ja"}
```
→ Respons: `"こんにちは, Bob!"`

**Langkah 4**: Klik "Save to Functions", deploy sebagai `greeting-api`

---

### Contoh 2: Validator Data JSON

**Tujuan**: Validasi struktur data yang masuk

**Langkah 1**: Buka Playground, pilih Python

**Langkah 2**: Tulis kode:
```python
def handler(event):
    required_fields = ["email", "age", "name"]

    # Cek field yang diperlukan
    missing = [f for f in required_fields if f not in event]
    if missing:
        return {
            "statusCode": 400,
            "body": f'{{"error": "Missing fields: {", ".join(missing)}"}}',
        }

    # Validasi email
    email = event["email"]
    if "@" not in email:
        return {
            "statusCode": 400,
            "body": '{"error": "Invalid email format"}',
        }

    # Validasi usia
    try:
        age = int(event["age"])
        if age < 0 or age > 150:
            raise ValueError()
    except:
        return {
            "statusCode": 400,
            "body": '{"error": "Age must be between 0 and 150"}',
        }

    return {
        "statusCode": 200,
        "body": '{"message": "Validation passed"}',
    }
```

**Langkah 3**: Tes dengan data tidak valid:
```json
{"name": "Alice", "email": "invalid", "age": -5}
```
→ Respons: `"Invalid email format"`

**Langkah 4**: Tes dengan data valid:
```json
{"name": "Alice", "email": "alice@example.com", "age": 30}
```
→ Respons: `"Validation passed"`

---

## Tips dan Trik

### Pintasan Editor

**Pintasan keyboard** (sama seperti VS Code):

| Pintasan | Aksi |
|----------|------|
| `Ctrl + S` | Simpan ke Functions |
| `Ctrl + F` | Cari di kode |
| `Ctrl + H` | Cari dan ganti |
| `Ctrl + /` | Toggle komentar |
| `Alt + Up/Down` | Pindahkan baris ke atas/bawah |
| `Ctrl + D` | Pilih kemunculan berikutnya |
| `Ctrl + Shift + K` | Hapus baris |
| `F11` | Toggle layar penuh |

---

### Tips Debugging

✅ **Tambahkan logging**:
```python
# Python
print(f"Event received: {event}")
print(f"Processing key1: {event.get('key1')}")
```

```typescript
// TypeScript
console.log("Event received:", event);
console.log("Processing key1:", event?.key1);
```

✅ **Tes edge case**:
- Payload kosong: `{}`
- Field yang hilang: `{"key1": 10}` (tanpa key2)
- Tipe tidak valid: `{"key1": "abc", "key2": "def"}`
- Nilai besar: `{"key1": 999999, "key2": 888888}`

---

### Pengujian Performa

✅ **Cek waktu eksekusi** di log:
- Tipikal: 50-200ms
- Lambat: >500ms (optimalkan kode)

✅ **Tes dengan payload besar**:
```json
{
  "data": [
    {"id": 1, "value": "..."},
    {"id": 2, "value": "..."},
    ... // 100 item
  ]
}
```

---

## Keterbatasan

**Batasan Playground**:
- ⚠️ **Tidak ada deployment** - Kode berjalan dalam mode tes saja
- ⚠️ **Tidak ada persistensi** - Kode hilang saat refresh halaman (kecuali disimpan)
- ⚠️ **Tidak ada paket kustom** - Hanya library bawaan yang tersedia
- ⚠️ **Eksekusi tunggal** - Bukan untuk load testing

**Untuk deploy**: Gunakan "Save to Functions" untuk membuat function yang sesungguhnya.

---

## Praktik Terbaik

✅ **Bereksperimen dengan bebas**:
- Coba berbagai pendekatan
- Rusak hal-hal dan belajar
- Tes input yang tidak biasa
- Jangan khawatir tentang kesalahan

✅ **Tes secara menyeluruh sebelum menyimpan**:
- Tes happy path
- Tes kasus error
- Tes edge case
- Verifikasi semua skenario berfungsi

✅ **Gunakan Playground untuk belajar**:
- Coba fitur khusus runtime
- Pelajari pola async
- Latih penanganan error
- Jelajahi API

✅ **Simpan saat siap**:
- Hanya simpan kode yang berfungsi
- Verifikasi tes lulus
- Tambahkan komentar untuk kejelasan
- Kemudian deploy sebagai function

---

## Pemecahan Masalah

### Masalah: Kode tidak berjalan

**Gejala**:
- Klik Run tetapi tidak ada yang terjadi
- Atau error langsung muncul

**Solusi**:
1. Periksa syntax error (garis bergelombang merah)
2. Pastikan function `handler` ada
3. Periksa tanda tangan function sudah benar
4. Verifikasi event tes JSON valid

---

### Masalah: Hasil yang tidak terduga

**Gejala**:
- Output tidak sesuai harapan
- Logika tampak salah

**Solusi**:
1. Tambahkan pernyataan `console.log` / `print`
2. Cek panel log untuk output
3. Verifikasi event tes memiliki data yang benar
4. Telusuri logika secara mental

---

### Masalah: Tidak dapat menyimpan ke Functions

**Gejala**:
- Tombol tidak berfungsi
- Atau error terjadi

**Solusi**:
1. Pastikan kode memiliki sintaks yang valid
2. Cek konsol browser untuk error
3. Coba refresh halaman dan tulis ulang kode
4. Hubungi administrator jika berlanjut

---

## Referensi Cepat

### Alur Kerja Playground

1. **Pilih runtime** → Pilih bahasa
2. **Tulis kode** → Buat function handler
3. **Masukkan event tes** → Payload JSON
4. **Klik Run** → Eksekusi function
5. **Lihat hasil** → Cek output dan log
6. **Iterasi** → Ubah dan uji kembali
7. **Simpan** → Deploy sebagai function

### Template Function

**Function minimal yang dapat berjalan**:
```typescript
export async function handler(event) {
  return {
    statusCode: 200,
    body: JSON.stringify({ message: "Hello" }),
  };
}
```

---

## Langkah Selanjutnya

- **[Buat Function](create-function/)** - Deploy kode Playground Anda
- **[Kelola Functions](manage-functions/)** - Pelajari operasi function
- **[Lihat Log](logs/)** - Debug eksekusi function
