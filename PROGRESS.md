# PROGRESS: Catatan Scalping

Terakhir diperbarui: 8 November 2026 (Fase UI U1 sampai U5 selesai dan lolos uji)
Status: **Semua fase rencana selesai dan lolos uji: Fase 0, Fase 1 (MVP), Fase 2 (T2.1 sampai T2.5), Fase 3 (T3.1 sampai T3.3), T4.1 (statistik), dan Fase UI (U1 sampai U5)** (build, tes Rust, dan uji HP dilaporkan pengguna 7 sampai 8 Nov 2026). Langkah berikutnya: PRD v1.3 (dokumentasi), lalu keputusan terbuka (pantauan otomatis setelah reboot, verifikasi mode fee deposit saat ada deposit nyata).

## Aturan kerja
- Satu task selesai = satu `git commit`.
- Setelah tiap task: perbarui file ini, lalu commit.
- Jangan pernah commit `.env` atau isi `data/` (sudah ada di `.gitignore`).
- Aplikasi hanya membaca data. Key API wajib izin **baca saja**.
- Build Android SELALU lewat `bash tools/bangun_android.sh` (skrip ini membuat ulang binding Kotlin dari Rust). Menjalankan Gradle langsung tidak memperbarui binding, sehingga fungsi FFI baru tampil sebagai "Unresolved reference".

---

## 1. Sudah dikerjakan

### 1a. Ringkasan riwayat (Fase 0 sampai Fase UI)

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
| T1.4b | `core/src/ws_klien.rs`: sambungan WebSocket privat sebenarnya (`pantau`, `KejadianPantau`): minta token, handshake tanpa Origin, connect, subscribe, balas ping `{}`, sambung ulang otomatis dengan token baru, picu `sinkron_koin` lewat `PemicuSinkron`. Pustaka: rustls (provider `ring` saja, sama dengan ureq), webpki-roots, base64; tanpa aws-lc-rs. Contoh `core/examples/pantau.rs`. Lolos uji komputer: rekonsiliasi awal, subscribe, beli dan jual market UAI terdeteksi dan tercatat tanpa dobel | Selesai |
| T1.4c | FFI: `Pemantau` (mulai, berhenti, sedang_jalan; thread pantau), callback `PendengarPantau`, enum `KejadianUi`. Android: `LayananLatar.kt` (foreground service tipe `specialUse`, satu notifikasi tetap dengan tombol Hentikan, START_STICKY), `PantauBersama` (status dan log untuk layar), izin notifikasi (Android 13+), permintaan abaikan optimasi baterai, sinkron otomatis saat app dibuka (FR-06, FR-10). Build dan tes berhasil, 7 poin uji HP lolos | Selesai |
| T1.5 | FFI `isi_notifikasi` dan `ringkasan_posisi_notif` (isi notifikasi dihitung di Rust, format Indonesia, 6 tes di `scalping-ffi`; estimasi hasil bersih dengan fee jual perkiraan 0,31%; beli tambahan menampilkan TP/SL order itu dan posisi keseluruhan). `LayananLatar.kt`: saluran "Order terisi" (prioritas tinggi), satu notifikasi per order, pekerja satu thread, tanpa notifikasi ganda (trade_baru > 0), ringkasan TP/SL saat rekonsiliasi menemukan trade terlewat. Mulai pantauan menyimpan setelan TP/SL (FR-15 sampai FR-17). Build dan tes berhasil, 7 poin uji HP lolos | Selesai |
| T2.1 | Target harian dan mingguan (FR-04): `Penyimpanan::atur_target/target` (tabel `meta`, kunci `target_harian_persen` dan `target_mingguan_persen`), `periksa_target` (lebih dari 0 dan paling banyak 100), FFI `simpan_target`/`baca_target` (`SetelanTarget`), kolom target di layar uji. 4 tes baru. Build dan tes berhasil, uji HP 6 poin lolos | Selesai |
| T2.2 | Arus kas: `core/src/arus_kas.rs` (parser `fiat/orders`, hanya status success dan IDR, ID sumber `<jenis>:<orderNo>`, jendela 29 hari, belah rentang bila hasil >= 100, `sinkron_arus_kas`), tabel `arus_kas` dan fungsi terkait di `store.rs`, `IndodaxClient::ambil_arus_kas`, FFI `sinkron_arus_kas`/`daftar_arus_kas`/`atur_amount_sudah_bersih`, tombol di layar uji. Juga: `busy_timeout` 5 detik pada koneksi SQLite | Selesai (build, tes, uji HP lulus 7 Nov) |
| T2.3 | `core/src/periode.rs`: periode WIB (hari 00:00, minggu mulai Senin), `DataSaldo` (saldo versi aplikasi dari catatan), pencapaian dan progress (FR-19 sampai FR-21), FFI `hitung_target` (sekalian mencatat snapshot harian/mingguan) | Selesai (build, tes, uji HP lulus 7 Nov) |
| T2.4 | Progress target harian dan mingguan di notifikasi jual (FR-16), penanda "TERCAPAI" saat target baru tercapai oleh penjualan itu (FR-18, di dalam notifikasi yang sama), tampilan progress di layar uji | Selesai (build, tes, uji HP lulus 7 Nov) |
| T2.5 | `LayananLatar.kt`: sinkron arus kas (deposit/withdraw IDR) tiap pantauan tersambung atau tersambung ulang, paling sering sekali per 5 menit, di thread sendiri (`pekerjaArusKas`) agar notifikasi order tidak tertunda | Selesai (build, tes, uji HP lulus 7 Nov) |
| T3.1 | Riwayat trade tertutup: FFI `riwayat_tertutup` (ringkasan hari ini WIB: jumlah, untung, hasil; daftar terbaru di atas), bagian "Riwayat" di layar uji (FR-23) | Selesai (build, tes, uji HP lulus 7 Nov) |
| T3.2 | Grafik saldo versi aplikasi dan modal aktif: `DataSaldo::deret` (core), FFI `data_grafik`, komponen `GrafikView.kt` (garis tebal saldo, putus-putus modal berundak) (FR-22) | Selesai (build, tes, uji HP lulus 7 Nov) |
| T3.3 | Koreksi saldo: tabel `koreksi_saldo`, `DataSaldo::selisih_saldo`, FFI `cek_selisih_saldo`/`terapkan_koreksi_saldo`/`daftar_koreksi_saldo`, tombol "Cek selisih saldo" dan "Catat koreksi" (dengan dialog konfirmasi) di layar uji | Selesai (build, tes, uji HP lulus 7 Nov) |
| T4.1 | Statistik (FR-25): field `TradeTertutup.fee` (fee beli proporsional + fee jual), modul `core/src/statistik.rs` (`hitung_statistik`: jumlah, untung/rugi/impas, win rate, rata-rata untung dan rugi, hasil bersih, total fee, profit kotor, persen profit termakan fee), FFI `statistik_trading(path_db, rentang)` dengan rentang hari/minggu/semua (WIB), bagian "Statistik trading" dengan tiga tombol di layar uji | Selesai (build, tes, uji HP lulus 7 Nov) |
| U1 | Kerangka 4 tab (navigasi bawah, tema terang/gelap sesuai prototype), `Format.kt` (titik ribuan, koma desimal), `Komponen.kt`, `Latar.kt`, `Sinkron.kt`, layar Setelan (TP/SL, target, kunci, pantauan, sinkron, deposit/withdraw, log). FFI baru: `baca_setelan`, `hasil_persen` (di `RingkasanTargetUi`), `daftar_posisi`, 1 tes baru (15 tes ffi) | Selesai (build berhasil, uji HP lulus 8 Nov) |
| U2 | Beranda: saldo, hasil Rp dan persen terhadap modal aktif, grafik (`GrafikView` bertema), progress target harian/mingguan, chip status koneksi | Selesai (build berhasil, uji HP lulus 8 Nov) |
| U3 | Posisi: kartu per posisi dengan tangga SL/modal efektif/TP dan estimasi hasil bersih; TANPA tombol Jual | Selesai (build berhasil, uji HP lulus 8 Nov) |
| U4 | Riwayat: ringkasan hari ini, daftar trade tertutup (tombol "Tampilkan lebih banyak"), statistik tiga rentang, koreksi saldo | Selesai (build berhasil, uji HP lulus 8 Nov) |
| U5 | Ekspor/impor data lokal: `Penyimpanan::ekspor_ke/periksa_cadangan/pulihkan` di `store.rs` (VACUUM INTO, cek integritas dan tabel wajib, verifikasi angka desimal, salinan `catatan.db.sebelum-impor`, ganti lewat rename), FFI `ekspor_cadangan/periksa_cadangan/pulihkan_cadangan` (`RingkasanCadanganUi`), `Cadangan.kt` (SAF), kartu "Cadangan data" di Setelan, `MainActivity.onActivityResult`. Kunci API tidak ikut. 4 tes baru di core, 1 di ffi (16 tes ffi) | Selesai (build, tes, uji HP lulus 8 Nov) |

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
- Pemisah ribuan sudah diterapkan di UI (U1): `Format.kt` untuk angka mentah dari Rust, dan teks berformat langsung dari Rust untuk layar Posisi dan notifikasi.

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
- Layar Android (saat T1.3, layar uji): kolom TP%/SL%, kotak centang "Hitung dari semua riwayat tersimpan (uji)", tombol Hitung posisi; menampilkan posisi terbuka (modal efektif, TP, SL), trade tertutup terbaru, total hasil, dan jumlah penyesuaian. Layar uji ini sudah diganti layar asli di U1 sampai U4 (kotak centang "semua riwayat" tidak dibawa; FFI `hitung_posisi` tetap ada).
- Mode hitung: default hanya trade sejak snapshot saldo awal (agar saldo versi aplikasi konsisten).
- Tes: 12 tes baru di `posisi.rs` (termasuk angka struk ICNT: 4.006,07 / TP 4.046 / SL 3.966 / +380) dan 1 di `store.rs`.

**Keputusan yang dipakai (usulan PRD, belum dikonfirmasi eksplisit oleh pengguna, mudah diubah)**
- Setelan TP/SL baru hanya berlaku untuk posisi yang dibuka sesudahnya.
- Koin yang dijual tanpa modal tercatat menjadi penyesuaian (profit 0), tidak diisi manual.

**Belum ada / catatan**
- Belum ada (saat T1.3): deteksi realtime (T1.4), notifikasi (T1.5), nilai posisi dalam saldo dan snapshot harian (Fase 2).
- Hitung ulang dari semua trade tiap kali cukup untuk volume pribadi; pertimbangkan cache bila mulai lambat.

### 1e. T0.6 dan T1.4a (detail)

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
- (Saat T1.4a) belum ada sambungan WebSocket sebenarnya dan foreground service; keduanya selesai di T1.4b dan T1.4c (lihat 1f). Notifikasi beli/jual masih T1.5.
- `tradeId` event WebSocket (contoh 144115188260590414) belum dibandingkan dengan `tradeId` REST; tidak menghalangi karena anti-dobel memakai ID dari REST.
- `ws.rs` memakai `Error::Json` untuk token ditolak (belum ada varian khusus di `error.rs`).
- Skrip `tes_websocket_token.py` hanya untuk uji manual; event mentah tersimpan di `data/ws_events_*.json` (tidak di-commit).

### 1f. T1.4b dan T1.4c (detail)

**Status: Selesai.** T1.4b lolos uji di komputer; T1.4c build dan tes berhasil tanpa error, 7 poin uji HP lolos (dilaporkan pengguna, 7 Nov 2026).

**Cara kerja**
- WebSocket hanya PEMICU. Saat order DONE (atau CANCELLED setelah sebagian terisi), Rust menarik trade koin itu lewat REST (`sinkron_koin`); angka resmi tetap dari REST. Rekonsiliasi REST juga dijalankan tiap pantauan dimulai dan tiap tersambung ulang.
- `Pemantau` (ffi) menjalankan `pantau` di thread sendiri, bendera henti baru per sesi, satu sesi pada satu waktu (mulai ganda ditolak). Panic di dalam pantau ditangkap sehingga status tidak macet.
- `LayananLatar` memakai `foregroundServiceType=specialUse` (bukan `dataSync`, yang dibatasi sekitar 6 jam per hari di Android 15+). Notifikasi tetap menampilkan status koneksi dan order terakhir; boleh disenyapkan lewat saluran notifikasi.
- Bila sistem mematikan layanan, preferensi `aktif` tetap true sehingga START_STICKY menyalakannya lagi. Galat fatal (contoh kunci dicabut) menghentikan layanan dan memunculkan notifikasi galat biasa.
- Mulai pantauan butuh sinkron manual pertama (agar snapshot saldo awal ada). Sinkron otomatis saat app dibuka hanya jalan bila pantauan mati, sinkron manual pernah dilakukan, dan jeda minimal 60 detik.
- Sinkron manual dimatikan saat pantauan berjalan (cegah dua koneksi SQLite menulis bersamaan).

**Belum ada / catatan**
- Belum ada: notifikasi beli dan jual berisi TP/SL (T1.5). Notifikasi sekarang hanya status koneksi.
- Belum diketahui apakah `Penyimpanan` memakai `busy_timeout`. Pantau bila muncul error "database is locked" (misal saat hitung posisi bersamaan dengan pantau menulis). (Sejak T2.2 `busy_timeout` 5 detik sudah ada.)
- Pantauan hanya aktif saat pengguna menyalakannya; belum menyala otomatis setelah HP reboot.
- Versi Android HP (`ro.build.version.sdk`) belum dilaporkan; kode menangani semua versi dari minSdk 30.
- Kunci API dibaca layanan dari `KunciAman` dan disalurkan ke Rust; tidak dicatat ke log.

### 1g. T1.5 (detail)

**Status: Selesai.** Build dan tes berhasil tanpa error; 7 poin uji HP lolos (dilaporkan pengguna, 7 Nov 2026).

**Yang dibangun**
- `ffi/src/lib.rs`: `isi_notifikasi`, `ringkasan_posisi_notif`, `IsiNotifOrder`. Isi notifikasi dihitung di Rust dari database lokal (sejak snapshot saldo awal), format Indonesia (titik ribuan, koma desimal). 6 tes di `scalping-ffi`.
- `LayananLatar.kt`: saluran "Order terisi" (prioritas tinggi, terpisah dari notifikasi layanan), satu notifikasi per order, pekerja satu thread, tanpa notifikasi ganda (hanya bila `trade_baru > 0`).
- `MainActivity.kt` (layar uji saat itu): mulai pantauan menyimpan setelan TP%/SL% dari kolom di layar lebih dulu (tidak menambah catatan bila sama). Perilaku yang sama sekarang ada di `LayarSetelan.kt`.
- Manifest, `core/*`, `Cargo.toml`, `build.gradle.kts` tidak berubah.

**Perilaku**
- Beli pertama: modal efektif, TP, SL, estimasi hasil bersih (contoh ringkas: `Modal 4.006 · TP 4.046 (≈+0,69%) · SL 3.966 (≈−1,31%)`).
- Beli tambahan pada koin yang masih dipegang: dua blok, ORDER INI (TP/SL dari modal efektif order itu) dan POSISI KESELURUHAN (TP/SL dari modal efektif rata-rata).
- Jual: hasil bersih nyata (Rp dan %), modal dan IDR diterima; jual sebagian menampilkan sisa posisi.
- Trade terlewat (ditemukan rekonsiliasi saat tersambung ulang): satu notifikasi "N trade baru dicatat saat sinkron" berisi TP/SL dan estimasi semua posisi terbuka.
- Notifikasi hanya informasi; tidak ada order dipasang di Indodax (FR-17).

**Keputusan baru**
- Estimasi hasil bersih di bawah TP/SL: DITAMPILKAN (diminta pengguna, 7 Nov 2026). Memakai fee jual perkiraan 0,31% (`fee_jual_perkiraan()` di ffi); nilai nyata tetap dari `commission` saat jual.
- Beli tambahan koin yang sama: tampil TP/SL order itu dan TP/SL posisi keseluruhan (diminta pengguna).
- Progress target harian di notifikasi jual (bagian FR-16): ditunda ke Fase 2 (selesai di T2.4).

**Catatan teknis dan batasan**
- Notifikasi jual mengambil trade tertutup terbaru koin itu, hanya bila waktu tutupnya tidak lebih lama dari order jual terbaru (cegah menampilkan hasil jual lama saat jual tanpa modal).
- Pembulatan tampilan: harga < 100 tampil 2 desimal, selebihnya rupiah utuh (aturan tick per koin masih belum diperiksa).
- Order beli dikenali sebagai order beli terbaru koin itu; dua beli koin sama yang selesai nyaris bersamaan menampilkan order yang sama (perbaikan butuh ID order di `OrderRingkas` yang cocok dengan ID event WebSocket).
- Fee jual perkiraan 0,31% dipakai tetap; belum ada setelan pengguna (jual market bisa lebih besar).
- Tombol "Jual di Indodax": DIHAPUS dari rancangan (keputusan 7 Nov 2026, lihat 1l).

### 1h. T2.1 (detail)

**Status: Selesai.** Build dan tes berhasil; 6 poin uji HP lolos (dilaporkan pengguna, 7 Nov 2026).

- Target disimpan di tabel `meta` (tanpa tabel baru); hanya nilai terbaru, tanpa riwayat. Input tidak valid ditolak dan nilai lama tidak berubah. Koma desimal diterima.
- Berkas berubah: `core/src/store.rs`, `core/src/posisi.rs` (`periksa_target`), `ffi/src/lib.rs`, `MainActivity.kt`.
- Batas atas 100% adalah pilihan (bukan dari PRD); ubah di `periksa_target` bila perlu.
- Progress target belum dihitung (T2.3).

### 1i. T2.2 sampai T2.4 (detail)

**Status: Selesai.** Kode ditulis 7 Nov 2026; build, tes, dan uji HP lulus (dilaporkan pengguna, 7 Nov 2026).

**Data nyata yang dipakai (tools/fase2_fiat.py, 7 Nov 2026)**
- Parameter `transactionType` (0 deposit, 1 withdraw), `beginTime`, `endTime` (ms) diterima Indodax (HTTP 200, camelCase). Respons: `data[]` berisi `orderNo`, `amount` (teks), `totalFee` (teks), `method`, `fiatCurrency`, `status` (contoh `success`), `createTime`, `updateTime` (angka ms). Respons deposit tidak memuat jenis; jenis diambil dari parameter permintaan.
- Withdraw: respons kosong (`{"data": []}`), bentuk baris withdraw BELUM teramati (diasumsikan sama).
- Waktu yang dipakai = `updateTime` (saat selesai), cadangan `createTime`.

**Cara kerja**
- Saldo versi aplikasi dihitung dari CATATAN, tanpa jaringan: `saldo(t) = saldo awal + arus kas bersih (awal, t] + profit trade tertutup (awal, t]`. Alasan: membeli koin hanya memindahkan IDR ke posisi pada modal efektif (nilai tetap), jadi saldo hanya berubah oleh profit/loss dan deposit/withdraw. Akibatnya snapshot awal hari/minggu selalu tepat walau layanan mati, dan mengoreksi satu catatan menghitung ulang semuanya (sesuai PRD bagian 11).
- Pencapaian periode memakai rumus PRD: (saldo sekarang - saldo awal periode - setoran bersih) / saldo awal periode x 100. Progress = pencapaian / target x 100. Periode di WIB (UTC+7), minggu mulai Senin. Bila periode dimulai sebelum snapshot awal, dipakai waktu snapshot.
- Batas rentang: awal periode TIDAK ikut, akhir ikut. Trade/arus kas tepat pada milidetik snapshot awal dianggap sudah ada di snapshot.
- Tabel `snapshot` jenis `harian`/`mingguan` diisi oleh `hitung_target` sebagai catatan (untuk grafik Fase 3). Perhitungan TIDAK memakainya (dihitung ulang dari catatan).
- Sinkron arus kas: pertama kali sejak 1 hari sebelum snapshot awal; berikutnya sejak 14 hari sebelum sinkron terakhir (menangkap order pending yang baru sukses). Dijalankan: tombol di Setelan, otomatis setelah sinkron trade, dan saat app dibuka bila pantauan hidup (jeda 60 detik). Aman saat pantauan hidup berkat `busy_timeout`.
- Notifikasi jual: baris "Harian: +x% dari target y% (z% tercapai)" dan "Mingguan: ...". "TERCAPAI" muncul bila sebelum penjualan itu belum tercapai dan sesudahnya sudah (dihitung dengan membuang trade itu). Bila progress gagal dihitung (mis. snapshot belum ada), notifikasi tetap tampil tanpa baris itu.

**Keputusan yang dipakai (usulan, BELUM dikonfirmasi pengguna)**
- Belum diketahui apakah `amount` deposit sudah bersih atau belum dikurangi `totalFee`. Dipakai dua mode: bawaan `amount - fee` untuk deposit dan `-(amount + fee)` untuk withdraw; kotak centang "Nominal sudah bersih" (sekarang di layar Setelan) mengganti ke `+/- amount`. Cara memastikan: bandingkan satu deposit nyata (nominal yang ditransfer, saldo Indodax sebelum dan sesudah) dengan baris "amount ... fee ... → saldo ..." di Setelan.
- Notifikasi "target tercapai" (FR-18) digabung ke notifikasi jual, bukan notifikasi terpisah (tidak mengubah `LayananLatar.kt`).

**Belum ada / catatan**
- Jumlah baris per halaman `fiat/orders` belum diketahui; rentang dengan >= 100 baris dibelah (dugaan, tidak berbahaya).
- Perilaku yang perlu diuji di HP: (1) sinkron arus kas, deposit terlihat benar; (2) progress hari/minggu cocok hitungan manual; (3) deposit baru tidak menaikkan pencapaian; (4) notifikasi jual memuat progress; (5) target tercapai muncul sekali; (6) tidak ada "database is locked" saat pantauan hidup.

### 1j. T2.5 dan Fase 3 (detail)

**Status: Selesai.** Kode ditulis 7 Nov 2026; build, tes, dan uji HP lulus (dilaporkan pengguna, 7 Nov 2026).

**Koreksi saldo (T3.3)**
- Perbandingan: IDR yang diharapkan menurut aplikasi = saldo versi aplikasi - modal posisi terbuka (pada modal efektif). Selisih = saldo IDR asli (`/account`, tersedia + terkunci) - IDR menurut aplikasi.
- Koreksi dicatat di tabel `koreksi_saldo` (waktu, jumlah, catatan) dan diperlakukan seperti setoran: mengubah saldo DAN modal aktif, tetapi bukan profit, jadi tidak masuk pencapaian; jarak garis saldo dan modal aktif di grafik tetap = profit. Tidak ada catatan lain yang dihapus atau diubah (sesuai PRD bagian 12).
- Selisih di bawah Rp 1 dianggap pembulatan dan tidak dicatat. Mencatat dua kali berturut-turut aman (selisih kedua nol).
- `terapkan_koreksi_saldo` mengambil saldo asli lagi saat dicatat (bukan memakai angka lama di layar).
- Hal yang menimbulkan selisih sementara dan BUKAN kesalahan: order limit terisi sebagian (belum dicatat sampai DONE/CANCELLED, FR-08), trade yang belum tersinkron, koin lama yang dijual (penyesuaian), deposit/withdraw koin (diabaikan di versi ini). Jangan koreksi pada saat itu; sinkron dulu.
- Dua hal pengguna: kunci wajib baca saja (cek ditolak bila key punya izin trade/withdraw).

**Grafik (T3.2)**: titik di snapshot awal, di tiap kejadian (trade tertutup, arus kas, koreksi), dan sekarang; dijarangkan bila lebih dari 120 titik (ujung selalu ada, nilai tetap tepat). Nilai diubah ke Double hanya untuk koordinat gambar di `GrafikView`.

**T2.5**: sinkron arus kas dipicu `KejadianUi.Tersambung`. Gagal tidak mengganggu pantauan (dicatat di log layar).

**Keputusan pengguna (7 Nov 2026)**
- Koin lama yang dipegang sebelum aplikasi dipasang: dibiarkan seperti sekarang (saldo aplikasi = IDR saja). Pengguna memastikan tidak ada koin menggantung dan saldo hanya dari kas sebelum peluncuran. T2.6 (isi modal koin lama) TIDAK dikerjakan. Pertanyaan terbuka no. 6 di PRD: dijawab "diabaikan".

**Belum diverifikasi sejak Fase 2**: mode fee deposit (kotak centang "Nominal sudah bersih"), karena belum ada deposit nyata sejak aplikasi dipasang (arus kas = 0). Verifikasi saat ada deposit pertama.

### 1k. T4.1 (detail)

**Status: Selesai.** `cargo test -p scalping-core`, `cargo test -p scalping-ffi`, build Android, dan uji HP lulus (dilaporkan pengguna, 7 Nov 2026).

**Yang dibangun**
- `core/src/posisi.rs`: `TradeTertutup.fee` = fee beli proporsional + fee jual bagian yang cocok. `Aktif.fee_beli` (privat) melacak fee beli yang masih melekat pada koin posisi; beli tambahan menambah, jual mengurangi secara proporsional (kali dulu baru bagi). Rumus profit TIDAK berubah, karena fee sudah terkandung di modal efektif dan IDR diterima.
- `core/src/statistik.rs` (baru): `Statistik` dan `hitung_statistik(tertutup, dari_ms)`. Tanpa jaringan, memakai Decimal. 6 tes (termasuk ICNT: fee 340, profit kotor 720, 47,22%, dan jual sebagian dengan jumlah fee tidak ada yang hilang).
- `ffi/src/lib.rs`: `StatistikUi` dan `statistik_trading(path_db, rentang)`; angka sudah berupa teks, "-" bila tidak ada data.
- Tampilan statistik sekarang di layar Riwayat (U4).

**Definisi yang dipakai (usulan, lolos uji pengguna)**
- Win rate = trade untung / semua trade x 100 (impas dan rugi tidak dihitung menang).
- Rata-rata untung dan rugi dihitung hanya dari trade untung atau rugi masing-masing.
- Profit kotor = hasil bersih + total fee. Persen profit termakan fee = total fee / profit kotor x 100; "-" bila profit kotor tidak positif.
- Rentang memfilter berdasarkan waktu TUTUP trade; hari mulai 00:00 WIB, minggu mulai Senin.
- Tampilan persen memakai `round_dp(2)` tanpa nol tambahan (100 tampil "100", bukan "100.00").

**Catatan**
- Fee hanya dari trade tertutup. Fee beli koin yang masih jadi posisi terbuka tidak masuk statistik sampai posisinya terjual.
- Estimasi fee jual 0,31% hanya untuk notifikasi dan layar Posisi; statistik memakai `commission` nyata dari API.

### 1l. Fase UI: U1 sampai U4 (terbaru, detail)

**Status: Selesai.** Kode ditulis dan build berhasil; uji HP empat layar lulus, semua sesuai kriteria (dilaporkan pengguna, 8 Nov 2026).

**Keputusan pengguna (7 Nov 2026)**
- Tombol "Jual di Indodax": DIHAPUS (aplikasi tidak membuat order; posisi tertutup otomatis saat order jual terdeteksi).
- Cadangan data: perlu, ke penyimpanan LOKAL saja (ekspor dan impor berkas). Dikerjakan di U5.
- Zona waktu WIB dan minggu mulai Senin: DIKONFIRMASI.

**Arsitektur UI (tanpa Compose, semua dari kode, sesuai T1.1b)**
- `MainActivity.kt`: kerangka 4 tab + navigasi bawah, edge-to-edge (`setDecorFitsSystemWindows(false)` dan jarak dari insets, navigasi disembunyikan saat papan ketik muncul). Memuat ulang layar aktif saat `onResume`, saat data berubah, dan (jeda 1,2 detik) saat pantauan mengirim kejadian.
- `Layar.kt` (dasar tiap layar), `LayarBeranda.kt`, `LayarPosisi.kt`, `LayarRiwayat.kt`, `LayarSetelan.kt`.
- `Tema.kt` (warna prototype terang/gelap), `Komponen.kt` (kartu, tombol, kolom isian, chip, bar), `Format.kt` (titik ribuan, koma desimal, tanda, zona WIB; hanya mengubah teks, tanpa hitung uang), `Latar.kt` (antrean satu thread untuk baca database + pool untuk jaringan), `Sinkron.kt` (sinkron trade/arus kas manual dan otomatis, dipakai bersama).
- `GrafikView.kt` diganti: warna dari tema, label berformat Rupiah, tinggi 150dp.
- `LayananLatar.kt`, `KunciAman.kt`, manifest, tema XML, `build.gradle.kts`: TIDAK berubah.

**FFI baru (`ffi/src/lib.rs`, agar Kotlin tidak menghitung uang)**
- `baca_setelan(path_db)` -> `SetelanTpSl` (TP/SL terbaru, untuk mengisi kolom Setelan).
- `RingkasanTargetUi.hasil_persen`: (saldo - modal aktif) / modal aktif x 100, 2 desimal (Beranda).
- `daftar_posisi(path_db)` -> `Vec<PosisiUi>`: teks berformat Indonesia, angka sama dengan notifikasi beli (estimasi bersih memakai fee jual perkiraan 0,31%).
- 1 tes baru `data_layar_ui_setelan_hasil_persen_dan_posisi` (total tes ffi: 15).

**Perilaku**
- Setelan: TP/SL, target harian/mingguan (kosongkan keduanya = tanpa target), kunci API (tes, simpan, ganti, hapus dengan konfirmasi), pantauan realtime, tombol optimasi baterai, sinkron manual, sinkron deposit/withdraw dan mode fee, log pantauan. Pertama kali tanpa kunci langsung membuka tab Setelan.
- Beranda: saldo, hasil Rp dan %, grafik, progress target harian/mingguan (bar, TERCAPAI), chip status koneksi.
- Posisi: per posisi tampil koin, modal, tangga SL/modal efektif/TP, estimasi hasil bersih TP dan SL, waktu buka. Tanpa tombol Jual.
- Riwayat: ringkasan hari ini (hasil, jumlah, untung, fee), daftar trade tertutup (30 per halaman), statistik tiga rentang, cek dan catat koreksi saldo (hasil terakhir dipertahankan saat layar disegarkan).

**Catatan**
- Galat build pertama ("Unresolved reference bacaSetelan/daftarPosisi/PosisiUi/hasilPersen") terjadi karena binding UniFFI belum dibuat ulang; selesai setelah patch FFI diterapkan dan build lewat `tools/bangun_android.sh`.
- Berkas `ffi-lib.rs.patch` hanya perantara (sudah diterapkan), tidak perlu disimpan di repo.
- Layar asli memakai teks sistem (bukan font Manrope prototype).

**Yang diuji di HP (lulus)**
1. Empat tab muncul dengan navigasi bawah; tema mengikuti terang/gelap sistem; papan ketik tidak menutup kolom isian.
2. Setelan: kolom TP/SL dan target terisi dari data tersimpan; Simpan setelan berhasil dan nilai salah ditolak dengan pesan; Sinkron sekarang; mulai/hentikan pantauan; tombol baterai; Sinkron deposit/withdraw.
3. Beranda: saldo dan hasil cocok dengan hitungan lama; grafik dua garis; bar target; chip status berubah saat pantauan hidup/mati.
4. Posisi: kartu per posisi sesuai notifikasi beli (contoh TP 4.046, SL 3.966); kosong saat tidak ada posisi.
5. Riwayat: ringkasan dan daftar cocok dengan sebelumnya; statistik tiga rentang; cek selisih dan catat koreksi.
6. Setelah ada order terisi saat app terbuka, layar aktif menyegarkan diri.

### 1m. U5: ekspor dan impor data lokal (detail)

**Status: Selesai.** `cargo test -p scalping-core`, `cargo test -p scalping-ffi`, build Android, dan uji HP lulus (dilaporkan pengguna, 8 Nov 2026).

**Yang dibangun**
- `core/src/store.rs`: `Penyimpanan::ekspor_ke(tujuan)` (salinan konsisten lewat `VACUUM INTO`, menolak menimpa berkas yang ada), `Penyimpanan::periksa_cadangan(path)` (buka hanya-baca, `PRAGMA integrity_check`, tabel wajib `trade`/`meta`/`snapshot`, kolom wajib; mengembalikan jumlah trade, arus kas, koreksi, dan waktu snapshot awal), `Penyimpanan::pulihkan(path_db, sumber)`. Dependensi tidak berubah (SQLite bundled mendukung `VACUUM INTO`). 4 tes baru.
- Urutan aman `pulihkan`: periksa berkas, salin bersih ke `<db>.impor.tmp`, verifikasi semua angka desimal terbaca, simpan database lama sebagai `<db>.sebelum-impor`, hapus `-wal`/`-shm`/`-journal`, lalu `rename`. Gagal di tahap mana pun tidak merusak database yang ada.
- `ffi/src/lib.rs`: `ekspor_cadangan`, `periksa_cadangan`, `pulihkan_cadangan`, `RingkasanCadanganUi`. 1 tes baru (`tes_cadangan`); total tes ffi 16.
- Android: `Cadangan.kt` (pemilih berkas sistem SAF: `ACTION_CREATE_DOCUMENT` dan `ACTION_OPEN_DOCUMENT`; berkas perantara di cache aplikasi, dihapus setelah dipakai), kartu "Cadangan data" di `LayarSetelan.kt` (tombol Ekspor dan Impor, tombol Impor nonaktif saat pantauan berjalan), `MainActivity.onActivityResult`. Berkas ekspor bernama `catatan-scalping-<tanggal WIB>.db`.

**Perilaku**
- Ekspor: Rust membuat salinan di cache, diperiksa ulang, lalu disalin ke lokasi pilihan pengguna. Pesan hasil memuat jumlah trade, deposit/withdraw, dan koreksi.
- Impor: berkas diperiksa dulu, dialog konfirmasi menampilkan ringkasan isi, baru database diganti. Setelah impor, kolom setelan dimuat ulang dan layar disegarkan.
- Kunci API TIDAK ikut dicadangkan (database tidak menyimpannya). Di HP baru, setelah impor: masukkan kunci, jalankan "Sinkron sekarang" satu kali, baru mulai pantauan (penanda sinkron pertama ada di preferensi aplikasi, bukan di database).

**Catatan dan batasan**
- Impor hanya mengecek pantauan (harus mati); belum mencegah impor saat sinkron manual atau sinkron otomatis sedang berjalan. Hindari impor pada saat itu.
- Berkas `.sebelum-impor` disimpan di folder privat aplikasi dan ditimpa pada impor berikutnya (hanya satu salinan terakhir).
- Cadangan manual saja; belum ada cadangan terjadwal.

---

## 2. Masalah

### Belum terselesaikan atau dikenal
| Masalah | Penyebab / dugaan | Status |
| --- | --- | --- |
| Impor cadangan saat sinkron manual atau otomatis berjalan tidak dicegah | Impor hanya mengecek pantauan mati | Dikenal (U5); jangan impor saat sinkron berjalan |
| `tradeId` event WebSocket vs REST belum dibandingkan | Belum ada data pembanding | Tidak menghalangi (anti-dobel dari REST) |
| Kemungkinan "database is locked" saat dua koneksi SQLite aktif bersamaan | `busy_timeout` 5 detik ada; layar kini membaca lewat antrean satu thread, tulis oleh pantau tetap bisa bersamaan | Pantau saat uji lanjutan |
| Pantauan tidak menyala sendiri setelah HP reboot | Belum ada penerima BOOT_COMPLETED | Dicatat; belum diputuskan perlu atau tidak |
| Clearing fee baru teramati bernilai 0 | Hanya 2 sampel kecil (UAI) | Pantau; bila `commission` REST berbeda dari fee+tax+clearing event, andalkan REST |
| Koin yang sudah tidak dipegang (contoh ICNT) tidak terpindai | Daftar koin dari saldo + riwayat tersimpan | Dikenal; solusi sementara `koin_tambahan`, belum ada isiannya di UI |
| `pxgidr` ditolak Indodax (kode 1109) | Dugaan: tidak ada pasangan IDR | Dilewati dengan benar; penyebab belum dipastikan |
| TP/SL dibulatkan ke rupiah utuh | Aturan tick harga per koin belum diperiksa | Belum diperiksa |
| Dua order beli koin sama yang selesai nyaris bersamaan menampilkan notifikasi order yang sama | Order dikenali dari order beli terbaru koin itu; `OrderRingkas` belum punya ID order | Dikenal (T1.5); perbaikan bila ID order cocok dengan ID event WebSocket |
| Fee jual perkiraan 0,31% tetap untuk estimasi bersih (notifikasi dan layar Posisi); jual market sekitar 0,61% (struk UAI 7 Nov: 123 dari 20.112) | Belum ada setelan | Dikenal; estimasi bisa terlalu optimistis untuk jual market; statistik memakai fee nyata |
| `amount` deposit: bersih atau belum dikurangi `totalFee`? | Belum diverifikasi dengan saldo nyata | Dua mode tersedia (1i); konfirmasi dengan satu deposit nyata |
| Bentuk baris withdraw belum teramati | Respons withdraw akun masih kosong | Parser toleran; validasi saat ada withdraw nyata |
| Ukuran halaman `fiat/orders` tidak diketahui | Tidak ada dokumen/uji | Rentang >= 100 baris dibelah; pantau |
| Selisih saldo sementara saat order limit terisi sebagian | Order dicatat saat DONE/CANCELLED (FR-08), tetapi saldo asli sudah berubah | Dikenal (T3.3); jangan koreksi saat ada order menggantung, sinkron dulu |
| Selisih 1 rupiah antara hasil jual dan modal beli berikutnya (data 3 Nov) | Diduga pembulatan | Belum dikonfirmasi, tidak berdampak |
| Apakah sisa 3 IDR (anggaran order tak terpakai) kembali ke saldo | Belum dicek di saldo | Belum dikonfirmasi; tidak memengaruhi rumus karena saldo diambil dari API |

### Sudah terselesaikan (ringkas)
Uji HP layar asli U1 sampai U4 dan U5 lulus, semua sesuai kriteria (8 Nov); Build UI pertama gagal karena binding UniFFI belum diperbarui (diselesaikan 8 Nov: terapkan patch FFI, build lewat `bangun_android.sh`); Notifikasi beli dan jual muncul otomatis, satu per order, tanpa ganda, termasuk notifikasi trade terlewat setelah sambung ulang (T1.5, uji HP 7 poin lolos); Pemilihan pustaka WebSocket + TLS (T1.4b: rustls provider `ring` + webpki-roots, tidak bentrok dengan ureq, build Android berhasil); foreground service tidak mati saat layar dikunci dan app dibuang dari recent apps (uji HP T1.4c); sambung ulang dan rekonsiliasi setelah koneksi putus (uji HP T1.4c); Token WebSocket privat dengan key v2 (T0.6: berhasil); format event order_update dan nama status FILL (terkonfirmasi); fee event = `commission` REST (cocok pada 2 trade); HTTP 403 dari skrip lama (file belum tertimpa); SHA256 untuk TAPI v2; format simbol huruf kecil dan parameter `startTime`/`endTime` maks 7 hari; "Jumlah Order" vs uang keluar (pakai "Order Terisi"); format respons `/account`; `adb: more than one device` (skrip menerima serial); HTTP 400 kode 1109 saat sinkron (simbol ditolak dilewati, terbukti di HP); build SQLite bundled untuk Android (berhasil); aset IDR ada di saldo akunmu (sinkron pertama berhasil membuat snapshot, yang membutuhkan IDR).

---

## 3. Tahap selanjutnya

### Lingkungan pengembangan (dilaporkan 7 November 2026)
Arch Linux; Rust 1.98.1 dengan 4 target Android dan `cargo ndk`; JDK 17; Gradle 9.8.2 di PATH; Android SDK di `~/android-sdk` (platform 33 sampai 36, build-tools 34 sampai 36, NDK 26/27/28.2, `ANDROID_NDK_HOME` diatur); tanpa Android Studio, build lewat CLI; HP fisik via adb nirkabel.

### Teknologi
Rust untuk semua logika dan backend (berjalan di dalam HP), Kotlin/Java hanya klien Android (UI, foreground service, notifikasi, penyimpanan kunci). Jembatan: UniFFI 0.31. Penyimpanan: SQLite (rusqlite bundled) dikelola dari Rust.

### Task berikutnya (urut)
| Task | Isi | Syarat |
| --- | --- | --- |
| Pekerjaan kecil | PRD v1.3 (lihat "Pekerjaan kecil tertunda") | Claude |
| Opsional | Pantauan menyala otomatis setelah HP reboot (penerima BOOT_COMPLETED); verifikasi mode fee deposit saat ada deposit nyata | Keputusan pengguna |

### Keputusan yang masih terbuka
| Hal | Pilihan | Status |
| --- | --- | --- |
| Setelan TP/SL baru | Hanya posisi baru atau juga yang terbuka | Dipakai usulan "hanya posisi baru" (T1.3); catatan ini tampil di layar Setelan; belum dikonfirmasi eksplisit |
| Koin yang dipegang sebelum app dipasang | Diabaikan atau diisi modalnya | Terjawab: diabaikan (7 Nov 2026) |
| Mode fee deposit/withdraw (`amount` bersih atau tidak) | Lihat 1i | Belum (butuh deposit nyata) |
| Pantauan menyala otomatis setelah HP reboot | Perlu atau tidak | Belum |

### Keputusan yang sudah terjawab
- Tombol "Jual di Indodax": dihapus (7 Nov 2026).
- Cadangan data: ekspor/impor ke penyimpanan lokal saja (7 Nov 2026); dikerjakan di U5, lulus uji HP 8 Nov 2026.
- Zona waktu WIB dan minggu mulai Senin: dikonfirmasi (7 Nov 2026).
- Estimasi hasil bersih di bawah TP/SL: tampil (T1.5, permintaan pengguna).
- Beli tambahan koin yang sama: tampil TP/SL order itu dan posisi keseluruhan (T1.5).
- Koin lama sebelum aplikasi dipasang: dibiarkan (saldo aplikasi = IDR saja); pengguna memastikan tidak ada koin menggantung saat peluncuran (7 Nov 2026).
- Koreksi saldo diperlakukan seperti setoran (mengubah saldo dan modal aktif, bukan profit); selisih di bawah Rp 1 diabaikan (T3.3, usulan, lolos uji HP).
- Saldo versi aplikasi dihitung dari catatan (bukan dari panggilan `/account`) dan notifikasi "tercapai" digabung ke notifikasi jual (T2.3/T2.4, usulan, lolos uji HP).
- Target harian dan mingguan: hanya nilai terbaru, batas lebih dari 0 dan paling banyak 100 persen (T2.1).
- Statistik (T4.1): tiga rentang (hari/minggu/semua), definisi win rate dan persen profit termakan fee seperti di 1k; lolos uji pengguna 7 Nov 2026.
- T2.5, Fase 3, dan koreksi saldo lolos uji HP 7 Nov 2026.
- Koreksi saldo ditempatkan di layar Riwayat, deposit/withdraw dan mode fee di layar Setelan (U1/U4).
- Layar asli U1 sampai U4 lolos uji HP, semua sesuai kriteria (8 Nov 2026).
- U5: salinan database lama otomatis disimpan sebagai `.sebelum-impor`; kunci API tidak dicadangkan; impor wajib dengan pantauan mati (usulan, lolos uji HP).

### Pekerjaan kecil tertunda
- PRD perlu v1.3: FR-22 (grafik), koreksi saldo, FR-23 (riwayat), FR-25 (statistik) sudah diimplementasikan dan kini ada di layar asli; FR-14 diubah (tombol Jual dihapus); saldo versi aplikasi dihitung dari catatan; koreksi diperlakukan seperti setoran; pertanyaan terbuka no. 3 (tombol Jual: dihapus), no. 4 (estimasi bersih tampil, fee jual perkiraan 0,31%), no. 6 (diabaikan), no. 8 (cadangan lokal), no. 9 (WIB, Senin) terjawab; FR-16 progress target selesai di T2.4; bagian 9 (token WebSocket dengan key v2 terjawab, tanda tangan SHA512 untuk token, nama status `FILL`, WebSocket sebagai pemicu) dan pertanyaan terbuka.

### Fase berikutnya
- **Fase 2**: selesai (T2.1 sampai T2.5)
- **Fase 3**: selesai (T3.1 sampai T3.3)
- **Fase 4**: selesai (T4.1)
- **Fase UI**: selesai (U1 sampai U5, lulus uji HP 8 Nov 2026)

---

## 4. Cara instalasi

Kebutuhan: Python 3.8+, git, Rust (rustup), dan toolchain Android (lihat bagian 3). Skrip Python tidak butuh `pip install`.

1. Masuk ke folder proyek: `cd catatan-scalping`
2. Buat key di indodax.com/trade_api: key **TAPI v2**, izin **baca saja**, tanpa pembatasan IP.
3. Salin `.env.example` menjadi `.env`, isi `INDODAX_API_KEY` dan `INDODAX_API_SECRET` (untuk skrip dan contoh Rust di komputer; aplikasi Android menyimpan key sendiri di HP).

Tes inti Rust:
```
cargo test -p scalping-core        # 51 tes per T1.4a; T4.1 menambah 6 di statistik.rs (periksa jumlah terbaru)
cargo test -p scalping-ffi         # T1.5: 6 tes; T2.1: +2 (8); T2.3/T2.4: +2 (10); T3: +4 (14); U1: +1 (15); U5: +1 (16); U5 juga menambah 4 tes di store.rs (periksa jumlah terbaru)
python tools/fase2_fiat.py             # T2.2: bentuk respons deposit (--jenis 1 untuk withdraw)
cargo run -p scalping-core --example pantau   # T1.4b: pantau realtime di komputer, Enter untuk berhenti
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

Build dan pasang Android (SELALU lewat skrip ini; Gradle langsung tidak memperbarui binding UniFFI):
```
cargo update            # bila dependensi berubah
bash tools/bangun_android.sh --pasang <serial-adb>
```
Uji di HP: buka tab Setelan, simpan kunci, isi TP/SL, "Sinkron sekarang", lalu mulai pantauan (matikan optimasi baterai, izinkan notifikasi). Lakukan transaksi kecil di Indodax: notifikasi beli/jual muncul otomatis (T1.5) dan Beranda, Posisi, Riwayat ikut menyegarkan diri.

Alur git setiap task:
```
git add .
git commit -m "Tx.y: deskripsi singkat"
```
Catatan: `git add .` wajib agar file Kotlin baru (Tema.kt, Format.kt, Komponen.kt, Latar.kt, Sinkron.kt, Layar.kt, Layar*.kt, Cadangan.kt) ikut tercatat.

---

## 5. Struktur proyek

### Saat ini (setelah U5)
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
│   │   ├── store.rs                     (SQLite: trade, snapshot, meta, setelan_tp_sl, arus_kas, koreksi_saldo; U5: ekspor_ke, periksa_cadangan, pulihkan)
│   │   ├── sync.rs                      (rekonsiliasi REST per koin, snapshot saldo awal)
│   │   ├── arus_kas.rs                  (T2.2: parser fiat/orders, arus kas, sinkron_arus_kas)
│   │   ├── periode.rs                   (T2.3: periode WIB, saldo versi aplikasi, pencapaian dan progress)
│   │   ├── posisi.rs                    (hitung ulang posisi, trade tertutup dengan fee, penyesuaian)
│   │   ├── statistik.rs                 (T4.1: win rate, rata-rata, total fee, persen profit termakan fee)
│   │   ├── ws.rs                        (protokol WebSocket privat: token, parser event, pemicu sinkron; tanpa jaringan)
│   │   └── ws_klien.rs                  (T1.4b: sambungan WebSocket sebenarnya, sambung ulang, picu sinkron_koin; fungsi pantau)
│   └── examples/
│       ├── cek_data.rs                  (cek hitungan dari file JSON hasil Fase 0)
│       ├── cek_koneksi.rs               (tes /account dengan key dari .env)
│       └── pantau.rs                    (T1.4b: uji pantauan realtime di komputer, db data/pantau_uji.db)
├── ffi/                                 (crate scalping-ffi, jembatan UniFFI)
│   ├── Cargo.toml
│   └── src/lib.rs                       (cek_koneksi, sinkron_sekarang, order_terbaru, simpan_setelan, baca_setelan, hitung_posisi, daftar_posisi, Pemantau, PendengarPantau, KejadianUi, isi_notifikasi, ringkasan_posisi_notif, IsiNotifOrder, target dan arus kas, hitung_target (hasil_persen), riwayat_tertutup, data_grafik, koreksi saldo, statistik_trading, ekspor_cadangan, periksa_cadangan, pulihkan_cadangan; 16 tes)
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
│           │   ├── MainActivity.kt      (U1: kerangka 4 tab, navigasi bawah, insets, siklus hidup; U5: onActivityResult)
│           │   ├── Layar.kt             (U1: dasar tiap layar)
│           │   ├── LayarBeranda.kt      (U2: saldo, grafik, progress target, status koneksi)
│           │   ├── LayarPosisi.kt       (U3: posisi terbuka, tangga SL/modal/TP, tanpa tombol Jual)
│           │   ├── LayarRiwayat.kt      (U4: ringkasan, daftar trade, statistik, koreksi saldo)
│           │   ├── LayarSetelan.kt      (U1: TP/SL, target, kunci, pantauan, sinkron, arus kas, log; U5: kartu Cadangan data)
│           │   ├── Cadangan.kt          (U5: ekspor/impor lewat pemilih berkas sistem)
│           │   ├── Tema.kt              (U1: warna terang/gelap sesuai prototype)
│           │   ├── Format.kt            (U1: titik ribuan, koma desimal, tanda, waktu WIB)
│           │   ├── Komponen.kt          (U1: kartu, tombol, kolom isian, chip, bar)
│           │   ├── Latar.kt             (U1: antrean baca database dan pool jaringan)
│           │   ├── Sinkron.kt           (U1: sinkron trade dan arus kas, manual dan otomatis)
│           │   ├── GrafikView.kt        (T3.2; U2: bertema, label Rupiah)
│           │   ├── LayananLatar.kt      (T1.4c: foreground service + PantauBersama; T1.5: saluran "Order terisi" dan pekerja notifikasi)
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
    ├── fase2_fiat.py                    (T2.2: lihat bentuk respons fiat/orders, angka disamarkan, baca saja)
    ├── cek_lingkungan.py                (laporan versi Rust/Android/adb)
    └── bangun_android.sh                (Rust -> binding -> APK -> adb)
```

### Rencana
Semua task pada rencana awal selesai. Pekerjaan tersisa: PRD v1.3 dan dua keputusan opsional (lihat bagian 3). Struktur di atas sudah mencakup U5; tidak ada file baru lain yang direncanakan.

---

## 6. Log commit
(isi hash dan pesan commit di sini)
- U5: `U5: ekspor dan impor data lokal` (hash: ...)
- U4: `U4: layar Riwayat, statistik, koreksi saldo` (hash: ...)
- U3: `U3: layar Posisi (tanpa tombol Jual)` (hash: ...)
- U2: `U2: layar Beranda` (hash: ...)
- U1: `U1: kerangka navigasi, tema, format Indonesia, layar Setelan; FFI baca_setelan/hasil_persen/daftar_posisi` (hash: ...)
- T4.1: `T4.1: statistik trading (win rate, rata-rata, total fee, persen termakan fee)` (hash: ...)
- T3.3: `T3.3: koreksi saldo terhadap saldo asli Indodax` (hash: ...)
- T3.2: `T3.2: grafik saldo dan modal aktif` (hash: ...)
- T3.1: `T3.1: riwayat trade tertutup` (hash: ...)
- T2.5: `T2.5: sinkron arus kas dari layanan latar` (hash: ...)
- T2.4: `T2.4: progress target di notifikasi jual dan layar uji` (hash: ...)
- T2.3: `T2.3: periode WIB, saldo versi aplikasi, pencapaian dan progress` (hash: ...)
- T2.2: `T2.2: arus kas deposit dan withdraw IDR` (hash: ...)
- T2.1: `T2.1: setelan target harian dan mingguan` (hash: ...)
- T1.5: `T1.5: notifikasi beli dan jual dengan TP/SL dan hasil bersih` (hash: ...)
- T1.4c: `T1.4c: layanan latar, jembatan FFI pantauan, izin notifikasi dan baterai` (hash: ...)
- T1.4b: `T1.4b: sambungan WebSocket privat, sambung ulang, pemicu sinkron` (hash: ...)
- T1.4a: `T1.4a: protokol WebSocket privat (token, parser event, pemicu sinkron)` (hash: ...)
- T0.6: `T0.6: token WebSocket privat dan format event terkonfirmasi` (hash: ...)
- T1.3: `T1.3: posisi, modal efektif, TP/SL, setelan` (hash: ...)
- T1.2: `T1.2: penyimpanan SQLite, rekonsiliasi REST, snapshot saldo awal` (hash: ...)
- T1.1b: `T1.1b: proyek Android, jembatan UniFFI, layar koneksi` (hash: ...)
- T1.1c: `T1.1c: parser /account dan cek key baca saja` (hash: ...)
- T1.1a/T1.0: `T1.1a: inti Rust (klien Indodax, kalkulasi posisi), keputusan teknologi, PRD v1.2`
