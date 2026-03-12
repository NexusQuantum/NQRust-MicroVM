+++
title = "Containers"
description = "Deploy dan orkestrasi Docker container dengan isolasi microVM"
weight = 50
date = 2025-12-18
+++

Deploy Docker container dengan isolasi kuat dari Firecracker microVM — keunggulan terbaik dari kedua dunia.

---

## Apa itu Containers?

NQRust-MicroVM Containers menggabungkan **kemudahan Docker** dengan **isolasi keamanan Firecracker**:

![Image: Container architecture diagram](/images/containers/architecture-overview.png)

- ✅ **Kompatibilitas Docker** - Gunakan Docker image apa pun dari Docker Hub atau registry pribadi
- ✅ **Isolasi kuat** - Setiap container berjalan di dalam Firecracker microVM-nya sendiri
- ✅ **Docker API penuh** - Kompatibel dengan perintah dan alat Docker standar
- ✅ **Keamanan tingkat kernel** - Isolasi berbasis hardware melalui virtualisasi KVM
- ✅ **Deployment cepat** - Runtime Alpine Linux yang dioptimalkan untuk container boot dalam ~2-3 detik

---

## Arsitektur Container-per-VM

Berbeda dengan platform container tradisional, NQRust-MicroVM menggunakan model **Container-per-VM**:

![Image: Container-per-VM diagram](/images/containers/container-per-vm-diagram.png)

**Container Tradisional** (Docker, containerd):
```
┌─────────────────────────────────────┐
│     Host Kernel (Bersama)           │
├─────────────────────────────────────┤
│ Container 1 │ Container 2 │ Container 3 │
└─────────────────────────────────────┘
```

**NQRust-MicroVM Containers**:
```
┌──────────────────────────────────────────────┐
│        Host (KVM Hypervisor)                 │
├──────────────┬──────────────┬────────────────┤
│  MicroVM 1   │  MicroVM 2   │  MicroVM 3     │
│ ┌──────────┐ │ ┌──────────┐ │ ┌──────────┐  │
│ │Container │ │ │Container │ │ │Container │  │
│ │  nginx   │ │ │ postgres │ │ │  redis   │  │
│ └──────────┘ │ └──────────┘ │ └──────────┘  │
└──────────────┴──────────────┴────────────────┘
```

**Keuntungan**:
- **Isolasi lebih kuat** - Container tidak dapat meloloskan diri ke kernel host
- **Multi-tenancy lebih baik** - Isolasi aman antar container milik pengguna berbeda
- **Keamanan** - Permukaan serangan dibatasi pada batas VM
- **Fleksibilitas** - Setiap container dapat memiliki parameter kernel yang berbeda

---

## Cara Kerja Containers

### Alur Deployment

![Image: Container deployment flow](/images/containers/deployment-flow.png)

1. **Pilih Image**:
   - Pilih dari registry lokal (image yang sudah di-cache)
   - Ambil dari Docker Hub (mis., `nginx:latest`, `postgres:15`)
   - Unggah tarball kustom (diekspor dengan `docker save`)

2. **Konfigurasi Container**:
   - Tentukan batas resource (CPU, Memory)
   - Konfigurasi pemetaan port (expose layanan)
   - Tambahkan variabel lingkungan
   - Mount volume untuk data persisten
   - Opsional: Autentikasi registry pribadi

3. **Deploy**:
   - Manager membuat Firecracker microVM khusus
   - VM booting dengan Alpine Linux + Docker daemon
   - Docker menarik dan menjalankan image container Anda
   - Container dapat diakses melalui port yang dipetakan

4. **Akses**:
   - Web UI menampilkan status container
   - Lihat log secara real-time melalui WebSocket
   - Monitor penggunaan resource dengan statistik
   - Jalankan perintah shell di dalam container

---

## Status Container

Container melewati beberapa status selama siklus hidupnya:

![Image: Container state diagram](/images/containers/state-diagram.png)

| Status | Deskripsi | Aksi Tersedia |
|--------|-----------|---------------|
| **Creating** | VM sedang dibuat | Tunggu |
| **Booting** | VM sedang booting | Tunggu |
| **Initializing** | Docker daemon sedang dimulai | Tunggu |
| **Running** | Container aktif | Stop, Restart, Pause, Lihat Log, Shell |
| **Stopped** | Container dihentikan | Start, Delete |
| **Paused** | Container dijeda | Resume, Stop |
| **Error** | Deployment gagal | Lihat log, Delete, Coba lagi |

**Lini waktu deployment tipikal**:
- Membuat VM: 1-2 detik
- Booting VM: 2-3 detik
- Menarik image: 10-60 detik (bergantung pada ukuran image)
- Menjalankan container: 1-2 detik
- **Total**: 15-70 detik untuk deployment pertama

---

## Image yang Didukung

NQRust-MicroVM mendukung **Docker image apa pun** dari:

### Docker Hub (Registry Publik)

Image-image populer langsung dapat digunakan:

![Image: Docker Hub popular images](/images/containers/dockerhub-popular.png)

- **Web server**: `nginx`, `httpd`, `caddy`
- **Database**: `postgres`, `mysql`, `mongo`, `redis`, `mariadb`
- **Bahasa pemrograman**: `node`, `python`, `golang`, `openjdk`, `ruby`
- **Message queue**: `rabbitmq`, `nats`, `kafka`
- **Caching**: `redis`, `memcached`, `varnish`
- **Monitoring**: `prometheus`, `grafana`

**Contoh**: Deploy PostgreSQL 15:
```
Image: postgres:15
Environment: POSTGRES_PASSWORD=mypassword
Ports: 5432:5432
```

### Registry Pribadi

Autentikasi dengan registry pribadi:

![Image: Private registry authentication](/images/containers/private-registry-auth.png)

- Repository pribadi Docker Hub
- GitHub Container Registry (ghcr.io)
- GitLab Container Registry
- Azure Container Registry
- Google Container Registry
- Registry yang di-host sendiri

**Kolom autentikasi**:
- Username
- Password atau access token
- Server registry (mis., `ghcr.io`, `registry.gitlab.com`)

### Image Kustom (Upload)

Unggah tarball Docker image:

![Image: Upload custom image](/images/containers/upload-image.png)

**Ekspor image**:
```bash
docker save -o myimage.tar myimage:latest
```

**Unggah melalui UI**:
1. Pilih tab "Upload"
2. Pilih file `.tar` atau `.tar.gz`
3. Deploy container

---

## Kasus Penggunaan

### Hosting Aplikasi Web

Deploy web server dan aplikasi:

![Image: Web app hosting](/images/containers/use-case-web-app.png)

**Contoh: Situs statis Nginx**:
- Image: `nginx:alpine`
- Ports: `80:80`
- Volume: `/srv/www:/usr/share/nginx/html`
- Resource: 0.5 vCPU, 256 MB

### Layanan Database

Jalankan server database dengan penyimpanan persisten:

![Image: Database hosting](/images/containers/use-case-database.png)

**Contoh: Database PostgreSQL**:
- Image: `postgres:15-alpine`
- Ports: `5432:5432`
- Environment: `POSTGRES_PASSWORD=secret`
- Volume: `/srv/pgdata:/var/lib/postgresql/data`
- Resource: 2 vCPU, 2048 MB

### Lingkungan Development

Lingkungan development terisolasi per proyek:

![Image: Dev environments](/images/containers/use-case-dev-env.png)

**Contoh: Aplikasi Node.js**:
- Image: `node:20-alpine`
- Ports: `3000:3000`
- Volume: `/srv/app:/app` (mount kode sumber)
- Command: `npm run dev`

### Microservices

Deploy dan orkestrasi microservices:

![Image: Microservices architecture](/images/containers/use-case-microservices.png)

**Contoh: Stack e-commerce**:
- Frontend: `nginx:alpine` (port 80)
- API: `node:20-alpine` (port 3000)
- Database: `postgres:15` (port 5432)
- Cache: `redis:7-alpine` (port 6379)
- Queue: `rabbitmq:3-alpine` (port 5672)

Setiap layanan berjalan di microVM terisolasi dengan resource independen.

---

## Fitur Utama

### Manajemen Resource

![Image: Resource configuration](/images/containers/resource-management.png)

**Batas CPU**:
- Rentang: 0.1 hingga 16 core
- Kontrol granular (kelipatan 0.1)
- Alokasi CPU dedicated

**Batas Memory**:
- Rentang: 64 MB hingga 32 GB
- Mencegah penggunaan memory berlebih
- Perlindungan OOM

**Contoh konfigurasi**:
```
Layanan kecil:   0.5 vCPU, 512 MB
Layanan sedang:  2 vCPU, 2048 MB
Layanan besar:   4 vCPU, 8192 MB
```

---

### Konfigurasi Jaringan

![Image: Port mapping configuration](/images/containers/port-mapping.png)

**Pemetaan Port**:
- Petakan port host ke port container
- Mendukung protokol TCP dan UDP
- Beberapa pemetaan port per container
- Akses container dari luar

**Contoh**:
```
Host:Container  Protokol  Tujuan
8080:80         TCP       Web server
5432:5432       TCP       PostgreSQL
6379:6379       TCP       Redis
53:53           UDP       DNS server
```

---

### Volume Mount

![Image: Volume management](/images/containers/volume-mounts.png)

**Penyimpanan Persisten**:
- Mount direktori host ke dalam container
- Data bertahan melewati restart container
- Berbagi data antar container
- Akses baca-saja atau baca-tulis

**Dua jenis**:
1. **Volume baru** - Buat penyimpanan baru untuk container
2. **Volume yang sudah ada** - Pasang volume yang sebelumnya dibuat

**Contoh**:
```
Data database:  /srv/pgdata:/var/lib/postgresql/data
Aplikasi:       /srv/app:/app
Konfigurasi:    /srv/config:/etc/myapp (baca-saja)
Log:            /srv/logs:/var/log
```

---

### Variabel Lingkungan

![Image: Environment variables](/images/containers/env-vars.png)

Konfigurasi container dengan variabel lingkungan:

**Penggunaan umum**:
- Kredensial database
- API key dan token
- Konfigurasi aplikasi
- Feature flag
- Parameter runtime

**Contoh**:
```
POSTGRES_PASSWORD=mypassword
DATABASE_URL=postgres://user:pass@db:5432/mydb
NODE_ENV=production
API_KEY=abc123xyz
LOG_LEVEL=debug
```

---

### Log Real-time

![Image: Container logs streaming](/images/containers/logs-streaming.png)

**Fitur Log**:
- Streaming real-time via WebSocket
- Stream stdout/stderr terpisah
- Auto-scroll ke log terbaru
- Unduh log sebagai file teks
- Timestamp untuk setiap entri

**Gunakan log untuk**:
- Debug masalah aplikasi
- Monitor kesehatan aplikasi
- Lacak permintaan dan error
- Audit aktivitas container

---

### Monitor Resource

![Image: Container stats dashboard](/images/containers/stats-dashboard.png)

**Metrik yang dilacak**:
- Penggunaan CPU (%)
- Penggunaan Memory (MB / %)
- I/O Jaringan (byte masuk/keluar)
- I/O Disk
- Uptime

**Pembaruan real-time**:
- Auto-refresh setiap 5 detik
- Chart dan grafik
- Data historis

---

## Perbandingan dengan VM

| Fitur | Containers | VM |
|-------|------------|----|
| **Isolasi** | Tingkat kernel (microVM) | Virtualisasi penuh |
| **Waktu boot** | 2-3 detik | 5-10 detik |
| **Sumber image** | Docker Hub, registry | Kernel/rootfs kustom |
| **Kasus penggunaan** | Jalankan Docker image yang ada | OS/kernel kustom |
| **Deployment** | Tarik image, konfigurasi, jalankan | Bangun rootfs, konfigurasi kernel |
| **Ekosistem** | Ekosistem Docker | Ekosistem microVM kustom |
| **Overhead resource** | Rendah (Alpine + Docker) | Sangat rendah (kernel kustom) |
| **Fleksibilitas** | Aplikasi kompatibel Docker | Distribusi Linux apa pun |

**Kapan menggunakan Containers**:
- ✅ Anda memiliki Docker image yang sudah ada
- ✅ Anda menginginkan ekosistem Docker Hub
- ✅ Anda membutuhkan kompatibilitas Docker
- ✅ Deployment cepat penting

**Kapan menggunakan VM**:
- ✅ Anda membutuhkan kernel/OS kustom
- ✅ Anda menginginkan fleksibilitas maksimal
- ✅ Anda membutuhkan distro Linux tertentu
- ✅ Anda menginginkan overhead minimal

---

## Memulai

### Prasyarat

Sebelum men-deploy container:

1. **Image runtime container** harus tersedia:
   - Manager secara otomatis membangunnya saat setup
   - Tersimpan di `/srv/images/container-runtime.ext4`
   - Alpine Linux 3.18 + Docker 25.0.5

2. **Network bridge** harus dikonfigurasi:
   - Default: `fcbr0`
   - Setup: `sudo ./scripts/fc-bridge-setup.sh fcbr0 <interface>`

3. **Host dengan agent** harus terdaftar:
   - Setidaknya satu host harus online
   - Cek: Dashboard → Hosts

### Mulai Cepat

![Image: Quick start flow](/images/containers/quick-start.png)

1. **Buka halaman Containers**:
   - Navigasi ke "Containers" di sidebar

2. **Klik "Deploy Container"**:
   - Membuka formulir deployment

3. **Konfigurasi container**:
   - Name: `my-nginx`
   - Image: `nginx:alpine`
   - Ports: `8080:80`

4. **Deploy**:
   - Klik "Deploy Container"
   - Tunggu deployment (15-30 detik)

5. **Akses**:
   - Container berjalan di port 8080
   - Lihat log, statistik, dan shell

**Langkah selanjutnya**:
- **[Deploy Container](deploy-container/)** - Panduan deployment langkah demi langkah
- **[Kelola Containers](manage-containers/)** - Start, stop, restart, delete
- **[Lihat Log](logs/)** - Streaming log real-time
- **[Monitor Statistik](stats/)** - Monitor penggunaan resource

---

## Praktik Terbaik

### Pemilihan Image

✅ **Gunakan image berbasis Alpine** bila memungkinkan:
- Ukuran lebih kecil (nginx:alpine = 40 MB vs nginx:latest = 187 MB)
- Pull dan deploy lebih cepat
- Penggunaan resource lebih rendah

✅ **Tentukan versi spesifik**:
```
Baik:    postgres:15.3-alpine
Buruk:   postgres:latest
```

✅ **Gunakan image resmi** dari Docker Hub:
- Publisher terverifikasi
- Pembaruan keamanan
- Dokumentasi yang baik

---

### Alokasi Resource

✅ **Mulai kecil, tingkatkan jika perlu**:
```
Awal:       0.5 vCPU, 512 MB
Jika perlu: 1 vCPU, 1024 MB
Jika perlu: 2 vCPU, 2048 MB
```

✅ **Monitor penggunaan resource**:
- Cek tab Stats secara berkala
- Sesuaikan berdasarkan penggunaan aktual
- Hindari over-provisioning

❌ **Jangan over-alokasi**:
- Membuang resource host
- Membatasi jumlah container
- Meningkatkan biaya

---

### Manajemen Volume

✅ **Gunakan volume untuk data persisten**:
- Database: Selalu gunakan volume
- Konfigurasi: Mount sebagai baca-saja
- Log: Opsional (bisa gunakan log Docker)

✅ **Atur volume berdasarkan tujuan**:
```
/srv/container-data/postgres-data    → Database
/srv/container-data/app-uploads      → File pengguna
/srv/container-config/nginx          → Konfigurasi
```

❌ **Jangan simpan data di dalam container**:
- Data hilang saat container dihapus
- Tidak dapat dibagi antar container
- Sulit untuk di-backup

---

### Keamanan

✅ **Gunakan variabel lingkungan untuk rahasia**:
- Jangan hardcode password di dalam image
- Inject saat runtime melalui variabel lingkungan
- Rotasi kredensial secara berkala

✅ **Batasi port yang di-expose**:
- Hanya petakan port yang diperlukan
- Gunakan port host non-standar (mis., 8080 bukan 80)
- Pertimbangkan aturan firewall

✅ **Selalu perbarui image**:
- Cek pembaruan keamanan
- Rebuild dengan image base terbaru
- Pantau pengumuman CVE

---

### Monitoring

✅ **Cek log secara berkala**:
- Identifikasi error lebih awal
- Monitor kesehatan aplikasi
- Lacak aktivitas tidak biasa

✅ **Monitor penggunaan resource**:
- Cegah khabisan resource
- Identifikasi masalah performa
- Rencanakan kapasitas

✅ **Siapkan alert** (fitur mendatang):
- Container berhenti secara tak terduga
- Penggunaan resource tinggi
- Ambang batas error rate

---

## Keterbatasan

### Keterbatasan Saat Ini

**Satu container per VM**:
- Tidak dapat menjalankan beberapa container dalam satu VM
- Setiap container membutuhkan VM khusus
- Gunakan container terpisah untuk microservices

**Tidak ada orkestrasi container**:
- Belum ada auto-scaling
- Belum ada service discovery
- Belum ada load balancing
- Kelola container secara individual

**Tidak ada dukungan Docker Compose**:
- Tidak dapat deploy stack multi-container dengan file compose
- Deploy setiap layanan sebagai container terpisah
- Konfigurasi jaringan secara manual

**Alokasi resource**:
- Resource ditetapkan saat deployment
- Tidak dapat hot-resize CPU/memory (harus menghentikan container)
- Edit konfigurasi saat container dihentikan

### Peningkatan Mendatang

Fitur yang direncanakan:
- **Orkestrasi container** - Auto-scaling, service discovery
- **Docker Compose** - Deploy aplikasi multi-container
- **Jaringan container** - Virtual network, service mesh
- **Health check** - Restart otomatis saat gagal
- **Hot-resize resource** - Sesuaikan CPU/memory tanpa restart
- **Cache image** - Deployment lebih cepat dengan cache lokal

---

## Pemecahan Masalah

### Container tertahan di status "Creating"

**Penyebab**: Pembuatan VM gagal atau agent tidak merespons

**Solusi**:
1. Cek status host: Dashboard → Hosts
2. Pastikan agent berjalan
3. Cek log agent di host
4. Hapus dan buat ulang container

---

### Container tertahan di status "Booting"

**Penyebab**: VM gagal booting atau masalah jaringan

**Solusi**:
1. Cek network bridge: `ip link show fcbr0`
2. Lihat VM container: Klik "View Container VM"
3. Cek log VM
4. Pastikan image runtime container ada

---

### Tidak dapat menarik image dari Docker Hub

**Penyebab**: Masalah jaringan, rate limit, atau nama image tidak valid

**Solusi**:
1. Verifikasi image ada di Docker Hub
2. Cek nama image dan tag yang benar
3. Tunggu jika terkena rate limit (anonim: 100 pull/6j)
4. Gunakan pull terautentikasi dengan akun Docker Hub

---

### Container langsung keluar

**Penyebab**: Proses container crash atau salah konfigurasi

**Solusi**:
1. Lihat log container (tab Logs)
2. Cek variabel lingkungan
3. Verifikasi image sudah benar
4. Cek dependensi yang hilang

---

### Pemetaan port tidak berfungsi

**Penyebab**: Konflik port, firewall, atau masalah jaringan

**Solusi**:
1. Pastikan port belum digunakan
2. Cek aturan firewall host
3. Pastikan bridge networking dikonfigurasi
4. Coba port host yang berbeda

---

## Referensi Cepat

### Siklus Hidup Container

| Aksi | Tersedia Saat | Hasil |
|------|---------------|-------|
| **Start** | Stopped | Menjalankan container |
| **Stop** | Running | Menghentikan container dengan baik |
| **Restart** | Running | Menghentikan lalu menjalankan container |
| **Pause** | Running | Menjeda eksekusi container |
| **Resume** | Paused | Melanjutkan container yang dijeda |
| **Delete** | Stopped, Error | Menghapus container dan VM secara permanen |

### Contoh Image Umum

| Image | Ports | Environment | Kasus Penggunaan |
|-------|-------|-------------|-----------------|
| `nginx:alpine` | 80:80 | - | Web server statis |
| `postgres:15-alpine` | 5432:5432 | `POSTGRES_PASSWORD` | Database |
| `redis:7-alpine` | 6379:6379 | - | Cache/Queue |
| `node:20-alpine` | 3000:3000 | `NODE_ENV` | Aplikasi Node.js |
| `python:3.11-alpine` | 8000:8000 | - | Aplikasi Python |
| `mongo:7` | 27017:27017 | `MONGO_INITDB_ROOT_PASSWORD` | MongoDB |

### Panduan Resource

| Jenis Layanan | CPU | Memory |
|---------------|-----|--------|
| Situs statis | 0.5 | 256 MB |
| API server | 1-2 | 512-1024 MB |
| Database | 2-4 | 2048-4096 MB |
| Cache | 0.5-1 | 512-1024 MB |
| Message queue | 1-2 | 1024-2048 MB |

---

## Langkah Selanjutnya

- **[Deploy Container](deploy-container/)** - Panduan deployment lengkap
- **[Kelola Containers](manage-containers/)** - Operasi siklus hidup
- **[Lihat Log](logs/)** - Panduan streaming log real-time
- **[Monitor Statistik](stats/)** - Dashboard monitor resource
