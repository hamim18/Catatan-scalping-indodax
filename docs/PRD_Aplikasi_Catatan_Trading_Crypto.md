# PRD: Aplikasi Catatan Trading Crypto

Versi 1.9 · 14 November 2026 (Fase 9 sebagian jalan: T9.0 sampai T9.2 selesai. Notifikasi lonjakan, layanan terpisah, dan tombol buka pair dibatalkan; lonjakan menjadi filter terpisah di layar Screener, dinilai dari harga sekarang dibanding rata-rata 1 jam terakhir. Perhitungan trading, target, dan grafik tidak berubah dari v1.8) · Pemilik produk: Raya Abdullah Ar Rasyid · Status: Fase 0 sampai 8 diimplementasikan; Fase 9: T9.0 sampai T9.2 selesai, T9.4 (filter Lonjakan) masih rancangan (bagian bertanda "usulan" belum final)

## 1. Ringkasan

Aplikasi mobile pribadi untuk mencatat trading crypto di Indodax (pasangan IDR) secara otomatis lewat API resmi. Aplikasi hanya membaca data dan menghitung. Ia tidak pernah membuat order, membatalkan order, atau memindahkan dana.

Empat hal yang diselesaikan:

1. Mengetahui total aset (nilai pasar) dan hasil trading (profit atau loss dari trade yang sudah ditutup), terhadap target harian dan mingguan.
2. Setiap kali membeli koin, langsung muncul harga Take Profit (TP) dan Stop Loss (SL) sebagai notifikasi.
3. Open posisi tercatat otomatis dan tertutup otomatis ketika dijual.
4. (Fase 9) Menyaring pair IDR yang memenuhi kriteria scalping buatan pengguna sendiri dan menandai pair yang harganya melonjak dibanding rata-rata 1 jam terakhir, semuanya di layar Screener. Tanpa notifikasi.

## 2. Latar belakang dan masalah

Pengguna melakukan trading cepat (scalping) dan hasilnya hanya terlihat sebagai struk per transaksi. Akibatnya:

- Sulit melihat total profit atau loss setelah fee, dari hari ke hari.
- Tidak ada target yang terukur, harian maupun mingguan.
- Keputusan SL dan TP dibuat spontan, tanpa perhitungan yang menyertakan fee.
- Fee beli dan jual berbeda dan menggerus profit trade kecil. Contoh nyata: dari modal ±65 ribu, total fee 340 IDR menggerus hampir separuh profit kotor (±720 IDR), tersisa +380 IDR.
- (Fase 9) Mencari pair yang layak di-scalping dilakukan manual dengan skrip Python di Termux (`screener_indodax.py`), terpisah dari aplikasi, dan tidak menandai pair yang sedang melonjak.

## 3. Tujuan dan non-tujuan

**Tujuan**

- Pencatatan trade otomatis dan akurat, tanpa input manual.
- Hanya ada satu angka saldo: Total aset (IDR sebagai "Buying power" + koin dinilai harga terakhir), selalu cocok dengan akun Indodax karena diambil langsung dari akun.
- Target dan grafik dihitung dari profit trade yang sudah ditutup, tidak terpengaruh deposit/withdraw maupun naik-turun harga koin yang sedang dipegang.
- Notifikasi TP/SL muncul cepat setelah order beli terisi.
- Logika perhitungan sesederhana mungkin.
- Tampilan minimalis: pengguna paham trading, jadi layar hanya memuat angka yang dipakai mengambil keputusan, tanpa kalimat penjelas, rumus, atau keterangan teknis (Fase 7).
- (Fase 9) Screener: daftar pair yang lolos kriteria pengguna, semua ambang bisa diatur di aplikasi, dan filter Lonjakan (naik maupun turun) terhadap rata-rata harga 1 jam. Fitur ini khusus penyaringan, tanpa notifikasi, dan tidak menyentuh perhitungan trading.

**Non-tujuan (di luar cakupan versi ini)**

- Membuat atau membatalkan order, atau penarikan dana (izin API hanya baca).
- Grafik harga dan tampilan order book. Harga terakhir (`last`) dipakai untuk Total aset dan floating P/L. Khusus fitur Screener (Fase 9), data pasar publik (ringkasan pasar, riwayat transaksi, kedalaman order book) dipakai untuk menyaring pair dan menandai lonjakan; hanya untuk itu. Notifikasi harga atau lonjakan TIDAK dibuat (diputuskan 14 November 2026: dikhawatirkan terlalu banyak notifikasi).
- Sinyal atau rekomendasi beli/jual. Screener hanya menyaring berdasarkan kriteria yang diatur pengguna sendiri; bukan saran membeli, tidak memengaruhi profit, target, grafik, atau statistik.
- Bursa selain Indodax, pasangan selain IDR.
- Multi-pengguna, akun cloud, dan platform selain mobile.
- Data bursa lain (misalnya Binance) untuk ide "ikuti pergerakan whale": ditunda, bukan bagian Fase 9.

## 4. Pengguna dan konteks

- Satu pengguna: trader individu, gaya scalping, beli dengan market order atau limit order (kedua jenis ditemukan di data nyata), jual dengan limit order.
- Modal per trade berubah-ubah (tidak tetap).
- Platform trading: Indodax. Target perangkat: HP Android (asumsi).
- Semua target dan batas dinyatakan dalam persen.

## 5. Definisi

| Istilah | Arti |
| --- | --- |
| Modal efektif | Total IDR yang benar-benar keluar untuk beli (nilai trade + fee, sama dengan "Order Terisi" di struk), dibagi koin yang diterima. Dihitung per koin. |
| Posisi | Koin yang sudah dibeli dan belum terjual. Satu posisi per koin, dirata-rata. |
| Total aset | Saldo IDR (tersedia + terkunci) + Σ (jumlah koin × harga terakhir `last`) dari GET /account dan ticker_all. Satu-satunya angka saldo di aplikasi. Koin tanpa pasangan IDR tidak dihitung dan tidak ditampilkan (sejak v1.7). |
| Buying power | Bagian IDR dari Total aset. |
| Profit trade tertutup | Per trade yang sudah ditutup: IDR diterima dari jual dikurangi modal efektif koin yang dijual (fee sudah termasuk). Sumber target dan grafik. |
| Floating P/L | Posisi terbuka: jumlah koin × harga terakhir − modal posisi, SEBELUM fee jual. Ditampilkan terpisah; tidak masuk target, grafik, atau statistik. |
| TP / SL | Harga target jual untung (Take Profit) dan harga batas rugi (Stop Loss). |
| Screener | Fitur penyaringan pair IDR berdasarkan kriteria pengguna (Fase 9). Hanya informasi. |
| Trx/mnt | Jumlah transaksi per menit pada sebuah pair, dihitung dari riwayat transaksi publik dalam jendela waktu tertentu. |
| Spread | Selisih harga jual terendah (ask) dan beli tertinggi (bid), dalam persen dari harga terakhir. |
| Gerak 10 menit | Selisih high dan low harga transaksi 10 menit terakhir, dalam persen dari low (usulan; menggantikan range 24 jam yang basi). |
| Menit aktif | Persentase menit dalam jendela yang punya minimal satu transaksi (usulan; menangkap lonjakan sesaat yang tertutup rata-rata). |
| Harga dasar 1 jam | Rata-rata harga terakhir (`last`) sebuah pair dari snapshot ringkasan pasar dalam 60 menit terakhir, disimpan hanya di memori (usulan; FR-34). |
| Lonjakan | Harga terakhir menyimpang dari harga dasar 1 jam sebesar X persen atau lebih: naik (▲) atau turun (▼). Ditampilkan di layar Screener sebagai filter terpisah; tanpa notifikasi (definisi di FR-34). |

## 6. Alur utama

1. **Setup**: pengguna memasukkan API key dan secret (izin baca saja) dan mengisi setelan persen (TP, SL, target harian, target mingguan). Aplikasi mencatat waktu mulai (snapshot awal); trade sebelum waktu itu tidak dihitung.
2. **Beli di Indodax**: aplikasi mendeteksi order beli yang terisi, mencatatnya, membuat atau memperbarui posisi, menghitung modal efektif, TP, dan SL, lalu menampilkan notifikasi.
3. **Posisi terbuka**: tampil di daftar Open Posisi lengkap dengan modal efektif, TP, dan SL.
4. **Jual di Indodax**: aplikasi mendeteksi order jual, mengurangi atau menutup posisi, menghitung profit atau loss bersih, dan memperbarui profit tertutup dan progress target.
5. **Pemantauan**: pengguna melihat pencapaian harian dan mingguan, total aset, grafik akumulasi profit, floating, dan riwayat trade.
6. **Screening (Fase 9)**: pengguna membuka layar Screener untuk melihat pair yang lolos kriteria, dan/atau memakai filter Lonjakan. Layar hanya informasi: pengguna membuka aplikasi Indodax sendiri dan membuat order di sana.

## 7. Kebutuhan fungsional

### 7.1 Akun dan koneksi

- **FR-01** Pengguna memasukkan API key dan secret TAPI v2 dengan izin baca saja. Kunci disimpan di penyimpanan aman perangkat dan tidak pernah ditampilkan ulang secara utuh.
- **FR-02** Aplikasi memvalidasi kunci (membaca saldo akun) dan menampilkan status koneksi (tersambung, terputus, kunci tidak valid). Kunci yang punya izin trade atau withdraw ditolak dan tidak disimpan.
- **FR-03** Saat sinkron pertama berhasil, aplikasi mencatat waktu mulai (snapshot awal). Hanya trade sejak waktu itu yang dihitung. Instal ulang atau hapus data mengosongkan profit tertutup, target, dan grafik, kecuali data dipulihkan lewat impor cadangan (FR-26).

### 7.2 Setelan

- **FR-04** Pengguna mengatur, semuanya dalam persen: TP per trade, SL per trade, target harian, target mingguan.
- **FR-05** Perubahan setelan TP/SL berlaku untuk posisi baru. Posisi yang sudah terbuka tidak berubah (dikonfirmasi 8 November 2026). Target harian dan mingguan hanya menyimpan nilai terbaru (lebih dari 0 dan paling banyak 100 persen); mengosongkan keduanya berarti tanpa target.

### 7.3 Deteksi dan pencatatan trade

- **FR-06** Layanan latar (foreground service, tipe specialUse) menjaga koneksi WebSocket privat Indodax agar order terdeteksi secara realtime. WebSocket hanya berfungsi sebagai PEMICU: angka resmi selalu diambil dari REST. Bila HP restart saat pantauan menyala, pantauan menyala lagi otomatis (T5.1); bila pengguna sudah menghentikannya, tetap mati.
- **FR-07** Event pengisian order (FILL) dikumpulkan per order. Satu order bisa terisi beberapa kali. Order dianggap selesai saat status DONE.
- **FR-08** Event NEW, CANCELLED tanpa eksekusi, dan REJECTED tidak dicatat sebagai trade. Order yang dibatalkan setelah terisi sebagian dicatat sesuai bagian yang terisi.
- **FR-09** Anti-dobel: setiap pengisian dikenali dari ID trade yang unik. Mencatat ulang data yang sama tidak boleh membuat catatan ganda.
- **FR-10** Rekonsiliasi: setiap koneksi tersambung kembali dan setiap aplikasi dibuka (jeda minimal 60 detik), aplikasi menarik riwayat trade terbaru lewat REST dan melengkapi yang terlewat. Tersedia juga tombol sinkron manual (dinonaktifkan saat pantauan berjalan).

### 7.4 Posisi dan TP/SL

- **FR-11** Satu posisi per koin. Beli berikutnya pada koin yang sama merata-ratakan modal efektif dan menghitung ulang TP/SL.
- **FR-12** Saat posisi dibuat atau diperbarui, aplikasi menghitung modal efektif, harga TP, dan harga SL dari persen di setelan.
- **FR-13** Jual sebagian mengurangi jumlah koin posisi, posisi tetap terbuka. Jual penuh menutup posisi dan memindahkannya ke riwayat.
- **FR-14** Layar Open Posisi menampilkan per posisi: koin, jumlah koin, modal, tangga SL/modal efektif/TP dengan penanda harga sekarang, satu baris harga sekarang beserta floating, satu baris estimasi hasil bersih di TP dan SL dalam persen (memakai fee jual perkiraan 0,31%), dan waktu beli. Tanpa teks penjelas; hanya galat pembaruan harga yang ditandai. TIDAK ada tombol Jual: aplikasi tidak membuat order, dan posisi tertutup otomatis saat order jual terdeteksi (keputusan 7 November 2026).

### 7.5 Notifikasi

- **FR-15** Setelah beli: notifikasi berprioritas tinggi berisi koin, modal efektif, TP, SL, dan estimasi hasil bersih. Pada beli tambahan koin yang sama, tampil TP/SL order itu dan TP/SL posisi keseluruhan. Trade yang terlewat saat koneksi putus dirangkum dalam satu notifikasi setelah rekonsiliasi.
- **FR-16** Setelah jual: notifikasi berisi hasil bersih (Rp dan %) serta progress target harian dan mingguan. Persen memakai total aset TERAKHIR yang disimpan Beranda (tanpa jaringan, bisa sedikit tertinggal); bila belum pernah tersimpan, tampil rupiah profit saja.
- **FR-17** Notifikasi TP/SL hanya informasi. Aplikasi tidak memasang order apa pun di Indodax.
- **FR-18** Penanda "TERCAPAI" saat target harian atau mingguan baru tercapai oleh suatu penjualan, ditampilkan di dalam notifikasi jual yang sama (bukan notifikasi terpisah).

### 7.6 Target harian dan mingguan

- **FR-19** Tidak ada snapshot saldo per periode. Profit periode dihitung dari daftar (waktu tutup, profit) trade tertutup: hari mulai 00:00 WIB, minggu mulai Senin 00:00 WIB. Trade yang tutup tepat pada awal periode milik periode baru.
- **FR-20** Persen periode = profit periode ÷ (total aset sekarang − profit periode) × 100 (rumus di bagian 8). Deposit dan withdraw tidak ikut dihitung.
- **FR-21** Tampilan: profit (Rp), persen, persen terhadap target, dan progress bar. Reset otomatis tiap hari dan tiap minggu. Bila total aset belum diketahui, tampil rupiah profit dengan tulisan "Menunggu total aset".

### 7.7 Total aset, grafik, dan riwayat

- **FR-22** Beranda menampilkan SATU kartu Total aset: total, Buying power (IDR), satu baris per koin (nilai Rupiah; koin tanpa pasangan IDR tidak ditampilkan), baris Profit trading (total sejak aplikasi dipasang), dan di bawahnya satu baris berisi jumlah trade di kiri dan floating di kanan (hilang bila tidak ada posisi berharga). Grafik = kurva akumulasi profit trade tertutup (satu garis, dengan garis putus-putus di Rp 0), tanpa legenda atau keterangan titik. Status koneksi di pojok atas: "Terhubung" atau "Pantauan mati". Total aset diperbarui saat layar dibuka dan tiap 10 detik selama terlihat.
- **FR-23** Riwayat trade tertutup: koin, modal, total diterima, profit atau loss bersih (Rp dan %), waktu buka dan tutup.
- **FR-24** Aplikasi mengambil riwayat deposit dan withdraw IDR dan menampilkannya di bagian bawah layar Riwayat, satu baris per item (jenis, tanggal, nominal; fee tidak ditampilkan). Hanya riwayat: tidak memengaruhi total aset, target, atau grafik. Sinkron berjalan otomatis (setelah sinkron trade dan saat pantauan tersambung); tidak ada tombol manual.
- **FR-25** Statistik (di layar Riwayat, rentang hari, minggu, dan semua): win rate (disertai jumlah untung, rugi, dan impas), rata-rata untung, rata-rata rugi, hasil bersih, serta fee beserta persennya dari profit kotor. Profit kotor tetap dihitung tetapi tidak ditampilkan sebagai baris sendiri.
- **FR-28** Floating P/L: layar Posisi menampilkan harga sekarang sebagai penanda pada tangga SL/modal efektif/TP (merah bila di bawah SL) dan floating per posisi; Beranda menampilkan jumlahnya. Dihitung SEBELUM fee jual karena fee nyata baru ada setelah koin dijual dan tarifnya berbeda per order (limit ±0,31%, market ±0,61%).

### 7.8 Cadangan

- **FR-26** Cadangan data lokal: ekspor salinan database ke berkas pilihan pengguna dan impor kembali (pemilih berkas sistem). Impor memeriksa berkas, meminta konfirmasi, hanya berjalan saat pantauan mati dan tidak ada sinkron berjalan, dan menyimpan database lama sebagai salinan otomatis. Kunci API tidak ikut dicadangkan. Cadangan lama (v1.x) yang masih memuat tabel snapshot, koreksi saldo, dan arus kas tetap bisa diimpor; tabel itu dibiarkan di database tetapi tidak lagi dipakai perhitungan.
- ~~**FR-27** Koreksi saldo~~ **Dihapus (T6.4, 8 November 2026)**: tidak ada lagi saldo versi aplikasi yang perlu dicocokkan dengan saldo asli.

### 7.9 Screener (Fase 9, sebagian diimplementasikan)

Semua butir di bagian ini hanya membaca data pasar publik (tanpa key, tanpa tanda tangan) dan tidak memengaruhi posisi, profit, target, grafik, atau statistik. Angka default dan rumus final ditetapkan setelah verifikasi API (T9.0).

- **FR-29** Layar Screener: satu daftar pair IDR yang lolos kriteria, urut menurut skor (FR-32). Kolom: pair, harga, trx/mnt, spread, gerak 10 menit, buy%. Tanpa teks penjelas (gaya Fase 7). Pair yang sedang menjadi posisi diberi penanda "Dipegang". Layar adalah tab ke-5 di navigasi bawah (T9.2, lulus uji HP 14 November 2026). Mengetuk baris tidak melakukan apa pun.
- **FR-30** Sumber data dan beban: tahap 1 dari satu request ringkasan pasar (filter murah); tahap 2 hanya untuk kandidat, memakai riwayat transaksi per pair (dan kedalaman order book bila filter kedalaman aktif). Penyegaran tiap 30 sampai 60 detik hanya saat layar terbuka, agar batas 300 permintaan per menit aman. Gagal ambil data satu pair tidak menggagalkan seluruh daftar.
- **FR-31** Filter dasar (bawaan skrip Python pengguna, semua dapat diatur): harga minimum dan maksimum, volume 24 jam minimum dan maksimum, trx/mnt minimum dalam jendela tertentu, spread maksimum, range 24 jam minimum, usia transaksi terakhir maksimum. Nilai awal: nilai yang dipakai skrip sekarang (harga 10 sampai 70.000; volume 10 juta sampai 10 miliar; trx/mnt ≥ 2 dalam jendela 10 menit; spread ≤ 10%; range ≥ 4%; usia transaksi ≤ 120 detik). Setiap filter dapat dimatikan.
- **FR-32** Filter dan urutan tambahan (usulan, tiap filter dapat diatur atau dimatikan): spread maksimum terikat TP di Setelan (bawaan ≤ sepertiga TP); gerak 10 menit minimum (≥ 2 × perkiraan biaya round-trip), menggantikan range 24 jam; menit aktif minimum (bawaan ≥ 70%); usia transaksi terakhir lebih ketat (30 sampai 60 detik); kedalaman order book dalam 0,5% dari harga minimum sebesar kelipatan "Modal trade ini"; arah 10 menit minimum agar tidak masuk saat harga jatuh; dominasi satu transaksi (bila satu trade lebih dari 40% volume jendela: tanda peringatan, bukan dibuang). Skor urutan = gerak 10 menit ÷ (spread + biaya round-trip).
- **FR-33** Setelan Screener: semua ambang disimpan di aplikasi (tabel `meta`) dan bisa diubah kapan saja. Modal per trade berubah-ubah, jadi TIDAK ada modal tetap: layar Screener punya isian opsional "Modal trade ini" yang tidak disimpan permanen; kosong berarti filter kedalaman dilewati.
- **FR-34** Filter Lonjakan (usulan; angka final setelah uji lapangan; diputuskan 14 November 2026: filter terpisah di layar Screener, tanpa notifikasi). Sebuah pair dinyatakan melonjak bila harga terakhir menyimpang dari harga dasar 1 jam (rata-rata `last` dari snapshot ringkasan pasar 60 menit terakhir) sebesar X persen atau lebih: ▲ bila di atas, ▼ bila di bawah; naik dan turun punya saklar terpisah. Tahap 1 (pemicu): satu request ringkasan pasar tiap 15 sampai 30 detik (dapat diatur) selama layar Screener terbuka dan mode Lonjakan aktif; pair yang sudah melewati separuh X diperiksa tahap 2 lewat riwayat transaksi, maksimal 15 pair per siklus. Tahap 2: trx/mnt jendela pendek (bawaan 3 menit) ≥ M absolut (usulan; perbandingan N kali dasar 10 menit belum dikonfirmasi). Syarat dasar (spread maksimum dan usia transaksi terakhir) tetap berlaku agar pair yang tidak bisa dijual tidak muncul. Snapshot: satu titik `last` per pair beserta waktunya, hanya di memori, titik lebih tua dari 60 menit dibuang, tidak pernah masuk database. Masa pemanasan: pair belum dinilai sebelum ada snapshot minimal sekitar 10 menit (usulan); harga dasar memakai data yang ada sampai 60 menit, berbobot waktu agar celah polling tidak membiaskan rata-rata. Modul Rust tetap murni: snapshot dikirim dari sisi Kotlin sebagai masukan.
- ~~**FR-35** Notifikasi lonjakan~~ **Dihapus (14 November 2026)**: dikhawatirkan terlalu banyak notifikasi. Lonjakan hanya tampil di layar Screener.
- ~~**FR-36** Layanan terpisah (`LayananScreener`)~~ **Dihapus (14 November 2026)**: tidak ada notifikasi, jadi tidak perlu layanan latar, saluran notifikasi, maupun perubahan manifest. `LayananLatar` tidak disentuh.
- ~~**FR-37** Tombol buka pair di Indodax~~ **Dihapus (14 November 2026)**: tautan web `https://indodax.com/market/<PAIR>` terbukti membuka browser, bukan aplikasi Indodax (dicek lewat adb), dan pengguna memutuskan tombol tidak diperlukan. Pengguna membuka Indodax sendiri.

## 8. Rumus dan aturan perhitungan

| Besaran | Rumus |
| --- | --- |
| Modal efektif per koin | IDR keluar (quoteQty + commission, = "Order Terisi" di struk) ÷ koin diterima (qty). Untuk posisi hasil beberapa kali beli: total IDR keluar ÷ total koin diterima. |
| Harga TP | Modal efektif × (1 + TP%) |
| Harga SL | Modal efektif × (1 − SL%) |
| Profit atau loss jual | IDR diterima − (modal efektif × jumlah koin yang dijual) |
| Total aset | Saldo IDR (tersedia + terkunci) + Σ (jumlah koin × harga terakhir). Diambil dari akun, tanpa catatan lokal. |
| Profit periode | Σ profit trade tertutup dengan waktu tutup ≥ awal periode dan ≤ sekarang (WIB; minggu mulai Senin) |
| Persen periode (%) | Profit periode ÷ (total aset sekarang − profit periode) × 100. Kosong bila total aset belum diketahui atau pembaginya tidak positif. |
| Progress target (%) | Persen periode ÷ target periode × 100. Target 0 atau kosong = belum diatur. |
| Floating posisi | Jumlah koin × harga terakhir − modal posisi (modal efektif × koin), sebelum fee jual. Persen = floating ÷ modal posisi × 100. |

**Rumus Screener (Fase 9, usulan; final setelah T9.0)**

| Besaran | Rumus |
| --- | --- |
| Spread (%) | (ask − bid) ÷ harga terakhir × 100 (bid atau ask nol berarti tidak lolos) |
| Range 24 jam (%) | (high − low) ÷ low × 100 (dari ringkasan pasar) |
| Trx/mnt | Jumlah transaksi dalam jendela ÷ lebar jendela (menit). Bila riwayat yang dikembalikan API tidak sampai ke awal jendela, jendela diperpendek (minimal 1 menit), seperti di skrip. |
| Gerak 10 menit (%) | (high − low harga transaksi 10 menit terakhir) ÷ low × 100 |
| Menit aktif (%) | Menit dengan ≥ 1 transaksi ÷ jumlah menit jendela × 100 |
| Buy% | Transaksi bertipe beli ÷ semua transaksi di jendela × 100 |
| Perkiraan biaya round-trip | Fee beli perkiraan + fee jual perkiraan; sekitar 0,41% untuk beli dan jual limit (0,10% + 0,31%), lebih besar bila market. Dapat diatur. |
| Skor | Gerak 10 menit ÷ (spread + biaya round-trip) |
| Harga dasar 1 jam | Rata-rata `last` dari snapshot dalam 60 menit terakhir (berbobot waktu). Kosong bila belum cukup data (masa pemanasan). |
| Lonjakan | (harga terakhir − harga dasar 1 jam) ÷ harga dasar 1 jam × 100 = perubahan; melonjak bila \|perubahan\| ≥ X% dan trx/mnt jendela pendek ≥ M (FR-34); ▲ bila perubahan positif, ▼ bila negatif |

**Aturan**

- Fee beli sudah terkandung di modal efektif. Fee jual tidak dihitung di harga TP/SL, tetapi sudah terkandung di IDR diterima saat jual.
- Target dan grafik hanya berubah setelah trade ditutup (profit tertutup). Naik-turun harga koin yang sedang dipegang tampil terpisah sebagai floating dan tidak masuk target (keputusan 8 November 2026, pilihan 1).
- Harga koin memakai `last` dari ticker_all. Harga bid mungkin ditambahkan nanti sebagai informasi, tanpa memengaruhi target.
- Selisih jual-beli mentah TIDAK dipakai (salah bila ada posisi terbuka); yang dipakai profit per trade tertutup.
- Estimasi hasil bersih di notifikasi dan layar Posisi memakai fee jual perkiraan 0,31% (limit). Jual market bisa sekitar 0,61%, jadi estimasi bisa terlalu optimistis; hasil nyata selalu memakai `commission` dari API saat jual.
- Statistik: win rate = trade untung / semua trade; persen profit termakan fee = total fee / profit kotor, dengan profit kotor = hasil bersih + total fee.
- Seluruh perhitungan uang memakai tipe angka desimal presisi (bukan floating point). Jumlah koin sampai 8 desimal. Hitungan Screener (persen, rasio) juga memakai Decimal di Rust.
- Hasil Screener dan snapshot harga lonjakan tidak pernah masuk ke profit, target, grafik, statistik, atau database trade; snapshot hanya di memori dan dibuang setelah 60 menit.

**Contoh nyata (ICNT, 5 November 2026)**

| Langkah | Data |
| --- | --- |
| Beli (market) 16:54 | Jumlah order 65.529 IDR (anggaran), order terisi 65.526 IDR (sudah termasuk fee 133), koin diterima 16,35667833 |
| Modal efektif | 65.526 ÷ 16,35667833 ≈ 4.006,07 per koin (harga rata-rata 3.998; selisihnya karena fee) |
| TP 1% | ≈ 4.046 |
| SL 1% | ≈ 3.966 |
| Jual (limit) 17:06 | Total diterima 65.906 IDR, fee 207 |
| Hasil bersih | 65.906 − 65.526 = **+380 IDR (+0,58%)** |
| Total fee | 340 IDR (≈ 0,5% dari modal) |

Catatan: bila TP 1% tercapai, profit bersih ±0,7% karena fee jual. Bila SL 1% tercapai, rugi bersih ±1,3%.

## 9. Integrasi Indodax

Berdasarkan dokumentasi resmi Indodax (repo btcid/indodax-official-api-docs) per 6 November 2026. Seluruh poin harus diverifikasi lagi saat implementasi.

**Jalur realtime: WebSocket privat**

- Minta token lewat REST (`POST https://indodax.com/api/private_ws/v1/generate_token`, badan `client=tapi&tapi_key=<KEY>`). Token berlaku 24 jam, jadi diminta ulang saat sambung ulang. Permintaan token ditandatangani HMAC-SHA512 (berbeda dari REST v2 yang SHA256). Key v2 baca saja diterima (terkonfirmasi di T0.6).
- Terhubung ke `wss://pws.indodax.com/ws/?cf_ws_frame_ping_pong=true` TANPA header Origin, kirim `connect` dengan token, lalu `subscribe` ke channel pribadi. Server mengirim `{}` sebagai ping aplikasi; balas dengan `{}`.
- Event order\_update: NEW, FILL (bukan FILLED), DONE, CANCELLED, REJECTED. Event FILL membawa harga (pembulatan), jumlah koin, dan rincian biaya (fee, pajak, clearing); `fee + tax + clearing` sama dengan `commission` REST pada sampel yang diuji. Market order mengirim NEW, FILL, DONE sekaligus dalam satu paket.
- Karena harga di event dibulatkan, angka resmi tidak diambil dari event: saat order DONE (atau CANCELLED setelah sebagian terisi), aplikasi menarik trade koin itu lewat REST.
- Hanya update order yang dikirim. Saldo tidak didorong lewat WebSocket.

**Jalur sinkron: REST TAPI v2** (https://api.indodax.com)

- GET /api/v2/myTrades: riwayat trade (harga, jumlah, nilai, total biaya, beli atau jual, waktu). Parameter symbol wajib, rentang maksimal 7 hari per permintaan, default 24 jam.
- GET /api/v2/account: saldo.
- GET /api/v2/fiat/orders: riwayat deposit dan withdraw IDR (rentang maksimal 30 hari).
- Batas laju umum: 300 permintaan per menit.
- Ditandatangani HMAC-SHA256 untuk REST v2, dengan header X-APIKEY dan Sign.

**Jalur publik (tanpa key): harga dan data pasar**

- GET `https://indodax.com/api/ticker_all`: harga `last` per pair `*_idr` (dipakai Total aset dan floating; bentuk respons terverifikasi dari data nyata 8 November 2026).
- Fase 9 (Screener), diverifikasi di T9.0 (14 November 2026): GET `https://indodax.com/api/summaries` (438 pair IDR, ±94,7 KB, ±0,2 detik; kunci `tickers`, `prices_24h`, `prices_7d`; per pair `buy` (bid), `sell` (ask), `high`, `low`, `last`, `vol_idr`, `server_time`; semua nilai string; kunci `tickers` memakai `btc_idr`, `prices_24h` memakai `btcidr`) dan GET `https://indodax.com/api/trades/<pair>` (selalu 500 transaksi, terbaru dulu: `tid`, `type`, `price`, `amount`, `date` epoch detik).
- GET `/api/depth/<pair>` (kedalaman order book): kunci `buy` dan `sell`, maksimal 150 level per sisi, level `[harga, jumlah koin]`. Dipakai filter kedalaman (T9.1).
- Terjawab di T9.0: `vol_idr` kemungkinan besar volume 24 jam bergulir (tidak dipakai sebagai trx/mnt); ringkasan segar tiap beberapa detik; WebSocket publik tidak diuji dan tidak diasumsikan ada. Belum terverifikasi: apakah respons `summaries` dikompresi gzip; pair dengan lebih dari 500 transaksi dalam 10 menit.

**Catatan penting**

- Metode lama tradeHistory dan orderHistory sudah dinonaktifkan sejak 7 April 2026. Jangan memakai library atau contoh yang masih memakainya.
- TAPI v2 butuh API key khusus v2 (dibuat di indodax.com/trade\_api). Key lama tidak bisa dipakai.
- Gunakan izin baca saja. Pembatasan IP untuk izin baca bersifat opsional. Karena IP jaringan seluler sering berubah, sebaiknya jangan diaktifkan agar koneksi tidak gagal.
- Rekonsiliasi riwayat harus berputar per koin dan per rentang 7 hari.
- Terjawab di Fase 0 (6 November 2026, trade ICNT 3 dan 5 November): (a) fee dalam IDR (`commissionAsset` = idr), koin yang diterima tidak dipotong; (b) `commission` dari API sama dengan "Biaya Transaksi" di struk; (c) `quoteQty` sama dengan "Total Diterima" (IDR) pada struk beli. Tanda tangan v2 memakai HMAC-SHA256.
- Terjawab di T0.6 (7 November 2026): key v2 baca saja dapat dipakai untuk token WebSocket privat.
- Terjawab (8 November 2026, deposit nyata Rp 11.000): `amount` 7.670, `totalFee` 3.330 (7.670 + 3.330 = 11.000), saldo asli naik 7.670. `amount` deposit SUDAH bersih: fee dipotong sebelum dana masuk akun, jadi `totalFee` hanya informasi dan tidak dikurangkan lagi. Perilaku aplikasi sebelum T5.2 (mengurangi fee lagi) salah dan diperbaiki di T5.2.
- Belum terverifikasi: bentuk baris withdraw dan apakah `amount` withdraw sudah termasuk fee. Withdraw nyata belum bisa diuji karena saldo di bawah batas minimum penarikan Indodax.
- Tarif fee tidak tetap (beli limit ±0,10%, beli market ±0,20%, jual limit ±0,31% pada data nyata), jadi selalu pakai nilai `commission` dari API.

## 10. Arsitektur dan teknologi (keputusan 7 November 2026)

- **Rust** adalah bahasa utama untuk seluruh logika dan "backend" yang berjalan di dalam HP: klien Indodax (REST v2, nanti WebSocket), anti-dobel, perhitungan modal efektif, TP/SL, posisi, dan penyimpanan lokal. Semua perhitungan uang memakai Decimal.
- **Kotlin/Java** hanya sebagai klien Android: tampilan, foreground service, notifikasi, dan penyimpanan kunci di penyimpanan aman perangkat. Tidak ada logika bisnis di sisi Kotlin.
- Tidak ada server terpisah dan tidak ada akun cloud (sesuai non-tujuan).
- Jembatan Rust ke Kotlin memakai UniFFI 0.31 (crate `ffi/`, `scalping-ffi`). Penyimpanan: SQLite (rusqlite, bundled) dikelola dari Rust.
- Kode Rust ada di `core/` (crate `scalping-core`), jembatan di `ffi/`, kode Android di `app/` (tanpa Compose, tampilan dari kode, minSdk 30).
- Posisi dan profit tertutup tidak disimpan sebagai tabel sendiri: dihitung ulang dari catatan trade setiap kali dibutuhkan. Total aset selalu dari akun (jaringan).
- Fase 9: modul Rust `core/src/screener.rs` (parser ringkasan pasar, transaksi, dan kedalaman; filter dua tahap; skor; murni, tanpa jaringan dan database) dan `core/src/screener_setelan.rs` (ambang di tabel `meta`), jembatan `ffi/src/screener_ffi.rs`, layar `LayarScreener.kt` sebagai tab ke-5 (T9.1 dan T9.2, selesai). T9.4 (rancangan): deteksi lonjakan di modul Rust yang sama, dengan snapshot harga dikirim dari Kotlin dan polling ringkasan hanya saat layar Screener terbuka. TIDAK ada layanan latar baru, saluran notifikasi, atau perubahan manifest; `LayananLatar.kt` tidak diubah.

## 11. Model data

| Entitas | Isi utama |
| --- | --- |
| Setelan | TP% dan SL% (dengan riwayat berlaku-sejak), target harian %, target mingguan %, zona waktu (WIB) |
| Trade (pengisian) | ID trade (unik), ID order, koin, sisi (beli atau jual), harga, jumlah koin, biaya, waktu |
| Order | ID order, status akhir, total IDR, total koin |
| Posisi | koin, jumlah koin terbuka, total IDR modal, modal efektif, TP%, SL%, harga TP, harga SL, waktu buka, status |
| Trade tertutup | koin, modal, IDR diterima, profit atau loss, persen, waktu buka dan tutup |
| Snapshot | hanya jenis awal: waktu mulai pencatatan (batas trade yang dihitung). Baris harian/mingguan lama dari versi sebelumnya tidak dipakai. |
| Arus kas | jenis (deposit atau withdraw IDR), jumlah, biaya, waktu, ID sumber (unik). Hanya riwayat. |
| Koreksi saldo | (tabel lama dibiarkan agar cadangan v1.x tetap bisa diimpor; tidak dipakai) |
| Total aset terakhir | rupiah dan waktu, disimpan di tabel meta oleh Beranda; dipakai notifikasi jual |
| Setelan Screener (Fase 9) | semua ambang filter, ambang lonjakan (X, M, jendela, interval polling), saklar naik/turun; disimpan di tabel `meta` (tanpa tabel baru). Hasil screening dan snapshot harga (60 menit) hanya di memori, tidak disimpan. |

Penyimpanan lokal di perangkat. Data disimpan sebagai catatan, bukan angka saldo mentah, sehingga mengoreksi satu catatan dapat menghitung ulang semuanya.

## 12. Kebutuhan non-fungsional

- **Keandalan**: koneksi realtime otomatis tersambung kembali, disusul rekonsiliasi REST agar tidak ada trade yang terlewat.
- **Kecepatan**: target notifikasi muncul dalam beberapa detik setelah order DONE (target awal, perlu diukur).
- **Baterai dan sistem**: layanan latar dengan satu notifikasi tetap yang bisa disenyapkan. Pengguna perlu mematikan optimasi baterai untuk aplikasi ini dan memberi izin notifikasi (Android 13 ke atas). Screener (Fase 9) tidak menambah layanan latar maupun notifikasi: berjalan hanya saat layarnya terbuka.
- **Keamanan**: kunci disimpan di penyimpanan aman perangkat, tidak masuk log, tidak masuk cadangan tanpa enkripsi, tidak dikirim ke pihak mana pun selain Indodax. Hanya izin baca. Screener memakai endpoint publik tanpa key.
- **Offline**: data lokal tetap bisa dilihat tanpa internet. Sinkron berjalan ulang saat koneksi kembali. Screener membutuhkan internet.
- **Cadangan**: ekspor dan impor database ke penyimpanan lokal (FR-26). Kunci API tidak ikut.
- **Ketepatan**: acuan uji tetap struk Indodax per trade (profit tertutup). Total aset diambil langsung dari akun sehingga tidak perlu dikoreksi.
- **Zona waktu dan periode**: WIB, minggu dimulai hari Senin (dikonfirmasi 7 November 2026).
- **Tampilan**: tanpa teks penjelas; log pantauan tersembunyi dan dibuka dengan mengetuk judulnya; tombol optimasi baterai disembunyikan bila sudah dimatikan (Fase 7). Layar Screener mengikuti gaya yang sama.
- **Diagnostik**: log permanen `diag.log`, tombol Bagikan log, dan penjaga pantauan yang memberi peringatan bila pantauan mati atau terputus lebih dari 5 menit (T8.1).
- **Beban API Screener**: total permintaan publik dijaga jauh di bawah 300 per menit (bersama permintaan lain). Mode Lonjakan memakai satu request ringkasan (±95 KB) tiap 15 sampai 30 detik hanya saat layar terbuka (±12 sampai 23 MB per jam bila tidak terkompresi) ditambah paling banyak 15 request `trades` per siklus; interval dapat diatur.

## 13. Kasus khusus

| Kasus | Penanganan yang diusulkan |
| --- | --- |
| Beli beberapa kali pada koin yang sama | Satu posisi, modal efektif dirata-rata, TP/SL dihitung ulang |
| Jual sebagian | Posisi tetap terbuka dengan sisa koin, profit dihitung proporsional |
| Order terisi beberapa kali | Dijumlahkan per order, dicatat saat DONE |
| Koneksi putus atau layanan dimatikan sistem | Rekonsiliasi saat tersambung kembali atau aplikasi dibuka |
| Pergantian hari tanpa layanan aktif | Tidak ada snapshot periode; profit periode dihitung ulang dari waktu tutup trade, jadi tetap benar |
| Deposit atau withdraw IDR di tengah periode | Hanya riwayat; tidak memengaruhi profit, target, atau grafik. Persen memakai total aset sekarang dari akun, yang sudah mencakup deposit/withdraw |
| Menjual koin yang dibeli sebelum aplikasi dipasang | Tidak ada posisi. Dicatat sebagai penyesuaian dan dikeluarkan dari pencapaian (profit dianggap 0). Modal tidak diisi manual (keputusan 7 November 2026). Koin lama itu tetap ikut Total aset (nilai pasar) karena Total aset dari akun |
| Withdraw atau deposit koin | Diabaikan di versi ini (Total aset tetap ikut karena diambil dari akun) |
| Token WebSocket kedaluwarsa | Minta token baru dan hubungkan ulang secara otomatis |
| Kunci API dicabut atau tidak valid | Tampilkan status terputus dan minta kunci baru |
| HP restart saat pantauan menyala | Pantauan menyala lagi otomatis (T5.1); bila pantauan sudah dihentikan pengguna, tetap mati |
| Order limit terisi sebagian dan dibiarkan terbuka | Baru tercatat saat DONE atau CANCELLED atau saat rekonsiliasi |
| Instal ulang atau hapus data | Profit tertutup, target, dan grafik kosong lagi (riwayat hanya sejak aplikasi dipasang). Total aset tetap benar. Pulihkan lewat impor cadangan |
| Screener: ringkasan pasar atau riwayat transaksi gagal diambil | Daftar terakhir tetap tampil dengan penanda galat kecil; pair yang gagal dilewati, tidak menggagalkan pair lain; siklus polling berikutnya mencoba lagi |
| Screener: riwayat transaksi tidak sampai awal jendela (koin ramai) | Jendela diperpendek (minimal 1 menit) seperti di skrip; perilaku pasti diverifikasi di T9.0 |
| Screener: pasar ramai, banyak pair melonjak sekaligus | Pemeriksaan `trades` dibatasi 15 pair per siklus (yang penyimpangannya terbesar didahulukan, usulan) |
| Screener: layar baru dibuka, snapshot belum cukup (masa pemanasan) | Filter Lonjakan belum menilai pair sampai ada snapshot minimal sekitar 10 menit; harga dasar memakai data yang ada sampai 60 menit; layar menunjukkan lama data dasar secara ringkas (usulan) |
| Screener: layar ditutup lalu dibuka kembali | Usulan: snapshot tetap di memori selama proses aplikasi hidup (titik > 60 menit dibuang); rata-rata berbobot waktu sehingga celah saat layar tertutup tidak membias. Belum dikonfirmasi (pertanyaan terbuka no. 21) |
| Screener: pair yang sedang dipegang muncul di daftar | Diberi penanda; tidak memengaruhi posisi |

## 14. Metrik keberhasilan

Aplikasi pribadi, jadi ukurannya sederhana (target awal, bisa disesuaikan):

- Profit tertutup per trade cocok dengan struk Indodax; Total aset cocok dengan aplikasi Indodax (selisih kecil karena harga `last` bergerak).
- Tidak ada trade ganda dan tidak ada trade terlewat dalam satu minggu pemakaian.
- Notifikasi TP/SL muncul untuk setiap pembelian yang terisi.
- Layanan latar tetap hidup sepanjang jam trading tanpa dimatikan sistem.
- Screener (Fase 9): daftar pair cocok dengan hasil skrip Python pada kriteria yang sama; pair yang melonjak terlihat di filter Lonjakan dalam satu siklus polling setelah masa pemanasan.

## 15. Rencana rilis bertahap

| Fase | Isi | Hasil | Status |
| --- | --- | --- | --- |
| 0. Validasi data | Ambil trade nyata (misalnya ICNT) lewat myTrades dengan key baca saja. Cek format fee dan koin diterima, dan kecocokan dengan struk. | Rumus modal efektif terkonfirmasi | Selesai |
| 1. MVP | Koneksi akun, WebSocket realtime, anti-dobel, posisi terbuka, TP/SL, notifikasi beli dan jual | Posisi dan notifikasi bekerja | Selesai |
| 2. Target dan saldo | Setelan target, snapshot harian dan mingguan, riwayat deposit dan withdraw IDR, pencapaian dan progress | Target terukur | Selesai |
| 3. Riwayat dan grafik | Daftar trade tertutup, grafik saldo dan modal aktif, tombol koreksi saldo (grafik dan koreksi diganti di Fase 6) | Pertumbuhan terlihat | Selesai |
| 4. Statistik | Win rate, total fee, persen profit termakan fee | Evaluasi gaya trading | Selesai |
| UI. Layar asli | Beranda, Posisi, Riwayat, Setelan sesuai prototype; cadangan data lokal (U1 sampai U5) | Aplikasi dipakai sehari-hari | Selesai |
| 5. Penyempurnaan | Pantauan menyala otomatis setelah HP restart (T5.1); logika deposit (T5.2) | Pantauan tidak perlu dinyalakan ulang | Selesai (lulus uji HP 8 Nov 2026) |
| 6. Total aset tunggal | T6.1 total aset nilai pasar; T6.2 Beranda tunggal, target dan grafik dari profit tertutup; T6.3 floating P/L; T6.4 pembersihan (snapshot periode, modal aktif, setoran bersih, koreksi saldo dihapus; deposit/withdraw jadi riwayat); T6.5 dan T6.6 pembersihan kode grafik dan celah impor cadangan | Satu angka saldo, target stabil | Selesai (lulus uji HP 8 Nov 2026) |
| 7. Penyederhanaan tampilan | T7.1 Beranda dan Posisi; T7.2 Riwayat dan Setelan (hanya Kotlin, perhitungan tidak berubah) | Informasi minimal dan efektif | Selesai (lulus uji HP 8 Nov 2026) |
| 8. Uji lapangan | T8.1 log diagnostik permanen dan penjaga pantauan; rilis uji pribadi 0.8.0-uji dipakai harian | Bug dan peningkatan terdeteksi | Selesai (uji lapangan sampai 14 Nov 2026 tanpa kendala; tidak ada perbaikan diperlukan) |
| 9. Screener | T9.0 verifikasi API publik (selesai); T9.1 modul Rust `screener.rs` dan tes (selesai); T9.2 FFI, layar Screener, setelan ambang (selesai, lulus uji HP); ~~T9.3 tombol buka pair~~ (dibatalkan); T9.4 filter Lonjakan di layar Screener: snapshot harga, harga dasar rata-rata 1 jam, pemicu dan tahap 2 dari `trades` (tanpa notifikasi dan layanan); PRD dan PROGRESS diperbarui tiap task | Pair layak scalping tersaring dan lonjakan terlihat | T9.0 sampai T9.2 selesai; T9.4 rencana (bagian "usulan" final setelah uji lapangan) |

## 16. Pertanyaan terbuka

1. ~~Fee beli dipotong dari koin atau dari IDR di API?~~ **Terjawab: dari IDR.** Ini menentukan detail rumus modal efektif (diselesaikan di Fase 0).
2. ~~Apakah total biaya di struk sama dengan commission dari API~~ **Terjawab: sama.** (lihat bagian 9) (diselesaikan di Fase 0)?
3. ~~Fungsi tombol Jual di Open Posisi~~ **Terjawab (7 Nov 2026): tombol dihapus**; penjualan terdeteksi otomatis (lihat FR-14).
4. ~~Estimasi hasil bersih di bawah TP dan SL~~ **Terjawab (7 Nov 2026): ditampilkan**, memakai fee jual perkiraan 0,31%.
5. ~~Setelan TP/SL baru: berlaku untuk posisi baru saja, atau boleh diubah per posisi yang sedang terbuka?~~ **Terjawab (8 Nov 2026): posisi baru saja.**
6. ~~Koin yang sudah dipegang sebelum aplikasi dipasang~~ **Terjawab (7 Nov 2026): diabaikan** (saldo aplikasi = IDR saja).
7. ~~Teknologi pembuatan aplikasi~~ **Terjawab (7 Nov 2026): Rust untuk logika, Kotlin/Java untuk klien Android** (lihat bagian 10).
8. ~~Cadangan data~~ **Terjawab (7 Nov 2026): perlu, ke penyimpanan lokal saja** (FR-26, selesai di U5).
9. ~~Hari mulai minggu dan zona waktu~~ **Terjawab (7 Nov 2026): Senin dan WIB.**
10. ~~Apakah `amount` deposit sudah bersih atau belum dikurangi `totalFee`?~~ **Terjawab (8 Nov 2026): sudah bersih** (deposit nyata Rp 11.000 menjadi Rp 7.670 masuk; fee 3.330 hanya informasi). Withdraw masih belum diverifikasi (saldo di bawah batas minimum penarikan).
11. ~~Pantauan menyala otomatis setelah HP restart~~ **Selesai (T5.1), lulus uji HP 8 Nov 2026.**
12. ~~Dasar target dan grafik~~ **Terjawab (8 Nov 2026): profit trade tertutup**, persen = profit ÷ (total aset − profit) × 100; floating tampil terpisah (pilihan 1); deposit/withdraw hanya riwayat; riwayat hanya sejak aplikasi dipasang.
13. ~~Rumus persen di notifikasi jual~~ **Terjawab (8 Nov 2026): Opsi A**, memakai total aset terakhir yang disimpan Beranda (tanpa jaringan di layanan latar).
14. ~~Penyederhanaan tampilan~~ **Terjawab (8 Nov 2026):** informasi diminimalkan karena pengguna paham trading; floating digabung ke baris di bawah Profit trading; koin tanpa harga IDR (contoh PXG) disembunyikan karena tidak akan ada trade lagi untuk koin itu; tombol sinkron deposit/withdraw manual dihapus (sinkron otomatis).
15. ~~Notifikasi lonjakan: satu layanan dengan pantauan order atau saklar sendiri?~~ **Dibatalkan (14 Nov 2026): notifikasi lonjakan dan layanan terpisah ditiadakan** karena dikhawatirkan terlalu banyak notifikasi; lonjakan menjadi filter di layar Screener (FR-34). Modal per trade berubah-ubah, jadi tidak ada modal tetap (FR-33). Semua ambang Screener bisa diatur di aplikasi.
16. ~~Layar Screener sebagai tab ke-5 atau kartu di Beranda?~~ **Terjawab (14 Nov 2026): tab ke-5**, dipakai di T9.2 dan lulus uji HP.
17. ~~Skema deep link Indodax~~ **Terjawab (14 Nov 2026, dicek lewat adb): tautan web membuka browser, bukan aplikasi Indodax.** Tombol buka pair dibatalkan atas keputusan pengguna; FR-37 dihapus.
18. ~~Bentuk respons API publik~~ **Terjawab (T9.0, 14 Nov 2026):** `trades` selalu 500 transaksi; bentuk `depth` dan arti `vol_idr` terjawab (bagian 9). Masih terbuka: kompresi gzip dan pair dengan lebih dari 500 transaksi dalam 10 menit.
19. Angka default filter Lonjakan (X persen, M trx/mnt, jendela pendek, interval polling, minimal snapshot pemanasan, maksimal pair per siklus) dan filter tambahan. **Ditetapkan lewat uji lapangan setelah T9.4.**
20. WebSocket publik Indodax untuk data pasar: ada atau tidak? Belum diverifikasi; tidak diasumsikan.
21. Snapshot harga lonjakan: tetap di memori selama proses aplikasi hidup, atau dikosongkan setiap layar Screener ditutup? Usulan: tetap di memori (tanpa database), titik > 60 menit dibuang, rata-rata berbobot waktu. Dan apakah syarat trx/mnt di tahap 2 berupa M absolut saja atau juga N kali dasar 10 menit? **Belum dijawab.**

## 17. Risiko dan asumsi

| Risiko atau asumsi | Dampak | Mitigasi |
| --- | --- | --- |
| Android mematikan layanan latar | Notifikasi terlambat | Pengecualian optimasi baterai, rekonsiliasi saat dibuka, penjaga pantauan (T8.1) |
| Format fee di API berbeda dari struk | Modal efektif salah | Fase 0 membandingkan data nyata dengan struk |
| Perubahan atau penonaktifan endpoint Indodax | Sinkron gagal | Hanya memakai endpoint resmi TAPI v2, pantau changelog |
| Batas laju (300 per menit) | Rekonsiliasi lambat; Screener bisa kena batas | Rekonsiliasi per koin yang aktif, jeda antar permintaan; Screener memakai satu request ringkasan, tahap 2 hanya kandidat, penyegaran 30 sampai 60 detik saat layar terbuka |
| Kebocoran kunci API | Data akun terbaca | Izin baca saja, simpan aman, tanpa izin trade atau withdraw |
| Pembatasan IP pada key | Koneksi gagal di jaringan seluler | Jangan aktifkan pembatasan IP untuk key baca |
| Fee jual market lebih besar dari perkiraan 0,31% (contoh ±0,61%) | Estimasi hasil bersih terlalu optimistis | Hasil nyata memakai `commission` dari API; estimasi hanya perkiraan. Diputuskan 8 Nov 2026: tanpa setelan fee jual |
| Aturan tick harga per koin belum diperiksa | TP/SL (dibulatkan ke rupiah utuh) mungkin tidak persis sesuai kelipatan harga koin murah | Diputuskan tidak perlu diperbaiki (8 Nov 2026): TP/SL hanya acuan, harga akhir diinput manual di Indodax |
| Instal ulang atau hapus data mengosongkan profit tertutup, target, dan grafik | Riwayat hilang | Ekspor cadangan berkala (FR-26); Total aset tetap benar karena dari akun |
| Total aset terakhir di notifikasi jual bisa tertinggal dari harga sekarang | Persen di notifikasi sedikit berbeda dari Beranda | Diterima (Opsi A); bila Beranda belum pernah dibuka, notifikasi menampilkan rupiah profit saja |
| Asumsi: satu pengguna, satu akun Indodax, hanya pasangan IDR | Cakupan terbatas | Dicatat sebagai non-tujuan |
| Screener dianggap rekomendasi trading | Keputusan trading bergantung pada alat ini | Dibingkai sebagai filter kriteria milik pengguna; tidak ada kata "sinyal" atau "rekomendasi" di tampilan; tidak memengaruhi perhitungan trading |
| Harga dasar 1 jam belum akurat setelah layar baru dibuka | Lonjakan terlewat atau salah nilai selama masa pemanasan | Pair tidak dinilai sebelum snapshot minimal sekitar 10 menit; lama data dasar ditampilkan ringkas; snapshot dipertahankan selama proses hidup (usulan) |
| Lonjakan di koin yang tidak bisa dijual (spread besar, sepi) | Penanda lonjakan menyesatkan | Syarat dasar spread dan usia transaksi berlaku untuk filter Lonjakan |
| Polling ringkasan menambah pemakaian data | ±12 sampai 23 MB per jam bila tidak terkompresi, hanya saat layar Screener terbuka dan mode Lonjakan aktif | Interval dapat diatur; kompresi gzip akan diperiksa; pantau pemakaian data |
| Ambang default Screener tidak cocok untuk modal yang berubah-ubah | Daftar terlalu longgar atau sempit | Semua ambang bisa diatur; filter kedalaman memakai isian modal opsional; angka default disesuaikan dari uji lapangan |
