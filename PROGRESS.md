# PROGRESS: Catatan Scalping

Terakhir diperbarui: 7 November 2026 (T0.6 dan T1.4a selesai)
Status: **Fase 0 selesai (T0.6 juga selesai). Fase 1 berjalan:** T1.0 sampai T1.3 lolos uji di HP; T0.6 lolos (token WebSocket privat berhasil dengan key v2); T1.4a (protokol WebSocket, logika murni) lolos `cargo test` (51 tes). Berikutnya: T1.4b (sambungan WebSocket sebenarnya di Rust), lalu T1.4c (foreground service Kotlin + jembatan FFI).

## Aturan kerja
- Satu task selesai = satu `git commit`.
- Setelah tiap task: perbarui file ini, lalu commit.
- Jangan pernah commit `.env` atau isi `data/` (sudah ada di `.gitignore`).
- Aplikasi hanya membaca data. Key API wajib izin **baca saja**.

---

## 1. Sudah dikerjakan

### 1a. Ringkasan riwayat (Fase 0 sampai T1.4a)

| Task | Hasil singkat | Status |
| --- | --- | --- |
| T0.1 sampai T0.5 | Repo, PRD, prototype. Skrip `tools/fase0_validasi.py` menarik trade ICNT nyata dengan key baca saja. Fee dan nilai trade dicocokkan dengan struk Indodax (semua cocok). Rumus modal efektif terkonfirmasi, PRD v1.1 | Selesai |
| T1.0 | Keputusan teknologi: Rust untuk semua logika (jalan di HP), Kotlin/Java hanya klien Android. PRD v1.2 | Selesai |
| T1.1a | Workspace Rust `core/` (`scalping-core`): model `myTrades`, HMAC-SHA256, klien REST baca saja, gabung per order, anti-dobel `tradeId`, modal efektif, TP/SL, profit jual. Hasil sama dengan skrip Python | Selesai |
| T1.1c | Parser `/api/v2/account`: saldo IDR (tersedia + terkunci), cek key baca saja | Selesai |
| T1.1b | Proyek Android `app/` (AGP 9.2.1, tanpa Compose, UI dari kode, minSdk 30), `ffi/` (UniFFI 0.31), `uniffi-bindgen/`, `tools/bangun_android.sh`. Layar koneksi, kunci terenkripsi (Android Keystore), tolak key yang punya izin trade/withdraw. Lolos uji HP | Selesai |
| T1.2 | `store.rs` (SQLite, anti-dobel `trade_id`), `sync.rs` (rekonsiliasi per koin, rentang maks 6 hari, rentang dibelah bila hasil penuh, snapshot saldo awal, simbol ditolak Indodax dilewati), FFI `sinkron_sekarang` dan `order_terbaru`. Lolos uji HP 7 Nov: 32 trade UAI tersinkron, sinkron ulang tanpa dobel, cocok dengan struk | Selesai |
| T0.6 | `tools/tes_websocket_token.py` (stdlib Python, klien WebSocket minimal). Token privat BERHASIL dibuat dengan key v2 baca saja (HMAC-SHA512), autentikasi dan subscribe channel pribadi berhasil (ttl 86400 detik). Event nyata diterima (UAI, 7 Nov): NEW, FILL, DONE, CANCELLED. Fee WebSocket (`fee + tax + clearing`) = `commission` REST (beli 43 = 43; jual 43+22+0 = 65) | Selesai |
| T1.4a | `core/src/ws.rs`: token (badan, tanda tangan HMAC-SHA512, parser respons, `Debug` tidak membocorkan token), pesan connect/subscribe, parser pesan masuk (ping `{}`, balasan, galat, event order_update, banyak pesan per frame), `PemicuSinkron` (memicu sinkron REST saat DONE atau CANCELLED setelah sebagian terisi; DONE ganda tidak memicu dua kali), jadwal ulang `jeda_ulang`, `order_sudah_tersimpan`, `umur_sambungan_maks`. Tanpa jaringan. `cargo test -p scalping-core`: 51 lulus | Selesai |

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

### 1d. T1.3 (detail)

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
- Belum ada (saat T1.3): deteksi realtime (T1.4), notifikasi (T1.5), nilai posisi dalam saldo dan snapshot harian (Fase 2).
- Hitung ulang dari semua trade tiap kali cukup untuk volume pribadi; pertimbangkan cache bila mulai lambat.

### 1e. T0.6 dan T1.4a (terbaru, detail)

**Status: Selesai.** T0.6 dijalankan pengguna di Arch (7 Nov 2026); T1.4a lolos `cargo test -p scalping-core` (51 lulus, 0 gagal).

**Temuan T0.6 (dasar desain T1.4)**
- Token: `POST https://indodax.com/api/private_ws/v1/generate_token`, badan `client=tapi&tapi_key=<KEY>`, header `Sign` = HMAC-**SHA512** dari badan (bukan SHA256 seperti REST v2). Key v2 baca saja diterima.
- WebSocket: `wss://pws.indodax.com/ws/?cf_ws_frame_ping_pong=true`, TANPA header `Origin`. Pesan `{"connect":{"token":...},"id":1}`, lalu `{"subscribe":{"channel":...},"id":2}`. Server mengirim `{}` sebagai ping aplikasi; balas `{}`. Token berlaku 24 jam (ttl 86400).
- Status event: `NEW`, `FILL` (bukan FILLED), `DONE`, `CANCELLED`, `REJECTED`. Market order: NEW, FILL, DONE tiba bersamaan dalam satu paket (`push.pub.data` berupa larik).
- Event FILL tidak membawa `quoteQty`; harga di event adalah pembulatan (beli: harga x qty = 10.955 vs REST 10.954; jual: 10.917 vs 10.916). Jangan dipakai menghitung modal.
- `fee + tax + clearing` dari event = `commission` REST pada dua trade UAI. Hanya satu sampel (market, clearing 0).
- Market order: `origQty` di event NEW adalah perkiraan dari anggaran (3,69352663), jumlah terisi akhir berbeda (4,24612403).
- Beli market lalu jual market UAI = -146 IDR (keluar 10.997, diterima 10.851): round-trip market bisa sekitar 1% biaya.

**Keputusan desain (usulan, belum diuji di HP)**
- WebSocket hanya PEMICU. Saat order DONE (atau CANCELLED setelah sebagian terisi), tarik trade koin itu lewat REST (`sinkron_koin`); angka resmi tetap dari REST. Bila REST belum memuat order itu, ulangi dengan jeda 0, 1, 2, 4, 8 detik, sisanya oleh rekonsiliasi.
- Order limit yang terisi sebagian dan dibiarkan terbuka baru tercatat saat DONE/CANCELLED atau saat rekonsiliasi (sesuai FR-07/FR-08).

**Belum ada / catatan**
- Belum ada: sambungan WebSocket sebenarnya (T1.4b), foreground service dan notifikasi (T1.4c, T1.5).
- `tradeId` event WebSocket (contoh 144115188260590414) belum dibandingkan dengan `tradeId` REST; tidak menghalangi karena anti-dobel memakai ID dari REST.
- `ws.rs` memakai `Error::Json` untuk token ditolak (belum ada varian khusus di `error.rs`).
- Skrip `tes_websocket_token.py` hanya untuk uji manual; event mentah tersimpan di `data/ws_events_*.json` (tidak di-commit).

---

## 2. Masalah

### Belum terselesaikan atau dikenal
| Masalah | Penyebab / dugaan | Status |
| --- | --- | --- |
| `tradeId` event WebSocket vs REST belum dibandingkan | Belum ada data pembanding | Tidak menghalangi (anti-dobel dari REST) |
| Pemilihan pustaka WebSocket Rust + TLS untuk Android (T1.4b) | Risiko dua penyedia kripto rustls bentrok (ureq sudah memakai rustls); belum dicoba | Belum dikerjakan; tunggu T1.4b |
| Clearing fee baru teramati bernilai 0 | Hanya 2 sampel kecil (UAI) | Pantau; bila `commission` REST berbeda dari fee+tax+clearing event, andalkan REST |
| Koin yang sudah tidak dipegang (contoh ICNT) tidak terpindai | Daftar koin dari saldo + riwayat tersimpan | Dikenal; solusi sementara `koin_tambahan`, belum ada isiannya di UI |
| `pxgidr` ditolak Indodax (kode 1109) | Dugaan: tidak ada pasangan IDR | Dilewati dengan benar; penyebab belum dipastikan |
| TP/SL dibulatkan ke rupiah utuh | Aturan tick harga per koin belum diperiksa | Belum diperiksa |
| Selisih 1 rupiah antara hasil jual dan modal beli berikutnya (data 3 Nov) | Diduga pembulatan | Belum dikonfirmasi, tidak berdampak |
| Apakah sisa 3 IDR (anggaran order tak terpakai) kembali ke saldo | Belum dicek di saldo | Belum dikonfirmasi; tidak memengaruhi rumus karena saldo diambil dari API |

### Sudah terselesaikan (ringkas)
Token WebSocket privat dengan key v2 (T0.6: berhasil); format event order_update dan nama status FILL (terkonfirmasi); fee event = `commission` REST (cocok pada 2 trade); HTTP 403 dari skrip lama (file belum tertimpa); SHA256 untuk TAPI v2; format simbol huruf kecil dan parameter `startTime`/`endTime` maks 7 hari; "Jumlah Order" vs uang keluar (pakai "Order Terisi"); format respons `/account`; `adb: more than one device` (skrip menerima serial); HTTP 400 kode 1109 saat sinkron (simbol ditolak dilewati, terbukti di HP); build SQLite bundled untuk Android (berhasil); aset IDR ada di saldo akunmu (sinkron pertama berhasil membuat snapshot, yang membutuhkan IDR).

---

## 3. Tahap selanjutnya

### Lingkungan pengembangan (dilaporkan 7 November 2026)
Arch Linux; Rust 1.98.1 dengan 4 target Android dan `cargo ndk`; JDK 17; Gradle 9.8.2 di PATH; Android SDK di `~/android-sdk` (platform 33 sampai 36, build-tools 34 sampai 36, NDK 26/27/28.2, `ANDROID_NDK_HOME` diatur); tanpa Android Studio, build lewat CLI; HP fisik via adb nirkabel.

### Teknologi
Rust untuk semua logika dan backend (berjalan di dalam HP), Kotlin/Java hanya klien Android (UI, foreground service, notifikasi, penyimpanan kunci). Jembatan: UniFFI 0.31. Penyimpanan: SQLite (rusqlite bundled) dikelola dari Rust.

### Task berikutnya (urut)
| Task | Isi | Syarat |
| --- | --- | --- |
| T1.4b | Sambungan WebSocket sebenarnya di Rust: minta token (POST), handshake tanpa Origin, connect, subscribe, baca pesan, balas ping `{}`, sambung ulang otomatis + token baru, picu `sinkron_koin` lewat `PemicuSinkron`. Pilih pustaka WebSocket + TLS dengan hati-hati (hindari bentrok penyedia rustls dengan ureq) | T1.4a selesai; hasil `cargo test` dan build Android pengguna |
| T1.4c | Foreground service Kotlin (`LayananLatar.kt`), jembatan FFI untuk mulai/hentikan pantauan dan menerima kejadian, rekonsiliasi saat tersambung ulang dan saat app dibuka (FR-06, FR-10) | T1.4b; berkas `MainActivity.kt`, `AndroidManifest.xml`, `KunciAman.kt`, `ffi/src/lib.rs` terbaru |
| T1.5 | Notifikasi beli dan jual (FR-15 sampai FR-17) | T1.4c |

### Keputusan yang masih terbuka
| Hal | Pilihan | Status |
| --- | --- | --- |
| Tombol "Jual di Indodax" | Buka app Indodax atau dihapus | Belum |
| Estimasi hasil bersih di bawah TP/SL | Tampil atau polos | Belum |
| Setelan TP/SL baru | Hanya posisi baru atau juga yang terbuka | Dipakai usulan "hanya posisi baru" (T1.3), belum dikonfirmasi |
| Koin yang dipegang sebelum app dipasang | Diabaikan atau diisi modalnya | Dipakai usulan "penyesuaian, profit 0" (T1.3), belum dikonfirmasi |
| Cadangan data (ekspor) | Perlu atau tidak, ke mana | Belum |
| Zona waktu WIB dan minggu mulai Senin | Perlu konfirmasi (layar sudah memakai WIB) | Belum |

### Pekerjaan kecil tertunda
- PRD perlu v1.3: bagian 9 (token WebSocket dengan key v2 terjawab, tanda tangan SHA512 untuk token, nama status `FILL`, WebSocket sebagai pemicu) dan pertanyaan terbuka.

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
cargo test -p scalping-core        # 51 tes lulus per T1.4a
cargo run -p scalping-core --example cek_data -- data/raw_icntidr_<waktu>.json
cargo run -p scalping-core --example cek_koneksi
```

Tes token dan event WebSocket privat (T0.6, baca saja; event tersimpan di data/):
```
python tools/tes_websocket_token.py --hanya-token
python tools/tes_websocket_token.py --tunggu 180
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

### Saat ini (setelah T1.4a)
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
│   │   ├── posisi.rs                    (hitung ulang posisi, trade tertutup, penyesuaian)
│   │   └── ws.rs                        (protokol WebSocket privat: token, parser event, pemicu sinkron; tanpa jaringan)
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
    ├── tes_websocket_token.py           (T0.6: tes token dan event WebSocket privat, baca saja)
    ├── cek_lingkungan.py                (laporan versi Rust/Android/adb)
    └── bangun_android.sh                (Rust -> binding -> APK -> adb)
```

### Rencana (belum ada)
```
core/src/ (file transport WebSocket)     sambungan WebSocket sebenarnya, nama file ditentukan di T1.4b
core/src/notif.rs (atau di posisi.rs)    isi notifikasi beli/jual (T1.5)
app/.../LayananLatar.kt                  foreground service (T1.4c)
app/.../Notifikasi.kt                    saluran dan notifikasi (T1.5)
```

---

## 6. Log commit
(isi hash dan pesan commit di sini)
- T1.4a: `T1.4a: protokol WebSocket privat (token, parser event, pemicu sinkron)` (hash: ...)
- T0.6: `T0.6: token WebSocket privat dan format event terkonfirmasi` (hash: ...)
- T1.3: `T1.3: posisi, modal efektif, TP/SL, setelan` (hash: ...)
- T1.2: `T1.2: penyimpanan SQLite, rekonsiliasi REST, snapshot saldo awal` (hash: ...)
- T1.1b: `T1.1b: proyek Android, jembatan UniFFI, layar koneksi` (hash: ...)
- T1.1c: `T1.1c: parser /account dan cek key baca saja` (hash: ...)
- T1.1a/T1.0: `T1.1a: inti Rust (klien Indodax, kalkulasi posisi), keputusan teknologi, PRD v1.2`
