+++
title = "Deploy Container"
description = "Panduan langkah demi langkah untuk men-deploy Docker container"
weight = 31
date = 2025-12-18
+++

Pelajari cara men-deploy Docker container dengan konfigurasi lengkap — mulai dari web server sederhana hingga database yang kompleks.

---

## Prasyarat

Sebelum men-deploy container, pastikan:

✅ **Image runtime container** tersedia:
- Cek: Halaman Registry → Images → Cari "container-runtime"
- Jika tidak ada, jalankan: `sudo ./scripts/build-container-runtime-v2.sh`

✅ **Setidaknya satu host** online:
- Cek: Dashboard → Hosts → Status harus "Online"

✅ **Network bridge** dikonfigurasi:
- Bridge default: `fcbr0`
- Setup: `sudo ./scripts/fc-bridge-setup.sh fcbr0 <uplink-interface>`

---

## Langkah 1: Buka Halaman Deployment

Navigasi ke halaman deployment container:

![Image: Containers page with Deploy button](/images/containers/deploy-step1-nav.png)

1. Klik **"Containers"** di sidebar
2. Klik tombol **"Deploy Container"**

Anda akan melihat formulir deployment:

![Image: Container deployment form overview](/images/containers/deploy-step1-form.png)

---

## Langkah 2: Konfigurasi Dasar

### Nama Container

Masukkan nama unik untuk container Anda:

![Image: Container name input](/images/containers/deploy-step2-name.png)

**Panduan**:
- Gunakan huruf kecil, angka, tanda hubung
- Buat nama yang deskriptif: `prod-api`, `dev-postgres`, `nginx-frontend`
- Hindari nama generik: `container1`, `test`, `my-container`

**Contoh**:
```
Web server:  nginx-prod, apache-dev
Database:    postgres-main, mysql-users, mongo-analytics
API:         api-gateway, auth-service, payment-api
Cache:       redis-sessions, memcached-cache
```

---

### Pilih Sumber Image

Pilih dari mana Docker image Anda akan diambil:

![Image: Image source tabs](/images/containers/deploy-step2-image-tabs.png)

Tiga opsi tersedia:

#### Opsi 1: Registry (Image yang Di-cache)

Gunakan image yang sudah diunduh ke registry lokal:

![Image: Registry selector](/images/containers/deploy-step2-registry.png)

**Kapan digunakan**:
- Image sudah diunduh melalui halaman Registry
- Menginginkan deployment lebih cepat (tidak perlu pull)
- Deployment offline

**Cara penggunaan**:
1. Klik tab **"Registry"**
2. Pilih image dari dropdown
3. Menampilkan ukuran image

**Jika tidak ada image**:
- Kunjungi halaman Registry terlebih dahulu
- Unduh image dari Docker Hub
- Atau unggah image kustom

---

#### Opsi 2: Docker Hub

Tarik image langsung dari Docker Hub:

![Image: Docker Hub input](/images/containers/deploy-step2-dockerhub.png)

**Kapan digunakan**:
- Butuh versi terbaru
- Image tidak ada di registry lokal
- Men-deploy image baru

**Cara penggunaan**:
1. Klik tab **"Docker Hub"**
2. Masukkan nama image dan tag
3. Format: `repository:tag`

**Image populer**:
```
Web server:
  nginx:alpine
  nginx:latest
  httpd:alpine
  caddy:latest

Database:
  postgres:15-alpine
  postgres:16
  mysql:8-oracle
  mariadb:11
  mongo:7
  redis:7-alpine

Bahasa pemrograman:
  node:20-alpine
  node:20
  python:3.11-alpine
  golang:1.21-alpine
  openjdk:21-slim

Message queue:
  rabbitmq:3-management-alpine
  nats:alpine
  kafka:latest
```

**Tips**:
- Gunakan varian Alpine untuk ukuran lebih kecil (mis., `nginx:alpine`)
- Tentukan versi spesifik (mis., `postgres:15.3-alpine`)
- Cek Docker Hub untuk tag yang tersedia

---

#### Opsi 3: Unggah Tarball Image

Unggah Docker image yang diekspor dari mesin Anda:

![Image: Upload image file](/images/containers/deploy-step2-upload.png)

**Kapan digunakan**:
- Image kustom yang tidak ada di Docker Hub
- Image pribadi tanpa registry
- Lingkungan air-gapped
- Image korporat internal

**Cara mengekspor image**:

Di mesin development Anda:
```bash
# Ekspor satu image
docker save -o myapp.tar myapp:latest

# Ekspor dengan kompresi
docker save myapp:latest | gzip > myapp.tar.gz

# Ekspor beberapa image
docker save -o images.tar nginx:alpine postgres:15 redis:7
```

**Cara penggunaan**:
1. Klik tab **"Upload"**
2. Klik **"Choose File"** dan pilih `.tar` atau `.tar.gz`
3. Nama image terisi otomatis dari nama file (dapat diedit)
4. Menampilkan ukuran file

**Catatan ukuran file**:
- Upload dapat memakan waktu untuk image berukuran besar
- Ukuran tipikal: 40 MB (Alpine) hingga 500 MB (OS penuh)

---

## Langkah 3: Konfigurasi Resource

Tentukan batas CPU dan memory untuk container Anda:

![Image: Resource sliders](/images/containers/deploy-step3-resources.png)

### Batas CPU

**Rentang**: 0.1 hingga 16 core
**Default**: 1 core

![Image: CPU slider](/images/containers/deploy-step3-cpu.png)

**Panduan berdasarkan jenis layanan**:
```
Situs web statis:        0.5 vCPU
API kecil:               1 vCPU
Aplikasi menengah:       2 vCPU
Database:                2-4 vCPU
Pemrosesan berat:        4-8 vCPU
```

**Tips**: Mulai dengan CPU yang lebih rendah, monitor penggunaan, dan tingkatkan jika diperlukan.

---

### Batas Memory

**Rentang**: 64 MB hingga 32 GB (32.768 MB)
**Default**: 512 MB

![Image: Memory slider](/images/containers/deploy-step3-memory.png)

**Panduan berdasarkan jenis layanan**:
```
Situs web statis:        256 MB
API kecil:               512 MB
Aplikasi Node.js:        1024 MB (1 GB)
Aplikasi Python:         1024-2048 MB
Database (kecil):        2048 MB (2 GB)
Database (menengah):     4096 MB (4 GB)
Database (besar):        8192-16384 MB (8-16 GB)
Redis/Memcached:         512-2048 MB
```

**Penting**:
- Container akan dihentikan paksa jika melampaui batas memory
- Cek kebutuhan image (beberapa database butuh minimal 1 GB)
- Monitor penggunaan aktual di tab Stats

---

## Langkah 4: Pemetaan Port (Opsional)

Expose port container ke jaringan host:

![Image: Port mappings section](/images/containers/deploy-step4-ports.png)

### Tambah Pemetaan Port

Klik **"Add Port"** untuk membuat pemetaan baru:

![Image: Add port button](/images/containers/deploy-step4-add-port.png)

### Konfigurasi Setiap Port

![Image: Port mapping row](/images/containers/deploy-step4-port-row.png)

**Kolom**:
1. **Host Port** - Port di mesin host (mis., 8080)
2. **Container Port** - Port di dalam container (mis., 80)
3. **Protocol** - TCP atau UDP

**Contoh pemetaan**:
```
Layanan          Host:Container  Protokol
────────────────────────────────────────
Web Nginx        8080:80         TCP
PostgreSQL       5432:5432       TCP
Redis            6379:6379       TCP
MongoDB          27017:27017     TCP
RabbitMQ         5672:5672       TCP
Mgmt RabbitMQ    15672:15672     TCP
DNS server       53:53           UDP
```

### Beberapa Port

Tambah beberapa pemetaan untuk layanan dengan banyak port:

![Image: Multiple port mappings](/images/containers/deploy-step4-multiple.png)

**Contoh: RabbitMQ dengan manajemen**:
```
5672:5672   TCP  (AMQP)
15672:15672 TCP  (Management UI)
```

**Contoh: Aplikasi full stack**:
```
3000:3000   TCP  (API)
3001:3001   TCP  (WebSocket)
```

### Hapus Pemetaan Port

Klik **tombol X** untuk menghapus pemetaan port:

![Image: Remove port button](/images/containers/deploy-step4-remove.png)

---

## Langkah 5: Variabel Lingkungan (Opsional)

Tetapkan variabel lingkungan untuk konfigurasi container:

![Image: Environment variables section](/images/containers/deploy-step5-env.png)

### Tambah Variabel Lingkungan

Klik **"Add Variable"** untuk membuat variabel baru:

![Image: Add environment variable](/images/containers/deploy-step5-add-env.png)

### Konfigurasi Variabel

![Image: Environment variable row](/images/containers/deploy-step5-env-row.png)

**Kolom**:
1. **KEY** - Nama variabel (konvensi huruf besar)
2. **value** - Nilai variabel

### Kasus Penggunaan Umum

**Konfigurasi database**:
```
POSTGRES_PASSWORD=mySecretPassword123
POSTGRES_USER=myapp
POSTGRES_DB=production
POSTGRES_INITDB_ARGS=--encoding=UTF8
```

**Konfigurasi aplikasi**:
```
NODE_ENV=production
API_KEY=abc123xyz789
DATABASE_URL=postgres://user:pass@db:5432/myapp
LOG_LEVEL=info
PORT=3000
```

**Autentikasi**:
```
JWT_SECRET=mySecretKey
API_TOKEN=secure-token-here
ADMIN_PASSWORD=changeMe123
```

**Feature flag**:
```
ENABLE_DEBUG=false
ENABLE_CACHE=true
MAX_CONNECTIONS=100
TIMEOUT_SECONDS=30
```

### Hapus Variabel Lingkungan

Klik **tombol X** untuk menghapus variabel:

![Image: Remove env var button](/images/containers/deploy-step5-remove.png)

---

## Langkah 6: Volume Mount (Opsional)

Mount penyimpanan persisten ke dalam container Anda:

![Image: Volume mounts section](/images/containers/deploy-step6-volumes.png)

### Mengapa Menggunakan Volume?

✅ **Persistensi data** - Data bertahan saat container di-restart/dihapus
✅ **Berbagi data** - Berbagi data antar container
✅ **Konfigurasi** - Mount file konfigurasi dari host
✅ **Log** - Simpan log di host untuk dianalisis

### Tambah Volume

Klik **"Add Volume"** untuk membuka dialog volume:

![Image: Add volume button](/images/containers/deploy-step6-add-button.png)

### Dialog Volume

![Image: Volume creation dialog](/images/containers/deploy-step6-volume-dialog.png)

Dua opsi:

#### Opsi 1: Buat Volume Baru

Buat volume baru untuk container ini:

![Image: New volume form](/images/containers/deploy-step6-new-volume.png)

**Kolom**:
1. **Volume Name** - Identifikasi unik (mis., `postgres-data`, `app-uploads`)
2. **Size (MB)** - Ukuran volume dalam megabyte (mis., 1024 = 1 GB)
3. **Container Path** - Tempat mount di dalam container (mis., `/data`)
4. **Read-only** - Centang untuk mencegah penulisan

**Contoh - Volume database**:
```
Volume Name:     postgres-data
Size:            10240 MB (10 GB)
Container Path:  /var/lib/postgresql/data
Read-only:       ☐ (tidak dicentang)
```

**Contoh - Volume konfigurasi**:
```
Volume Name:     nginx-config
Size:            100 MB
Container Path:  /etc/nginx/conf.d
Read-only:       ☑ (dicentang)
```

---

#### Opsi 2: Gunakan Volume yang Sudah Ada

Mount volume yang sudah ada:

![Image: Existing volume selector](/images/containers/deploy-step6-existing.png)

**Kapan digunakan**:
- Berbagi data antar container
- Menggunakan kembali volume dari container yang dihapus
- Mount data yang sudah diisi sebelumnya

**Kolom**:
1. **Select Volume** - Pilih dari dropdown
2. **Container Path** - Tempat mount di dalam container
3. **Read-only** - Centang untuk mencegah penulisan

---

### Tabel Volume

Setelah menambah volume, akan muncul dalam tabel:

![Image: Volume table](/images/containers/deploy-step6-volume-table.png)

**Kolom**:
- **Name** - Identifikasi volume
- **Host Path** - Tempat disimpan di host (dibuat otomatis)
- **Container Path** - Titik mount di dalam container
- **Size** - Ukuran volume dalam MB
- **Read Only** - Apakah volume bersifat baca-saja
- **Actions** - Tombol hapus

**Indikator badge**:
- **New** (hijau) - Volume akan dibuat
- **Existing** - Volume sudah ada

### Hapus Volume

Klik **ikon tempat sampah** untuk menghapus volume mount:

![Image: Remove volume button](/images/containers/deploy-step6-remove-volume.png)

---

### Pola Volume Umum

**Data database**:
```
PostgreSQL: /var/lib/postgresql/data
MySQL:      /var/lib/mysql
MongoDB:    /data/db
Redis:      /data
```

**Data aplikasi**:
```
Upload:     /app/uploads
Media:      /app/media
Storage:    /app/storage
```

**Konfigurasi**:
```
Nginx:      /etc/nginx/conf.d (baca-saja)
App config: /app/config (baca-saja)
```

**Log**:
```
Log app:    /app/logs
Log Nginx:  /var/log/nginx
```

---

## Langkah 7: Autentikasi Registry Pribadi (Opsional)

Autentikasi dengan registry Docker pribadi:

![Image: Private registry section](/images/containers/deploy-step7-registry.png)

### Aktifkan Autentikasi

Centang **"Use private registry authentication"**:

![Image: Enable private registry checkbox](/images/containers/deploy-step7-enable.png)

### Konfigurasi Kredensial

![Image: Registry authentication fields](/images/containers/deploy-step7-fields.png)

**Kolom**:
1. **Registry Username** - Username atau service account Anda
2. **Registry Password** - Password atau access token
3. **Registry Server** - Alamat server (opsional untuk Docker Hub)

---

### Repository Pribadi Docker Hub

Untuk repository pribadi Docker Hub:

![Image: Docker Hub auth example](/images/containers/deploy-step7-dockerhub.png)

```
Username: your-dockerhub-username
Password: your-dockerhub-password (atau access token)
Server:   (kosongkan untuk Docker Hub)
```

**Tips**: Gunakan access token Docker Hub alih-alih password untuk keamanan yang lebih baik.

---

### GitHub Container Registry (ghcr.io)

Untuk paket GitHub:

![Image: GitHub registry auth example](/images/containers/deploy-step7-github.png)

```
Username: your-github-username
Password: ghp_your_personal_access_token
Server:   ghcr.io
```

**Membuat token GitHub**:
1. GitHub → Settings → Developer settings → Personal access tokens
2. Generate token baru dengan scope `read:packages`
3. Gunakan token sebagai password

---

### GitLab Container Registry

Untuk paket GitLab:

![Image: GitLab registry auth example](/images/containers/deploy-step7-gitlab.png)

```
Username: your-gitlab-username
Password: your-gitlab-access-token
Server:   registry.gitlab.com
```

---

### Registry Lainnya

**Azure Container Registry**:
```
Server: yourregistry.azurecr.io
```

**Google Container Registry**:
```
Server: gcr.io
Username: _json_key
Password: <service account JSON>
```

**Registry yang di-host sendiri**:
```
Server: registry.company.com:5000
```

---

## Langkah 8: Tinjau dan Deploy

Tinjau semua konfigurasi sebelum deployment:

![Image: Deployment summary](/images/containers/deploy-step8-review.png)

**Periksa**:
- ✅ Nama container unik dan deskriptif
- ✅ Nama image sudah benar (dengan tag)
- ✅ Resource sesuai untuk beban kerja
- ✅ Port dipetakan dengan benar
- ✅ Variabel lingkungan sudah diset
- ✅ Volume dikonfigurasi untuk data persisten

### Tombol Deploy

Klik **"Deploy Container"** untuk memulai deployment:

![Image: Deploy button](/images/containers/deploy-step8-button.png)

**Status tombol**:
- **Enabled** - Siap untuk di-deploy
- **Disabled** - Ada kolom yang belum terisi
- **Uploading...** - Sedang mengunggah tarball image (jika menggunakan upload)
- **Loading...** - Sedang membuat container

---

## Langkah 9: Progress Deployment

Setelah mengklik deploy, Anda akan diarahkan ke halaman detail container:

![Image: Container deployment progress](/images/containers/deploy-step9-progress.png)

### Tahapan Deployment

Perhatikan transisi status container:

**Creating** 🟡 (1-2 detik):
```
Membuat Firecracker microVM...
```
![Image: Creating state badge](/images/containers/deploy-step9-creating.png)

- **Warna Badge**: Kuning
- **Status**: Proses pembuatan VM awal

**Booting** ⚪ (2-3 detik):
```
Booting microVM dengan container runtime...
```
![Image: Booting state badge](/images/containers/deploy-step9-booting.png)

- **Warna Badge**: Abu-abu
- **Status**: MicroVM sedang dimulai

**Initializing** 🔵 (2-5 detik):
```
Menjalankan Docker daemon...
Mempersiapkan lingkungan container...
```
![Image: Initializing state badge](/images/containers/deploy-step9-initializing.png)

- **Warna Badge**: Cyan (biru muda)
- **Status**: Docker daemon dimulai, mempersiapkan runtime container

**Menarik image** (10-60 detik, bergantung ukuran image):
```
Menarik alpine/nginx:latest...
Unduhan sedang berlangsung...
```

**Running** 🟢 - Deployment selesai!
```
Container sekarang berjalan
```
![Image: Running state badge](/images/containers/deploy-step9-running.png)

- **Warna Badge**: Hijau
- **Status**: Container aktif dan beroperasi

---

### Monitor Deployment

Selama deployment, Anda dapat:

**Lihat log**:
- Klik tab **"Logs"**
- Lihat log deployment secara real-time
- Pantau progress pull Docker

![Image: Deployment logs](/images/containers/deploy-step9-logs.png)

**Cek event**:
- Klik tab **"Events"**
- Lihat timeline event deployment

**Refresh status**:
- Klik tombol **"Refresh"** untuk memperbarui status

---

## Contoh Lengkap

### Contoh 1: Nginx Web Server

Hosting situs web statis sederhana:

![Image: Nginx deployment example](/images/containers/example-nginx.png)

**Konfigurasi**:
```
Name:        nginx-prod
Image:       nginx:alpine (Docker Hub)
CPU:         0.5 vCPU
Memory:      256 MB

Port Mappings:
  8080:80 (TCP)

Volume Mounts:
  Volume Baru:
    Name: nginx-html
    Size: 1024 MB
    Container Path: /usr/share/nginx/html
    Read-only: Tidak
```

**Akses**:
- Buka browser: `http://<host-ip>:8080`
- Upload file ke volume untuk konten

---

### Contoh 2: Database PostgreSQL

Database produksi dengan penyimpanan persisten:

![Image: PostgreSQL deployment example](/images/containers/example-postgres.png)

**Konfigurasi**:
```
Name:        postgres-main
Image:       postgres:15-alpine (Docker Hub)
CPU:         2 vCPU
Memory:      2048 MB

Port Mappings:
  5432:5432 (TCP)

Environment Variables:
  POSTGRES_PASSWORD=mySecretPassword123
  POSTGRES_USER=myapp
  POSTGRES_DB=production

Volume Mounts:
  Volume Baru:
    Name: postgres-data
    Size: 10240 MB (10 GB)
    Container Path: /var/lib/postgresql/data
    Read-only: Tidak
```

**Koneksi**:
```bash
psql -h <host-ip> -p 5432 -U myapp -d production
```

---

### Contoh 3: Redis Cache

Cache in-memory dengan persistensi:

![Image: Redis deployment example](/images/containers/example-redis.png)

**Konfigurasi**:
```
Name:        redis-cache
Image:       redis:7-alpine (Docker Hub)
CPU:         1 vCPU
Memory:      1024 MB

Port Mappings:
  6379:6379 (TCP)

Environment Variables:
  (tidak ada - Redis menggunakan konfigurasi default)

Volume Mounts:
  Volume Baru:
    Name: redis-data
    Size: 2048 MB (2 GB)
    Container Path: /data
    Read-only: Tidak
```

**Koneksi**:
```bash
redis-cli -h <host-ip> -p 6379
```

---

### Contoh 4: Aplikasi Node.js

Aplikasi web dengan konfigurasi lingkungan:

![Image: Node.js deployment example](/images/containers/example-nodejs.png)

**Konfigurasi**:
```
Name:        api-server
Image:       node:20-alpine (Docker Hub)
CPU:         1 vCPU
Memory:      1024 MB

Port Mappings:
  3000:3000 (TCP)

Environment Variables:
  NODE_ENV=production
  PORT=3000
  DATABASE_URL=postgres://user:pass@db:5432/myapp
  API_KEY=abc123xyz
  LOG_LEVEL=info

Volume Mounts:
  Volume Baru:
    Name: app-logs
    Size: 1024 MB
    Container Path: /app/logs
    Read-only: Tidak
```

**Catatan**: Biasanya Anda akan membangun image kustom dengan kode aplikasi Anda.

---

### Contoh 5: Paket GitHub Pribadi

Deploy dari GitHub Container Registry:

![Image: GitHub package deployment](/images/containers/example-github.png)

**Konfigurasi**:
```
Name:        my-private-app
Image:       ghcr.io/mycompany/myapp:latest (Docker Hub)
CPU:         2 vCPU
Memory:      2048 MB

Port Mappings:
  8000:8000 (TCP)

Environment Variables:
  APP_ENV=production

Private Registry:
  ✓ Use private registry authentication
  Username: myusername
  Password: ghp_myPersonalAccessToken
  Server: ghcr.io
```

---

## Pemecahan Masalah

### Masalah: Error "Image not found"

**Gejala**:
- Container tertahan di status "Creating" atau "Error"
- Log menampilkan "image not found" atau "pull failed"

![Image: Image not found error](/images/containers/troubleshoot-image-not-found.png)

**Solusi**:
1. **Periksa nama image**:
   - Pastikan ejaan dan tag benar
   - Contoh: `nginx:alpine` bukan `nginx:alpne`

2. **Verifikasi image ada**:
   - Cari di Docker Hub: https://hub.docker.com
   - Pastikan tag tersedia

3. **Periksa jaringan**:
   - Pastikan host dapat menjangkau Docker Hub
   - Tes: `curl https://hub.docker.com`

4. **Rate limit**:
   - Batas Docker Hub: 100 pull/6j (anonim)
   - Tunggu atau autentikasi dengan akun Docker Hub

---

### Masalah: Container langsung keluar

**Gejala**:
- Container mencapai "Running" lalu menjadi "Stopped"
- Tidak ada error selama deployment

![Image: Container exited](/images/containers/troubleshoot-exited.png)

**Solusi**:
1. **Cek log**:
   - Buka tab Logs
   - Cari pesan error
   - Umum: variabel lingkungan yang hilang, error konfigurasi

2. **Verifikasi variabel lingkungan yang diperlukan**:
   - Beberapa image membutuhkan variabel tertentu
   - Contoh: PostgreSQL membutuhkan `POSTGRES_PASSWORD`

3. **Periksa dokumentasi image**:
   - Baca dokumentasi image di Docker Hub
   - Pastikan semua persyaratan terpenuhi

4. **Tes secara lokal terlebih dahulu**:
   ```bash
   docker run -it --rm nginx:alpine
   ```

---

### Masalah: Tidak dapat terhubung ke port yang di-expose

**Gejala**:
- Container "Running"
- Pemetaan port dikonfigurasi
- Koneksi ditolak atau timeout

![Image: Connection refused](/images/containers/troubleshoot-connection.png)

**Solusi**:
1. **Verifikasi container berjalan**:
   - Cek status "Running"
   - Lihat log untuk error

2. **Periksa pemetaan port**:
   - Pastikan port host sudah benar
   - Port container sesuai layanan
   - Contoh: Nginx mendengarkan di 80, bukan 8080

3. **Periksa firewall**:
   ```bash
   # Tes apakah port dapat diakses
   telnet <host-ip> 8080
   ```

4. **Verifikasi network bridge**:
   ```bash
   ip link show fcbr0
   ```

5. **Cek konflik port**:
   ```bash
   # Lihat apakah port sudah digunakan
   netstat -tlnp | grep 8080
   ```

---

### Masalah: Kehabisan memory

**Gejala**:
- Container crash atau restart
- Log menampilkan "OOM killed" atau "out of memory"

![Image: OOM error](/images/containers/troubleshoot-oom.png)

**Solusi**:
1. **Tingkatkan batas memory**:
   - Hentikan container
   - Edit konfigurasi
   - Tingkatkan memory
   - Restart container

2. **Periksa kebutuhan image**:
   - Beberapa database butuh memory minimal
   - PostgreSQL: minimal 1 GB direkomendasikan
   - MongoDB: minimal 2 GB direkomendasikan

3. **Monitor penggunaan aktual**:
   - Buka tab Stats
   - Cek penggunaan memory
   - Set batas sedikit di atas penggunaan puncak

---

### Masalah: Data volume tidak tersimpan

**Gejala**:
- Data hilang setelah container di-restart
- Perubahan tidak tersimpan

![Image: Data not persisting](/images/containers/troubleshoot-volume.png)

**Solusi**:
1. **Verifikasi volume di-mount**:
   - Buka tab Config
   - Periksa bagian Volume Mounts
   - Pastikan container path sudah benar

2. **Periksa path yang benar**:
   - Image berbeda menyimpan data di path berbeda
   - PostgreSQL: `/var/lib/postgresql/data`
   - MySQL: `/var/lib/mysql`
   - MongoDB: `/data/db`

3. **Verifikasi tidak baca-saja**:
   - Pastikan volume tidak ditandai baca-saja
   - Tambah ulang volume tanpa flag baca-saja

4. **Periksa izin volume**:
   - Beberapa container membutuhkan UID/GID tertentu
   - Cek log untuk error izin

---

### Masalah: Autentikasi registry pribadi gagal

**Gejala**:
- Error "authentication required"
- Error "unauthorized"
- Pull gagal

![Image: Auth failed](/images/containers/troubleshoot-auth-failed.png)

**Solusi**:
1. **Verifikasi kredensial**:
   - Periksa username sudah benar
   - Periksa password/token sudah benar
   - Tidak ada typo atau spasi ekstra

2. **Periksa izin token**:
   - GitHub: Perlu scope `read:packages`
   - GitLab: Perlu scope `read_registry`

3. **Verifikasi alamat server**:
   - GitHub: `ghcr.io`
   - GitLab: `registry.gitlab.com`
   - Azure: `yourregistry.azurecr.io`

4. **Tes kredensial secara lokal**:
   ```bash
   docker login ghcr.io -u username -p token
   ```

---

### Masalah: Upload gagal atau sangat lambat

**Gejala**:
- Upload macet atau sangat lambat
- Browser timeout
- Upload gagal dengan error

![Image: Upload failed](/images/containers/troubleshoot-upload.png)

**Solusi**:
1. **Periksa ukuran file**:
   - Image besar (>1 GB) membutuhkan waktu lama
   - Gunakan Docker Hub untuk image besar

2. **Periksa jaringan**:
   - Kecepatan upload bergantung pada koneksi
   - Gunakan koneksi kabel jika memungkinkan

3. **Kompres image**:
   ```bash
   docker save myapp:latest | gzip > myapp.tar.gz
   ```

4. **Gunakan Docker Hub**:
   - Push ke Docker Hub dari lokal
   - Pull dari Docker Hub saat deployment

---

## Praktik Terbaik

### Deployment

✅ **Tes secara lokal terlebih dahulu**:
```bash
# Tes image berfungsi sebelum di-deploy
docker run -it --rm -p 8080:80 nginx:alpine
```

✅ **Gunakan nama yang deskriptif**:
```
Baik:  prod-api-gateway, staging-postgres, redis-sessions
Buruk: container1, test, my-container
```

✅ **Tentukan versi image**:
```
Baik:  postgres:15.3-alpine, node:20.10-alpine
Buruk: postgres:latest, node
```

✅ **Mulai kecil, tingkatkan jika perlu**:
- Deploy dengan resource minimal
- Monitor penggunaan di tab Stats
- Tingkatkan resource sesuai kebutuhan

---

### Keamanan

✅ **Gunakan variabel lingkungan untuk rahasia**:
- Jangan hardcode password di dalam image
- Gunakan variabel lingkungan untuk menginjeksi rahasia
- Rotasi kredensial secara berkala

✅ **Gunakan volume baca-saja untuk konfigurasi**:
```
File konfigurasi:  Baca-saja ✓
Data aplikasi:     Baca-tulis
```

✅ **Batasi expose port**:
- Hanya expose port yang diperlukan
- Gunakan port host non-standar
- Pertimbangkan aturan firewall

✅ **Selalu perbarui image**:
- Secara berkala pull versi terbaru
- Cek pembaruan keamanan
- Rebuild dengan image base baru

---

### Performa

✅ **Gunakan image Alpine**:
```
nginx:alpine    (40 MB)  vs  nginx:latest     (187 MB)
postgres:15-alpine (230 MB) vs  postgres:15  (420 MB)
```

✅ **Sesuaikan ukuran resource**:
- Jangan over-alokasi (membuang resource)
- Jangan under-alokasi (menyebabkan kegagalan)
- Monitor dan sesuaikan

✅ **Gunakan volume secara bijak**:
- Hanya mount yang diperlukan
- Ukuran volume secukupnya
- Jangan gunakan volume untuk data sementara

---

### Pemeliharaan

✅ **Dokumentasikan konfigurasi Anda**:
- Simpan catatan deployment
- Dokumentasikan variabel lingkungan
- Catat pemetaan port dan volume

✅ **Monitor secara berkala**:
- Cek log untuk error
- Monitor penggunaan resource
- Perhatikan perilaku yang tidak biasa

✅ **Rencanakan untuk data**:
- Selalu gunakan volume untuk database
- Backup rutin (fitur mendatang)
- Uji proses pemulihan

---

## Referensi Cepat

### Kolom yang Diperlukan

| Kolom | Diperlukan | Default |
|-------|------------|---------|
| Nama Container | Ya | - |
| Image | Ya | - |
| Batas CPU | Tidak | 1 vCPU |
| Batas Memory | Tidak | 512 MB |

### Kolom Opsional

| Kolom | Kapan Digunakan |
|-------|----------------|
| Pemetaan Port | Saat layanan butuh akses eksternal |
| Variabel Lingkungan | Saat image butuh konfigurasi |
| Volume Mount | Saat data harus persisten |
| Auth Registry | Saat menggunakan image pribadi |

### Waktu Deployment

| Tahap | Durasi |
|-------|--------|
| Membuat VM | 1-2 detik |
| Booting VM | 2-3 detik |
| Startup Docker | 2-5 detik |
| Pull image kecil (Alpine) | 5-15 detik |
| Pull image menengah | 15-30 detik |
| Pull image besar | 30-120 detik |
| **Total (Alpine)** | **~15-30 detik** |
| **Total (Standar)** | **~30-90 detik** |

---

## Referensi Status Container

### Semua Kemungkinan Status

| Status | Warna Badge | Emoji | Deskripsi |
|--------|-------------|-------|-----------|
| **Creating** | Kuning | 🟡 | Pembuatan VM awal, menjalankan Firecracker |
| **Booting** | Abu-abu | ⚪ | MicroVM sedang booting |
| **Initializing** | Cyan | 🔵 | Docker daemon dimulai, mempersiapkan runtime |
| **Running** | Hijau | 🟢 | Container aktif dan beroperasi |
| **Stopped** | Merah | 🔴 | Container telah dihentikan |
| **Error** | Merah | ❌ | Container mengalami error |
| **Paused** | Kuning tua | 🟠 | Container dijeda/disuspend |

### Siklus Hidup Status

**Alur Deployment Normal**:
```
Creating (🟡) → Booting (⚪) → Initializing (🔵) → Running (🟢)
```

**Alur Stop**:
```
Running (🟢) → Stopped (🔴)
```

**Alur Error**:
```
Status Apa Pun → Error (❌)
```

**Alur Resume**:
```
Paused (🟠) → Running (🟢)
```

### Indikator Status di UI

Setiap status ditampilkan dengan:
- **Badge berwarna** - Identifikasi visual yang mudah
- **Indikator emoji** - Pengenalan status cepat (🟢🔴🟡🔵)
- **Teks status** - Nama status yang jelas

**Contoh di tabel**:
- Creating: Badge kuning dengan teks "Creating"
- Running: Badge hijau dengan teks "Running"
- Error: Badge merah dengan teks "Error"

---

## Langkah Selanjutnya

- **[Kelola Containers](manage-containers/)** - Start, stop, restart, hapus container
- **[Lihat Log](logs/)** - Streaming log dan debugging real-time
- **[Monitor Statistik](stats/)** - Metrik penggunaan resource dan performa
- **[Ikhtisar Container](./#getting-started)** - Pelajari lebih lanjut tentang container
