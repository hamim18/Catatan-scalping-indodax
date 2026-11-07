# PROGRESS: Catatan Scalping

Terakhir diperbarui: 7 November 2026 (T1.2)
Status: **Fase 0 selesai. Fase 1 berjalan:** T1.1a, T1.1b, T1.1c selesai. T1.2 (rekonsiliasi REST + penyimpanan SQLite + snapshot saldo awal) **lolos uji sinkron pertama di HP** (7 Nov 2026: 32 trade tersimpan, 15 order tampil). Sisa uji: sinkron kedua harus 0 trade baru dan pencocokan satu order dengan struk, lalu tandai Selesai dan commit. Berikutnya T1.3.

## Aturan kerja
- Satu task selesai = satu `git commit`.
- Setelah tiap task: perbarui file ini, lalu commit.
- Jangan pernah commit `.env` atau isi `data/` (sudah ada di `.gitignore`).
- Aplikasi hanya membaca data. Key API wajib izin **baca saja**.

---

## 1. Sudah dikerjakan

| Task | Hasil | Status |
| --- | --- | --- |
| T0.1 | Struktur repo, PRD, prototype UI, `.gitignore`, `.env.example`, README | Selesai |
| T0.2 | Skrip `tools/fase0_validasi.py` (SHA256, `--diagnosa`, ringkasan per order) | Selesai |
| T0.3 | Skrip dijalankan dengan key baca saja, trade ICNT nyata berhasil ditarik | Selesai |
| T0.4 | Fee dan nilai trade dari API dicocokkan dengan struk Indodax (beli dan jual 5 Nov) | Selesai |
| T0.5 | Rumus modal efektif terkonfirmasi, PRD dan prototype dikoreksi (v1.1) | Selesai |
| T1.0 | Keputusan teknologi: Rust untuk logika/backend di HP, Kotlin/Java hanya klien. PRD v1.2 (bagian 10 baru) | Selesai |
| T1.1a | Workspace Rust `core/` (crate `scalping-core`): model `myTrades`, HMAC-SHA256, klien REST baca saja, gabung per order + anti-dobel `tradeId`, modal efektif, TP/SL, profit jual. 9 tes lolos; hasil pada 18 trade nyata sama dengan skrip Python | Selesai |
| T1.1c | Parser `/api/v2/account` (`core/src/account.rs`): saldo IDR (tersedia+terkunci), cek key baca saja, `IndodaxClient::ambil_akun()`. 12 tes lolos. Penyimpanan snapshot menunggu lapisan penyimpanan (T1.2) | Selesai |
| T1.1b | Proyek Android `app/` (AGP 9.2.1, Kotlin bawaan AGP, tanpa Compose, UI dari kode, minSdk 30), crate `ffi/` (UniFFI 0.31), crate `uniffi-bindgen/`, `tools/bangun_android.sh`. Layar koneksi: simpan key terenkripsi (Android Keystore), tes `/account`, tolak simpan bila key punya izin trade/withdraw. Build APK berhasil (Gradle 9.8.2). Uji di HP lolos: tes koneksi tersambung, IDR ada, izin baca saja, kunci tetap tersimpan setelah aplikasi dibuka ulang | Selesai |
| T1.2 | `core/src/store.rs` (SQLite, rusqlite bundled: tabel trade/meta/snapshot, anti-dobel `trade_id` PRIMARY KEY), `core/src/sync.rs` (rekonsiliasi per koin, rentang maks 6 hari, belah rentang bila hasil penuh, tumpang tindih 1 menit, batas waktu per koin hanya maju bila sukses, snapshot saldo awal, tolak key bukan baca saja), FFI `sinkron_sekarang` dan `order_terbaru`, tombol "Sinkron sekarang" di layar Android. 11 tes baru ditulis (store 4, sync 7) | **Lolos uji HP (sinkron pertama); sisa 2 uji, lihat Catatan T1.2** |

### Hasil pencocokan dengan struk (ICNT, 5 November 2026)

| Besaran | Struk Indodax | API (`myTrades`) | Cocok |
| --- | --- | --- | --- |
| Beli: koin diterima | 16,35667833 | `qty` 16.35667833 | Ya |
| Beli: biaya | 133 | `commission` 133 (idr) | Ya |
| Beli: Total Diterima (IDR) | 65.393 | `quoteQty` 65393 | Ya |
| Beli: Order Terisi | 65.526 | `quoteQty` + `commission` = 65.526 | Ya |
| Beli: Jumlah Order | 65.529 | tidak ada di API | Anggaran order, bukan uang yang keluar |
| Jual: nilai terisi | 66.113 | `quoteQty` 66113 | Ya |
| Jual: biaya | 207 | `commission` 207 (idr) | Ya |
| Jual: Total Diterima | 65.906 | `quoteQty` - `commission` = 65.906 | Ya |

### Catatan T1.2
- Keputusan (usulan, belum dikonfirmasi pengguna): SQLite dikelola dari Rust (bukan Room di Kotlin); koin yang dipindai = saldo koin tidak nol + koin yang sudah ada di riwayat tersimpan + `koin_tambahan`; sinkron pertama menarik 7 hari ke belakang (parameter `hari_awal`, maks 30).
- Kelemahan yang diketahui: koin yang sudah terjual habis SEBELUM sinkron pertama dan tidak ada di daftar tambahan tidak terpindai. Atasi dengan `koin_tambahan` bila perlu.
- Snapshot saldo awal = saldo IDR (tersedia + terkunci) saat sinkron pertama. Koin yang sudah dipegang tidak dihitung (sesuai pertanyaan terbuka no. 6, default diabaikan).
- Limit 100 per permintaan (sama dengan skrip Fase 0). Hasil penuh membuat rentang dibelah; bila rentang 1 menit masih penuh, sinkron berhenti dengan error (tidak diam-diam kehilangan trade).
- Database: `getDatabasePath("catatan.db")` (penyimpanan privat aplikasi). Berisi trade saja, tanpa key.
- Pertama kali build: `cargo update` lalu `bash tools/bangun_android.sh --pasang <serial>`. Kompilasi SQLite bundled butuh compiler C dari NDK; bila gagal di `cargo ndk`, laporkan pesan errornya.
- Belum diperiksa: apakah `myTrades` menerima `limit` di atas 100 dan urutan hasilnya (kode tidak bergantung pada urutan).

### Temuan uji T1.2 di HP (7 November 2026)
- Sinkron pertama berhasil: koin dipindai `pxgidr`, `uaiidr`; **trade baru 32, total tersimpan 32**. Layar "Lihat order tersimpan" menampilkan order UAI dari 6 Nov 23:12 sampai 7 Nov 07:15 (beli dan jual, waktu WIB), termasuk satu order jual dengan 2 pengisian (7 Nov 06:02), jadi penggabungan per order bekerja pada data nyata.
- `pxgidr` ditolak Indodax (HTTP 400, kode 1109 "invalid symbol") dan dilewati; perbaikan "simbol ditolak dilewati" terbukti di HP. Penyebab pasti belum diketahui (dugaan: aset PXG di saldo tidak punya pasangan IDR).
- **Belum dikonfirmasi:** (a) sinkron kedua menghasilkan "Trade baru: 0" (bukti anti-dobel di perangkat nyata), (b) satu order dicocokkan dengan struk Indodax.
- **Temuan untuk T1.3:** koin yang dijual tidak selalu sama dengan koin yang dibeli, misalnya beli 14,29066022 (7 Nov 05:01) lalu jual 14 (05:15). Posisi harus menangani jual sebagian dan sisa koin kecil (FR-13), serta beli berikutnya yang merata-ratakan sisa itu.
- **Temuan:** trade ICNT (3 dan 5 Nov) tidak ikut terpindai karena ICNT tidak lagi ada di saldo dan belum ada di riwayat tersimpan. Ini kelemahan yang sudah dicatat; pakai `koin_tambahan` bila ingin menarik koin lama. Belum ada kolom isian untuk itu di layar Android.
- Tampilan: koin ditampilkan apa adanya (contoh "koin 14"), tanpa pemisah ribuan. Cukup untuk uji; dirapikan di fase UI.

### Catatan T1.1b
- Versi dipilih dari dokumentasi resmi (dicari 7 Nov 2026): AGP 9.2.x butuh Gradle minimal 9.4.1, JDK 17, build-tools 36.0.0 (semua terpenuhi di lingkunganmu). AGP 9 membawa Kotlin sendiri, jadi plugin `org.jetbrains.kotlin.android` tidak dipakai.
- UniFFI 0.31.x, JNA 5.17.0 (aar). Binding Kotlin dibuat dari build host lalu dikompilasi bersama; `jniLibs/` dan `java/uniffi/` dibuat otomatis dan tidak di-commit.
- `minSdk = 30` (dugaan dari adb nirkabel); cek dengan `adb shell getprop ro.build.version.sdk`.
- Hanya ABI arm64-v8a yang dibangun.
- Peringatan build yang aman diabaikan: `Unable to strip libjnidispatch.so` (library bawaan JNA) dan deprecation Gradle 10 (berasal dari plugin Android).
- Bila ada lebih dari satu perangkat adb, pakai `bash tools/bangun_android.sh --hanya-pasang <serial>` (serial dari `adb devices`).

### Catatan T1.1a
- Tes Rust memakai angka struk ICNT 5 Nov (modal efektif 4.006,07, TP 4.046, SL 3.966, hasil +380 / +0,58%).
- Pencocokan Rust vs Python pada `raw_icntidr_20261106_202131.json`: modal efektif, IDR keluar, IDR diterima sama untuk semua order.
- Belum diuji langsung ke server Indodax dari Rust (`cek_koneksi`); jalankan di komputermu.
- Fee selain IDR sengaja ditolak (error), karena belum pernah ditemui.
- Pembulatan TP/SL ke rupiah utuh hanya untuk tampilan (`bulatkan_idr`); aturan tick harga per koin belum diperiksa.

### Kesimpulan Fase 0
- **Fee dalam IDR.** Koin yang diterima tidak dipotong fee.
- **Modal efektif** = (`quoteQty` + `commission`) / `qty`, yaitu "Order Terisi" di struk dibagi koin. Untuk ICNT 5 Nov: 65.526 / 16,35667833 = **4.006,07** (bukan 4.006,25 seperti PRD lama, yang memakai "Jumlah Order" 65.529).
- **Hasil bersih ICNT 5 Nov = +380 IDR (+0,58%)**, bukan +377. Selisih 3 IDR karena "Jumlah Order" adalah anggaran, bukan uang yang keluar. TP 1% = 4.046, SL 1% = 3.966 (tidak berubah setelah dibulatkan).
- **Tarif fee tidak tetap.** Beli limit sekitar 0,10% (data 3 Nov), beli market sekitar 0,20% (5 Nov), jual limit sekitar 0,31%. Aplikasi harus memakai `commission` dari API.
- **Beli bisa limit maupun market.** PRD dikoreksi.
- Satu order bisa terisi beberapa kali (order `icntidr-limit-4777436` = 2 pengisian), jadi pengumpulan per order diperlukan.
- Field `myTrades` yang dipakai: `tradeId` (anti-dobel), `orderId`, `qty`, `quoteQty`, `commission`, `commissionAsset`, `isBuyer`, `time`.

---

## 2. Masalah yang muncul

| Masalah | Penyebab | Status |
| --- | --- | --- |
| HTTP 403 "respons bukan JSON" | File skrip lama belum tertimpa file baru (hanya sebagian yang tersalin) | Terselesaikan |
| SHA256 atau SHA512 | Dokumen resmi terbaru: SHA256 untuk TAPI v2 | Terselesaikan |
| Format simbol dan parameter waktu | `icntidr` (huruf kecil), `startTime`/`endTime`, maks 7 hari | Terselesaikan |
| Trade 5 Nov tidak terlihat di salinan hasil | Hanya sebagian cetakan yang tersalin, trade-nya ada | Terselesaikan |
| "Jumlah Order" (65.529) beda dengan uang yang keluar (65.526) | Jumlah Order = anggaran order. Yang dipakai: "Order Terisi" | Terselesaikan |
| Format respons `/api/v2/account` belum diketahui | Skrip lama hanya memeriksa HTTP 200 | Terselesaikan (T1.1c): `balances[{asset,free,locked}]`, flag `canTrade/canWithdraw/canWithdrawFiat` |
| Apakah aset IDR selalu ada di `balances` | Daftar tampilan terpotong | Belum dikonfirmasi; parser memberi error bila IDR tidak ada (`cek_koneksi` menampilkan ya/TIDAK) |
| `adb: more than one device/emulator` saat memasang APK | Lebih dari satu perangkat adb tersambung | Terselesaikan: `tools/bangun_android.sh` menerima serial dan menampilkan daftar bila ambigu |
| Sinkron gagal HTTP 400 kode 1109 "invalid symbol" (uji T1.2 di HP, 7 Nov) | Simbol disusun dari semua aset bersaldo; `pxgidr` ditolak Indodax | Terselesaikan: simbol ditolak dilewati dan ditampilkan; terbukti di HP (`pxgidr` dilewati, `uaiidr` tersinkron) |
| Koin yang sudah tidak dipegang (contoh ICNT) tidak terpindai | Daftar koin dari saldo + riwayat tersimpan | Dikenal; solusi sementara `koin_tambahan`, belum ada isiannya di UI |
| Build SQLite bundled untuk Android (T1.2) | Butuh compiler C dari NDK lewat `cargo ndk` | Belum diuji |
| Selisih 1 rupiah antara hasil jual dan modal beli berikutnya (data 3 Nov) | Diduga pembulatan | Belum dikonfirmasi, tidak berdampak |
| Apakah sisa 3 IDR (anggaran tak terpakai) kembali ke saldo | Belum dicek di saldo | Belum dikonfirmasi. Tidak memengaruhi rumus karena saldo diambil dari API |

---

## 3. Tahap selanjutnya

### Lingkungan pengembangan (dilaporkan 7 November 2026)
Arch Linux; Rust 1.98.1 dengan 4 target Android dan `cargo ndk`; JDK 17; Gradle 9.8.2 di PATH; Android SDK di `~/android-sdk` (platform 33 sampai 36, build-tools 34 sampai 36, NDK 26/27/28.2, `ANDROID_NDK_HOME` diatur); tanpa Android Studio, build lewat CLI; HP fisik via adb nirkabel.

### Teknologi (diputuskan 7 November 2026)
Rust untuk semua logika dan backend (berjalan di dalam HP), Kotlin/Java hanya sebagai klien Android (UI, foreground service, notifikasi, penyimpanan kunci). Jembatan: UniFFI (rencana, dipasang di T1.1b).

### Task berikutnya (urut)
| Task | Isi | Syarat |
| --- | --- | --- |
| T0.6 | Skrip tes token WebSocket privat dengan key v2 (`generate_token`). Ini satu-satunya ketidakpastian PRD yang tersisa | Key sudah ada |
| T1.2 | ~~Rekonsiliasi REST, penyimpanan lokal, snapshot saldo awal~~ (lolos uji sinkron pertama di HP; sisa uji sinkron ulang dan pencocokan struk) | T1.1b |
| T1.3 | Hitung posisi, modal efektif, TP/SL (FR-11 sampai FR-13) | T1.2 |
| T1.4 | Layanan latar dan WebSocket realtime (FR-06 sampai FR-08) | T1.3 |
| T1.5 | Notifikasi beli dan jual (FR-15 sampai FR-17) | T1.4 |

### Keputusan lain yang masih terbuka
| Hal | Pilihan | Status |
| --- | --- | --- |
| Tombol "Jual di Indodax" | Buka app Indodax atau dihapus | Belum |
| Estimasi hasil bersih di bawah TP/SL | Tampil atau polos | Belum |
| Setelan TP/SL baru | Hanya posisi baru atau juga yang terbuka | Belum |
| Koin yang dipegang sebelum app dipasang | Diabaikan atau diisi modalnya | Belum |
| Cadangan data (ekspor) | Perlu atau tidak, ke mana | Belum |
| Zona waktu WIB dan minggu mulai Senin | Perlu konfirmasi | Belum |

### Fase berikutnya
- **Fase 2**: setelan target, snapshot harian dan mingguan, deposit/withdraw IDR, progress target
- **Fase 3**: riwayat trade tertutup, grafik saldo dan modal aktif, tombol koreksi saldo
- **Fase 4**: statistik (win rate, total fee, persen profit termakan fee)

---

## 4. Cara instalasi

Kebutuhan: Python 3.8+, git, dan Rust (rustup). Skrip Python tidak butuh `pip install`.

1. Masuk ke folder proyek: `cd catatan-scalping`
2. Pastikan skrip versi terbaru: `python tools/fase0_validasi.py -h` harus menampilkan `--diagnosa`.
3. Buat key di indodax.com/trade_api: key **TAPI v2**, izin **baca saja**, tanpa pembatasan IP.
4. Salin `.env.example` menjadi `.env`, isi `INDODAX_API_KEY` dan `INDODAX_API_SECRET`.
5. Tes koneksi tanpa key: `python tools/fase0_validasi.py --diagnosa`
6. Tarik trade: `python tools/fase0_validasi.py icntidr --hari 7`
7. Hasil mentah tersimpan di `data/raw_<simbol>_<waktu>.json`.

Build Android (setelah T1.1b):
```
cargo update            # sekali saja, sesuaikan dependensi ke Rust-mu
bash tools/bangun_android.sh --pasang
```

Tes inti Rust:
```
cargo test -p scalping-core
cargo run -p scalping-core --example cek_data -- data/raw_icntidr_<waktu>.json
cargo run -p scalping-core --example cek_koneksi
```

Alur git setiap task:
```
git add .
git commit -m "T0.x: deskripsi singkat"
```

---

## 5. Struktur proyek

### Saat ini
```
catatan-scalping/
├── README.md
├── PROGRESS.md                      <- file ini
├── .gitignore                       (.env, data/*, __pycache__)
├── .env.example                     (salin menjadi .env)
├── data/                            (hasil tarikan, tidak di-commit)
│   └── .gitkeep
├── Cargo.toml                       (workspace Rust)
├── Cargo.lock
├── ffi/                             (crate scalping-ffi, UniFFI)
├── uniffi-bindgen/                  (pembuat binding Kotlin)
├── app/                             (proyek Gradle Android, modul :aplikasi)
├── core/                            (crate scalping-core)
│   ├── Cargo.toml
│   ├── src/  lib.rs error.rs model.rs sign.rs calc.rs client.rs account.rs store.rs sync.rs
│   └── examples/  cek_data.rs cek_koneksi.rs
├── docs/
│   ├── PRD_Aplikasi_Catatan_Trading_Crypto.md     (v1.2)
│   └── prototype/
│       └── Catatan_trading__prototype_UI.html
└── tools/
    ├── fase0_validasi.py            (hanya baca; SHA256; --diagnosa)
    ├── bangun_android.sh            (Rust -> binding -> APK -> adb)
    ├── fase0_akun.py                (bentuk respons /account, angka disamarkan)
    └── cek_lingkungan.py            (laporan versi Rust/Android/adb)
```

### Rencana
```
core/       Rust (sudah ada)
docs/       PRD, prototype, catatan hasil validasi
tools/      skrip bantu (validasi data, tes WebSocket)
```

---

## 6. Log commit
(isi hash dan pesan commit di sini)
- T1.2: `T1.2: penyimpanan SQLite, rekonsiliasi REST, snapshot saldo awal` (hash: ...)  <- commit setelah sinkron ulang = 0 trade baru dan satu order cocok dengan struk
- T1.1b: `T1.1b: proyek Android, jembatan UniFFI, layar koneksi` (hash: ...)
- T1.1c: `T1.1c: parser /account dan cek key baca saja` (hash: ...)
- T1.1a/T1.0: `T1.1a: inti Rust (klien Indodax, kalkulasi posisi), keputusan teknologi, PRD v1.2`
