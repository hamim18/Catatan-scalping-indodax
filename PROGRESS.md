# PROGRESS: Catatan Scalping

Terakhir diperbarui: 14 November 2026 (riwayat Fase 0 sampai 8 diringkas; Fase 9 Screener dirancang; PRD v1.8)

**Status:** Fase 0 sampai 8 SELESAI dan lulus uji HP. Rilis uji pribadi 0.8.0-uji (versionCode 2) dipakai harian; uji lapangan sampai 14 Nov 2026: tidak ada kendala, aplikasi nyaman dipakai, tidak ada perbaikan yang diperlukan. Jumlah tes terakhir: core 99, ffi 24.

**Sekarang:** Fase 9 (Screener) tahap RANCANGAN. Belum ada kode. Langkah berikutnya: T9.0 (verifikasi API publik lewat skrip kecil), lihat bagian 5.

## Aturan kerja
- Satu task selesai = satu `git commit`. Setelah tiap task: perbarui file ini, lalu commit.
- Jangan pernah commit `.env` atau isi `data/` (sudah di `.gitignore`).
- Aplikasi hanya membaca data. Key API wajib izin **baca saja**. Fitur Screener memakai endpoint publik tanpa key.
- Build Android SELALU lewat `bash tools/bangun_android.sh` (membuat ulang binding Kotlin dari Rust). Gradle langsung tidak memperbarui binding.
- Setelah mengganti file sumber dari unduhan: `touch` file itu (atau `touch core/src/*.rs ffi/src/lib.rs app/aplikasi/src/main/kotlin/id/catatanscalping/app/*.kt`) sebelum build. Bila ragu, `cargo clean` lalu build ulang (pelajaran T5.2: Cargo tidak mengompilasi ulang file yang waktu modifikasinya lebih tua dari build terakhir, sehingga tes "lulus" dari sumber lama).
- Pastikan tes baru benar-benar jalan: jumlah tes bertambah dan filter nama menemukan tesnya.
- Pemasangan ulang APK lewat adb mematikan pantauan; nyalakan lagi manual. Jangan uninstall (data hilang); ekspor cadangan dulu.

---

## 1. Yang sudah dikerjakan (ringkas)

| Fase | Isi singkat | Status |
| --- | --- | --- |
| 0 (T0.1 sampai T0.6) | Repo, PRD, prototype; validasi data nyata ICNT dengan key baca saja (fee, modal efektif cocok dengan struk); token WebSocket privat dengan key v2 baca saja (HMAC-SHA512), event nyata diterima | Selesai |
| 1 (T1.0 sampai T1.5) | Keputusan teknologi (Rust untuk logika, Kotlin klien Android); klien REST, parser akun, penyimpanan SQLite, rekonsiliasi per koin, posisi dan TP/SL; protokol dan sambungan WebSocket privat (hanya pemicu sinkron); `Pemantau` dan layanan latar (specialUse); notifikasi beli dan jual | Selesai |
| 2 (T2.1 sampai T2.5) | Setelan target, arus kas deposit/withdraw, periode WIB, progress target di notifikasi jual, sinkron arus kas dari layanan latar | Selesai |
| 3 dan 4 | Riwayat trade tertutup, statistik (win rate, rata-rata, fee, persen termakan fee); grafik saldo dan koreksi saldo (kemudian diganti di Fase 6) | Selesai |
| UI (U1 sampai U5) | Layar asli 4 tab (Beranda, Posisi, Riwayat, Setelan) tanpa Compose; ekspor/impor database lokal | Selesai |
| 5 (T5.1, T5.2) | Pantauan menyala otomatis setelah HP restart; deposit `amount` sudah bersih, fee hanya informasi | Selesai |
| 6 (T6.1 sampai T6.6) | Satu angka saldo: Total aset nilai pasar (IDR berlabel "Buying power"); target dan grafik dari profit trade tertutup; floating P/L terpisah; pembersihan saldo lama (snapshot periode, modal aktif, setoran, koreksi saldo dihapus); deposit/withdraw jadi riwayat; notifikasi jual memakai total aset terakhir (Opsi A); impor cadangan ditolak saat sinkron berjalan | Selesai |
| 7 (T7.1, T7.2) | Tampilan disederhanakan: tanpa teks penjelas, floating digabung ke baris Profit trading, koin tanpa harga IDR disembunyikan, sinkron deposit/withdraw otomatis, log tersembunyi | Selesai |
| 8 (T8.1) | Log diagnostik permanen `diag.log`, tombol Bagikan log, penjaga pantauan (alarm 5 menit); rilis uji 0.8.0-uji; panduan `docs/PANDUAN_RELEASE.md` | Selesai (uji lapangan lulus 14 Nov 2026) |

## 2. Kesimpulan teknis yang masih berlaku

**Data Indodax (Fase 0 dan sesudahnya)**
- Fee dalam IDR; koin yang diterima tidak dipotong. Fee selain IDR sengaja ditolak (error).
- Modal efektif = (`quoteQty` + `commission`) ÷ `qty` (= "Order Terisi" di struk ÷ koin). Contoh ICNT 5 Nov: 4.006,07; TP 1% = 4.046; SL 1% = 3.966; hasil bersih jual +380 IDR (+0,58%).
- Tarif fee tidak tetap (beli limit ±0,10%, beli market ±0,20%, jual limit ±0,31%, jual market ±0,61%). Selalu pakai `commission` dari API; estimasi di notifikasi dan layar Posisi memakai 0,31% (diputuskan tanpa setelan fee).
- Deposit: `amount` sudah bersih, `totalFee` hanya informasi (uji nyata Rp 11.000 menjadi Rp 7.670). Withdraw: bentuk baris dan mode fee BELUM teramati (saldo di bawah batas minimum penarikan); parser toleran.
- WebSocket privat: token via `POST /api/private_ws/v1/generate_token` (HMAC-SHA512), `wss://pws.indodax.com/ws/?cf_ws_frame_ping_pong=true` tanpa header Origin, ping `{}` dibalas `{}`. Status: NEW, FILL, DONE, CANCELLED, REJECTED. Harga di event dibulatkan, jadi event hanya PEMICU; angka resmi dari REST.
- `ticker_all` (publik, `https://indodax.com/api/ticker_all`): ambil `last` pasangan `*_idr`. Harga `last` saja dipakai untuk Total aset dan floating.
- Simbol ditolak Indodax (HTTP 400 kode 1109, contoh `pxgidr`) dilewati.

**Build dan lingkungan**
- AGP 9.2.x butuh Gradle ≥ 9.4.1, JDK 17, build-tools 36.0.0; Kotlin dari AGP; UniFFI 0.31; JNA 5.17.0; hanya arm64-v8a. `jniLibs/` dan `java/uniffi/` otomatis, tidak di-commit. `minSdk = 30` (dugaan; cek `adb shell getprop ro.build.version.sdk`).
- Peringatan aman diabaikan: `Unable to strip libjnidispatch.so`, deprecation Gradle 10.
- adb nirkabel mendaftarkan HP dua kali; pakai serial tanpa spasi (`export ANDROID_SERIAL=...`). Bila banyak perangkat: `bash tools/bangun_android.sh --hanya-pasang <serial>` (tidak membangun ulang).
- Lingkungan: Arch Linux, Rust + `cargo ndk` (4 target Android), JDK 17, Gradle di PATH, Android SDK di `~/android-sdk`, tanpa Android Studio, HP via adb nirkabel.

**Arsitektur**
- Semua logika dan perhitungan uang di Rust (Decimal); Kotlin hanya tampilan, layanan latar, notifikasi, penyimpanan kunci (Android Keystore).
- Posisi dan profit tertutup dihitung ulang dari catatan trade tiap kali (tanpa tabel sendiri). Total aset selalu dari akun (jaringan). Database `catatan.db` di penyimpanan privat aplikasi.
- Tabel lama (`snapshot`, `koreksi_saldo`, `arus_kas`) dibiarkan agar cadangan v1.x tetap bisa diimpor.
- Sinkron: limit 100 per permintaan, rentang dibelah bila hasil penuh; berhenti dengan error bila rentang 1 menit masih penuh. `busy_timeout` 5 detik.

## 3. Keputusan yang sudah dijawab (ringkas)

- Aplikasi tidak membuat order; tombol Jual dihapus (7 Nov). Cadangan hanya lokal (7 Nov). Zona WIB, minggu mulai Senin (7 Nov). Koin lama sebelum pemasangan diabaikan (7 Nov).
- TP/SL baru hanya untuk posisi baru; aturan tick harga dan setelan fee jual tidak dikerjakan (8 Nov).
- Satu angka saldo: Total aset nilai pasar; target = profit periode ÷ (total aset − profit periode) × 100; grafik = akumulasi profit tertutup; floating terpisah dan dihitung sebelum fee jual; deposit/withdraw hanya riwayat; riwayat hanya sejak aplikasi dipasang (8 Nov).
- Notifikasi jual memakai total aset terakhir yang disimpan Beranda (Opsi A, 8 Nov). Harga memakai `last` saja.
- Tampilan minimalis (Fase 7, 8 Nov): lihat tabel di atas.
- **Fase 9 (14 Nov 2026):** fitur Screener khusus penyaringan, tidak menyentuh perhitungan trading; semua parameter bisa diatur di aplikasi; modal per trade berubah-ubah (tidak ada modal tetap; isian opsional "Modal trade ini" tidak disimpan permanen); ada filter lonjakan ▲/▼ dengan trx/mnt tinggi yang langsung masuk notifikasi; notifikasi lonjakan di **layanan terpisah dengan saklar sendiri** (`LayananScreener`, `LayananLatar` tidak diubah); garis besar format notifikasi disetujui.

## 4. Masalah dikenal dan hal yang belum tuntas

| Hal | Keterangan | Status |
| --- | --- | --- |
| Uji withdraw nyata | Bentuk baris dan mode fee withdraw belum teramati; menunggu saldo di atas batas minimum penarikan (pengguna menunda karena fee penarikan besar) | Opsional, terbuka |
| ID order di notifikasi beli | Dua beli koin sama yang selesai nyaris bersamaan menampilkan order yang sama (`OrderRingkas` belum punya ID order yang cocok dengan event WebSocket) | Dikenal, opsional |
| `PenerimaBoot.kt` vs penjaga pantauan | Bila boot gagal menyalakan layanan, `PREF_AKTIF` dimatikan dan penjaga berhenti tanpa peringatan; `PenerimaBoot.kt` belum diperiksa untuk T8.1 | Terbuka, kirim file bila ingin ditutup |
| Thread arus kas di `LayananLatar.kt` | Tidak tercatat di penanda impor (T6.6); risiko kecil karena impor mensyaratkan pantauan mati | Dikenal |
| `tradeId` event WebSocket vs REST | Belum dibandingkan; tidak menghalangi (anti-dobel dari REST) | Dikenal |
| "database is locked" | `busy_timeout` ada; pantau saat dua koneksi aktif | Belum teramati di uji lapangan |
| Koin yang sudah tidak dipegang (contoh ICNT) | Tidak terpindai bila habis terjual sebelum sinkron pertama; solusi `koin_tambahan`, belum ada isian di UI | Dikenal, tidak relevan di pemakaian normal |
| Selisih 1 rupiah jual vs modal beli berikutnya, sisa 3 IDR anggaran order | Diduga pembulatan | Tidak berdampak |
| Clearing fee WebSocket baru teramati bernilai 0 | Sampel kecil; REST selalu menjadi acuan | Pantau |
| Ukuran halaman `fiat/orders` tidak diketahui | Rentang ≥ 100 baris dibelah | Pantau |
| Jenis build APK (debug atau release) dari `bangun_android.sh` | Menentukan apakah pembaruan menimpa tanpa uninstall; cek `grep -n "buildTypes\|signingConfig\|isMinify" app/aplikasi/build.gradle.kts` | Belum dikonfirmasi |
| Commit T8.1, commit dokumen, tag `v0.8.0-uji` | Belum dikonfirmasi sudah dijalankan; jalankan sebelum memulai T9.1 agar ada titik rollback bersih | Menunggu pengguna |

## 5. Fase 9: Screener (rancangan, 14 Nov 2026)

**Latar:** pengguna sudah punya skrip manual `screener_indodax.py` (Termux, tanpa library). Skrip: tahap 1 dari satu request `/api/summaries` (filter harga, volume IDR, spread, range 24 jam); tahap 2 dari `/api/trades/<pair>` (trx/mnt dalam jendela, buy%, usia transaksi terakhir), diurutkan menurut trx/mnt. Nilai di skrip: harga 10 sampai 70.000; volume 10 juta sampai 10 miliar; trx/mnt ≥ 2 (jendela 10 menit); spread ≤ 10%; range ≥ 4%; usia ≤ 120 detik; 4 pekerja paralel. Komentar di skrip sisa versi lama; nilai yang berlaku adalah nilai di atas.

**Kelemahan filter skrip (dasar penyempurnaan)**
- Range 24 jam basi untuk scalping; trx/mnt rata-rata menyembunyikan lonjakan sesaat; spread tidak dikaitkan dengan fee dan TP; kedalaman order book tidak dicek; arah harga tidak dilihat; pengurutan hanya dari trx/mnt.

**Penyempurnaan yang diusulkan (semua bisa diatur atau dimatikan; angka final setelah T9.0)**
1. Spread maksimum terikat TP (bawaan ≤ sepertiga TP di Setelan).
2. Gerak 10 menit (high-low trade 10 menit terakhir) ≥ 2 × perkiraan biaya round-trip (sekitar 0,41% limit), menggantikan range 24 jam.
3. Menit aktif ≥ 70%.
4. Usia transaksi terakhir 30 sampai 60 detik.
5. Kedalaman order book dalam 0,5% dari harga ≥ kelipatan "Modal trade ini" (kosong = dilewati).
6. Arah 10 menit minimum (hindari harga jatuh); buy% sebagai informasi.
7. Dominasi satu transaksi (> 40% volume jendela) hanya diberi tanda peringatan.
8. Skor urutan = gerak 10 menit ÷ (spread + biaya round-trip).

**Lonjakan dan notifikasi**
- Lonjakan: jendela pendek (bawaan 3 menit) vs dasar 10 menit sebelumnya; trx/mnt ≥ N × dasar dan ≥ M absolut; perubahan harga ≥ X%; ▲ naik dan ▼ turun punya saklar terpisah; syarat dasar (spread, usia transaksi) tetap berlaku.
- Notifikasi: judul contoh `▲ ABC +2,4% · 18 trx/mnt`; satu notifikasi per pair diperbarui di tempat; jeda per pair sekitar 5 menit; hilang sendiri sekitar 2 menit; maksimal 3 terpisah per menit, sisanya rangkuman "+N pair lonjakan"; saluran notifikasi sendiri; ketuk membuka Indodax. Informasi saja.
- Deteksi tahap 1: satu request ringkasan pasar tiap sekitar 10 sampai 15 detik, bandingkan dengan snapshot di memori; hanya pair yang tampak melonjak diperiksa lewat `trades`. Notifikasi bisa terlambat sekitar satu siklus (bukan realtime).

**Layar dan tombol**
- Layar Screener: satu daftar (pair, harga, trx/mnt, spread, gerak 10m, buy%), tanpa teks penjelas; penanda untuk pair yang sedang dipegang. Penempatan sebagai tab ke-5 adalah USULAN, belum dikonfirmasi.
- Tombol buka pair di Indodax: urutan rencana (1) tautan web `https://indodax.com/market/<PAIR>` bila terbukti membuka aplikasi di pair itu, (2) fallback salin nama pair ke papan klip lalu buka aplikasi Indodax. Skema deep link BELUM diverifikasi; tidak ditebak.

**Yang harus diverifikasi dulu (T9.0, jangan diasumsikan)**
- Jumlah transaksi yang dikembalikan `/api/trades/<pair>` (apakah jendela 10 menit terjangkau untuk koin ramai).
- Format `/api/depth/<pair>`.
- Arti `vol_idr` di `/api/summaries` (volume 24 jam bergulir?) sehingga selisih antar snapshot hanya perkiraan kasar.
- Seberapa cepat harga di ringkasan pasar diperbarui.
- Apakah Indodax punya WebSocket publik untuk data pasar (tidak diasumsikan ada).

**Rencana task (tiap task satu commit; PRD dan PROGRESS diperbarui tiap task)**
| Task | Isi | Catatan |
| --- | --- | --- |
| T9.0 | Skrip Python kecil di `tools/` untuk memverifikasi 4 hal di atas dan cek deep link di HP | Hasil menentukan rumus lonjakan dan default |
| T9.1 | `core/src/screener.rs`: parser ringkasan pasar, transaksi, kedalaman; filter dua tahap; skor; Decimal; tes dengan data nyata dari T9.0; tanpa UI | Hanya Rust |
| T9.2 | FFI + layar Screener + setelan ambang (tabel `meta`) + isian "Modal trade ini" | Kotlin + FFI; butuh melihat `ffi/src/lib.rs`, `MainActivity.kt`, `Layar.kt`, `LayarSetelan.kt`, `Komponen.kt`, `Latar.kt`, `Tema.kt` |
| T9.3 | Tombol buka pair di Indodax (deep link atau fallback) | Setelah hasil cek deep link |
| T9.4 | Deteksi lonjakan, `LayananScreener` (layanan terpisah, saklar sendiri), saluran dan notifikasi lonjakan | Butuh melihat `LayananLatar.kt` (sebagai acuan, tidak diubah) dan manifest |
| Penutup | Uji bersamaan dengan pantauan order; PRD v1.9 bila ada perubahan; rilis 0.9.0-uji | Pastikan notifikasi TP/SL tidak terganggu |

**Berkas yang akan diminta sebelum menulis kode (jangan menebak isinya):** sesuai kolom Catatan di atas, ditambah `core/src/client.rs`, `core/src/lib.rs`, `core/src/harga.rs` untuk pola parser publik.

**Pertanyaan terbuka Fase 9:** tab ke-5 atau kartu di Beranda; skema deep link; angka default lonjakan dan filter tambahan; WebSocket publik. Lihat PRD bagian 16 (no. 16 sampai 20).

---

## 6. Cara instalasi dan tes

Kebutuhan: git, Rust (rustup), toolchain Android (lihat bagian 2); Python 3.8+ hanya untuk skrip di `tools/` (tanpa `pip install`).

1. `cd catatan-scalping`
2. Key di indodax.com/trade_api: **TAPI v2**, izin **baca saja**, tanpa pembatasan IP.
3. Salin `.env.example` menjadi `.env`, isi `INDODAX_API_KEY` dan `INDODAX_API_SECRET` (untuk skrip dan contoh Rust di komputer; aplikasi Android menyimpan key sendiri di HP).

```
cargo test -p scalping-core        # 99 tes
cargo test -p scalping-ffi         # 24 tes
bash tools/bangun_android.sh --pasang <serial-adb>
```

Uji di HP: tab Setelan, simpan kunci, isi TP/SL, "Sinkron sekarang", mulai pantauan (matikan optimasi baterai, izinkan notifikasi), lakukan transaksi kecil di Indodax: notifikasi beli/jual muncul dan Beranda, Posisi, Riwayat menyegarkan diri. Ekspor cadangan dari Setelan sebelum memasang pembaruan.

Skrip bantu di `tools/`: `fase0_validasi.py`, `fase0_akun.py`, `tes_websocket_token.py`, `fase2_fiat.py`, `cek_lingkungan.py`, `bangun_android.sh`; contoh Rust: `cek_data`, `cek_koneksi`, `pantau`.

Alur git setiap task:
```
git add .
git commit -m "Tx.y: deskripsi singkat"
```
`git add .` wajib agar file Kotlin baru ikut tercatat.

---

## 7. Struktur proyek

```
catatan-scalping/
├── README.md
├── PROGRESS.md                          <- file ini
├── .gitignore, .env.example, Cargo.toml (workspace), Cargo.lock
├── data/                                (tarikan lokal, tidak di-commit)
├── core/                                (scalping-core: semua logika, tanpa UI)
│   └── src/ lib.rs, error.rs, model.rs, sign.rs, client.rs, account.rs,
│            harga.rs, calc.rs, store.rs, sync.rs, arus_kas.rs, periode.rs,
│            posisi.rs, statistik.rs, ws.rs, ws_klien.rs
│            [Fase 9 BARU: screener.rs]
├── ffi/                                 (scalping-ffi, jembatan UniFFI; src/lib.rs)
├── uniffi-bindgen/
├── app/                                 (proyek Gradle Android, modul :aplikasi)
│   └── aplikasi/src/main/
│       ├── AndroidManifest.xml
│       └── kotlin/id/catatanscalping/app/
│           MainActivity, Layar, LayarBeranda, LayarPosisi, LayarRiwayat,
│           LayarSetelan, Cadangan, PenerimaBoot, Tema, Format, Komponen,
│           Latar, Sinkron, GrafikView, LayananLatar, LogDiag, Aplikasi,
│           PenjagaPantauan, KunciAman
│           [Fase 9 BARU: LayarScreener, LayananScreener, (kemungkinan) PembukaIndodax]
├── docs/
│   ├── PRD_Aplikasi_Catatan_Trading_Crypto.md     (v1.8)
│   ├── PANDUAN_RELEASE.md
│   └── prototype/Catatan_trading__prototype_UI.html
└── tools/
    fase0_validasi.py, fase0_akun.py, tes_websocket_token.py, fase2_fiat.py,
    cek_lingkungan.py, bangun_android.sh
    [Fase 9 BARU: verifikasi_api_publik.py (T9.0)]
```

Nama berkas Fase 9 di atas masih rencana; final saat dikerjakan.

---

## 8. Log commit (ringkas)

Hash per task Fase 0 sampai 7 tidak lagi dirinci (lihat `git log --oneline`). Hash yang tercatat sebelumnya: T6.3 `bb0bfa7`, T6.4 `483cfc8`, T6.5 `3db3e8e`.

Menunggu dijalankan pengguna (isi hash dari `git log --oneline`):
- T8.1: `T8.1: log diagnostik permanen dan penjaga pantauan`
- Dokumen: `Dokumen: README, panduan rilis uji, PROGRESS (T8.1 lulus)`
- Tag: `git tag -a v0.8.0-uji -m "Rilis uji 0.8.0-uji"` (setelah semua commit di atas)
- Dokumen: `Dokumen: PRD v1.8 dan PROGRESS (Fase 8 selesai, rancangan Fase 9 Screener)` (hash: ...)
