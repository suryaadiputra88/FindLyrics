# FindLyrics 🎵

Perangkat lunak berbasis **Rust** berkecepatan tinggi untuk memindai koleksi musik lokal (FLAC, WAV, MP3, M4A, OGG), mencari lirik lagu secara otomatis via **Multi-Provider Lyrics Engine** (mengutamakan *time-synced lyrics* / format LRC), dan **langsung menyematkannya (*direct embed*)** ke dalam metadata file audio tanpa meninggalkan file `.lrc` terpisah.

Tersedia dalam 2 antarmuka:
1. **Desktop GUI (Tauri v2)**: Antarmuka visual modern dengan Folder Picker native, progress bar live saat scan & fetch, tabel lagu interaktif, tombol **Fetch Lyrics**, filter real-time, dan jendela modal preview lirik.
2. **Interactive CLI (Command Line)**: Tool terminal cepat dan otomatis dengan spinner dan live progress bar untuk scanning batch, server, dan skrip otomasi.

---

## 🌐 Multi-Provider Lyrics Waterfall

FindLyrics tidak bergantung pada satu sumber saja. Sistem menggunakan algoritma pencarian bertingkat (*waterfall*):

1. **LRCLIB API** (*Prioritas 1*):
   - Database lirik *open-source* dengan koleksi *time-synced* (LRC) dan *plain text* yang sangat presisi.
2. **Kugou Music API** (*Prioritas 2*):
   - Database musik raksasa dengan miliaran lirik *time-synced* berkualitas tinggi (sangat kaya untuk lagu-lagu hits internasional, rock, pop, asia, anime, dan rilisan klasik).
3. **Lyrics.ovh API** (*Prioritas 3 - Fallback Plain Text*):
   - Sumber cadangan cepat untuk lirik teks lengkap jika lirik berwaktu tidak ditemukan dan opsi `--synced-only` dimatikan.

Aplikasi secara otomatis mencatat dan menampilkan provider yang berhasil menemukan lirik pada setiap lagu.

---

## Fitur Utama

- **Batch Directory Scan**: Memindai seluruh file musik dalam satu direktori (atau secara rekursif).
- **Live Progress Monitoring**:
  - **Saat Scanning**: Menampilkan ticker nama file dan jumlah file yang terdeteksi secara real-time via event bridge Tauri.
  - **Saat Fetching**: Progress bar persentase, judul lagu yang sedang diproses, indikator provider aktif, chip statistik real-time (*Berhasil, Dilewati, Tidak Ada, Gagal*), dan tombol pembatalan (*Cancel/Stop*).
- **Direct Tag Embedding (Zero Clutter)**: Lirik ditanam langsung ke dalam tag standar audio:
  - **MP3**: ID3v2 frame `USLT`
  - **FLAC & OGG**: Vorbis Comment field `LYRICS`
  - **WAV**: ID3v2 chunk di dalam container RIFF
  - **M4A / AAC**: MP4 atom `©lyr`
- **Smart Duration Tolerance**: Memverifikasi durasi lagu dengan lirik dari provider ($\le \pm 3$ detik secara default) agar lirik sinkron sempurna dan tidak salah versi lagu.
- **Title Normalization**: Membersihkan judul dari kata-kata seperti `(Remastered 2021)`, `[Official Music Video]`, `feat. ...`, dsb.
- **Modal Preview Lirik**: Tinjau lirik yang tersimpan atau baru didownload langsung di aplikasi.
- **Safety First**: File yang sudah memiliki lirik tidak akan ditimpa kecuali opsi *Overwrite* diaktifkan.

---

## 1. Menjalankan Desktop GUI (Tauri)

### Prasyarat
- Node.js (v18+)
- Rust toolchain

### Menjalankan dalam Mode Pengembangan
```bash
# Menjalankan Desktop GUI
npm run tauri dev
```

### Build Aplikasi Desktop Standalone
```bash
npm run tauri build
# Binary GUI dan macOS .app / Windows .msi / Linux .deb akan dibuat di:
# src-tauri/target/release/bundle/
```

---

## 2. Menjalankan CLI (Terminal Tool)

### Menjalankan Langsung via Cargo
```bash
cargo run -- /path/to/folder_musik
```

### Build Binary Release Mandiri
```bash
cargo build --release
# Binary CLI: target/release/findlyrics
```

### Opsi Flag CLI

| Flag | Parameter | Deskripsi |
| :--- | :--- | :--- |
| `-r, --recursive` | - | Pindai subdirektori secara rekursif |
| `-o, --overwrite` | - | Timpa file yang sudah memiliki lirik |
| `-s, --synced-only` | - | Hanya simpan jika menemukan lirik berwaktu (*time-synced*) |
| `-t, --tolerance` | `<DETIK>` | Batas toleransi selisih durasi lagu vs lirik (default: `3.0` detik) |
| `-d, --dry-run` | - | Simulasi pencarian lirik tanpa memodifikasi file audio |
| `-v, --verbose` | - | Menampilkan log detail per file |
| `-y, --yes` | - | Langsung jalankan Fetch Lyrics tanpa prompt interaktif |

**Contoh CLI:**
```bash
# Scan folder musik dan jalankan dry-run
findlyrics ~/Music/Rock --dry-run

# Pindai rekursif dan hanya embed synced lirik
findlyrics ~/Music/HiRes -r --synced-only
```

---

## Pengujian (Testing)

Jalankan seluruh rangkaian automated unit & multi-provider tests:
```bash
cargo test
```

---

## Lisensi
MIT License
