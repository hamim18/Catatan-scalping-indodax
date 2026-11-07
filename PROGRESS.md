# PROGRESS: Catatan Scalping

Terakhir diperbarui: 7 November 2026 (T1.3 selesai)
Status: **Fase 0 selesai. Fase 1 berjalan:** T1.0 sampai T1.3 selesai dan lolos uji di HP (koneksi, sinkron, posisi dan TP/SL). Berikutnya: T0.6 (tes token WebSocket privat), lalu T1.4 (layanan latar + WebSocket realtime).

## Aturan kerja
- Satu task selesai = satu `git commit`.
- Setelah tiap task: perbarui file ini, lalu commit.
- Jangan pernah commit `.env` atau isi `data/` (sudah ada di `.gitignore`).
- Aplikasi hanya membaca data. Key API wajib izin **baca saja**.

---

## 1. Sudah dikerjakan

### 1a. Ringkasan riwayat (Fase 0 sampai T1.2)

| Task | Hasil singkat | Status |
| --- | --- | --- |
| T0.1 sampai T0.5 | Repo, PRD, prototype. Skrip `tools/fase0_validasi.py` menarik trade ICNT nyata dengan key baca saja. Fee dan nilai trade dicocokkan dengan struk Indodax (semua cocok). Rumus modal efektif terkonfirmasi, PRD v1.1 | Selesai |
| T1.0 | Keputusan teknologi: Rust untuk semua logika (jalan di HP), Kotlin/Java hanya klien Android. PRD v1.2 | Selesai |
| T1.1a | Workspace Rust `core/` (`scalping-core`): model `myTrades`, HMAC-SHA256, klien REST baca saja, gabung per order, anti-dobel `tradeId`, modal efektif, TP/SL, profit jual. Hasil sama dengan skrip Python | Selesai |
| T1.1c | Parser `/api/v2/account`: saldo IDR (tersedia + terkunci), cek key baca saja | Selesai |
| T1.1b | Proyek Android `app/` (AGP 9.2.1, tanpa Compose, UI dari kode, minSdk 30), `ffi/` (UniFFI 0.31), `uniffi-bindgen/`, `tools/bangun_android.sh`. Layar koneksi, kunci terenkripsi (Android Keystore), tolak key yang punya izin trade/withdraw. Lolos uji HP | Selesai |
| T1.2 | `store.rs` (SQLite, anti-dobel `trade_id`), `sync.rs` (rekonsiliasi per koin, rentang maks 6 hari, rentang dibelah bila hasil penuh, snapshot saldo awal, simbol ditolak Indodax dilewati), FFI `sinkron_sekarang` dan `order_terbaru`. Lolos uji HP 7 Nov: 32 trade UAI tersinkron, sinkron ulang tanpa dobel, cocok dengan struk | Selesai |

### 1b. Kesimpulan Fase 0 (masih dipakai)
- **Fee dalam IDR.** Koin yang diterima tidak dipotong fee. Fee selain IDR sengaja ditolak (error) karena belum pernah ditemui.
- **Modal efektif** = (`quoteQty` + `commission`) / `qty` = "Order Terisi" di struk dibagi koin. ICNT 5 Nov: 65.526 / 16,35667833 = **4.006,07**. "Jumlah Order" (65.529) hanya anggaran order, bukan uang yang keluar.
- **Hasil bersih jual** = "Total Diterima" (`quoteQty` - `commission`) dikurangi modal. ICNT 5 Nov: **+380 IDR (+0,58%)**, TP 1% = 4.046, SL 1% = 3.966.
- **Tarif fee tidak tetap** (beli limit ±0,10%, beli market ±0,20%, jual limit ±0,31%). Selalu pakai `commission` dari API.
- Beli bisa limit maupun market. Satu order bisa terisi beberapa kali, jadi pengumpulan per order wajib.
- Field `myTrades` yang dipakai: `tradeId`, `orderId`, `qty`, `quoteQty`, `commission`, `commissionAsset`, `isBuyer`, `time`.

### 1c. Catatan teknis yang masih berlaku
- **Build Android:** AGP 9.2.x butuh Gradle minimal 9.4.1, JDK 17, build-tools 36.0.0. AGP 9 membawa Kotlin sendiri (plugin `org.jetbrains.kotlin.android` tidak dipakai). UniFFI 0.31.x, JNA 5.17.0 (aar). Hanya ABI arm64-v8a. `jniLibs/` dan `java/uniffi/` dibuat otomatis, tidak di-commit.
- Peringatan build yang aman diabaikan: `Unable to strip libjnidispatch.so` dan deprecation Gradle 10.
- Bila ada lebih dari satu perangkat adb: `bash tools/bangun_android.sh --hanya-pasang <serial>` (serial dari `adb devices`).
- `minSdk = 30` adalah dugaan; cek dengan `adb shell getprop ro.build.version.sdk`.
- **Sinkron:** limit 100 per permintaan (sama dengan skrip Fase 0). Hasil penuh membuat rentang dibelah; bila rentang 1 menit masih penuh, sinkron berhenti dengan error (tidak diam-diam kehilangan trade). Belum diperiksa apakah `limit` di atas 100 diterima. Kode tidak bergantung urutan hasil.
- **Daftar koin yang dipindai:** saldo koin tidak nol + koin di riwayat tersimpan + `koin_tambahan`. Koin yang sudah habis terjual SEBELUM sinkron pertama (contoh ICNT) tidak terpindai kecuali lewat `koin_tambahan`; belum ada kolom isiannya di UI.
- **Simbol ditolak Indodax** (HTTP 400, kode 1109, contoh `pxgidr`) dilewati dan ditampilkan di layar. Penyebab pasti belum diketahui (dugaan: aset itu tidak punya pasangan IDR).
- **Snapshot saldo awal** = saldo IDR (tersedia + terkunci) saat sinkron pertama. Koin yang sudah dipegang saat itu tidak dihitung.
- **Database:** `getDatabasePath("catatan.db")` di penyimpanan privat aplikasi. Berisi trade, snapshot, setelan, meta. Tanpa key.
- **Pembulatan TP/SL** ke rupiah utuh hanya untuk tampilan (`bulatkan_idr`). Aturan tick harga per koin belum diperiksa (penting untuk koin murah seperti UAI).
- Tampilan koin apa adanya, tanpa pemisah ribuan. Dirapikan di fase UI.

### 1d. T1.3 (terbaru, detail)

**Status: Selesai.** Build, tes Rust, dan layar "Posisi dan TP/SL" di HP berjalan tanpa error (dilaporkan pengguna, 7 Nov 2026).

**Yang dibangun**
- `core/src/posisi.rs`: `hitung(trades, sejak_ms, setelan)` menghitung ulang SEMUA posisi dari catatan trade tiap kali (tanpa tabel posisi), sehingga mengoreksi satu catatan menghitung ulang semuanya.
  - Satu posisi per koin. Beli berikutnya merata-ratakan modal efektif; TP/SL dihitung ulang dari modal efektif baru.
  - Jual sebagian: posisi tetap terbuka, profit proporsional (modal dikali dulu baru dibagi, jadi jual penuh tepat nol). Jual penuh menutup posisi. Setiap order jual yang cocok menjadi satu baris `TradeTertutup`.
  - Jual melebihi posisi, atau jual tanpa posisi: bagian tanpa modal dicatat sebagai `Penyesuaian` dan tidak masuk profit.
  - Sisa koin kecil (contoh data nyata: beli 14,29 lalu jual 14) tetap jadi posisi dan ikut dirata-rata pada beli berikutnya.
  - Urutan: waktu order, beli lebih dulu dari jual bila waktu sama.
- `RiwayatSetelan`: TP%/SL% diambil dari setelan yang berlaku saat posisi DIBUKA. Tabel baru `setelan_tp_sl` (`berlaku_sejak_ms`, `tp_persen`, `sl_persen`) di `store.rs`. Bila belum ada setelan sebelum posisi dibuka, dipakai setelan tertua.
- `periksa_persen`: TP > 0; SL > 0 dan < 100.
- FFI: `simpan_setelan(path_db, tp, sl)` (menerima koma desimal, tidak menambah catatan bila sama dengan setelan terakhir) dan `hitung_posisi(path_db, semua_riwayat)`.
- Layar Android: kolom TP%/SL%, kotak centang "Hitung dari semua riwayat tersimpan (uji)", tombol Hitung posisi; menampilkan posisi terbuka (modal efektif, TP, SL), trade tertutup terbaru, total hasil, dan jumlah penyesuaian.
- Mode hitung: default hanya trade sejak snapshot saldo awal (agar saldo versi aplikasi konsisten). Kotak centang hanya untuk uji.
- Tes: 12 tes baru di `posisi.rs` (termasuk angka struk ICNT: 4.006,07 / TP 4.046 / SL 3.966 / +380) dan 1 di `store.rs`.

**Keputusan yang dipakai (usulan PRD, belum dikonfirmasi eksplisit oleh pengguna, mudah diubah)**
- Setelan TP/SL baru hanya berlaku untuk posisi yang dibuka sesudahnya.
- Koin yang dijual tanpa modal tercatat menjadi penyesuaian (profit 0), tidak diisi manual.

**Belum ada / catatan**
- Belum ada: deteksi realtime (T1.4), notifikasi (T1.5), nilai posisi dalam saldo dan snapshot harian (Fase 2).
- Hitung ulang dari semua trade tiap kali cukup untuk volume pribadi; pertimbangkan cache bila mulai lambat.

---

## 2. Masalah

### Belum terselesaikan atau dikenal
| Masalah | Penyebab / dugaan | Status |
| --- | --- | --- |
| Token WebSocket privat belum terbukti bisa dibuat dengan key v2 | Belum dites (T0.6). Satu-satunya ketidakpastian PRD yang tersisa; menentukan T1.4. Skrip `tools/tes_websocket_token.py` tidak ada di repo saat ini | Belum dikerjakan |
| Koin yang sudah tidak dipegang (contoh ICNT) tidak terpindai | Daftar koin dari saldo + riwayat tersimpan | Dikenal; solusi sementara `koin_tambahan`, belum ada isiannya di UI |
| `pxgidr` ditolak Indodax (kode 1109) | Dugaan: tidak ada pasangan IDR | Dilewati dengan benar; penyebab belum dipastikan |
| TP/SL dibulatkan ke rupiah utuh | Aturan tick harga per koin belum diperiksa | Belum diperiksa |
| Selisih 1 rupiah antara hasil jual dan modal beli berikutnya (data 3 Nov) | Diduga pembulatan | Belum dikonfirmasi, tidak berdampak |
| Apakah sisa 3 IDR (anggaran order tak terpakai) kembali ke saldo | Belum dicek di saldo | Belum dikonfirmasi; tidak memengaruhi rumus karena saldo diambil dari API |

### Sudah terselesaikan (ringkas)
HTTP 403 dari skrip lama (file belum tertimpa); SHA256 untuk TAPI v2; format simbol huruf kecil dan parameter `startTime`/`endTime` maks 7 hari; "Jumlah Order" vs uang keluar (pakai "Order Terisi"); format respons `/account`; `adb: more than one device` (skrip menerima serial); HTTP 400 kode 1109 saat sinkron (simbol ditolak dilewati, terbukti di HP); build SQLite bundled untuk Android (berhasil); aset IDR ada di saldo akunmu (sinkron pertama berhasil membuat snapshot, yang membutuhkan IDR).

---

## 3. Tahap selanjutnya

### Lingkungan pengembangan (dilaporkan 7 November 2026)
Arch Linux; Rust 1.98.1 dengan 4 target Android dan `cargo ndk`; JDK 17; Gradle 9.8.2 di PATH; Android SDK di `~/android-sdk` (platform 33 sampai 36, build-tools 34 sampai 36, NDK 26/27/28.2, `ANDROID_NDK_HOME` diatur); tanpa Android Studio, build lewat CLI; HP fisik via adb nirkabel.

### Teknologi
Rust untuk semua logika dan backend (berjalan di dalam HP), Kotlin/Java hanya klien Android (UI, foreground service, notifikasi, penyimpanan kunci). Jembatan: UniFFI 0.31. Penyimpanan: SQLite (rusqlite bundled) dikelola dari Rust.

### Task berikutnya (urut)
| Task | Isi | Syarat |
| --- | --- | --- |
| T0.6 | Skrip tes token WebSocket privat dengan key v2 (`generate_token`). Disarankan SEBELUM T1.4 | Key sudah ada |
| T1.4 | Layanan latar (foreground service) dan WebSocket realtime (FR-06 sampai FR-08), rekonsiliasi saat tersambung ulang (FR-10) | T1.3, T0.6 |
| T1.5 | Notifikasi beli dan jual (FR-15 sampai FR-17) | T1.4 |

### Keputusan yang masih terbuka
| Hal | Pilihan | Status |
| --- | --- | --- |
| Tombol "Jual di Indodax" | Buka app Indodax atau dihapus | Belum |
| Estimasi hasil bersih di bawah TP/SL | Tampil atau polos | Belum |
| Setelan TP/SL baru | Hanya posisi baru atau juga yang terbuka | Dipakai usulan "hanya posisi baru" (T1.3), belum dikonfirmasi |
| Koin yang dipegang sebelum app dipasang | Diabaikan atau diisi modalnya | Dipakai usulan "penyesuaian, profit 0" (T1.3), belum dikonfirmasi |
| Cadangan data (ekspor) | Perlu atau tidak, ke mana | Belum |
| Zona waktu WIB dan minggu mulai Senin | Perlu konfirmasi (layar sudah memakai WIB) | Belum |

### Fase berikutnya
- **Fase 2**: setelan target, snapshot harian dan mingguan, deposit/withdraw IDR, progress target
- **Fase 3**: riwayat trade tertutup, grafik saldo dan modal aktif, tombol koreksi saldo
- **Fase 4**: statistik (win rate, total fee, persen profit termakan fee)

---

## 4. Cara instalasi

Kebutuhan: Python 3.8+, git, Rust (rustup), dan toolchain Android (lihat bagian 3). Skrip Python tidak butuh `pip install`.

1. Masuk ke folder proyek: `cd catatan-scalping`
2. Buat key di indodax.com/trade_api: key **TAPI v2**, izin **baca saja**, tanpa pembatasan IP.
3. Salin `.env.example` menjadi `.env`, isi `INDODAX_API_KEY` dan `INDODAX_API_SECRET` (untuk skrip dan contoh Rust di komputer; aplikasi Android menyimpan key sendiri di HP).

Tes inti Rust:
```
cargo test -p scalping-core
cargo run -p scalping-core --example cek_data -- data/raw_icntidr_<waktu>.json
cargo run -p scalping-core --example cek_koneksi
```

Skrip validasi data (Fase 0):
```
python tools/fase0_validasi.py --diagnosa
python tools/fase0_validasi.py icntidr --hari 7
```
Hasil mentah tersimpan di `data/raw_<simbol>_<waktu>.json`.

Build dan pasang Android:
```
cargo update            # bila dependensi berubah
bash tools/bangun_android.sh --pasang <serial-adb>
```
Uji di HP: simpan kunci, Sinkron sekarang, lalu Hitung posisi (centang "semua riwayat" untuk data lama).

Alur git setiap task:
```
git add .
git commit -m "Tx.y: deskripsi singkat"
```

---

## 5. Struktur proyek

### Saat ini (setelah T1.3)
```
catatan-scalping/
├── README.md
├── PROGRESS.md                          <- file ini
├── .gitignore                           (.env, data/*, target/, __pycache__)
├── .env.example                         (salin menjadi .env)
├── Cargo.toml                           (workspace Rust: core, ffi, uniffi-bindgen)
├── Cargo.lock
├── data/                                (hasil tarikan, tidak di-commit)
│   └── .gitkeep
├── core/                                (crate scalping-core: SEMUA logika, tanpa UI)
│   ├── Cargo.toml                       (serde, rust_decimal, hmac/sha2, ureq, rusqlite bundled)
│   ├── README.md
│   ├── src/
│   │   ├── lib.rs                       (daftar modul dan ekspor)
│   │   ├── error.rs                     (tipe error)
│   │   ├── model.rs                     (bentuk data myTrades)
│   │   ├── sign.rs                      (HMAC-SHA256 TAPI v2)
│   │   ├── client.rs                    (klien REST, GET saja)
│   │   ├── account.rs                   (parser /account, saldo, cek key baca saja)
│   │   ├── calc.rs                      (gabung per order, anti-dobel, modal efektif, TP/SL, profit jual)
│   │   ├── store.rs                     (SQLite: trade, snapshot, meta, setelan_tp_sl)
│   │   ├── sync.rs                      (rekonsiliasi REST per koin, snapshot saldo awal)
│   │   └── posisi.rs                    (hitung ulang posisi, trade tertutup, penyesuaian)
│   └── examples/
│       ├── cek_data.rs                  (cek hitungan dari file JSON hasil Fase 0)
│       └── cek_koneksi.rs               (tes /account dengan key dari .env)
├── ffi/                                 (crate scalping-ffi, jembatan UniFFI)
│   ├── Cargo.toml
│   └── src/lib.rs                       (cek_koneksi, sinkron_sekarang, order_terbaru, simpan_setelan, hitung_posisi)
├── uniffi-bindgen/                      (pembuat binding Kotlin)
│   ├── Cargo.toml
│   └── src/main.rs
├── app/                                 (proyek Gradle Android, modul :aplikasi; klien tipis)
│   ├── .gitignore
│   ├── build.gradle.kts
│   ├── gradle.properties
│   ├── settings.gradle.kts
│   └── aplikasi/
│       ├── build.gradle.kts
│       └── src/main/
│           ├── AndroidManifest.xml
│           ├── kotlin/id/catatanscalping/app/
│           │   ├── MainActivity.kt      (layar uji: koneksi, sinkron, order, posisi dan TP/SL)
│           │   └── KunciAman.kt         (simpan key terenkripsi, Android Keystore)
│           ├── res/values/themes.xml
│           ├── res/values-night/themes.xml
│           ├── java/uniffi/             (OTOMATIS dari bindgen, tidak di-commit)
│           └── jniLibs/                 (OTOMATIS dari cargo ndk, tidak di-commit)
├── docs/
│   ├── PRD_Aplikasi_Catatan_Trading_Crypto.md     (v1.2)
│   └── prototype/
│       └── Catatan_trading__prototype_UI.html
└── tools/
    ├── fase0_validasi.py                (hanya baca; SHA256; --diagnosa)
    ├── fase0_akun.py                    (bentuk respons /account, angka disamarkan)
    ├── cek_lingkungan.py                (laporan versi Rust/Android/adb)
    └── bangun_android.sh                (Rust -> binding -> APK -> adb)
```

### Rencana (belum ada)
```
core/src/ws.rs                           WebSocket privat Indodax: token, order_update (T1.4)
core/src/notif.rs (atau di posisi.rs)    isi notifikasi beli/jual (T1.5)
app/.../LayananLatar.kt                  foreground service (T1.4)
app/.../Notifikasi.kt                    saluran dan notifikasi (T1.5)
tools/tes_websocket_token.py             tes token WebSocket dengan key v2 (T0.6)
```

---

## 6. Log commit
(isi hash dan pesan commit di sini)
- T1.3: `T1.3: posisi, modal efektif, TP/SL, setelan` (hash: ...)
- T1.2: `T1.2: penyimpanan SQLite, rekonsiliasi REST, snapshot saldo awal` (hash: ...)
- T1.1b: `T1.1b: proyek Android, jembatan UniFFI, layar koneksi` (hash: ...)
- T1.1c: `T1.1c: parser /account dan cek key baca saja` (hash: ...)
- T1.1a/T1.0: `T1.1a: inti Rust (klien Indodax, kalkulasi posisi), keputusan teknologi, PRD v1.2`
