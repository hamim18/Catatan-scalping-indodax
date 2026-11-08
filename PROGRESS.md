# PROGRESS: Catatan Scalping

Terakhir diperbarui: 8 November 2026 (Fase 7 selesai: T7.1 dan T7.2 lulus uji HP, tampilan disederhanakan; PRD v1.7; lihat 1v; T8.1 lulus uji HP; rilis uji 0.8.0-uji siap dipakai, lihat 1w dan 1x)
Status: **Fase 6 SELESAI (8 Nov 2026, lihat 1r, 1s, 1t): hanya ada satu angka saldo, yaitu TOTAL ASET nilai pasar (IDR sebagai "Buying power" + koin × harga terakhir). Target dan grafik dihitung dari profit trade yang sudah ditutup; kerugian koin yang sedang dipegang tampil terpisah sebagai floating P/L. T6.1 sampai T6.5 SUDAH lulus tes dan uji HP (tes terakhir: core 99, ffi 24); T6.6 (celah impor cadangan, hanya Kotlin) lulus uji HP. Saldo versi aplikasi, snapshot periode, modal aktif, setoran bersih, koreksi saldo, dan sisa kode grafik dua garis sudah dihapus.** **Fase 7 SELESAI (8 Nov 2026, lihat 1v): tampilan Beranda, Posisi, Riwayat, dan Setelan disederhanakan (hanya Kotlin, perhitungan tidak berubah); T7.1 dan T7.2 lulus uji HP.** Semua fase rencana selesai dan lolos uji (Fase 0 sampai 7, Fase UI U1 sampai U5, T4.1, T5.1, T5.2). Tersisa pekerjaan opsional saja: uji withdraw nyata dan ID order di notifikasi beli (lihat bagian 3).

**Fase 8 (uji lapangan) BERJALAN (8 Nov 2026, lihat 1w dan 1x): T8.1 (log diagnostik permanen `diag.log`, tombol Bagikan log, penjaga pantauan) LULUS uji HP sesuai kriteria (dilaporkan pengguna 8 Nov 2026); hanya Kotlin, jumlah tes tetap core 99, ffi 24. Rilis uji pribadi 0.8.0-uji (versionCode 2) siap dipakai harian untuk mencari bug dan peningkatan; panduan di `docs/PANDUAN_RELEASE.md`, README diperbarui. Belum ada commit/tag rilis sampai pengguna menjalankannya.**

## Aturan kerja
- Satu task selesai = satu `git commit`.
- Setelah tiap task: perbarui file ini, lalu commit.
- Jangan pernah commit `.env` atau isi `data/` (sudah ada di `.gitignore`).
- Aplikasi hanya membaca data. Key API wajib izin **baca saja**.
- Build Android SELALU lewat `bash tools/bangun_android.sh` (skrip ini membuat ulang binding Kotlin dari Rust). Menjalankan Gradle langsung tidak memperbarui binding, sehingga fungsi FFI baru tampil sebagai "Unresolved reference".
- Setiap kali mengganti file sumber dari unduhan atau salinan, jalankan `touch` pada file itu (atau `touch core/src/*.rs ffi/src/lib.rs`) sebelum build. Cargo menilai perubahan dari waktu modifikasi file; file bertanggal lebih tua dari build terakhir dianggap tidak berubah (dialami di T5.2, lihat 1q).

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
| T5.2 | Logika deposit: `ArusKas::efek_saldo` untuk deposit selalu `+amount` (fee hanya informasi); kotak centang "nominal sudah bersih" DIHAPUS dari layar (withdraw tetap `-(amount + fee)`); teks baris deposit di Setelan diperjelas; 2 tes baru dan 2 tes diubah. Lihat 1p dan 1q | Selesai (build, tes, uji HP lulus 8 Nov: baris deposit `saldo +Rp 7.670`) |
| T6.1 | Total aset (nilai pasar) di Beranda: bagian A (Rust: `harga.rs`, `ambil_harga_idr`, FFI `total_aset`) dan bagian B (Kotlin `LayarBeranda.kt`: kartu Total aset). Lihat 1r | Lulus: `cargo test` (core 105, ffi 16) dan uji HP Beranda ("bagus, tidak ada masalah dan sesuai", dilaporkan pengguna 8 Nov 2026) |
| T6.2 | Beranda tunggal: satu kartu Total aset (Buying power + koin per aset), baris Profit trading, grafik akumulasi profit trade tertutup, target harian/mingguan dari profit trade tertutup (persen = profit ÷ (total aset − profit) × 100). Lihat 1r | Lulus: `cargo test` (core 114, ffi 19) dan uji HP Beranda sesuai kriteria (dilaporkan pengguna 8 Nov 2026) |
| T6.3 | Floating P/L: FFI `ambil_harga_terakhir` (jaringan) dan `floating_posisi` (database), harga sekarang sebagai penanda di tangga layar Posisi (merah bila di bawah SL), satu baris "Floating" di Beranda. Floating dihitung SEBELUM fee jual dan tidak masuk target/grafik. Lihat 1r | Lulus: `cargo test` (ffi 25) dan uji HP sesuai kriteria (dilaporkan pengguna 8 Nov 2026) |
| T6.4 | Pembersihan saldo lama: dihapus snapshot periode, modal aktif, setoran bersih, koreksi saldo (`DataSaldo` dan turunannya, fungsi FFI terkait, 13+3 tes lama); deposit/withdraw jadi riwayat di layar Riwayat; notifikasi jual memakai total aset terakhir yang disimpan Beranda (Opsi A); PRD v1.6. Lihat 1s | Lulus: `cargo test` (core 99, ffi 24) dan uji HP sesuai kriteria (dilaporkan pengguna 8 Nov 2026) |
| T6.5 | Pembersihan kode grafik lama: `GrafikView.setData` (dua garis) dan record `TitikGrafikUi` di FFI dihapus. Hanya `setProfit` yang tersisa. Lihat 1t | Lulus: `cargo test` (core 99, ffi 24, jumlah tidak berubah) dan uji HP sesuai kriteria (dilaporkan pengguna 8 Nov 2026) |
| T6.6 | Cegah impor cadangan saat sinkron berjalan: penanda `Sinkron.imporBerjalan`, `sedangSinkron()`, `mulaiImpor()/selesaiImpor()`; sinkron trade dan arus kas menolak mulai saat impor; impor dicek ulang saat tombol "Ganti" ditekan. Hanya Kotlin (3 berkas). Lihat 1u | Lulus: build berhasil dan uji HP sesuai kriteria (dilaporkan pengguna 8 Nov 2026). Tes Rust tidak berubah (core 99, ffi 24) |
| T5.1 | Pantauan menyala otomatis setelah HP restart: `PenerimaBoot.kt` (BOOT_COMPLETED), izin `RECEIVE_BOOT_COMPLETED` dan `<receiver>` di `AndroidManifest.xml`. Hanya bila preferensi `aktif` = true (pantauan terakhir menyala), kunci ada, dan pernah sinkron; bila tidak, penanda dimatikan dan alasan dicatat di log | Selesai (build, tes, uji HP lulus 8 Nov) |
| T7.1 | Sederhanakan Beranda dan Posisi: dihapus teks penjelas, jumlah × harga per koin, "Diperbarui", koin tanpa harga IDR (PXG), legenda dan keterangan titik grafik, rumus di kartu Target; floating DIGABUNG ke baris di bawah Profit trading ("N trade" kiri, "floating ..." kanan); chip menjadi "Terhubung"/"Pantauan mati"; Posisi: satu baris harga + floating, satu baris estimasi bersih (persen) + waktu buka. Hanya Kotlin (2 berkas). Lihat 1v | Selesai (build berhasil, uji HP sesuai kriteria, dilaporkan pengguna 8 Nov 2026) |
| T7.2 | Sederhanakan Riwayat dan Setelan: Riwayat tanpa chip dan baris "Semua: ...", statistik dipadatkan (Win rate + untung/rugi/impas, Fee + persen, profit kotor tidak ditampilkan), deposit/withdraw satu baris tanpa fee; Setelan tanpa catatan penjelas, kartu Deposit/withdraw IDR dihapus, tombol baterai disembunyikan bila aman, log pantauan tersembunyi, "Inti Rust versi" dihapus. Hanya Kotlin (2 berkas). Lihat 1v | Selesai (build berhasil, uji HP sesuai kriteria, dilaporkan pengguna 8 Nov 2026) |
| T8.1 | Diagnostik untuk uji lapangan: `LogDiag.kt` (log permanen `diag.log`), `Aplikasi.kt` (catat awal proses, alasan proses sebelumnya berakhir, crash), `PenjagaPantauan.kt` (alarm 5 menit, peringatan bila pantauan mati/terputus lebih dari 5 menit), tombol "Bagikan log" di Setelan, versi 0.8.0-uji. Hanya Kotlin. Lihat 1w | Lulus uji HP (dilaporkan pengguna, 8 Nov 2026) |

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
- **Pembulatan TP/SL** ke rupiah utuh hanya untuk tampilan (`bulatkan_idr`). Aturan tick harga per koin TIDAK akan diperiksa: diputuskan 8 Nov 2026 tidak perlu, karena TP/SL hanya acuan dan harga akhir diinput manual di Indodax.
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

**Keputusan yang dipakai (setelan TP/SL baru DIKONFIRMASI 8 Nov 2026; penyesuaian tanpa modal dikonfirmasi 7 Nov 2026)**
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
- Fee jual perkiraan 0,31% dipakai tetap; diputuskan 8 Nov 2026 TIDAK perlu setelan (estimasi hanya perkiraan, hasil nyata dari `commission` API).
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
- TERJAWAB 8 Nov 2026 untuk deposit: `amount` sudah bersih, fee tidak dikurangkan lagi (lihat 1o). Mode bawaan lama (`amount - fee`) salah dan diperbaiki di T5.2. Untuk withdraw masih dua mode (bawaan `-(amount + fee)`, kotak "sudah bersih" `-amount`), belum terverifikasi.
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

**Diverifikasi 8 Nov 2026 (deposit):** `amount` deposit sudah bersih (lihat 1o). Withdraw masih belum diverifikasi.

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
- Impor mengecek pantauan (harus mati). Sejak T6.6 impor juga ditolak saat sinkron manual atau otomatis berjalan (lihat 1u).
- Berkas `.sebelum-impor` disimpan di folder privat aplikasi dan ditimpa pada impor berikutnya (hanya satu salinan terakhir).
- Cadangan manual saja; belum ada cadangan terjadwal.

### 1n. T5.1: pantauan menyala otomatis setelah HP restart (detail)

**Status: Selesai.** Kode ditulis 8 Nov 2026; build, tes, dan uji HP lulus (dilaporkan pengguna, 8 Nov 2026).

**Yang dibangun**
- `PenerimaBoot.kt` (baru): menerima `BOOT_COMPLETED`. Bila `PREF_AKTIF` = true, kunci API ada, dan `PREF_PERNAH_SINKRON` = true, memanggil `LayananLatar.mulai`. Bila syarat tidak terpenuhi atau mulai gagal, `PREF_AKTIF` dimatikan (supaya layar tidak menampilkan "Berjalan" palsu) dan alasannya dicatat di Log pantauan. Dilewati bila pemantau sudah berjalan.
- `AndroidManifest.xml`: izin `RECEIVE_BOOT_COMPLETED`, `<receiver android:name=".PenerimaBoot" android:exported="false">` dengan filter BOOT_COMPLETED.
- Tidak berubah: Rust, FFI, `LayananLatar.kt`, `Sinkron.kt`, layar. Tidak ada tombol baru: mengikuti pilihan terakhir pengguna (menyala atau dihentikan).

**Catatan**
- Android 15 melarang beberapa tipe foreground service dimulai dari boot (dataSync, mediaPlayback, dan lain-lain); `specialUse` yang dipakai tidak termasuk.
- Pasang ulang/update APK lewat adb mematikan pantauan dan harus dinyalakan manual (perilaku lama; `MY_PACKAGE_REPLACED` sengaja tidak ditangani untuk menghindari mulai ganda dengan START_STICKY).
- Bila di HP tertentu tidak menyala: izinkan Autostart untuk aplikasi (sebagian merk membatasinya), pastikan optimasi baterai dimatikan; cadangan terakhir ubah `exported` ke `true`.

**Yang diuji di HP (lulus, 8 Nov 2026)**
1. Mulai pantauan, restart HP, buka kunci layar, tunggu sekitar 1 menit: notifikasi tetap pantauan muncul tanpa membuka aplikasi.
2. Buka Setelan: pantauan "Berjalan"; log memuat "dinyalakan otomatis setelah HP restart" dan "Tersambung".
3. Transaksi kecil di Indodax: notifikasi beli/jual muncul.
4. Hentikan pantauan, restart HP: tetap mati.

### 1o. Uji deposit nyata (8 Nov 2026, detail)

**Status: Terverifikasi untuk deposit; withdraw belum teruji.** Dilaporkan pengguna dengan screenshot HP, 8 Nov 2026.

**Data nyata**
- Pengguna mentransfer Rp 11.000. API `fiat/orders` memberi `amount` Rp 7.670 dan `totalFee` Rp 3.330 (7.670 + 3.330 = 11.000).
- Saldo IDR asli Indodax: Rp 69.652 sebelum, Rp 77.322 sesudah (naik **Rp 7.670** = `amount`).
- Sebelum deposit: "Cek selisih saldo" = Rp 0 (cocok). Sesudah deposit, dengan mode bawaan: aplikasi mencatat +Rp 4.340 (amount - fee), jadi selisih +Rp 3.330 (saldo aplikasi 73.992 vs asli 77.322).

**Kesimpulan**
- `amount` deposit SUDAH bersih: fee dipotong Indodax sebelum dana masuk akun. `totalFee` hanya informasi, JANGAN dikurangkan lagi dari saldo.
- Mode bawaan (`amount - fee` untuk deposit) salah. Memperbaiki lewat kotak "Nominal sudah bersih" menyelesaikan satu kasus, tetapi logika bawaannya perlu diubah (T5.2).
- Jangan memakai "Catat koreksi" untuk menutup selisih ini (akan menyembunyikan penyebab).

**Withdraw**
- Belum bisa diuji: saldo pengguna di bawah batas minimum penarikan. Bentuk baris withdraw dan mode fee withdraw tetap BELUM terverifikasi. Uji saat saldo cukup (bandingkan A - B dengan `amount` dan `amount + fee`; bila beda dari deposit, butuh pengaturan terpisah untuk withdraw).

**Catatan lain dari uji**
- Log "gagal tersambung (os error 103)" pukul 07:47 hanya gangguan jaringan sesaat; sambung ulang langsung berhasil (perilaku normal).

### 1p. T5.2: perbaiki logika deposit (detail)

**Status: Selesai.** Kode ditulis 8 Nov 2026; setelah build ulang bersih, baris deposit di HP menampilkan `saldo +Rp 7.670` (dilaporkan pengguna, 8 Nov 2026). Sempat tertunda karena build memakai sumber lama (lihat 1q).

**Yang diubah (hanya 4 file yang sudah ada, tanpa FFI baru)**
- `core/src/arus_kas.rs`: `efek_saldo` untuk deposit selalu `jumlah` (amount), apa pun modenya; withdraw tetap dua mode (bawaan `-(amount + fee)`, mode bersih `-amount`). Tes `efek_saldo_dua_mode` diganti nama dan isinya; tes baru `deposit_nyata_8_nov_amount_7670_fee_3330_menambah_7670`.
- `core/src/periode.rs`: tes `fee_deposit_mengubah_saldo_sesuai_mode` diganti menjadi `fee_deposit_tidak_mengubah_saldo_fee_withdraw_sesuai_mode`; tes baru `deposit_nyata_8_nov_saldo_cocok_dengan_saldo_asli` (69.652 + 7.670 = 77.322).
- `core/src/store.rs`: hanya komentar `amount_sudah_bersih` (berlaku untuk withdraw saja).
- `LayarSetelan.kt`: kotak centang "nominal sudah bersih" dan fungsi `ubahModeFee` DIHAPUS (usulan pengguna 8 Nov 2026, karena tidak berpengaruh untuk deposit). Teks kartu diperjelas, baris deposit tampil "amount X (fee Y sudah dipotong) → saldo +X". Saat layar dimuat, bila database lama masih menyimpan mode "bersih", dikembalikan ke bawaan (false) lewat `aturAmountSudahBersih`, agar withdraw selalu `-(amount + fee)`. Fungsi FFI dan `store.rs` untuk mode itu tetap ada (tidak dipakai layar).
- Tidak berubah: `ffi/src/lib.rs` (file ini tidak diunggah; perubahan ini mengandalkan FFI memanggil `efek_saldo` untuk `efekSaldo`; periksa dengan `grep -n "efek_saldo" ffi/src/lib.rs`), skema database, `setoran_bersih`, `sinkron_arus_kas`.
- Karena saldo dihitung ulang dari catatan, deposit 8 Nov otomatis benar tanpa migrasi data.

**Temuan uji awal (8 Nov 2026):** dengan build yang masih memakai logika lama, baris deposit menampilkan +7.670 bila kotak dicentang dan +4.340 bila tidak. Itu membuktikan kotak memang memengaruhi deposit di logika lama; setelah T5.2 baris harus +7.670 tanpa kotak. Pengguna mengusulkan kotak dihapus. Catatan: data aplikasi sempat dihapus pengguna (database baru), sehingga deposit 8 Nov (07:51) berada sebelum snapshot awal baru dan tidak dihitung ke saldo; ada satu posisi koin lama yang belum terjual (penjualannya akan tercatat sebagai penyesuaian).

**Yang diuji**
1. `cargo test -p scalping-core` dan `cargo test -p scalping-ffi` lulus (periksa jumlah tes terbaru).
2. Build lewat `bash tools/bangun_android.sh --pasang <serial>`.
3. Di HP (kotak centang sudah tidak ada): Riwayat, "Cek selisih saldo": IDR menurut aplikasi = saldo asli, selisih Rp 0 (dengan deposit 8 Nov: 77.322).
4. Setelan: baris deposit tampil "amount Rp 7.670 (fee Rp 3.330 sudah dipotong) → saldo +Rp 7.670".
5. Beranda: saldo dan modal aktif ikut naik 7.670; hasil dan pencapaian tidak berubah. Sinkron ulang tidak menggandakan.


### 1q. T5.2: penelusuran "saldo +Rp 4.340 masih muncul" (8 Nov 2026, detail)

**Status: Selesai.** Setelah `touch` pada file sumber, `cargo clean`, dan build ulang lewat `tools/bangun_android.sh`, baris deposit di HP menjadi benar (`saldo +Rp 7.670`), dilaporkan pengguna 8 Nov 2026. Kode sumber memang sudah benar sejak T5.2; masalahnya ada di build.

**Gejala (screenshot HP, 8 Nov 09:36)**
- Teks baris deposit sudah format baru T5.2 ("amount Rp 7.670 (fee Rp 3.330 sudah dipotong) → saldo ..."), jadi Kotlin terbaru terpasang.
- Angka sesudah panah masih `+Rp 4.340` (= 7.670 - 3.330), yaitu hasil logika lama. Seharusnya `+Rp 7.670`.

**Yang sudah diperiksa dan BENAR (dari file yang diunggah)**
- `core/src/arus_kas.rs`: `efek_saldo` untuk deposit = `self.jumlah` (baris 78); tes `deposit_nyata_8_nov_amount_7670_fee_3330_menambah_7670` ada.
- `core/src/periode.rs`: `DataSaldo::setoran` memakai `setoran_bersih` dari `arus_kas.rs`; tes `fee_deposit_tidak_mengubah_saldo_fee_withdraw_sesuai_mode` dan `deposit_nyata_8_nov_saldo_cocok_dengan_saldo_asli` ada.
- `ffi/src/lib.rs`: `daftar_arus_kas` memanggil `a.efek_saldo(bersih)` dari core (baris 939). Tidak ada hitungan fee lain di FFI; `muat_data_saldo` memakai `semua_arus_kas()`.
- `LayarSetelan.kt`: hanya menampilkan `x.efekSaldo` dari Rust (baris 248 dan 250). Tidak ada hitungan sendiri di Kotlin.
- `core/src/lib.rs`: modul dan ekspor wajar. `ffi/Cargo.toml`: `scalping-core = { path = "../core" }` (benar). Workspace `Cargo.toml`: profil release `strip = true`.
- Kesimpulan: tidak ada kode lain yang mengurangi fee. Yang tersisa adalah masalah build atau berkas di repo berbeda dari yang diunggah.

**Temuan dari pemeriksaan di komputer pengguna**
- `grep "Deposit, _" core/src/arus_kas.rs` menemukan baris 78 baru, tetapi `cargo test -p scalping-core deposit_nyata` menemukan 0 tes dari 98 (selesai 0,16 detik tanpa kompilasi). Berarti binary tes dibangun dari sumber lama: Cargo tidak mengompilasi ulang setelah file diganti (dugaan: waktu modifikasi file lebih tua dari build terakhir). Ini juga menjelaskan mengapa tes sebelumnya "lulus" tetapi HP tetap salah. Setelah `touch`, `cargo clean`, dan build ulang dijalankan berurutan sampai selesai, layar HP menjadi benar (dilaporkan pengguna).

**Perubahan alat**
- `tools/bangun_android.sh` diperbarui: (a) memakai APK terbaru berdasarkan waktu, bukan sembarang APK; (b) memperingatkan bahwa `--hanya-pasang` tidak membangun ulang; (c) mencetak waktu dan hash `.so`; (d) setelah build membandingkan isi section `.text` kode Rust di dalam APK dengan `.so` hasil `cargo ndk` (`llvm-objcopy` dari NDK), dan menghentikan build bila beda. Hash seluruh `.so` TIDAK dipakai karena `strip` mengubah byte. Catatan: pemeriksaan ini hanya membuktikan APK = `.so` terbaru, BUKAN bahwa `.so` itu dibangun dari sumber terbaru.
- adb nirkabel mendaftarkan HP dua kali (`... (2)._adb-tls-connect._tcp` dan `...._adb-tls-connect._tcp`). Pakai serial tanpa spasi: `adb-R9CX4015L1D-dnKRzU._adb-tls-connect._tcp` (atau `export ANDROID_SERIAL=...`).

**Penyebab dan pelajaran (terkonfirmasi oleh hasil: layar benar setelah build bersih)**
- Penyebab: build Rust yang terpasang di HP memakai sumber lama, sementara Kotlin sudah baru (teks baris deposit sudah format baru, angka masih hasil logika lama). Dugaan terkuat: Cargo tidak mengompilasi ulang file yang diganti dari unduhan karena waktu modifikasinya lebih tua dari build terakhir (tes `deposit_nyata` ditemukan 0 dari 98 sebelum build bersih). Pemicu persis tidak dibuktikan terpisah, tetapi `touch` + `cargo clean` + build ulang menyelesaikannya.
- Tes Rust "lulus" tidak menjamin sumber baru ikut terkompilasi. Pastikan tes baru benar-benar jalan (jumlah tes bertambah, filter nama menemukan tesnya).
- Setelah mengganti file dari unduhan: `touch` file itu, dan bila ragu `cargo clean` lalu bangun lewat `bash tools/bangun_android.sh --pasang <serial>`.
- Pemeriksaan `.text` di `bangun_android.sh` hanya menjamin APK = `.so` terbaru, bukan bahwa `.so` dibangun dari sumber terbaru.
- Poin uji lain T5.2 (Riwayat "Cek selisih saldo" = Rp 0, Beranda saldo dan modal aktif naik 7.670, sinkron ulang tidak menggandakan) belum dilaporkan terpisah oleh pengguna; periksa bila sempat.


**Langkah yang dijalankan (riwayat, sudah selesai)**
1. `grep -n "deposit_nyata" core/src/arus_kas.rs core/src/periode.rs`: harus ada dua fungsi di repo.
2. `touch core/src/*.rs ffi/src/lib.rs`, `cargo clean`, lalu `cargo test -p scalping-core deposit_nyata`: harus mengompilasi ulang dan menampilkan `2 passed`.
3. `bash tools/bangun_android.sh --pasang adb-R9CX4015L1D-dnKRzU._adb-tls-connect._tcp`: harus `OK: kode Rust di APK sama ...` dan `Success`.
4. Buka Setelan: baris deposit harus `saldo +Rp 7.670`. Riwayat, "Cek selisih saldo": Rp 0 bila deposit berada setelah snapshot awal; Beranda: saldo dan modal aktif naik 7.670, hasil dan pencapaian tidak berubah; sinkron ulang tidak menggandakan.
5. Bila tes `2 passed` dan layar tetap +4.340: uji pemisah, ubah `efek_saldo` di `daftar_arus_kas` (ffi) menjadi `if a.jenis.teks() == "deposit" { a.jumlah } else { a.efek_saldo(bersih) }`, bangun ulang. Layar jadi +7.670 = core yang terpakai masih lama; tetap +4.340 = FFI tidak ikut terbangun atau layar bukan dari fungsi ini (kirim `app/aplikasi/build.gradle.kts`).
6. Setelah lulus: catat penyebab di sini, ubah status T5.2 menjadi lulus uji HP, lalu commit. (Dilakukan 8 Nov 2026.)


### 1r. T6.1 sampai T6.5: Total aset tunggal, target dari profit tertutup (rancangan 8 Nov 2026; seluruhnya lulus tes dan uji HP; T6.4 di 1s, T6.5 di 1t)

**Riwayat keputusan (8 Nov 2026)**
1. Usulan awal pengguna: saldo, snapshot, target, dan grafik semua dari total aset nilai pasar (IDR + koin × harga). Dipakai sementara untuk T6.1 (kartu Total aset terpisah). DIGANTI oleh rancangan di bawah.
2. Rancangan sekarang (permintaan pengguna): TIDAK ada dua angka (saldo dan total aset). Hanya **Total aset**; saldo IDR di dalamnya diberi label **"Buying power"**, koin per aset di bawahnya.
3. Penggerak target dan grafik = akumulasi hasil trading, bukan snapshot saldo. Ide pengguna: nilai jual (masuk ke saldo IDR) dikurangi nilai beli (diambil dari saldo IDR). Disesuaikan: selisih jual-beli MENTAH salah bila ada posisi terbuka (beli 100 ribu belum dijual terbaca rugi 100 ribu; beli 23:50 dan jual 00:10 terpecah di dua hari). Karena itu dipakai **profit per trade yang sudah ditutup** (IDR diterima dari jual dikurangi modal efektif koin yang dijual, termasuk fee), yaitu angka yang sudah dihitung aplikasi untuk Riwayat dan statistik dan cocok dengan struk Indodax (Fase 0). Per periode WIB, hari mulai 00:00, minggu mulai Senin.
4. Floating P/L (koin yang dipegang dan sedang rugi/untung): **PILIHAN 1, dikonfirmasi pengguna 8 Nov 2026.** Target tetap dari profit tertutup. Floating tampil terpisah di Beranda (satu baris di bawah profit trading) dan di layar Posisi (harga sekarang pada tangga SL/modal efektif/TP, merah bila di bawah SL). Floating per posisi = jumlah koin × (harga terakhir − modal efektif). Pilihan 2 (tanpa floating) dan 3 (target = profit tertutup + floating, butuh snapshot floating sehingga tidak bebas dari instal ulang) TIDAK dipilih.
5. Harga memakai `last` saja. Bid sebagai angka tambahan ("nilai jual cepat") boleh nanti, tanpa memengaruhi target.
6. Pencatatan trade tetap untuk TP/SL, notifikasi, riwayat, statistik, dan sekarang juga target dan grafik.

**Keputusan lanjutan (pengguna, 8 Nov 2026)**
- Persen target = profit periode ÷ (total aset sekarang − profit periode) × 100: DISETUJUI.
- Grafik = kurva akumulasi profit tertutup: DISETUJUI.
- Deposit dan withdraw (IDR) DIHAPUS dari perhitungan. Tidak ada lagi snapshot awal/harian/mingguan, modal aktif, setoran bersih, dan koreksi saldo (FR-27). Deposit/withdraw tetap disinkronkan dari `fiat/orders` tetapi hanya sebagai **riwayat**, ditampilkan seperti trade di layar Riwayat, tanpa memengaruhi saldo, target, atau grafik. Bentuk baris withdraw masih belum teramati (belum teruji).
- Riwayat HANYA sejak aplikasi dipasang. Akibatnya: instal ulang atau hapus data mengosongkan profit tertutup, target, dan grafik, kecuali data dipulihkan lewat impor cadangan (U5). Ini diterima pengguna (menggantikan harapan awal "bebas dari instal ulang"). Total aset tetap benar setelah instal ulang karena realtime dari akun. Pertanyaan jangkauan `myTrades` dan penemuan koin lama jadi tidak relevan; koin yang di-trading sesudah pemasangan terdeteksi lewat WebSocket dan sinkron.
- Pembanding profit dengan platform Indodax: bebas (tidak ditentukan). Acuan uji tetap struk per trade.
- Fee jual pada floating P/L: pengguna bertanya mengapa perlu data fee jual karena struk sudah memuat fee nyata. Jawaban: fee nyata hanya ada setelah koin DIJUAL; untuk posisi yang masih dipegang belum ada struk, dan fee beda per order (limit sekitar 0,31%, market sekitar 0,61%). Usulan: floating ditampilkan SEBELUM fee jual dengan label "sebelum fee jual", tanpa menebak tarif. DIPAKAI di T6.3 dan lulus uji HP tanpa keberatan pengguna (8 Nov 2026). Profit tertutup tetap memakai `commission` nyata dari API, bukan estimasi.

**Tahap (tiap tahap diuji di HP dan di-commit sendiri)**
| Task | Isi | Catatan |
| --- | --- | --- |
| T6.1 | Sumber harga `ticker_all` + total aset + kartu "Total aset" di Beranda (kartu terpisah SEMENTARA, tampilan saja) | LULUS uji HP 8 Nov 2026 (dilaporkan pengguna). Commit: `git commit -m "T6.1: total aset nilai pasar di Beranda (harga ticker_all, total aset)"` |
| T6.2 | Beranda tunggal: kartu Saldo lama (saldo, hasil terhadap modal aktif, modal aktif) DIHAPUS; kartu Total aset dengan "Buying power" (IDR) dan koin per aset; baris "Profit trading"; target harian/mingguan dari profit trade tertutup per periode WIB; grafik = akumulasi profit tertutup | LULUS tes dan uji HP 8 Nov 2026 (dilaporkan pengguna). Commit: `git commit -m "T6.2: Beranda tunggal, target dan grafik dari profit trade tertutup"`. Detail di bawah tabel |
| T6.3 | Floating P/L: harga sekarang di layar Posisi (penanda di tangga, merah di bawah SL, floating per posisi); satu baris "Floating" di Beranda | LULUS tes (ffi 25) dan uji HP 8 Nov 2026 (dilaporkan pengguna). Commit: `git commit -m "T6.3: floating P/L di Beranda dan layar Posisi"`. Detail di bawah tabel. Hanya posisi yang dicatat aplikasi; koin lama/deposit tidak punya modal efektif |
| T6.4 | Pembersihan: hapus snapshot, modal aktif, setoran bersih, dan koreksi saldo; deposit/withdraw tetap disinkronkan tetapi hanya tampil sebagai riwayat seperti trade; PRD v1.6 (non-tujuan "harga pasar realtime" dicabut, rumus bagian 8 diganti: total aset, profit tertutup, floating, persen target) | LULUS tes (core 99, ffi 24) dan uji HP sesuai kriteria (dilaporkan pengguna 8 Nov 2026; detail di 1s). Cadangan lama (v1.x) tetap bisa diimpor (tabel lama dibiarkan, tidak dihapus dari database). Commit: `git commit -m "T6.4: pembersihan saldo lama, deposit/withdraw jadi riwayat, notifikasi jual dari profit tertutup"` |
| T6.5 | Pembersihan kode grafik lama: hapus `GrafikView.setData` dan record `TitikGrafikUi` (FFI) | LULUS tes (core 99, ffi 24) dan uji HP sesuai kriteria (dilaporkan pengguna 8 Nov 2026). Commit: `git commit -m "T6.5: hapus GrafikView.setData dan TitikGrafikUi"`. Detail di 1t |

**T6.2 (detail; lulus tes dan uji HP, dilaporkan pengguna 8 Nov 2026)**
- `core/src/periode.rs`: `pencapaian_profit(profit_tertutup, mulai_ms, sekarang_ms, total_aset, target)` (persen = profit ÷ (total aset − profit) × 100; `None` bila total aset belum diketahui atau dasar hitung tidak positif; target 0 dianggap belum diatur; trade dihitung bila `>= mulai` dan `<= sekarang`, jadi trade yang tutup tepat pada 00:00 WIB milik periode baru), `deret_profit` (kurva akumulasi, titik di awal, tiap trade tertutup, dan sekarang; dijarangkan bila lebih dari `maks_titik`), tipe `PencapaianProfit` dan `TitikProfit`. 9 tes baru di `mod tes_profit` (core 105 menjadi 114).
- `ffi/src/lib.rs`: `ringkasan_profit(path_db, total_aset: Option<String>)`, `data_grafik_profit(path_db, maks_titik)`, record `TargetProfitUi`, `RingkasanProfitUi`, `TitikProfitUi`, dan field baru `TotalAsetUi.total_angka` (rupiah utuh tanpa format, diteruskan Kotlin ke `ringkasan_profit`; Kotlin tidak menghitung uang). 3 tes baru di `mod tes_beranda_profit` (ffi 16 menjadi 19).
- `LayarBeranda.kt` ditulis ulang: kartu Saldo lama (saldo, hasil terhadap modal aktif, modal aktif) dihapus. Satu kartu Total aset: total, "Buying power" (IDR), koin per aset, baris "Profit trading" (total profit trade tertutup sejak aplikasi dipasang dan jumlah trade), grafik, lalu bagian Target. Total aset diperbarui lewat jaringan (saat layar dibuka dan tiap 10 detik); profit, target, dan grafik dari database lokal. Persen target memakai total aset terakhir yang sudah diterima; sebelum itu tampil rupiah profit dengan tulisan "Menunggu total aset".
- `GrafikView.kt`: tambah `setProfit` (satu garis tebal, garis putus-putus di Rp 0 yang selalu ikut dalam rentang). `setData` (dua garis) sempat dipertahankan dan dihapus di T6.5 (lihat 1t).
- Tidak berubah (sengaja, dibersihkan di T6.4): `hitung_target`, `data_grafik`, snapshot, modal aktif, setoran bersih, koreksi saldo, deposit/withdraw, dan notifikasi jual.
- Persen target dan "Menunggu total aset": bila jaringan mati, persen tetap memakai total aset terakhir yang berhasil dimuat; bila belum pernah berhasil, persen kosong.

**T6.3 (detail; lulus tes dan uji HP, dilaporkan pengguna 8 Nov 2026)**
- Rumus: floating posisi = jumlah koin × harga terakhir − modal posisi (`total_idr_keluar`, sama dengan modal efektif × koin); persen = floating ÷ modal posisi × 100. SEBELUM fee jual (fee nyata baru ada setelah dijual, tarif beda per order). Total Beranda = jumlah floating posisi yang punya harga; persen terhadap modal posisi yang berharga saja. Koin tanpa harga IDR ditandai dan tidak masuk total. Tidak masuk target, grafik, atau statistik (pilihan 1).
- `ffi/src/lib.rs` (hanya penambahan): record `HargaKoinUi`, `FloatingPosisiUi` (harga_sekarang berformat; floating_angka dan floating_persen mentah agar Kotlin memakai pemformat yang sama dengan profit; `skala` 0,0 = SL, 0,5 = modal efektif, 1,0 = TP, dijepit; `di_bawah_sl` bila harga <= SL; `di_atas_tp` bila harga >= TP), `FloatingUi`; fungsi `ambil_harga_terakhir(key, secret, koin)` (jaringan, tanpa database), `floating_posisi(path_db, harga)` (database, tanpa jaringan), pembantu `harga_untuk_koin`, `skala_tangga`, `floating_dari`. Field baru `AsetUi.harga_angka` (harga mentah). 6 tes baru di `mod tes_floating` (ffi 19 menjadi 25).
- `LayarPosisi.kt` (ditulis ulang): daftar posisi dari database tampil dulu, lalu harga lewat `Latar.jaringan` (satu permintaan pada satu waktu, jeda minimal 5 detik, saat layar dibuka dan tiap 10 detik selama terlihat), floating dihitung Rust lewat `Latar.baca`. Gagal ambil harga tidak mengganggu daftar; angka terakhir tetap tampil dengan keterangan galat.
- `LayarBeranda.kt`: `muatFloating()` memakai harga dari total aset yang sama (`AsetUi.hargaAngka`), tanpa permintaan jaringan tambahan; satu baris "Floating" di bawah Profit trading, hilang bila tidak ada posisi.
- Catatan: floating hanya untuk posisi yang dicatat aplikasi. Posisi yang koinnya tidak ada di saldo akun tampil "tanpa harga".
- Yang diuji di HP (lulus): titik harga di tangga; floating per posisi cocok hitungan manual; titik merah dan tulisan "di bawah SL"; harga diperbarui tiap sekitar 10 detik dan tetap tampil saat internet mati; baris Floating di Beranda sama dengan jumlah di tab Posisi; target dan grafik tidak berubah dibanding T6.2.

**Keputusan dari T6.2: rumus persen di notifikasi jual. TERJAWAB 8 Nov 2026: Opsi A (lihat 1s).** Notifikasi jual (baris "Harian: +x% dari target y%") memakai rumus lama (persen dari saldo versi aplikasi) sampai T6.4, sehingga angkanya bisa berbeda dari persen target di Beranda; sejak T6.4 memakai rumus baru. Rumus baru butuh total aset dari jaringan, sedangkan notifikasi dihitung dari database lokal. Opsi A: biarkan sampai T6.4, lalu notifikasi memakai total aset terakhir yang disimpan. Opsi B: layanan latar mengambil total aset saat order jual terisi dan notifikasi memakai rumus baru. Dijawab pengguna: Opsi A, dikerjakan di T6.4 (lihat 1s).

**T6.1 bagian A (tes Rust lulus: core 105 tes, ffi 16 tes, dilaporkan pengguna 8 Nov 2026; tes `harga::` belum dipastikan terpisah lewat filter nama)**
- `core/src/harga.rs` (baru): `parse_ticker_all` (ambil `last` pasangan `*_idr`; last nol atau hilang dilewati; error bila bentuk tidak dikenal), `hitung_total_aset` (Decimal; saldo koin nol dilewati; koin tanpa harga masuk `tanpa_harga`, tidak dihitung; daftar diurutkan nilai terbesar; error bila aset IDR tidak ada), 5 tes (data dari curl nyata 8 Nov: `btc_idr` last 1368000000, `ab_idr` last 8.953, `ach_idr` last 79).
- `core/src/client.rs`: `BASE_PUBLIK = https://indodax.com` dan `IndodaxClient::ambil_harga_idr()` (GET publik `/api/ticker_all`, tanpa key dan tanda tangan, User-Agent yang sama).
- `core/src/lib.rs`: `pub mod harga;` dan ekspor.
- `ffi/src/lib.rs`: `AsetUi`, `TotalAsetUi`, `total_aset(key, secret)` (dua panggilan jaringan: `/account` dan `ticker_all`; teks sudah berformat Indonesia dengan "Rp"; tidak menyentuh database).
- Bentuk respons `ticker_all` diverifikasi dari curl pengguna: `{"tickers":{"btc_idr":{"buy","high","last","low","sell","server_time",...}}}`, nilai berupa teks. Belum ada dokumentasi resmi yang dibaca langsung.

**T6.1 bagian B (Kotlin, ditulis 8 Nov 2026, lulus tes dan uji HP 8 Nov 2026, lihat tabel di atas)**
- Hanya `LayarBeranda.kt` yang berubah. Kartu "Total aset (nilai pasar)" di bawah kartu Saldo: total, rincian IDR dan nilai koin, per koin (jumlah × harga, nilai), daftar koin tanpa harga, waktu pembaruan, catatan "harga = transaksi terakhir".
- Pembaruan: saat layar dibuka (`segarkan()`) dan tiap 10 detik selama Beranda terlihat dan jendela aplikasi di depan (`root.isShown` dan `windowVisibility`). Satu permintaan pada satu waktu, jeda minimal 5 detik. Putaran berhenti sendiri bila layar sudah dilepas dari jendela.
- Panggilan jaringan lewat `Latar.jaringan` (tidak menahan antrean baca database); kunci dari `a.kunci.ambil()`. Tanpa kunci, kartu disembunyikan.
- Bila gagal memperbarui, angka terakhir tetap tampil dengan keterangan galat; bila belum pernah berhasil, tampil pesan galat.
- Kartu ini berdiri sendiri: kartu Saldo, target, dan grafik tidak berubah.

**Yang diuji di HP (T6.1)**
1. Beranda menampilkan kartu "Total aset (nilai pasar)" dengan total = IDR + nilai koin; cocokkan dengan total di aplikasi Indodax (boleh selisih kecil karena harga `last` bergerak).
2. Koin yang dipegang muncul dengan jumlah, harga, nilai; koin tanpa pasangan IDR muncul di baris "Tanpa harga IDR" dan tidak masuk total.
3. Biarkan Beranda terbuka sekitar 30 detik: waktu "Diperbarui" berubah. Pindah tab atau ke aplikasi lain: tidak ada pembaruan berjalan (periksa log bila perlu).
4. Matikan internet lalu tunggu: angka terakhir tetap tampil dengan "gagal memperbarui". Nyalakan lagi: pulih sendiri.
5. Kartu Saldo, target, dan grafik tidak berubah dari sebelumnya. Tanpa kunci: kartu tidak tampil.
6. Bila HTTP 403 dari `indodax.com`: catat isi galat (host publik berbeda dari `api.indodax.com`).

**Yang harus dilakukan setelah bagian A**
1. (Selesai) Salin file ke repo, `touch`, `cargo test`: lulus.
2. Salin `LayarBeranda.kt`, `touch` file itu, build lewat `bash tools/bangun_android.sh --pasang <serial>`, uji 6 poin di atas. Commit hanya setelah uji HP lulus; lalu ubah status T6.1 di file ini menjadi lulus.
3. Perhatian: `ambil_harga_idr` memakai host `indodax.com`, bukan `api.indodax.com`. Bila HP mendapat HTTP 403 dari Cloudflare, laporkan isi galatnya.

### 1s. T6.4: pembersihan saldo lama (detail; kode ditulis 8 Nov 2026; LULUS tes dan uji HP, dilaporkan pengguna 8 Nov 2026)

**Keputusan:** Opsi A untuk notifikasi jual (disetujui pengguna 8 Nov 2026): persen target di notifikasi memakai TOTAL ASET TERAKHIR yang disimpan Beranda (tabel `meta`), tanpa jaringan di layanan latar. Bisa sedikit tertinggal dari harga sekarang; bila belum pernah tersimpan (Beranda belum pernah dibuka), notifikasi menampilkan rupiah profit saja ("Harian: +Rp 380").

**Yang diubah (8 file; `LayananLatar.kt`, `sync.rs`, dan semua file lain TIDAK berubah)**
- `core/src/periode.rs`: dihapus `DataSaldo`, `Pencapaian`, `RingkasanTarget`, `TitikGrafik`, `SelisihSaldo` dan 13 tesnya. Tersisa `awal_hari_wib`, `awal_minggu_wib`, `pencapaian_profit`, `deret_profit` (+ 3 tes WIB dan `tes_profit`). Tidak lagi mengimpor `arus_kas`.
- `core/src/arus_kas.rs`: dihapus `ArusKas::efek_saldo` dan `setoran_bersih` (+ 3 tes). Parser `fiat/orders` dan `sinkron_arus_kas` tetap; arus kas hanya riwayat.
- `core/src/lib.rs`: ekspor `setoran_bersih`, `DataSaldo`, `Pencapaian`, `RingkasanTarget`, `SelisihSaldo`, `TitikGrafik` dihapus; ditambah ekspor `deret_profit`, `pencapaian_profit`, `PencapaianProfit`, `TitikProfit`.
- `core/src/store.rs`: tambah `total_aset_terakhir()` dan `atur_total_aset_terakhir()` (kunci meta `total_aset_terakhir` dan `total_aset_terakhir_ms`) + 1 tes. Tabel `snapshot`, `koreksi_saldo`, `arus_kas` dan fungsi `tambah_koreksi/semua_koreksi/amount_sudah_bersih` DIBIARKAN agar cadangan v1.x tetap terbaca dan `periksa_cadangan` tetap jalan.
- `ffi/src/lib.rs`: DIHAPUS `hitung_target`, `data_grafik`, `cek_selisih_saldo`, `terapkan_koreksi_saldo`, `daftar_koreksi_saldo`, `atur_amount_sudah_bersih`, record `PencapaianUi`, `RingkasanTargetUi`, `SelisihSaldoUi`, `BarisKoreksi`, dan `BarisArusKas.efek_saldo`. DIPERTAHANKAN SEMENTARA: record `TitikGrafikUi` (hanya agar `GrafikView.setData` lama tetap terkompilasi). BARU: `progres_untuk_notif` ditulis ulang dari profit tertutup + total aset tersimpan (`progres_untuk_notif_pada` menerima waktu agar bisa dites), `ringkasan_profit` menyimpan total aset terakhir bila diberikan (> 0). Tes: 3 tes koreksi dan `tes_target_progres` dihapus, `tes_progres_notif` (4 tes) ditambah, `riwayat_dan_grafik_dari_database` menjadi `riwayat_dan_statistik_dari_database`.
- `LayarRiwayat.kt`: bagian Koreksi saldo diganti bagian "Deposit dan withdraw" (daftar seperti trade: Masuk/Keluar, tanggal, fee; deposit "(sudah dipotong)"; tombol "Tampilkan lebih banyak").
- `LayarSetelan.kt`: dihapus pemanggilan `hitungTarget`/`aturAmountSudahBersih`; kartu deposit/withdraw kini hanya penjelasan dan jumlah tercatat (daftar ada di Riwayat).
- Dokumen: PRD v1.6 dan PROGRESS ini.

**Jumlah tes sebenarnya (dilaporkan pengguna, 8 Nov 2026):** core 99 lulus, ffi 24 lulus, 0 gagal, sesuai perkiraan (core 114 menjadi 99; ffi 25 menjadi 24). Pelajaran 1q tetap berlaku: pastikan tes baru benar-benar jalan lewat filter nama (`cargo test -p scalping-ffi progres`, `cargo test -p scalping-ffi notif_jual`, `cargo test -p scalping-core total_aset_terakhir`).

**Pemeriksaan sebelum build (file yang tidak saya lihat bisa saja memakai simbol yang dihapus):**
```
grep -rn "DataSaldo\|setoran_bersih\|efek_saldo\|TitikGrafik\b\|SelisihSaldo\|RingkasanTarget" core/ ffi/ --include=*.rs
grep -rn "hitungTarget\|dataGrafik\|cekSelisihSaldo\|terapkanKoreksiSaldo\|daftarKoreksiSaldo\|aturAmountSudahBersih\|efekSaldo\|RingkasanTargetUi\|PencapaianUi\|SelisihSaldoUi\|BarisKoreksi" app/aplikasi/src/main/kotlin
```
Hasil harus kosong (selain `TitikGrafikUi`/`TitikGrafik` di `GrafikView.kt` yang saat itu dipertahankan; dihapus di T6.5). Bila ada baris lain, jangan build: kirim file itu.

**Yang diuji di HP (T6.4; semua lulus dan sesuai kriteria, dilaporkan pengguna 8 Nov 2026)**
1. `cargo test -p scalping-core` dan `-p scalping-ffi` lulus dengan jumlah tes seperti di atas.
2. Build lewat `bash tools/bangun_android.sh --pasang <serial>` (setelah `touch core/src/*.rs ffi/src/lib.rs`).
3. Beranda: total aset, profit trading, floating, target, dan grafik SAMA seperti sebelum T6.4.
4. Riwayat: tidak ada lagi kartu Koreksi saldo; bagian "Deposit dan withdraw" menampilkan deposit nyata 8 Nov ("Masuk Rp 7.670", "fee Rp 3.330 (sudah dipotong)").
5. Setelan: kartu deposit/withdraw menampilkan jumlah tercatat; "Sinkron deposit/withdraw" berjalan; sinkron deposit tidak mengubah angka Beranda.
6. Buka Beranda sekitar 10 detik, lalu jual koin kecil: notifikasi jual memuat "Harian: +x% dari target y%" yang sama dengan persen Beranda (selisih kecil bila harga bergerak). Target harian baru tercapai menampilkan "TERCAPAI" sekali.
7. Impor cadangan lama (dibuat sebelum T6.4): berhasil dan Beranda, Riwayat, Posisi tampil normal.

**Catatan**
- Snapshot awal tetap dibuat `sync.rs` dan dipakai sebagai batas "sejak aplikasi dipasang"; hanya snapshot harian/mingguan yang tidak ditulis lagi (`hitung_target` yang menulisnya sudah dihapus). Baris lama di database tidak mengganggu.
- Layar tidak lagi memanggil `atur_amount_sudah_bersih`; nilai lama di database diabaikan karena `efek_saldo` sudah tidak ada.
- Sisa `GrafikView.setData` dan record `TitikGrafikUi` dikerjakan di T6.5 (lihat 1t).

### 1t. T6.5: hapus kode grafik dua garis (detail; LULUS tes dan uji HP, dilaporkan pengguna 8 Nov 2026)

**Latar:** `GrafikView.setData` (grafik saldo dan modal aktif) dan record FFI `TitikGrafikUi` sengaja dipertahankan di T6.4 karena `GrafikView.kt` belum diperiksa. Grep (8 Nov 2026) menunjukkan keduanya hanya dipakai di `GrafikView.kt` dan didefinisikan di `ffi/src/lib.rs`, tanpa pemanggil lain.

**Yang diubah (2 berkas, tanpa perubahan perilaku)**
- `ffi/src/lib.rs`: record `TitikGrafikUi` beserta komentar dan atributnya dihapus (9 baris). Bagian lain tidak berubah (diverifikasi dengan `diff`).
- `GrafikView.kt`: dihapus `setData`, impor `TitikGrafikUi`, properti `titik` dan `modeProfit`, serta cabang gambar dua garis di `onDraw`. Tersisa `setProfit` dan `gambarProfit`; `onDraw` langsung memanggil `gambarProfit`. Komentar kelas diperbarui.

**Hasil**
- `cargo test -p scalping-core`: 99 lulus. `cargo test -p scalping-ffi`: 24 lulus. Jumlah tes tidak berubah (record itu tidak punya tes).
- Build lewat `tools/bangun_android.sh` berhasil tanpa error. Uji HP: grafik Beranda sama seperti sebelumnya (satu garis profit, garis putus-putus di Rp 0).

### 1u. T6.6: cegah impor cadangan saat sinkron berjalan (detail; LULUS uji HP, dilaporkan pengguna 8 Nov 2026)

**Latar:** celah dari U5. Impor hanya mengecek pantauan mati, padahal sinkron manual atau otomatis bisa sedang menulis ke database yang akan diganti. Dialog konfirmasi juga bisa terbuka lama sehingga sinkron otomatis sempat menyelip sebelum tombol "Ganti".

**Yang diubah (3 berkas Kotlin; tanpa perubahan Rust, FFI, atau manifest)**
- `Sinkron.kt`: `imporBerjalan` (volatile), `sedangSinkron()` (trade atau arus kas berjalan), `mulaiImpor()` (pemeriksaan dan penandaan satu kesatuan lewat `synchronized`; gagal bila sinkron atau impor lain berjalan), `selesaiImpor()`. `bolehSinkron`, `sinkronTrade`, dan `jalankanArusKas` menolak mulai saat `imporBerjalan`; pemeriksaan dan penandaan `tradeBerjalan`/`arusKasBerjalan` kini juga di dalam `synchronized`.
- `Cadangan.kt`: saat "Ganti" ditekan, `mulaiImpor()` dipanggil dan `pantauAktif` dicek lagi; bila gagal, berkas sementara dihapus dan pesan penolakan ditampilkan. Penanda dilepas di blok `finally` setelah `pulihkanCadangan`, jadi tidak macet bila impor gagal.
- `LayarSetelan.kt`: tombol Impor nonaktif saat pantauan menyala, sinkron berjalan, atau impor berjalan; pilih berkas saat sinkron berjalan ditolak ("Tunggu sinkron selesai dulu sebelum impor."); "Mulai pantauan" dan "Sinkron deposit/withdraw" menampilkan pesan bila impor berjalan; `segarkanStatus()` dipanggil setelah impor selesai.

**Hasil uji HP (lulus, dilaporkan pengguna 8 Nov 2026):** sinkron manual dan arus kas normal; tombol Impor nonaktif selama sinkron lalu aktif lagi; impor normal saat pantauan mati dan tidak ada sinkron; tombol Impor nonaktif saat pantauan menyala; ekspor tetap normal.

**Catatan dan batasan**
- Thread arus kas di `LayananLatar.kt` (`pekerjaArusKas`) tidak tercatat di penanda ini (berkasnya belum diperiksa). Risiko kecil karena impor mensyaratkan pantauan mati. Bila ingin ditutup rapat, periksa `LayananLatar.kt`.
- Skenario "sinkron otomatis menyelip saat dialog terbuka" sulit dipancing di HP (sinkron otomatis hanya jalan bila pantauan mati dan jeda minimal 60 detik); logikanya dijaga oleh pemeriksaan ulang di tombol "Ganti".

### 1v. T7.1 dan T7.2: penyederhanaan tampilan (detail; LULUS build dan uji HP, dilaporkan pengguna 8 Nov 2026)

**Latar:** pengguna paham trading, jadi layar diminimalkan: hanya angka yang dipakai mengambil keputusan, tanpa kalimat penjelas. Perhitungan tidak berubah; Rust, FFI, dan manifest TIDAK diubah (jumlah tes tetap core 99, ffi 24).

**Keputusan pengguna (8 Nov 2026)**
1. Floating digabung dengan baris Profit trading (bukan baris sendiri).
2. Koin tanpa harga IDR (PXG) disembunyikan sepenuhnya, karena tidak mungkin ada trade untuk koin itu lagi.
3. Usulan lain (hapus teks penjelas, kartu Deposit/withdraw di Setelan, fee deposit/withdraw tidak ditampilkan, profit kotor tidak ditampilkan, log tersembunyi) dijalankan dan lolos uji HP.

**T7.1 (2 berkas Kotlin): `LayarBeranda.kt`, `LayarPosisi.kt`**
- Beranda: satu baris per koin (nilai Rp), tanpa jumlah × harga, tanpa daftar koin tanpa harga, tanpa "Diperbarui"; hanya "Gagal memperbarui" (kecil, kuning) bila pembaruan gagal. Baris Profit trading, lalu satu baris: "N trade" di kiri dan "floating ±Rp x (±y%)" berwarna di kanan (hilang bila tidak ada posisi berharga). Grafik tanpa legenda, tanpa keterangan titik, tanpa catatan harga. Kartu Target tanpa rumus; teks "Belum diatur" dan baris hasil tanpa kalimat tambahan. Chip: "Terhubung", "Pantauan mati", dan seterusnya.
- Posisi: satu baris "Harga x | floating", satu baris "Bersih TP ±a% · SL ±b%" dengan waktu buka di kanan (nominal Rp estimasi dan catatan fee jual perkiraan tidak ditampilkan; perhitungan di Rust tetap memakai 0,31%). Tulisan "di bawah SL"/"mencapai TP" dihapus; titik merah/hijau di tangga tetap. Posisi tanpa harga IDR: baris harga tidak tampil. Catatan kaki dihapus, hanya "Gagal memperbarui harga" bila galat.

**T7.2 (2 berkas Kotlin): `LayarRiwayat.kt`, `LayarSetelan.kt`**
- Riwayat: chip "Hari ini" dan baris "Semua: ..." dihapus. Statistik: Win rate (disertai "N untung · N rugi", impas hanya bila > 0), Rata-rata untung, Rata-rata rugi, Hasil bersih, Fee (Rp x (y% dari profit)); Profit kotor tidak ditampilkan (tetap dihitung di Rust). Deposit/withdraw: satu baris per item (jenis, tanggal, nominal), tanpa teks penjelas dan tanpa fee.
- Setelan: catatan di bawah TP/SL dan target dihapus (target kosong = tanpa target, ditunjukkan hint kolom); label "Kunci API" dan "Pantauan order"; tombol Sinkron selalu berlabel "Sinkron sekarang" (nonaktif saat pantauan berjalan); tombol baterai disembunyikan bila optimasi baterai sudah dimatikan; kartu "Deposit dan withdraw IDR" (tombol sinkron manual) DIHAPUS karena sinkron arus kas berjalan otomatis (setelah sinkron trade dan saat pantauan tersambung); keterangan Cadangan data diringkas; Log pantauan tersembunyi (ketuk judul untuk membuka); "Inti Rust versi" dihapus; pesan hasil tes kunci satu baris.

**Catatan**
- `Sinkron.jalankanArusKas` dan `Sinkron.arusKasBerjalan` tetap dipakai pemanggil lain (sinkron trade, layanan latar); hanya pemanggil di layar Setelan yang dihapus.
- Bila suatu saat daftar deposit/withdraw di Riwayat tidak terisi otomatis, kembalikan tombol sinkron manual di Setelan.
- `asetWaktuMs` di Beranda dan `Format.waktuSingkat` kini hanya diisi atau tidak dipakai (aman; hanya peringatan).

### 1w. T8.1: diagnostik dan penjaga pantauan (detail; LULUS uji HP, dilaporkan pengguna 8 Nov 2026)

**Tujuan:** bukti yang cukup untuk melacak bug selama uji beberapa hari, dan peringatan bila pantauan mati diam-diam. Hanya Kotlin; Rust, FFI, dan skema database tidak berubah.

**Berkas**
- Baru (3), di `app/aplikasi/src/main/kotlin/id/catatanscalping/app/`: `LogDiag.kt`, `Aplikasi.kt`, `PenjagaPantauan.kt`.
- Diubah, timpa (4): `LayananLatar.kt`, `LayarSetelan.kt`, `app/aplikasi/src/main/AndroidManifest.xml` (tambah `android:name=".Aplikasi"` dan receiver `PenjagaPantauan`), `app/aplikasi/build.gradle.kts` (versionCode 2, versionName `0.8.0-uji`).

**Cara pasang**
- Sebelum apa pun: ekspor cadangan dari Setelan. JANGAN uninstall. Pemasangan ulang lewat adb mematikan pantauan; nyalakan lagi manual.
```
cd catatan-scalping
unzip -o ~/Downloads/T8.1.zip
touch app/aplikasi/src/main/kotlin/id/catatanscalping/app/*.kt
bash tools/bangun_android.sh --pasang <serial-adb>
```

**Yang berubah**
1. Log permanen: semua baris Log pantauan juga ditulis ke berkas `diag.log` (maks sekitar 256 KB; separuh terlama dibuang bila penuh). Tampilan log di Setelan memuat 40 baris dengan tanggal dan jam (dd/MM HH:mm:ss) dan tetap ada setelah proses dimatikan.
2. Catatan baru di log: "Proses dimulai · versi · Android", alasan proses sebelumnya berakhir (memori habis, dibunuh sistem/baterai, force stop, crash, dan lain-lain), "Layanan dihidupkan ulang oleh sistem", "Layanan dihentikan", dan CRASH beserta jejaknya.
3. Setelan, Log pantauan, tombol "Bagikan log": mengirim 400 baris terakhir lewat menu bagikan (Telegram, Drive, email). Tidak memuat kunci API atau saldo.
4. Penjaga pantauan: alarm tiap sekitar 5 menit selama pantauan ingin menyala. Bila pantauan mati, atau terputus lebih dari 5 menit, dan keadaan itu bertahan sekitar 4 menit lagi, muncul notifikasi "Peringatan pantauan" (paling sering sekali per 30 menit). Hilang sendiri saat pulih. Berhenti saat pantauan dihentikan manual. Penjaga TIDAK menyalakan layanan sendiri (Android melarang); buka aplikasi lalu nyalakan.

**Yang diuji di HP (lulus sesuai kriteria, dilaporkan pengguna 8 Nov 2026)**
1. Setelan, ketuk "Log pantauan": ada baris "Proses dimulai · versi 0.8.0-uji". Tombol "Bagikan log" muncul dan membuka menu bagikan berisi teks log.
2. Mulai pantauan, geser aplikasi dari recent apps, buka lagi: log masih ada dan memuat "Proses sebelumnya berakhir: ..." bila prosesnya sempat mati.
3. Matikan pantauan lewat tombol Hentikan: tidak ada peringatan muncul walau ditunggu 10 menit.
4. Uji peringatan terputus: pantauan menyala, aktifkan mode pesawat sekitar 10 menit. Notifikasi "Peringatan pantauan" muncul dengan teks "terputus sejak jam ...". Matikan mode pesawat: setelah tersambung peringatan hilang.
5. (Opsional, memaksa) pantauan menyala, jalankan `adb shell am force-stop id.catatanscalping.app`. Alarm ikut terhapus oleh force stop, jadi tidak ada peringatan; ini normal (peringatan hanya untuk mati yang dilakukan sistem). Kondisi itu tampak di log berikutnya: "Proses sebelumnya berakhir: force stop".
6. Pantauan dan notifikasi beli/jual berjalan seperti biasa; chip dan layar tidak berubah.

**Catatan**
- `PenerimaBoot.kt` belum diperiksa untuk T8.1. Bila boot gagal menyalakan layanan, `PREF_AKTIF` dimatikan dan penjaga berhenti (tidak ada peringatan). Kirim `PenerimaBoot.kt` bila ingin ditutup.
- PRD tidak diubah untuk T8.1 (tidak ada perubahan perilaku perhitungan atau fungsional utama); tambahkan ke PRD hanya bila diminta.
- Uji HP lulus; tinggal commit T8.1 (lihat bagian 6).

### 1x. Rilis uji 0.8.0-uji (8 Nov 2026)

**Tujuan:** aplikasi dipakai nyata sehari-hari (bukan rilis publik) untuk mencari bug, kekurangan, dan peningkatan. Hanya satu pengguna dan satu HP.

**Yang dibuat (dokumen saja, tanpa perubahan kode)**
- `docs/PANDUAN_RELEASE.md` (baru): persiapan rilis, penandaan versi git, build dan pasang, aturan selama masa uji, skenario uji lapangan, cara mencatat dan melaporkan bug, tingkat keparahan, alur perbaikan dan penomoran versi, rollback, kriteria selesai uji.
- `README.md` diperbarui (sebelumnya masih berisi petunjuk Fase 0): ringkasan, status, fitur, cara pasang, tes, cadangan, keterbatasan, pelaporan bug.

**Yang masih perlu dikonfirmasi pengguna (tidak diasumsikan)**
- Jenis build dan tanda tangan APK dari `bangun_android.sh` (debug atau release) belum terlihat. Ini menentukan apakah pembaruan menimpa versi lama tanpa uninstall. Pemeriksaan: `grep -n "buildTypes\|signingConfig\|isMinify" app/aplikasi/build.gradle.kts` lalu kirim hasilnya.
- Lama masa uji dan kriteria selesai: panduan memakai USULAN 7 hari tanpa bug kritis; BELUM dikonfirmasi.

**Urutan yang disarankan**
1. Commit T8.1 lalu commit dokumen (README, PROGRESS, panduan), lalu `git tag -a v0.8.0-uji`.
2. Pakai harian sesuai panduan; ekspor cadangan tiap malam.
3. Bug dilaporkan dengan log dari tombol "Bagikan log"; tiap perbaikan menjadi task baru (T8.2 dan seterusnya) dengan versi naik.

**Yang diketahui belum tuntas saat rilis uji:** uji withdraw nyata, ID order di notifikasi beli, `PenerimaBoot.kt` terhadap penjaga pantauan, thread arus kas di `LayananLatar.kt` yang tidak tercatat di penanda impor (T6.6), koin lama yang dijual tercatat sebagai penyesuaian.

### Belum terselesaikan atau dikenal
| Masalah | Penyebab / dugaan | Status |
| --- | --- | --- |
| Layar menampilkan deposit "saldo +Rp 4.340" padahal kode sumber benar (T5.2) | Build Rust memakai sumber lama (Cargo tidak mengompilasi ulang file yang diganti) | TERJAWAB (8 Nov 2026): selesai setelah `touch` + `cargo clean` + build ulang; lihat 1q |
| `PenerimaBoot.kt` belum diperiksa terhadap penjaga pantauan (T8.1) | Bila boot gagal menyalakan layanan, `PREF_AKTIF` dimatikan dan penjaga berhenti tanpa peringatan | Terbuka; kirim `PenerimaBoot.kt` untuk ditutup (lihat 1w) |
| `tradeId` event WebSocket vs REST belum dibandingkan | Belum ada data pembanding | Tidak menghalangi (anti-dobel dari REST) |
| Kemungkinan "database is locked" saat dua koneksi SQLite aktif bersamaan | `busy_timeout` 5 detik ada; layar kini membaca lewat antrean satu thread, tulis oleh pantau tetap bisa bersamaan | Pantau saat uji lanjutan |
| Pantauan tidak menyala sendiri setelah HP reboot | Belum ada penerima BOOT_COMPLETED | Selesai (T5.1, lulus uji HP 8 Nov 2026) |
| Clearing fee baru teramati bernilai 0 | Hanya 2 sampel kecil (UAI) | Pantau; bila `commission` REST berbeda dari fee+tax+clearing event, andalkan REST |
| Koin yang sudah tidak dipegang (contoh ICNT) tidak terpindai | Daftar koin dari saldo + riwayat tersimpan | Dikenal; solusi sementara `koin_tambahan`, belum ada isiannya di UI |
| `pxgidr` ditolak Indodax (kode 1109) | Dugaan: tidak ada pasangan IDR | Dilewati dengan benar; penyebab belum dipastikan |
| TP/SL dibulatkan ke rupiah utuh | Aturan tick harga per koin belum diperiksa | Diputuskan tidak perlu (8 Nov 2026): TP/SL hanya acuan, harga akhir diinput manual di Indodax |
| Dua order beli koin sama yang selesai nyaris bersamaan menampilkan notifikasi order yang sama | Order dikenali dari order beli terbaru koin itu; `OrderRingkas` belum punya ID order | Dikenal (T1.5); perbaikan bila ID order cocok dengan ID event WebSocket |
| Fee jual perkiraan 0,31% tetap untuk estimasi bersih (notifikasi dan layar Posisi); jual market sekitar 0,61% (struk UAI 7 Nov: 123 dari 20.112) | Belum ada setelan | Diputuskan tidak perlu setelan (8 Nov 2026): estimasi hanya perkiraan; hasil nyata memakai `commission` API (riwayat, saldo, statistik) |
| `amount` deposit: bersih atau belum dikurangi `totalFee`? | Diuji 8 Nov 2026: saldo asli naik sebesar `amount` | TERJAWAB: sudah bersih. Logika bawaan diperbaiki di T5.2 |
| Bentuk baris withdraw dan mode fee withdraw belum teramati | Withdraw nyata belum bisa diuji (saldo di bawah batas minimum penarikan) | Parser toleran; validasi saat ada withdraw nyata |
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
| T6.1 | Total aset nilai pasar di Beranda (A: Rust, B: Kotlin) | SELESAI, lulus tes dan uji HP 8 Nov 2026 (lihat 1r) |
| T6.2 | Beranda tunggal (Total aset + Buying power), target dan grafik dari profit trade tertutup | SELESAI, lulus tes (core 114, ffi 19) dan uji HP 8 Nov 2026 (lihat 1r) |
| T6.3 | Floating P/L di Beranda dan layar Posisi | SELESAI, lulus tes (ffi 25) dan uji HP 8 Nov 2026 (lihat 1r) |
| T6.4 | Pembersihan snapshot/modal aktif/setoran/koreksi saldo; deposit-withdraw jadi riwayat saja; PRD v1.6 | SELESAI, lulus tes (core 99, ffi 24) dan uji HP 8 Nov 2026 (lihat 1s) |
| T6.5 | Hapus `GrafikView.setData` dan `TitikGrafikUi` | SELESAI, lulus tes (core 99, ffi 24) dan uji HP 8 Nov 2026 (lihat 1t) |
| T5.2 | Perbaiki logika deposit | SELESAI, lulus uji HP 8 Nov 2026 (lihat 1p dan 1q) |
| Opsional | Uji withdraw nyata saat saldo cukup (bentuk baris dan mode fee) | Menunggu saldo di atas batas minimum penarikan (8 Nov 2026: pengguna menunda uji karena fee penarikan besar, sekitar 11 ribu dari 100 ribu) |
| T6.6 | Cegah impor cadangan saat sinkron manual/otomatis sedang berjalan | SELESAI, lulus uji HP 8 Nov 2026 (lihat 1u) |
| T7.1 | Sederhanakan Beranda dan Posisi (floating digabung, teks penjelas dihapus) | SELESAI, lulus uji HP 8 Nov 2026 (lihat 1v) |
| T7.2 | Sederhanakan Riwayat dan Setelan (statistik dipadatkan, kartu deposit/withdraw di Setelan dihapus, log tersembunyi) | SELESAI, lulus uji HP 8 Nov 2026 (lihat 1v) |
| T8.1 | Log diagnostik permanen dan penjaga pantauan (hanya Kotlin) | SELESAI, lulus uji HP 8 Nov 2026 (lihat 1w) |
| Rilis uji | Rilis uji pribadi 0.8.0-uji: dipakai harian beberapa hari, catat bug dan peningkatan (panduan `docs/PANDUAN_RELEASE.md`) | BERJALAN sejak 8 Nov 2026 (lihat 1x) |
| Opsional | ID order di notifikasi beli (dua beli koin sama yang selesai nyaris bersamaan) | Butuh pengecekan ID order event WebSocket vs REST lebih dulu |
| Opsional | Verifikasi mode fee deposit saat ada deposit nyata pertama | Menunggu deposit nyata |

### Keputusan yang masih terbuka
| Hal | Pilihan | Status |
| --- | --- | --- |
| Rumus persen di notifikasi jual (setelah T6.2) | A: total aset terakhir yang disimpan, atau B: ambil total aset saat jual | Terjawab: Opsi A (8 Nov 2026, dikerjakan di T6.4, lihat 1s) |
| Fee jual pada floating P/L (T6.3) | Sebelum fee jual (berlabel) atau dikurangi perkiraan tarif | Terjawab: sebelum fee jual, dipakai di T6.3, lulus uji HP (8 Nov 2026) |
| Setelan TP/SL baru | Hanya posisi baru atau juga yang terbuka | Terjawab: hanya posisi baru (dikonfirmasi 8 Nov 2026) |
| Koin yang dipegang sebelum app dipasang | Diabaikan atau diisi modalnya | Terjawab: diabaikan (7 Nov 2026) |
| Mode fee deposit/withdraw (`amount` bersih atau tidak) | Lihat 1i dan 1o | Deposit: TERJAWAB, bersih (8 Nov 2026). Withdraw: dipakai `-(amount + fee)` tanpa pilihan di layar, belum terverifikasi (batas minimum penarikan) |
| Pantauan menyala otomatis setelah HP reboot | Perlu atau tidak | Terjawab: dikerjakan di T5.1, lulus uji HP (8 Nov 2026) |

### Keputusan yang sudah terjawab
- Fase 7 (8 Nov 2026): tampilan disederhanakan; floating digabung ke baris di bawah Profit trading; koin tanpa harga IDR disembunyikan; tombol sinkron deposit/withdraw manual dihapus (sinkron otomatis); fee deposit/withdraw dan profit kotor tidak ditampilkan. T7.1 dan T7.2 lulus uji HP (lihat 1v).
- T6.4 dan T6.5 lulus tes (core 99, ffi 24) dan uji HP sesuai kriteria; Fase 6 selesai (8 Nov 2026). `GrafikView.setData` dan `TitikGrafikUi` dihapus.
- T6.6 lulus uji HP (8 Nov 2026): impor cadangan ditolak saat sinkron trade, sinkron arus kas, atau pantauan berjalan; sinkron tidak bisa mulai selama impor berjalan.
- Notifikasi jual memakai total aset terakhir yang disimpan Beranda (Opsi A, 8 Nov 2026); bila belum ada, hanya rupiah profit. Saldo versi aplikasi, snapshot periode, modal aktif, setoran bersih, dan koreksi saldo dihapus di T6.4 (tabel lama dibiarkan).
- Hanya satu angka saldo: Total aset nilai pasar, IDR berlabel "Buying power" (8 Nov 2026, lihat 1r). Menggantikan keputusan awal "saldo/snapshot/target/grafik dari total aset".
- T6.1 lulus tes Rust dan uji HP (8 Nov 2026).
- T6.2 lulus tes Rust (core 114, ffi 19) dan uji HP sesuai kriteria (8 Nov 2026).
- T6.3 lulus tes Rust (ffi 25) dan uji HP sesuai kriteria (8 Nov 2026); floating dihitung sebelum fee jual, berlabel, tidak masuk target dan grafik.
- Persen target = profit periode ÷ (total aset sekarang − profit periode); grafik = akumulasi profit tertutup; deposit/withdraw dihapus dari perhitungan dan jadi riwayat saja; riwayat hanya sejak aplikasi dipasang; pembanding platform bebas (8 Nov 2026, lihat 1r).
- Target dan grafik dari profit trade tertutup (bukan selisih jual-beli mentah, bukan snapshot saldo); floating P/L tampil terpisah dan tidak masuk target (pilihan 1, dikonfirmasi 8 Nov 2026).
- Harga koin memakai `last` saja (8 Nov 2026); bid mungkin sebagai informasi tambahan nanti.
- Deposit: `amount` dari API sudah bersih, `totalFee` hanya informasi (uji nyata 8 Nov 2026, Rp 11.000 menjadi Rp 7.670 masuk). Perbaikan logika = T5.2.
- Withdraw nyata tidak bisa diuji sekarang (saldo di bawah batas minimum penarikan); dicatat, diuji nanti (8 Nov 2026).
- Setelan TP/SL baru: berlaku untuk posisi baru saja (dikonfirmasi 8 Nov 2026).
- Aturan tick harga per koin: tidak perlu dikerjakan; TP/SL hanya acuan, harga akhir diinput manual di Indodax (8 Nov 2026).
- Setelan fee jual (limit/market): tidak perlu; estimasi hanya perkiraan, hasil nyata dari `commission` API (8 Nov 2026).
- T5.1 (pantauan menyala otomatis setelah HP restart): lulus uji HP (8 Nov 2026).
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
- U5: salinan database lama otomatis disimpan sebagai `.sebelum-impor`; kunci API tidak dicadangkan; impor wajib dengan pantauan mati (usulan, lolos uji HP); sejak T6.6 juga wajib tanpa sinkron berjalan.

### Usulan T5.2 (dikerjakan 8 Nov 2026 sesuai usulan ini; hasil di 1p; lulus uji HP, lihat 1q)
- Deposit: selalu `+amount`; `totalFee` hanya disimpan dan ditampilkan sebagai informasi di baris deposit. Kotak "Nominal sudah bersih" tidak lagi memengaruhi deposit.
- Withdraw: selalu `-(amount + fee)` tanpa pilihan di layar (usulan pengguna menghapus kotak, 8 Nov 2026). Ini dugaan berdasarkan deposit (amount = nominal bersih, fee terpisah), BELUM terverifikasi; bila meleset, "Cek selisih saldo" akan menunjukkan selisih dan perlu ditinjau lagi saat ada withdraw nyata.
- Catatan deposit lama di database dihitung ulang otomatis bila fee tidak disimpan sebagai angka jadi (saldo dihitung dari catatan); perlu dipastikan di kode.
- Kode yang perlu dilihat (jangan berasumsi isinya, minta dulu): `core/src/arus_kas.rs`, `core/src/store.rs` (fungsi arus kas dan `atur_amount_sudah_bersih`), `core/src/periode.rs` (pemakaian arus kas bersih), `ffi/src/lib.rs` (bagian arus kas), `app/aplikasi/src/main/kotlin/id/catatanscalping/app/LayarSetelan.kt` (kotak centang dan teks baris deposit).
- Kriteria uji: dengan data deposit nyata di atas, tanpa mencentang apa pun, saldo aplikasi = Rp 77.322, "Cek selisih saldo" = Rp 0, modal aktif naik 7.670, hasil dan pencapaian tidak berubah, sinkron ulang tidak menggandakan.

### Pekerjaan kecil
- PRD v1.7: SELESAI (8 Nov 2026). Perubahan: tujuan tampilan minimalis; FR-14, FR-22, FR-24, FR-25 disesuaikan tampilan baru (FR-10 tidak diubah); definisi Total aset (koin tanpa harga IDR tidak ditampilkan); bagian 12 (tampilan); Fase 7 di bagian 15; pertanyaan terbuka no. 14. Perhitungan tidak berubah. Simpan sebagai `docs/PRD_Aplikasi_Catatan_Trading_Crypto.md` (timpa v1.6).
- PRD bagian 15: status Fase 6 diperbarui menjadi "Selesai (T6.1 sampai T6.5 lulus uji HP 8 Nov 2026)" (8 Nov 2026). Isi PRD v1.6 lainnya tidak berubah.
- PRD v1.6: SELESAI (8 Nov 2026). Perubahan: Fase 6 (Total aset, Buying power, profit tertutup, floating; FR-19 sampai FR-22, FR-24, FR-28 baru; FR-27 dihapus), rumus bagian 8, definisi, model data, kasus khusus, risiko. Simpan sebagai `docs/PRD_Aplikasi_Catatan_Trading_Crypto.md` (timpa v1.5).
- PRD v1.5: SELESAI (8 Nov 2026). Perubahan: hasil uji deposit nyata (bagian 9), pertanyaan terbuka no. 10 terjawab untuk deposit, status withdraw. Simpan sebagai `docs/PRD_Aplikasi_Catatan_Trading_Crypto.md` (timpa v1.4).
- PRD v1.4: SELESAI (8 Nov 2026). Perubahan: FR-05 dikonfirmasi, pertanyaan terbuka no. 5 dan 11 terjawab, Fase 5 selesai, dua baris risiko (fee jual, tick) diberi keputusan. Simpan sebagai `docs/PRD_Aplikasi_Catatan_Trading_Crypto.md` (timpa v1.3).
- PRD v1.3: SELESAI (8 Nov 2026). Berisi FR-14 tanpa tombol Jual, FR-16 dan FR-18 (progress dan penanda tercapai di notifikasi jual), FR-22/23/25 sudah diimplementasikan, FR-26 (cadangan), FR-27 (koreksi saldo), saldo versi aplikasi dihitung dari catatan, bagian 9 (token WebSocket dengan key v2, SHA512, status FILL, WebSocket sebagai pemicu), pertanyaan terbuka no. 3, 4, 6, 8, 9 terjawab. Simpan sebagai `docs/PRD_Aplikasi_Catatan_Trading_Crypto.md` (timpa v1.2).

### Fase berikutnya
- **Fase 2**: selesai (T2.1 sampai T2.5)
- **Fase 3**: selesai (T3.1 sampai T3.3)
- **Fase 4**: selesai (T4.1)
- **Fase UI**: selesai (U1 sampai U5, lulus uji HP 8 Nov 2026)
- **Fase 5 (penyempurnaan)**: selesai. T5.1 lulus uji HP (8 Nov 2026); T5.2 (logika deposit) lulus uji HP (8 Nov 2026)
- **Fase 7 (Penyederhanaan tampilan)**: selesai. T7.1 dan T7.2 lulus uji HP (8 Nov 2026)
- **Fase 8 (Uji lapangan)**: berjalan. T8.1 (log diagnostik dan penjaga pantauan) lulus uji HP 8 Nov 2026; rilis uji 0.8.0-uji dipakai harian (lihat 1x)
- **Fase 6 (Total aset tunggal)**: selesai. T6.1 sampai T6.5 lulus tes dan uji HP (8 Nov 2026); T6.6 (celah impor) lulus uji HP (8 Nov 2026)

---

## 4. Cara instalasi

Kebutuhan: Python 3.8+, git, Rust (rustup), dan toolchain Android (lihat bagian 3). Skrip Python tidak butuh `pip install`.

1. Masuk ke folder proyek: `cd catatan-scalping`
2. Buat key di indodax.com/trade_api: key **TAPI v2**, izin **baca saja**, tanpa pembatasan IP.
3. Salin `.env.example` menjadi `.env`, isi `INDODAX_API_KEY` dan `INDODAX_API_SECRET` (untuk skrip dan contoh Rust di komputer; aplikasi Android menyimpan key sendiri di HP).

Tes inti Rust:
```
cargo test -p scalping-core        # 51 tes per T1.4a; T4.1 menambah 6 di statistik.rs; per T6.2 total 114; T6.4: 99 (sebenarnya, 8 Nov 2026; T6.5 tidak mengubah jumlah)
cargo test -p scalping-ffi         # T1.5: 6 tes; T2.1: +2 (8); T2.3/T2.4: +2 (10); T3: +4 (14); U1: +1 (15); U5: +1 (16); U5 juga menambah 4 tes di store.rs; T6.2: +3 (19); T6.3: +6 (25); T6.4: 24 (sebenarnya, 8 Nov 2026; T6.5 tidak mengubah jumlah)
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
Catatan: `git add .` wajib agar file Kotlin baru (Tema.kt, Format.kt, Komponen.kt, Latar.kt, Sinkron.kt, Layar.kt, Layar*.kt, Cadangan.kt, PenerimaBoot.kt) ikut tercatat.

---

## 5. Struktur proyek

### Saat ini (setelah T5.1)
```
catatan-scalping/
├── README.md                            (diperbarui 8 Nov 2026: status, cara pasang, rilis uji)
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
│   │   ├── harga.rs                     (T6.1: parser ticker_all, total aset = IDR + koin x harga last)
│   │   ├── calc.rs                      (gabung per order, anti-dobel, modal efektif, TP/SL, profit jual)
│   │   ├── store.rs                     (SQLite: trade, snapshot, meta, setelan_tp_sl, arus_kas, koreksi_saldo; U5: ekspor_ke, periksa_cadangan, pulihkan)
│   │   ├── sync.rs                      (rekonsiliasi REST per koin, snapshot saldo awal)
│   │   ├── arus_kas.rs                  (T2.2: parser fiat/orders, arus kas, sinkron_arus_kas)
│   │   ├── periode.rs                   (periode WIB; T6.2: pencapaian_profit, deret_profit; T6.4: saldo versi aplikasi/DataSaldo dihapus)
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
│   └── src/lib.rs                       (cek_koneksi, sinkron_sekarang, order_terbaru, simpan_setelan, baca_setelan, hitung_posisi, daftar_posisi, Pemantau, PendengarPantau, KejadianUi, isi_notifikasi, ringkasan_posisi_notif, IsiNotifOrder, target dan arus kas, hitung_target (hasil_persen), riwayat_tertutup, data_grafik, koreksi saldo, statistik_trading, ekspor_cadangan, periksa_cadangan, pulihkan_cadangan, total_aset, ringkasan_profit, data_grafik_profit (T6.2), ambil_harga_terakhir, floating_posisi (T6.3); T6.4: koreksi saldo dan target lama dihapus; T6.5: TitikGrafikUi dihapus; 24 tes)
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
│           │   ├── LayarBeranda.kt      (T6.2: kartu Total aset tunggal + Buying power, Profit trading, grafik akumulasi profit, target dari profit tertutup, baris Floating (T6.3), status koneksi; T7.1: disederhanakan, floating digabung ke baris di bawah Profit trading)
│           │   ├── LayarPosisi.kt       (U3: posisi terbuka, tangga SL/modal/TP, tanpa tombol Jual; T6.3: harga sekarang, penanda di tangga, floating per posisi; T7.1: satu baris harga + floating, satu baris estimasi bersih)
│           │   ├── LayarRiwayat.kt      (U4: ringkasan, daftar trade, statistik; T6.4: riwayat deposit/withdraw, koreksi saldo dihapus; T7.2: statistik dipadatkan, tanpa teks penjelas)
│           │   ├── LayarSetelan.kt      (U1: TP/SL, target, kunci, pantauan, sinkron, log; U5: kartu Cadangan data; T7.2: tanpa teks penjelas, kartu arus kas dihapus, log tersembunyi)
│           │   ├── Cadangan.kt          (U5: ekspor/impor lewat pemilih berkas sistem; T6.6: cek ulang sinkron/pantauan sebelum database diganti)
│           │   ├── PenerimaBoot.kt      (T5.1: menyalakan pantauan lagi setelah HP restart)
│           │   ├── Tema.kt              (U1: warna terang/gelap sesuai prototype)
│           │   ├── Format.kt            (U1: titik ribuan, koma desimal, tanda, waktu WIB)
│           │   ├── Komponen.kt          (U1: kartu, tombol, kolom isian, chip, bar)
│           │   ├── Latar.kt             (U1: antrean baca database dan pool jaringan)
│           │   ├── Sinkron.kt           (U1: sinkron trade dan arus kas, manual dan otomatis; T6.6: penanda imporBerjalan, mulaiImpor/selesaiImpor)
│           │   ├── GrafikView.kt        (T3.2; U2: bertema; T6.2: mode profit via setProfit; T6.5: hanya grafik profit, setData dihapus)
│           │   ├── LayananLatar.kt      (T1.4c: foreground service + PantauBersama; T1.5: saluran "Order terisi" dan pekerja notifikasi)
│           │   ├── LogDiag.kt           (T8.1: log permanen diag.log, maks sekitar 256 KB)
│           │   ├── Aplikasi.kt          (T8.1: kelas Application; catat awal proses, alasan proses sebelumnya berakhir, crash)
│           │   ├── PenjagaPantauan.kt   (T8.1: alarm 5 menit, peringatan bila pantauan mati/terputus)
│           │   └── KunciAman.kt         (simpan key terenkripsi, Android Keystore)
│           ├── res/values/themes.xml
│           ├── res/values-night/themes.xml
│           ├── java/uniffi/             (OTOMATIS dari bindgen, tidak di-commit)
│           └── jniLibs/                 (OTOMATIS dari cargo ndk, tidak di-commit)
├── docs/
│   ├── PRD_Aplikasi_Catatan_Trading_Crypto.md     (v1.7)
│   ├── PANDUAN_RELEASE.md                         (rilis uji 0.8.0-uji: pasang, uji lapangan, lapor bug, rollback)
│   └── prototype/
│       └── Catatan_trading__prototype_UI.html
└── tools/
    ├── fase0_validasi.py                (hanya baca; SHA256; --diagnosa)
    ├── fase0_akun.py                    (bentuk respons /account, angka disamarkan)
    ├── tes_websocket_token.py           (T0.6: tes token dan event WebSocket privat, baca saja)
    ├── fase2_fiat.py                    (T2.2: lihat bentuk respons fiat/orders, angka disamarkan, baca saja)
    ├── cek_lingkungan.py                (laporan versi Rust/Android/adb)
    └── bangun_android.sh                (Rust -> binding -> APK -> adb; memeriksa .text kode Rust di APK; APK terbaru)
```

### Rencana
Semua task pada rencana selesai, termasuk Fase 6 (T6.1 sampai T6.6) dan Fase 7 (T7.1 dan T7.2); PRD v1.7 selesai. Tersisa hanya yang opsional: uji withdraw nyata saat saldo di atas batas minimum penarikan (bentuk baris dan mode fee withdraw; pengguna menunda karena fee penarikan besar, 8 Nov 2026) dan ID order di notifikasi beli (lihat bagian 3). Struktur di atas sudah mencakup semua file; tidak ada file baru yang direncanakan.

---

## 6. Log commit
(isi hash dan pesan commit di sini)
- T8.1: `T8.1: log diagnostik permanen dan penjaga pantauan` (hash: ...)
- Dokumen: `Dokumen: README, panduan rilis uji, PROGRESS (T8.1 lulus)` (hash: ...)
- Tag: `git tag -a v0.8.0-uji -m "Rilis uji 0.8.0-uji"` (setelah semua commit)
- Dokumen: `Dokumen: PRD v1.7 dan PROGRESS (Fase 7, T7.1 dan T7.2)` (hash: ...)
- T7.2: `T7.2: sederhanakan Riwayat dan Setelan` (hash: ...)
- T7.1: `T7.1: sederhanakan Beranda dan Posisi` (hash: ...)
- T6.6: `T6.6: cegah impor cadangan saat sinkron berjalan` (hash: ...) (commit bersama PROGRESS ini; isi hash dari `git log --oneline`)
- T6.5: `T6.5: hapus GrafikView.setData dan TitikGrafikUi` (hash: 3db3e8e)
- T6.4: `T6.4: pembersihan saldo lama` (hash: 483cfc8)
- Dokumen: `Dokumen: PRD v1.6 dan PROGRESS (Fase 6, T6.4)` (hash: ...)
- Dokumen: `Dokumen: PRD status Fase 6 selesai dan PROGRESS (hash commit T6.3 sampai T6.5)` (hash: ...)
- T6.3: `T6.3: floating P/L di Beranda dan layar Posisi` (hash: bb0bfa7)
- T6.2: `T6.2: Beranda tunggal, target dan grafik dari profit trade tertutup` (hash: ...)
- T6.1: `T6.1: total aset nilai pasar di Beranda (harga ticker_all, total aset)` (hash: ...)
- Alat: `Alat: bangun_android.sh memeriksa .text kode Rust di APK dan memakai APK terbaru` (hash: ...)
- T5.2: `T5.2: deposit amount sudah bersih, fee hanya informasi` (hash: ...)
- Dokumen: `Dokumen: PRD v1.5 dan PROGRESS (hasil uji deposit nyata, rencana T5.2)` (hash: ...)
- Dokumen: `Dokumen: PRD v1.4 dan PROGRESS (T5.1 lulus, keputusan 8 Nov)` (hash: ...)
- T5.1: `T5.1: pantauan menyala otomatis setelah HP restart` (hash: ...)
- PRD v1.3: `Dokumen: PRD v1.3 sesuai aplikasi` (hash: ...)
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
