# PROGRESS: Catatan Scalping

Terakhir diperbarui: 14 November 2026 (uji lapangan Lonjakan berhasil; fitur Harga global exchange luar diputuskan, direncanakan T9.5 dan T9.6; PRD v1.11; jawaban pertanyaan 22 sampai 24 dicatat, PRD v1.12 menyusul)

**Status:** Fase 0 sampai 8 SELESAI dan lulus uji HP. Rilis uji pribadi 0.8.0-uji (versionCode 2) dipakai harian; uji lapangan sampai 14 Nov 2026: tidak ada kendala, aplikasi nyaman dipakai, tidak ada perbaikan yang diperlukan. Jumlah tes terakhir: core 142, ffi 32.

**Sekarang:** Fase 9 (Screener). T9.0, T9.1, T9.2, dan T9.4 SELESAI (tes lulus, uji HP lulus 14 Nov 2026). T9.3 dan notifikasi/`LayananScreener` DIBATALKAN. Uji lapangan Lonjakan (14 Nov 2026, beberapa jam, beberapa setelan): berhasil dan sesuai kriteria; angka final belum dicatat. Fitur baru DIPUTUSKAN 14 Nov: **Harga global** (harga exchange luar di layar Screener), dua tahap: **T9.5** (snapshot: harga wajar, selisih, daftar 3 exchange, status koneksi) lalu **T9.6** (arah harga global). Pertanyaan no. 23 dan 24 dijawab (tiga exchange: Binance, Bybit, OKX; kurs pembanding dari Binance saja); no. 22 (DNS-over-HTTPS) masih usulan. Langkah berikutnya: perbarui PRD ke v1.12 (menyelaraskan FR-38 sampai FR-40 dengan jawaban ini), kirim berkas yang diminta untuk T9.5 (bagian 5), lalu T9.5. Penutup Fase 9 (rilis 0.9.0-uji; tetap cek notifikasi TP/SL normal) setelah T9.6, atau setelah T9.5 bila T9.6 ditunda; pemakaian data Screener tetap dipantau.

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
| 8 (T8.1, T8.2) | T8.1: log diagnostik permanen `diag.log`, tombol Bagikan log, penjaga pantauan (alarm 5 menit). T8.2: ikon aplikasi (commit `1edaa1c`; tag `v0.8.0-uji` menunjuk commit ini; detail tidak dirinci di sini). Rilis uji 0.8.0-uji; panduan `docs/PANDUAN_RELEASE.md` | Selesai (uji lapangan lulus 14 Nov 2026) |

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
- **Fase 9 (14 Nov 2026):** fitur Screener khusus penyaringan, tidak menyentuh perhitungan trading; semua parameter bisa diatur di aplikasi; modal per trade berubah-ubah (tidak ada modal tetap; isian opsional "Modal trade ini" tidak disimpan permanen).
- **Notifikasi lonjakan DIBATALKAN (14 Nov 2026):** pengguna khawatir terlalu banyak notifikasi. FR-35 dan FR-36 dihapus; `LayananScreener`, saluran notifikasi, dan perubahan manifest tidak dibuat; `LayananLatar` tidak disentuh.
- **Lonjakan = filter terpisah di layar Screener (14 Nov 2026):** menjangkau semua pair lewat pemicu snapshot ringkasan pasar (bukan hanya 40 pair teramai). Dasar penilaian: harga sekarang dibanding RATA-RATA HARGA 1 JAM TERAKHIR (bukan harga sebelumnya). Snapshot hanya di memori, tidak disimpan ke database; titik lebih tua dari 60 menit dibuang.
- **Deep link (14 Nov 2026, dicek lewat adb):** tautan web `https://indodax.com/market/<PAIR>` membuka BROWSER, bukan aplikasi Indodax. Opsi (1) FR-37 gugur. **Keputusan pengguna (14 Nov 2026): tombol buka pair TIDAK dibuat** (T9.3 dibatalkan; fallback salin pair juga tidak dikerjakan). Pengguna membuka Indodax sendiri.
- **Uji lapangan Lonjakan (14 Nov 2026):** pengguna mencoba beberapa jam dengan beberapa setelan; fitur berhasil dan sesuai kriteria. Angka default final belum dicatat (isi bila sudah ditetapkan).
- **Harga global (14 Nov 2026):** fitur penting di Screener: harga koin yang sama di exchange luar. Yang ditampilkan: hanya 3 exchange terbesar dan berpengaruh, ditambah harga rata-rata semua exchange (untuk menilai harga wajar) dan daftar harga tiap exchange. Dibagi dua tahap (disetujui pengguna): T9.5 snapshot harga, T9.6 arah harga global. Hanya informasi pembanding, bukan sinyal; tidak menyentuh perhitungan trading. Mengubah non-tujuan PRD "data bursa lain" (PRD v1.11); ide "ikuti pergerakan whale" tetap ditunda.
- **Koneksi exchange luar (14 Nov 2026, hasil uji skrip pengguna):** API exchange luar terblokir hanya di tingkat DNS pada jaringan pengguna; berjalan setelah DNS diganti ke `one.one.one.one` atau `dns.google`. Aplikasi harus punya pengecekan koneksi per exchange (FR-40). Cara penanganan (peringatan + petunjuk Private DNS atau DoH sendiri) belum diputuskan: pertanyaan no. 22.
- **Jawaban pertanyaan Harga global (14 Nov 2026):** (23) pengguna setuju dengan usulan: tampil Binance, Bybit, OKX, dengan median; ditafsirkan ketiganya juga satu-satunya sumber harga wajar (Gate, MEXC, KuCoin tidak diambil); konfirmasi bila yang dimaksud 6 sumber. (24) kurs pembanding USDT/IDR dari Binance saja. (22) pengguna bertanya DoH apa yang cocok dan apakah berpengaruh ke API Indodax; jawaban dan usulan ada di bagian 5 (belum final).

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
| Jenis build APK (debug atau release) dari `bangun_android.sh` | Hasil grep 14 Nov: `buildTypes` ada, `isMinifyEnabled = false` (baris 20), tidak ada `signingConfig`. Nama blok di baris 19 dan perintah Gradle di `bangun_android.sh` belum terlihat, jadi debug atau release belum pasti | Menunggu `sed -n 10,35p app/aplikasi/build.gradle.kts` dan `grep -n "assemble" tools/bangun_android.sh` |
| Pengunduhan data Screener | Ringkasan pasar ±94,7 KB per request; `trades` ±45 KB per pair. Satu pemindaian layar Screener: sampai 40 pair, ±1,8 MB (+ `depth` bila filter kedalaman aktif). Mode Lonjakan (T9.4): satu ringkasan tiap 15 sampai 30 dtk (±12 sampai 23 MB per jam bila tidak terkompresi) + maksimal 15 `trades` per siklus, hanya saat layar terbuka. Gzip di `ureq` tidak diaktifkan; apakah server mengompresi belum diverifikasi | Terbuka; pantau pemakaian data. `MAKS_KANDIDAT` bisa diubah |
| Filter "spread terikat TP" (FR-32) | Belum ada di modul T9.1 sehingga tidak ada di layar Screener | Opsional, terbuka |
| Kedalaman gagal diambil di tahap 2 | Filter kedalaman dilewati untuk pair itu (pair tetap lolos) | Dikenal, diterima |
| Exchange luar terblokir di DNS | Terjadi pada jaringan pengguna (skrip gagal sampai tahap DNS; berhasil setelah DNS diganti ke `one.one.one.one` atau `dns.google`). Belum diuji apakah resolver aplikasi (Rust di Android) mengikuti pengaturan Private DNS HP, dan belum diketahui apakah semua exchange terblokir atau sebagian | Terbuka; pertanyaan no. 22 (usulan: DoH sendiri hanya untuk host exchange luar, belum final); diuji di HP saat T9.5 |
| Ukuran respons dan batas laju exchange luar | Skrip pengguna mengunduh daftar harga SEMUA pair USDT dari 6 exchange (kemungkinan beberapa MB sekali ambil, belum diukur). Endpoint per simbol atau daftar simbol dan batas laju tiap exchange belum diverifikasi | Terbuka; ukur di T9.5; rencana: 3 exchange, hanya kandidat, interval 30 sampai 60 dtk |
| Pembanding kurs USDT/IDR | Harga Indodax dikonversi ke USDT memakai `usdt_idr` Indodax sendiri; bila USDT di Indodax premium atau diskon, selisih semua pair bergeser. Kurs pembanding independen: Binance saja (diputuskan 14 Nov 2026) | Sebagian terjawab: endpoint Binance USDTIDR diuji di T9.5; kurs mana yang dipakai untuk harga wajar masih usulan |

## 5. Fase 9: Screener (rancangan dan hasil verifikasi, 14 Nov 2026)

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

**Lonjakan (filter di layar Screener; notifikasi dibatalkan 14 Nov 2026)**
- Harga dasar = rata-rata `last` dari snapshot ringkasan pasar dalam 60 menit terakhir, berbobot waktu (usulan). Melonjak bila |harga sekarang - dasar| ÷ dasar ≥ X%; ▲ naik dan ▼ turun punya saklar terpisah. Tahap 2 dari `trades`: trx/mnt jendela pendek (bawaan 3 menit) ≥ M (usulan; apakah juga N kali dasar 10 menit: pertanyaan no. 21). Syarat dasar (spread, usia transaksi) tetap berlaku.
- Pemicu: satu request ringkasan tiap 15 sampai 30 dtk (bisa diatur) hanya saat layar Screener terbuka dan mode Lonjakan aktif; pair yang sudah melewati separuh X diperiksa lewat `trades`, maksimal 15 pair per siklus. Rust tetap murni: snapshot dikirim dari Kotlin sebagai masukan; Rust yang menghitung dasar, pemicu, dan arah (Decimal).
- Masa pemanasan: pair belum dinilai sebelum ada snapshot minimal sekitar 10 menit (usulan); dasar memakai data yang ada sampai 60 menit. Ukuran memori kecil (satu titik per pair per polling, atau satu titik per menit).

**Layar dan tombol**
- Layar Screener: satu daftar (pair, harga, trx/mnt, spread, gerak 10m, buy%), tanpa teks penjelas; penanda untuk pair yang sedang dipegang. Dipasang sebagai tab ke-5 di T9.2 (indeks 4; Setelan tetap indeks 3) dan lulus uji HP.
- Tombol buka pair di Indodax: DIBATALKAN (14 Nov 2026). Hasil cek adb: tautan web membuka browser, bukan aplikasi Indodax, dan pengguna memutuskan tombol tidak diperlukan. Baris di layar Screener hanya informasi.

**Hasil T9.0 (14 Nov 2026, Sabtu pagi, pasar relatif sepi; skrip `tools/verifikasi_api_publik.py`, data mentah `data/verif_publik_20261114_093323.json`, tidak di-commit)**
- `/api/summaries`: 438 pair IDR, ±94,7 KB, ±0,2 dtk. Kunci atas `tickers`, `prices_24h`, `prices_7d`. Ticker IDR: `buy` (bid), `sell` (ask), `high`, `low` (24 jam), `last`, `name`, `server_time`, `vol_idr`, plus satu `vol_<koin>` (tidak dipakai). Semua nilai string. `server_time` global (sama di semua pair), berubah di 16 dari 17 polling selang 5 dtk, jadi ringkasan segar tiap beberapa detik. Kunci `tickers` pakai `btc_idr`, `prices_24h` pakai `btcidr` (tanpa garis bawah): parser harus memetakan. `last` sama dengan harga transaksi terbaru di `trades`.
- `/api/trades/<pair>`: selalu 500 transaksi, terbaru dulu; item `tid`, `type` (buy/sell), `price`, `amount`, `date` (epoch dtk), semua string. Jendela 10 menit terjangkau di 5 pair uji (rentang 122 sampai 20.800 menit). Pair sangat ramai (lebih dari 500 transaksi dalam 10 menit) BELUM teruji; aturan skrip (jendela diperpendek, minimal 1 menit) tetap dipakai.
- `/api/depth/<pair>`: kunci `buy`, `sell`; maks 150 level per sisi; level `[harga, jumlah]` (string); sisi buy menurun. Sisi sepi bisa kurang dari 150 level dan jangkauan harganya lebar (sampai ratusan persen), jadi kedalaman dalam 0,5% dihitung dengan memotong level. Contoh `fil_idr`: bid dalam 0,5% hanya Rp 196 ribu, dalam 1% Rp 11 juta.
- `vol_idr`: kemungkinan besar volume 24 jam bergulir (teramati turun 1 kali dari 7 perubahan di usdt_idr; selisih Rp 74,0 juta vs transaksi baru Rp 97,7 juta). Dari satu sesi. Selisih `vol_idr` antar snapshot TIDAK dipakai sebagai trx/mnt; trx/mnt hanya dari `trades`.
- Latensi rata-rata ringkasan 0,196 dtk; polling 10 sampai 15 dtk aman dari sisi kecepatan.
- Deep link: lihat bagian 3 (membuka browser; tombol buka pair dibatalkan).

**Usulan desain dari hasil T9.0 (belum dikonfirmasi pengguna)**
1. Pemicu lonjakan tahap 1 (diperbarui 14 Nov): bandingkan `last` sekarang dengan harga dasar 1 jam dari snapshot di memori; pair yang melewati separuh ambang X baru diperiksa lewat `trades`. `vol_idr` bukan pemicu.
2. Usia transaksi dihitung dari `server_time` ringkasan, bukan jam HP.
3. Kedalaman: nilai = harga x jumlah (jumlah dianggap koin, BELUM dikonfirmasi), dihitung dari best bid dan best ask sejauh 0,5%.
4. Interval polling bawaan 15 sampai 30 dtk dan bisa diatur: 94,7 KB tiap 15 dtk = ±23 MB per jam bila tidak terkompresi.

**Masih terbuka setelah T9.0**
- Arti `jumlah` di depth (koin atau Rupiah): bandingkan best bid usdt_idr (18.266, jumlah 63.997,62) dengan order book di aplikasi Indodax.
- Ulangi skrip di jam ramai (hari kerja sore), terutama untuk pair sangat ramai dan arah `vol_idr`.
- Apakah respons `summaries` dikompresi gzip (header `Content-Encoding`).
- WebSocket publik Indodax: tidak diuji, tidak diasumsikan ada.

**Rencana task (tiap task satu commit; PRD dan PROGRESS diperbarui tiap task)**
| Task | Isi | Catatan |
| --- | --- | --- |
| T9.0 | Skrip Python kecil di `tools/` untuk memverifikasi API publik dan cek deep link di HP | **Selesai 14 Nov 2026** (hasil di atas; deep link web gugur). Dicommit |
| T9.1 | `core/src/screener.rs`: parser ringkasan pasar, transaksi, kedalaman; filter dua tahap; skor; Decimal; tanpa UI | **Selesai 14 Nov 2026** (core 119 tes lulus). Lihat catatan T9.1 di bawah |
| T9.2 | FFI + layar Screener + setelan ambang (tabel `meta`) + isian "Modal trade ini" | **Selesai 14 Nov 2026** (core 125 tes, ffi 29 tes lulus; uji HP lulus). Lihat catatan T9.2 di bawah |
| T9.3 | Tombol buka pair di Indodax | **DIBATALKAN 14 Nov 2026** (tautan web membuka browser; pengguna tidak memerlukan tombol ini). Tidak ada kode; nomor T9.3 tidak dipakai ulang |
| T9.4 | **Selesai 14 Nov 2026** (core 142 tes, ffi 32 tes lulus; uji HP lulus). Filter Lonjakan di layar Screener (versi baru): Rust: snapshot harga sebagai masukan, harga dasar rata-rata 60 menit, pemicu, tahap 2 dari `trades`, arah ▲/▼; FFI: fungsi baru di `screener_ffi.rs`; Kotlin: sakelar Lonjakan, polling ringkasan 15 sampai 30 dtk saat layar terbuka, ambang X/M di kartu "Ambang". TANPA `LayananScreener`, manifest, atau notifikasi | Butuh `screener.rs`, `screener_ffi.rs`, `LayarScreener.kt` terbaru; jawab pertanyaan no. 21 dulu |
| T9.5 | **Rencana (14 Nov 2026).** Harga global tahap 1: `core/src/harga_global.rs` (murni: parser tiap exchange, median, konversi IDR, selisih, label kualitas), pengambilan data mentah di `client.rs`, setelan di tabel `meta`, FFI, tampilan di layar Screener (harga wajar, selisih, 3 exchange, status koneksi). TANPA layanan, notifikasi, atau perubahan manifest | Butuh `client.rs`, `screener.rs`, `screener_setelan.rs`, `lib.rs`, `screener_ffi.rs`, `LayarScreener.kt` terbaru; selesaikan no. 22 dan sisa no. 24 dulu (no. 23 terjawab) |
| T9.6 | **Rencana.** Harga global tahap 2: arah harga global (perubahan N menit terakhir dibanding Indodax) dari snapshot di memori, memakai pola `lonjakan.rs` | Butuh T9.5 selesai dan teruji, serta `lonjakan.rs` terbaru |
| Penutup | PRD bila ada perubahan; angka final lonjakan dan harga global dari uji lapangan; rilis 0.9.0-uji | Tidak perlu uji bersamaan dengan pantauan order karena tidak ada layanan baru; tetap cek notifikasi TP/SL normal |

**Catatan T9.1 (selesai 14 Nov 2026)**
- Modul `core/src/screener.rs` MURNI: tanpa jaringan dan tanpa database; hanya mengurai teks JSON yang diberikan pemanggil. Pengambilan data publik (summaries, trades, depth) ditambahkan di T9.2 pada sisi klien (`client.rs`, selesai).
- API (diekspor dari `lib.rs`): `parse_ringkasan`, `parse_transaksi`, `parse_kedalaman`, `saring_tahap1`, `nilai_tahap2`, `hitung_metrik`, `hitung_spread`, `urutkan`, `pair_path`; tipe `KonfigScreener`, `UrutanScreener`, `RingkasanPasar`, `KandidatScreener`, `TransaksiPublik`, `JenisTransaksi`, `MetrikPair`, `KedalamanPasar`, `BarisScreener`.
- `KonfigScreener::default()` = nilai skrip Python (harga 10 sampai 70.000; volume 10 juta sampai 10 miliar; trx/mnt >= 2 jendela 10 menit; spread <= 10%; range 24 jam >= 4%; usia transaksi <= 120 dtk). Filter tambahan (gerak, menit aktif, arah, kedalaman) MATI bawaan; tanda dominasi 40% aktif (hanya peringatan); biaya round-trip 0,41%; urutan bawaan skor (bisa trx/mnt seperti skrip).
- Aturan jendela trx/mnt sama dengan skrip (jendela diperpendek, minimal 60 dtk). Usia transaksi dihitung dari `server_time` ringkasan, bukan jam HP.
- Kedalaman: nilai = harga x jumlah koin (jumlah di depth dikonfirmasi pengguna = KOIN), dihitung dari best bid dan best ask sejauh `kedalaman_persen` (bawaan 0,5). Filter aktif hanya bila kelipatan diatur, "Modal trade ini" diisi positif, dan data kedalaman ada; syarat: sisi beli DAN sisi jual masing-masing >= kelipatan x modal.
- Pembagian memakai `checked_div` (tanpa panik). Lonjakan (FR-34) BELUM ada; dikerjakan di T9.4.
- Batasan tes: data nyata di `verif_publik_*.json` hanya 2 contoh transaksi per pair, jadi tes memakai potongan nyata `usdt_idr` ditambah data rekaan. Parity penuh dengan skrip Python belum diuji; bandingkan di T9.2 dengan data langsung.
- Sisi fungsi: `saring_tahap1` dan `nilai_tahap2` dipanggil dari `ffi/src/screener_ffi.rs` sejak T9.2.

**Catatan T9.2 (selesai 14 Nov 2026)**
- `core/src/client.rs`: `get_publik` + `ambil_ringkasan_mentah`, `ambil_transaksi_mentah`, `ambil_kedalaman_mentah` (JSON mentah; diurai `screener.rs`). Pair divalidasi (huruf kecil dan angka setelah `pair_path`). `ambil_harga_idr` kini lewat `get_publik`, perilaku sama.
- `core/src/screener_setelan.rs` (baru): ambang di tabel `meta`, kunci `screener_<nama>`. Kunci belum ada = bawaan; nilai kosong = filter mati; nilai rusak = bawaan. `periksa_konfig` menolak min > maks, jendela di luar 1..60, jarak kedalaman <= 0, dan nilai negatif. Penyimpanan tidak atomik (satu `atur_meta` per kunci); risiko kecil.
- `ffi/src/screener_ffi.rs` (modul baru; `ffi/src/lib.rs` hanya ditambah `mod screener_ffi;`): `setelan_screener_bawaan`, `baca_setelan_screener`, `simpan_setelan_screener`, `jalankan_screener(path_db, modal_trade)`.
- Pemindaian: ringkasan -> tahap 1 -> maksimal 40 kandidat (urut volume 24 jam menurun) -> `trades` dengan 4 pekerja paralel (+ `depth` bila kelipatan kedalaman diatur DAN modal diisi) -> tahap 2 -> urut. Pair gagal dihitung, tidak menggagalkan daftar. Penanda "Dipegang" dari posisi terbuka (hanya baca).
- Kotlin: `LayarScreener.kt` sebagai tab ke-5; `MainActivity.kt` hanya berubah 3 baris (nama tab, daftar layar, komentar). Penyegaran 45 dtk hanya saat layar terlihat; jeda minimal 40 dtk (otomatis) dan 10 dtk (tombol Segarkan). Baris dua tingkat: nama, penanda (⚠ dominasi, Dipegang), harga; lalu trx/mnt, spread, gerak, buy. Kartu "Ambang" tersembunyi sampai diketuk. Layar jalan tanpa kunci API. Mengetuk baris tidak melakukan apa pun (tombol buka pair dibatalkan); komentar kelas di `LayarScreener.kt` masih menyebut T9.3, rapikan saat berkas itu disentuh lagi.
- `tools/terapkan_t9_2.sh` hanya alat pemasangan sekali pakai (menambah baris `mod` dan ekspor); tidak perlu disimpan di repo bila tidak mau.
- Tes: core 119 -> 125 (+5 setelan, +1 client), ffi 24 -> 29 (+5).
- Belum ada: filter spread terikat TP (FR-32), filter Lonjakan (T9.4, tanpa layanan). Buka pair di Indodax dibatalkan.

**Catatan T9.4 (selesai 14 Nov 2026; tes, build, dan uji HP lulus)**
- Keputusan pertanyaan no. 21: snapshot tetap di memori selama proses aplikasi hidup (statik di `ffi`, bukan di Kotlin; `core` tetap murni); syarat trx/mnt tahap 2 = M absolut saja (tanpa perbandingan N kali dasar 10 menit).
- Berkas baru: `core/src/lonjakan.rs` (murni: `KonfigLonjakan`, `RiwayatHarga`, `cari_pemicu`, `nilai_lonjakan`, `urutkan_lonjakan`; 12 tes) dan `core/src/lonjakan_setelan.rs` (ambang di tabel `meta`, kunci `lonjakan_<nama>`; 5 tes). Berkas diubah: `core/src/lib.rs` (2 `mod` dan ekspor), `ffi/src/screener_ffi.rs` (tambahan di akhir bagian fungsi, +3 tes), `LayarScreener.kt`. `screener.rs` dan `screener_setelan.rs` TIDAK diubah; `ffi/src/lib.rs`, `MainActivity.kt`, manifest, dan `LayananLatar.kt` tidak disentuh.
- Harga dasar = rata-rata berbobot waktu (trapesium) antar dua titik berurutan dalam `dasar_menit` terakhir; selisih dua titik > 180 dtk dianggap celah dan TIDAK dihitung. Titik sekarang tidak ikut dasar. Pair dinilai setelah data tercakup >= `pemanasan_menit`.
- Pemicu: kandidat tahap 1 (harga, volume, spread; range 24 jam dimatikan) dengan penyimpangan >= separuh X, saklar naik/turun, urut penyimpangan terbesar, dipotong `maks_periksa`. Tahap 2: `trades` jendela pendek, trx/mnt >= M, usia transaksi, lalu X penuh. Filter tambahan (gerak, menit aktif, arah, kedalaman) dimatikan untuk Lonjakan.
- Angka awal SEMENTARA (tebakan, tentukan dari uji lapangan): X 2%, M 3 trx/mnt, jendela 3 mnt, dasar 60 mnt, pemanasan 10 mnt, maks 15 pair, interval 20 dtk. Batas valid: dasar 5 sampai 60, interval 10 sampai 120 dtk, maks 1 sampai 40.
- UI: tombol "Mode: Daftar/Lonjakan" (tidak disimpan; bawaan Daftar). Mode Lonjakan memolling hanya saat layar terlihat; mode Daftar tidak mengumpulkan snapshot. Bagian "Lonjakan" di kartu Ambang; "Simpan ambang" menyimpan keduanya.
- Pemakaian data: `trades` diambil ulang tiap siklus untuk pair pemicu (maks 15 x ±45 KB) di samping ringkasan ±95 KB; belum dicek apakah perlu cache. Pantau.
- Tes: core 125 -> 142 (+12 lonjakan, +5 setelan), ffi 29 -> 32 (+3). Uji HP lulus sesuai kriteria (mode Lonjakan, status pemanasan, baris muncul saat X kecil).
- Berikutnya: uji lapangan angka default; pertimbangkan cache `trades` bila pemakaian data besar.

**Harga global (rancangan, 14 Nov 2026; bagian "usulan" belum final)**
- Acuan: skrip pengguna `harga_wajar_indodax.py` (belum ada di repo; salin ke `tools/` bila ingin disimpan). Alurnya: `summaries` -> filter harga dan volume -> `trades` untuk trx/mnt -> harga USDT dari 6 exchange (Tier 1: Binance, Bybit, OKX; Tier 2: Gate, MEXC, KuCoin) lewat endpoint daftar harga semua pair -> harga Indodax dikonversi ke USDT dengan `usdt_idr` -> median, selisih, label kualitas, CSV. Skrip memakai `last` dan berjalan di jaringan pengguna setelah DNS diganti.
- Hasil diskusi (penilaian skrip): cukup sebagai tahap awal untuk menilai harga wajar, BELUM cukup untuk scalping. Kekurangan, urut kepentingan: (1) hanya satu foto selisih, tanpa arah (global bergerak dulu, Indodax menyusul) -> T9.6; (2) kurs `usdt_idr` Indodax sendiri bisa bias -> pembanding kurs dari Binance (pair USDTIDR); koreksi: usulan awal "selisih dalam USDT" ternyata identik dengan selisih IDR bila memakai kurs Indodax yang sama (harga Indodax ÷ kurs ÷ median = harga Indodax ÷ (median × kurs)), jadi tidak ditampilkan; hanya kurs independen yang menghilangkan bias; (3) `last` tanpa waktu bisa basi di exchange sepi -> usulan harga tengah bid-ask atau bobot lebih rendah; (4) selisih punya bias struktural (fee dan spread) -> beri konteks terhadap biaya round-trip 0,41%; (5) ticker sama bisa koin berbeda atau token berskala -> tanda peringatan bila selisih terlalu besar (usulan > 10%).
- Tahap 1 (T9.5): harga wajar = median harga dari semua exchange sumber yang punya koin itu x kurs `usdt_idr`; selisih Indodax terhadap harga wajar (%); daftar harga 3 exchange terbesar dan berpengaruh (IDR dan selisih); label kualitas (dengan 3 sumber yang semuanya Tier 1 di skrip; usulan: tinggi bila minimal 2 dari 3 punya koin, rendah bila hanya 1, rendah (sebaran tinggi) bila sebaran antar sumber > 2%, tanpa pembanding bila 0); satu baris kurs: `usdt_idr` Indodax dibanding USDTIDR Binance beserta selisihnya (usulan: harga wajar tetap memakai kurs Indodax; bisa diubah); status koneksi per exchange. Diputuskan 14 Nov (no. 23, "setuju"): Binance, Bybit, OKX; median. Catatan: pencarian web 14 Nov 2026 menunjukkan Binance menyediakan kuotasi USDT/IDR; skrip pengguna hanya mengambil simbol berakhiran USDT, jadi USDTIDR perlu permintaan simbol terpisah dan endpoint API-nya belum diuji.
- Tahap 2 (T9.6): perubahan harga global dalam N menit terakhir (usulan 10 menit) dibanding perubahan harga Indodax periode yang sama, dari snapshot di memori seperti Lonjakan (tidak masuk database). Angka N dan ambang dari uji lapangan.
- Arsitektur (usulan): `client.rs` mengambil JSON mentah (tanpa key, tanpa data akun); `core/src/harga_global.rs` murni dan Decimal; ambang dan daftar exchange di tabel `meta` (kunci `global_<nama>`); FFI di `screener_ffi.rs` atau modul FFI baru (diputuskan setelah membaca berkas); Kotlin di `LayarScreener.kt`. Tanpa layanan latar, notifikasi, atau perubahan manifest. Pengambilan hanya untuk kandidat di daftar (maksimal 40 pair) dan hanya saat layar Screener terbuka, interval minimal 30 sampai 60 dtk; gagal satu exchange tidak menggagalkan daftar.
- Koneksi (FR-40): exchange yang tidak terjangkau dilewati dan statusnya ditampilkan ringkas; sisanya tetap dipakai dan label kualitas turun. Penanganan DNS (usulan 14 Nov 2026, menjawab pertanyaan pengguna tentang DoH; belum final): aplikasi memakai DNS-over-HTTPS sendiri HANYA untuk host exchange luar (Binance, Bybit, OKX); host Indodax tetap memakai resolver sistem, jadi API Indodax tidak terpengaruh. Penyedia: Cloudflare utama (bentuk JSON `application/dns-json`, tanpa parser paket DNS biner), Google cadangan; terhubung lewat alamat IP langsung (1.1.1.1 atau 8.8.8.8) agar tidak butuh DNS biasa, nama host sertifikat tetap diverifikasi; hasil disimpan sesuai TTL. Bila DoH gagal: tampilkan status dan petunjuk Private DNS Android (`dns.google` atau `one.one.one.one`) sebagai cadangan manual. Belum diuji: (a) apakah DoH lewat port 443 ke alamat itu tidak terblokir di jaringan pengguna (yang terbukti baru resolver yang diganti lewat Private DNS), (b) apakah versi `ureq` di `Cargo.toml` mendukung resolver kustom, (c) apakah resolver aplikasi mengikuti Private DNS.
- Berkas yang diminta untuk T9.5 (jangan menebak isinya): `core/src/client.rs`, `core/src/screener.rs`, `core/src/screener_setelan.rs`, `core/src/lib.rs`, `ffi/src/screener_ffi.rs`, `app/aplikasi/src/main/kotlin/id/catatanscalping/app/LayarScreener.kt`. Untuk T9.6 tambah `core/src/lonjakan.rs`. `core/Cargo.toml` (wajib bila DoH sendiri dikerjakan: cek versi dan fitur `ureq`).

**Berkas yang diminta sebelum T9.4 (diperbarui 14 Nov; jangan menebak isinya):** `core/src/screener.rs`, `ffi/src/screener_ffi.rs`, `app/aplikasi/src/main/kotlin/id/catatanscalping/app/LayarScreener.kt`. Manifest, `LayananLatar.kt`, `PenerimaBoot.kt`, dan `Aplikasi.kt` tidak lagi diperlukan untuk T9.4 (tanpa layanan dan notifikasi).

**Pertanyaan terbuka Fase 9:** (21) TERJAWAB 14 Nov 2026 (keputusan terbaik, bisa diubah): snapshot tetap di memori selama proses hidup; tahap 2 pakai M absolut saja. Angka default lonjakan (X, M, jendela, interval polling, minimal pemanasan, maks pair per siklus) ditentukan dari uji lapangan; WebSocket publik tidak diasumsikan. Terjawab: tab ke-5, arti `jumlah` depth = KOIN, deep link membuka browser (tombol buka pair dibatalkan), notifikasi dan layanan terpisah dibatalkan. PRD v1.9 sudah memperbarui bagian 16 no. 15 sampai 20 dan menghapus FR-35, FR-36, FR-37.

**Pertanyaan Harga global (14 Nov 2026; PRD v1.11 bagian 16, no. 22 sampai 24):** (22) DNS: pengguna bertanya DoH apa dan dampaknya ke Indodax; usulan DoH hanya untuk host exchange luar (lihat rancangan); BELUM final, menunggu konfirmasi. (23) TERJAWAB ("setuju"): Binance, Bybit, OKX, median; ditafsirkan ketiganya juga satu-satunya sumber harga wajar. (24) TERJAWAB sebagian: kurs pembanding dari Binance saja; tersisa: harga wajar memakai kurs Indodax atau Binance (usulan: kurs Indodax, ditambah satu baris selisih kurs) dan ambang tanda selisih kurs. Selesaikan no. 22 dan sisa no. 24 sebelum T9.5.

---

## 6. Cara instalasi dan tes

Kebutuhan: git, Rust (rustup), toolchain Android (lihat bagian 2); Python 3.8+ hanya untuk skrip di `tools/` (tanpa `pip install`).

1. `cd catatan-scalping`
2. Key di indodax.com/trade_api: **TAPI v2**, izin **baca saja**, tanpa pembatasan IP.
3. Salin `.env.example` menjadi `.env`, isi `INDODAX_API_KEY` dan `INDODAX_API_SECRET` (untuk skrip dan contoh Rust di komputer; aplikasi Android menyimpan key sendiri di HP).

```
cargo test -p scalping-core        # 142 tes
cargo test -p scalping-ffi         # 32 tes
bash tools/bangun_android.sh --pasang <serial-adb>
```

Uji di HP: tab Setelan, simpan kunci, isi TP/SL, "Sinkron sekarang", mulai pantauan (matikan optimasi baterai, izinkan notifikasi), lakukan transaksi kecil di Indodax: notifikasi beli/jual muncul dan Beranda, Posisi, Riwayat menyegarkan diri. Ekspor cadangan dari Setelan sebelum memasang pembaruan.

Skrip bantu di `tools/`: `fase0_validasi.py`, `fase0_akun.py`, `tes_websocket_token.py`, `fase2_fiat.py`, `cek_lingkungan.py`, `verifikasi_api_publik.py`, `bangun_android.sh`; contoh Rust: `cek_data`, `cek_koneksi`, `pantau`.

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
│            [Fase 9: screener.rs (T9.1), screener_setelan.rs (T9.2), lonjakan.rs dan lonjakan_setelan.rs (T9.4, selesai); harga_global.rs (T9.5, rencana)]
├── ffi/                                 (scalping-ffi, jembatan UniFFI; src/lib.rs,
│                                         src/screener_ffi.rs [T9.2])
├── uniffi-bindgen/
├── app/                                 (proyek Gradle Android, modul :aplikasi)
│   └── aplikasi/src/main/
│       ├── AndroidManifest.xml
│       └── kotlin/id/catatanscalping/app/
│           MainActivity, Layar, LayarBeranda, LayarPosisi, LayarRiwayat,
│           LayarSetelan, Cadangan, PenerimaBoot, Tema, Format, Komponen,
│           Latar, Sinkron, GrafikView, LayananLatar, LogDiag, Aplikasi,
│           PenjagaPantauan, KunciAman
│           [Fase 9: LayarScreener (T9.2, selesai); T9.4 mengubah file ini, tanpa layanan baru]
├── docs/
│   ├── PRD_Aplikasi_Catatan_Trading_Crypto.md     (v1.11)
│   ├── PANDUAN_RELEASE.md
│   └── prototype/Catatan_trading__prototype_UI.html
└── tools/
    fase0_validasi.py, fase0_akun.py, tes_websocket_token.py, fase2_fiat.py,
    cek_lingkungan.py, bangun_android.sh
    [Fase 9: verifikasi_api_publik.py (T9.0, selesai)]
```

Tidak ada berkas layanan baru di Fase 9 (`LayananScreener` dibatalkan 14 Nov 2026).

---

## 8. Log commit (ringkas)

Hash per task Fase 0 sampai 7 tidak lagi dirinci (lihat `git log --oneline`). Hash yang tercatat sebelumnya: T6.3 `bb0bfa7`, T6.4 `483cfc8`, T6.5 `3db3e8e`.

Terkonfirmasi dari `git log` (14 Nov 2026):
- T8.1: `T8.1: log diagnostik permanen dan penjaga pantauan` (hash: `5679387`)
- T8.2: `T8.2: ikon aplikasi` (hash: `1edaa1c`, bertanda tag `v0.8.0-uji`)
- Dokumen: `Dokumen: PRD v1.8 dan PROGRESS (Fase 8 selesai, rancangan Fase 9 Screener)` (hash: `7b83716`)
- T9.0: verifikasi API publik Indodax (hash: isi dari `git log --oneline -1`)
- T9.1: modul screener (hash: isi dari `git log --oneline -1`)
- T9.2: FFI, layar Screener, setelan ambang (hash: isi dari `git log --oneline -1`)
- T9.4: filter Lonjakan (hash: isi dari `git log --oneline -1` setelah commit)
- Dokumen: PRD v1.9 dan PROGRESS (notifikasi lonjakan dibatalkan; T9.4 jadi filter Lonjakan, dasar rata-rata 1 jam) (hash: isi dari `git log --oneline -1`)
- Dokumen: PRD v1.11 dan PROGRESS (fitur Harga global exchange luar; T9.5 dan T9.6 direncanakan) (hash: isi dari `git log --oneline -1` setelah commit)
- Dokumen: PROGRESS (jawaban pertanyaan 22 sampai 24 Harga global; PRD v1.12 menyusul) (hash: isi dari `git log --oneline -1` setelah commit)

Titik rollback sebelum Fase 9: tag `v0.8.0-uji`.
