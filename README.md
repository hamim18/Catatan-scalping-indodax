# Catatan Scalping

Aplikasi Android pribadi untuk mencatat trading crypto di Indodax (pasangan IDR) secara otomatis. Aplikasi **hanya membaca data dan menghitung**. Ia tidak pernah membuat order, membatalkan order, atau memindahkan dana (kunci API wajib izin baca saja).

**Versi:** 0.9.0-uji (build release, rilis uji pribadi, dipakai harian untuk mencari bug dan peningkatan)
**Status:** semua fase rencana selesai (Fase 0 sampai 10, termasuk Screener dan polish tampilan) dan lulus uji HP; sedang uji lapangan. Detail di `PROGRESS.md`.

## Fitur

- Deteksi order beli dan jual realtime lewat WebSocket privat Indodax (REST sebagai sumber angka resmi), berjalan di layanan latar.
- Notifikasi setelah beli: modal efektif, TP, SL, dan estimasi hasil bersih. Notifikasi setelah jual: hasil bersih (Rp dan %) dan progress target.
- Navigasi 4 tab: Beranda, Posisi, Riwayat, Screener. Setelan dan Panduan/FAQ lewat menu ☰ di pojok kanan atas.
- Beranda: Total aset (IDR sebagai "Buying power" + koin), profit trading, grafik akumulasi profit, target harian dan mingguan.
- Posisi terbuka dengan tangga SL, modal efektif, TP, harga sekarang, dan floating P/L.
- Riwayat: statistik (win rate, rata-rata, fee), riwayat deposit/withdraw IDR, dan trade tertutup dengan harga beli dan jual per koin.
- Screener (data pasar publik, tanpa key, tanpa notifikasi, hanya informasi): daftar pair IDR yang lolos kriteria buatan sendiri, filter Lonjakan (harga menyimpang dari rata-rata 1 jam), dan pembanding harga dengan exchange luar (Binance, Bybit, OKX): harga wajar, selisih, dan arah harga global. Semua ambang bisa diatur di aplikasi.
- Cadangan data lokal (ekspor dan impor database; kunci API tidak ikut).
- Diagnostik uji lapangan: log permanen `diag.log`, tombol "Bagikan log", dan peringatan bila pantauan mati atau terputus lebih dari 5 menit.
- Saat aplikasi dibuka dan pantauan tidak berjalan, trade disinkron otomatis.

## Teknologi

Rust untuk semua logika (klien Indodax, perhitungan, Screener, SQLite), Kotlin hanya klien Android (tampilan, layanan latar, notifikasi, kunci terenkripsi). Jembatan: UniFFI 0.31. Tanpa server dan tanpa akun cloud.

## Dokumen

- Konsep: `docs/PRD_Aplikasi_Catatan_Trading_Crypto.md` (v1.12)
- Tampilan awal (prototype, sebelum Fase 9 dan 10): buka `docs/prototype/Catatan_trading__prototype_UI.html` di browser
- Status kerja dan riwayat keputusan: `PROGRESS.md`
- Panduan rilis uji (pasang, uji lapangan, lapor bug, rollback): `docs/PANDUAN_RELEASE.md`

## Kebutuhan

Rust (rustup) dengan target Android dan `cargo ndk`, JDK 17, Gradle, Android SDK (build-tools 36.0.0) dan NDK, HP Android 11 ke atas (minSdk 30) dengan adb. Python 3.8+ hanya untuk skrip di `tools/`. Rincian versi ada di `PROGRESS.md` bagian 2 dan 3.

## Menyiapkan API key

1. Buat key di indodax.com/trade_api: **TAPI v2**, izin **BACA SAJA**, tanpa pembatasan IP.
2. Untuk skrip dan contoh di komputer: salin `.env.example` menjadi `.env`, isi `INDODAX_API_KEY` dan `INDODAX_API_SECRET`. Jangan pernah commit `.env`.
3. Di aplikasi Android, kunci dimasukkan lewat menu ☰ → Setelan dan disimpan terenkripsi di HP. Kunci yang punya izin trade atau withdraw ditolak. Screener tidak memakai kunci.

## Membangun dan memasang

Selalu lewat skrip (Gradle langsung tidak memperbarui binding UniFFI). Sejak 0.9.0-uji skrip membangun APK **release**, ditandatangani dengan kunci debug bawaan AGP (sama dengan APK 0.8.0-uji) supaya bisa menimpa versi lama tanpa uninstall:

```
cd catatan-scalping
touch core/src/*.rs ffi/src/lib.rs app/aplikasi/src/main/kotlin/id/catatanscalping/app/*.kt
bash tools/bangun_android.sh --pasang <serial-adb>
```

- Serial dari `adb devices` (untuk adb nirkabel pakai serial tanpa spasi).
- Skrip mencetak sidik jari tanda tangan APK (SHA-256). Bila berbeda dari APK yang terpasang, Android menolak menimpa; lihat `docs/PANDUAN_RELEASE.md` bagian 3.
- Pasang ulang lewat adb mematikan pantauan: nyalakan lagi manual.
- Jangan uninstall untuk memperbarui: data lokal hilang. Ekspor cadangan dulu dari menu ☰ → Setelan.

Pemakaian pertama di HP: menu ☰ → Setelan, simpan kunci, isi TP/SL dan target, "Sinkron sekarang" satu kali, lalu nyalakan pantauan (izinkan notifikasi, matikan optimasi baterai untuk aplikasi ini).

## Tes

```
cargo test -p scalping-core    # 166 tes
cargo test -p scalping-ffi     # 43 tes
```

Pastikan tes baru benar-benar ikut jalan (jumlah tes bertambah, filter nama menemukan tesnya). Setelah mengganti file dari unduhan, `touch` dulu atau Cargo bisa memakai build lama.

## Skrip bantu (`tools/`)

- `fase0_validasi.py`: tarik trade dengan key baca saja dan cocokkan dengan struk (`python tools/fase0_validasi.py icntidr --hari 3`).
- `tes_websocket_token.py`, `fase2_fiat.py`, `fase0_akun.py`, `cek_lingkungan.py`: uji manual dan laporan lingkungan.
- `verifikasi_api_publik.py`: verifikasi API publik Indodax untuk Screener.
- `uji_koneksi_global.sh`: uji jangkauan dan ukuran data API exchange luar (opsional, dari Termux).
- `bangun_android.sh`: Rust, binding, APK release, adb.

## Cadangan dan data

Database ada di penyimpanan privat aplikasi. Ekspor dan impor lewat menu ☰ → Setelan, kartu "Cadangan data". Impor hanya bisa saat pantauan mati dan tidak ada sinkron berjalan. Setelah impor di HP baru: masukkan kunci, "Sinkron sekarang" sekali, baru nyalakan pantauan. Riwayat hanya sejak aplikasi dipasang; instal ulang tanpa impor mengosongkan profit tertutup, target, dan grafik. Hasil Screener, snapshot harga, dan harga exchange luar hanya di memori dan tidak ikut disimpan.

## Keterbatasan yang diketahui

- Withdraw nyata belum teruji (bentuk baris dan mode fee).
- Dua order beli koin sama yang selesai nyaris bersamaan bisa menampilkan notifikasi order yang sama.
- Estimasi hasil bersih memakai fee jual perkiraan 0,31%; jual market bisa sekitar 0,61%. Hasil nyata selalu dari `commission` API.
- Koin yang dipegang sebelum aplikasi dipasang dan dijual kemudian tercatat sebagai penyesuaian (profit 0).
- Screener: pemakaian data seluler belum diukur (layar terbuka memolling terus); filter "spread terikat TP" belum ada; arah harga global baru muncul sekitar N menit setelah layar dibuka.
- Exchange luar bisa terblokir DNS di sebagian jaringan: aplikasi menampilkan statusnya; atasi dengan Private DNS Android (`dns.google`).
- Status pantauan di Setelan bisa menampilkan "Berjalan" sesaat setelah layanan dimatikan sistem.
- Hanya pasangan IDR di Indodax, satu pengguna, satu akun.

## Melaporkan bug

Menu ☰ → Setelan, ketuk "Log pantauan", lalu "Bagikan log" sesegera mungkin setelah kejadian, dan sertakan langkah, hasil, dan harapan. Format lengkap di `docs/PANDUAN_RELEASE.md`. Jangan kirim kunci API atau secret.
