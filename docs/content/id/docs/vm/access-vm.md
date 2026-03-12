+++
title = "Akses ke VM"
description = "Hubungkan ke VM Anda melalui web console atau SSH"
weight = 42
date = 2025-12-16
+++

Pelajari cara mengakses mesin virtual Anda menggunakan web console atau SSH.

---

## Akses Web Console

Web console menyediakan akses terminal berbasis browser langsung di UI — tidak diperlukan perangkat lunak tambahan.

### Membuka Console

1. Pergi ke **Virtual Machines** dan klik nama VM Anda
2. Klik tab **Terminal**

![VM console with login credentials and terminal](/images/vm/vm-console.png)

### Kredensial Login

Kartu **Login Credentials** di atas terminal menampilkan nama pengguna dan kata sandi yang ditetapkan saat pembuatan VM. Anda dapat menyalin setiap nilai dengan tombol salin di sebelah kanan.

- **Username**: ditampilkan dalam warna oranye (mis. `root`)
- **Password**: ditampilkan dalam warna oranye (mis. `root`)

Ketik nilai-nilai ini pada prompt `login:` di terminal untuk masuk.

### Status Koneksi

Pojok kanan atas panel console menampilkan indikator **Connected** berwarna hijau dan tombol **Disconnect**. Gunakan Disconnect untuk menutup sesi WebSocket dengan bersih tanpa meninggalkan halaman.

### Tips Console

**Tempel ke console**: `Ctrl+Shift+V` (bukan `Ctrl+V`)

**Salin dari console**: Pilih teks dengan mouse, lalu `Ctrl+Shift+C`

**Bersihkan layar**: `clear` atau `Ctrl+L`

**Logout**: `exit` atau `Ctrl+D`

---

## Akses SSH

SSH memberikan performa yang lebih baik dan memungkinkan Anda menggunakan alat lokal seperti editor, SCP, dan port forwarding. Alamat IP VM ditampilkan di header halaman detail (mis. `192.168.18.4`).

### Prasyarat

- VM sedang **Running**
- VM memiliki alamat IP yang terlihat di header detail
- Mesin Anda dapat menjangkau jaringan VM

### Terhubung

```bash
ssh root@192.168.18.4
```

Pada koneksi pertama, konfirmasi sidik jari kunci host saat diminta.

### Port Kustom

```bash
ssh -p 2222 root@192.168.18.4
```

### Pintasan SSH Config

```
# ~/.ssh/config
Host my-vm
    HostName 192.168.18.4
    User root
    IdentityFile ~/.ssh/id_ed25519
```

Kemudian cukup jalankan `ssh my-vm`.

---

## Transfer File

### SCP

```bash
# Upload
scp /local/file.txt root@192.168.18.4:/root/

# Download
scp root@192.168.18.4:/root/file.txt /local/

# Upload directory
scp -r /local/dir root@192.168.18.4:/root/
```

### SFTP

```bash
sftp root@192.168.18.4
```

Perintah SFTP umum: `put`, `get`, `ls`, `cd`, `lcd`, `exit`.

---

## Pemecahan Masalah

### Console tidak terhubung atau menampilkan layar kosong

1. Verifikasi status VM adalah **Running**
2. Refresh browser
3. Coba browser yang berbeda (Chrome, Firefox, Edge)
4. Periksa bahwa koneksi WebSocket tidak diblokir oleh firewall atau proxy

### Tidak bisa tempel dengan Ctrl+V

Gunakan `Ctrl+Shift+V` sebagai gantinya, atau klik kanan → Paste.

### Console lambat/lag

Beralih ke SSH untuk pekerjaan interaktif — web console paling cocok untuk akses cepat dan output boot-time.

### SSH: Connection refused

- Verifikasi VM sedang berjalan dan memiliki IP
- Periksa SSH sedang berjalan di dalam VM melalui console:
  ```bash
  # Alpine
  rc-service sshd status

  # Ubuntu/Debian
  systemctl status sshd
  ```

### SSH: Permission denied (publickey)

- Konfirmasi SSH key telah ditambahkan saat pembuatan VM
- Jalankan `ssh -v root@<ip>` untuk melihat key mana yang sedang dicoba
- Beralih ke autentikasi kata sandi: `ssh -o PreferredAuthentications=password root@<ip>`

---

## Langkah Berikutnya

- **[Kelola VM](manage-vm/)** — Operasi start, stop, pause
- **[Pemantauan](monitoring/)** — Lihat metrik secara real-time
- **[Backup & Snapshot](backup-snapshot/)** — Lindungi data VM Anda
