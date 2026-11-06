# PRD: Aplikasi Catatan Trading Crypto

Versi 1.0 · 6 November 2026 · Pemilik produk: Raya Abdullah Ar Rasyid · Status: draf untuk ditinjau

## 1. Ringkasan

Aplikasi mobile pribadi untuk mencatat trading crypto di Indodax (pasangan IDR) secara otomatis lewat API resmi. Aplikasi hanya membaca data dan menghitung. Ia tidak pernah membuat order, membatalkan order, atau memindahkan dana.

Tiga hal yang diselesaikan:

1. Mengetahui pertumbuhan saldo (profit atau loss), terhadap target harian dan mingguan.
2. Setiap kali membeli koin, langsung muncul harga Take Profit (TP) dan Stop Loss (SL) sebagai notifikasi.
3. Open posisi tercatat otomatis dan tertutup otomatis ketika dijual.

## 2. Latar belakang dan masalah

Pengguna melakukan trading cepat (scalping) dan hasilnya hanya terlihat sebagai struk per transaksi. Akibatnya:

- Sulit melihat total profit atau loss setelah fee, dari hari ke hari.
- Tidak ada target yang terukur, harian maupun mingguan.
- Keputusan SL dan TP dibuat spontan, tanpa perhitungan yang menyertakan fee.
- Fee beli dan jual berbeda dan menggerus profit trade kecil. Contoh nyata: dari modal ±65 ribu, total fee 340 IDR memakan hampir seluruh profit 377 IDR.

## 3. Tujuan dan non-tujuan

**Tujuan**

- Pencatatan trade otomatis dan akurat, tanpa input manual.
- Angka saldo di aplikasi selalu cocok dengan saldo asli.
- Notifikasi TP/SL muncul cepat setelah order beli terisi.
- Logika perhitungan sesederhana mungkin.

**Non-tujuan (di luar cakupan versi ini)**

- Membuat atau membatalkan order, atau penarikan dana (izin API hanya baca).
- Harga pasar realtime dan penilaian open posisi berdasarkan harga pasar.
- Sinyal atau rekomendasi trading.
- Bursa selain Indodax, pasangan selain IDR.
- Multi-pengguna, akun cloud, dan platform selain mobile.

## 4. Pengguna dan konteks

- Satu pengguna: trader individu, gaya scalping, memakai market order untuk beli dan limit order untuk jual.
- Platform trading: Indodax. Target perangkat: HP Android (asumsi).
- Semua target dan batas dinyatakan dalam persen.

## 5. Definisi

| Istilah | Arti |
| --- | --- |
| Modal efektif | Total IDR yang keluar untuk beli, dibagi koin yang benar-benar diterima (fee beli otomatis ikut terhitung). Dihitung per koin. |
| Posisi | Koin yang sudah dibeli dan belum terjual. Satu posisi per koin, dirata-rata. |
| Saldo (versi aplikasi) | Saldo IDR (tersedia + terkunci) + nilai open posisi pada modal efektif. |
| Setoran bersih | Total deposit IDR dikurangi withdraw IDR pada suatu periode. |
| Modal aktif | Saldo awal + setoran bersih sejak aplikasi dipasang. |
| Snapshot | Catatan saldo di awal hari atau awal minggu. |
| TP / SL | Harga target jual untung (Take Profit) dan harga batas rugi (Stop Loss). |

## 6. Alur utama

1. **Setup**: pengguna memasukkan API key dan secret (izin baca saja) dan mengisi setelan persen (TP, SL, target harian, target mingguan). Aplikasi menyimpan snapshot saldo pertama.
2. **Beli di Indodax**: aplikasi mendeteksi order beli yang terisi, mencatatnya, membuat atau memperbarui posisi, menghitung modal efektif, TP, dan SL, lalu menampilkan notifikasi.
3. **Posisi terbuka**: tampil di daftar Open Posisi lengkap dengan modal efektif, TP, dan SL.
4. **Jual di Indodax**: aplikasi mendeteksi order jual, mengurangi atau menutup posisi, menghitung profit atau loss bersih, dan memperbarui progress target.
5. **Pemantauan**: pengguna melihat pencapaian harian dan mingguan, grafik saldo, dan riwayat trade.

## 7. Kebutuhan fungsional

### 7.1 Akun dan koneksi

- **FR-01** Pengguna memasukkan API key dan secret TAPI v2 dengan izin baca saja. Kunci disimpan di penyimpanan aman perangkat dan tidak pernah ditampilkan ulang secara utuh.
- **FR-02** Aplikasi memvalidasi kunci (membaca saldo akun) dan menampilkan status koneksi (tersambung, terputus, kunci tidak valid).
- **FR-03** Saat setup berhasil, aplikasi menyimpan snapshot saldo pertama sebagai saldo awal.

### 7.2 Setelan

- **FR-04** Pengguna mengatur, semuanya dalam persen: TP per trade, SL per trade, target harian, target mingguan.
- **FR-05** Perubahan setelan TP/SL berlaku untuk posisi baru. Posisi yang sudah terbuka tidak berubah (lihat pertanyaan terbuka).

### 7.3 Deteksi dan pencatatan trade

- **FR-06** Layanan latar (foreground service) menjaga koneksi WebSocket privat Indodax agar order terdeteksi secara realtime.
- **FR-07** Event pengisian order (FILL) dikumpulkan per order. Satu order bisa terisi beberapa kali. Order dianggap selesai saat status DONE.
- **FR-08** Event NEW, CANCELLED tanpa eksekusi, dan REJECTED tidak dicatat sebagai trade. Order yang dibatalkan setelah terisi sebagian dicatat sesuai bagian yang terisi.
- **FR-09** Anti-dobel: setiap pengisian dikenali dari ID trade yang unik. Mencatat ulang data yang sama tidak boleh membuat catatan ganda.
- **FR-10** Rekonsiliasi: setiap koneksi tersambung kembali dan setiap aplikasi dibuka, aplikasi menarik riwayat trade terbaru lewat REST dan melengkapi yang terlewat. Tersedia juga tombol sinkron manual.

### 7.4 Posisi dan TP/SL

- **FR-11** Satu posisi per koin. Beli berikutnya pada koin yang sama merata-ratakan modal efektif dan menghitung ulang TP/SL.
- **FR-12** Saat posisi dibuat atau diperbarui, aplikasi menghitung modal efektif, harga TP, dan harga SL dari persen di setelan.
- **FR-13** Jual sebagian mengurangi jumlah koin posisi, posisi tetap terbuka. Jual penuh menutup posisi dan memindahkannya ke riwayat.
- **FR-14** Layar Open Posisi menampilkan per posisi: koin, jumlah koin, modal efektif, TP, SL, waktu beli. Fungsi tombol Jual perlu diputuskan (lihat bagian 15).

### 7.5 Notifikasi

- **FR-15** Setelah beli: notifikasi berprioritas tinggi berisi koin, modal efektif, TP, dan SL.
- **FR-16** Setelah jual: notifikasi berisi hasil bersih (Rp dan %) dan progress target harian.
- **FR-17** Notifikasi TP/SL hanya informasi. Aplikasi tidak memasang order apa pun di Indodax.
- **FR-18** (disarankan) Notifikasi saat target harian atau mingguan tercapai.

### 7.6 Target harian dan mingguan

- **FR-19** Aplikasi menyimpan snapshot saldo di awal hari dan awal minggu. Bila layanan tidak aktif saat pergantian periode, snapshot diambil pada sinkron pertama setelahnya.
- **FR-20** Pencapaian periode dihitung dari selisih saldo (rumus di bagian 8), dikoreksi setoran bersih IDR pada periode itu.
- **FR-21** Tampilan: persen pencapaian, persen terhadap target, dan progress bar. Reset otomatis tiap hari dan tiap minggu.

### 7.7 Saldo dan riwayat

- **FR-22** Aplikasi menampilkan saldo versi aplikasi dan grafik pertumbuhannya, dengan dua garis: saldo dan modal aktif. Jarak antara keduanya adalah profit.
- **FR-23** Riwayat trade tertutup: koin, modal, total diterima, profit atau loss bersih (Rp dan %), waktu buka dan tutup.
- **FR-24** Aplikasi mengambil riwayat deposit dan withdraw IDR untuk menghitung setoran bersih dan modal aktif.
- **FR-25** (fase lanjut) Statistik: win rate, rata-rata profit dan loss, total fee, dan persen profit yang termakan fee.

## 8. Rumus dan aturan perhitungan

| Besaran | Rumus |
| --- | --- |
| Modal efektif per koin | IDR keluar ÷ koin diterima. Untuk posisi hasil beberapa kali beli: total IDR keluar ÷ total koin diterima. |
| Harga TP | Modal efektif × (1 + TP%) |
| Harga SL | Modal efektif × (1 − SL%) |
| Profit atau loss jual | IDR diterima − (modal efektif × jumlah koin yang dijual) |
| Saldo (versi aplikasi) | Saldo IDR (tersedia + terkunci) + Σ (jumlah koin posisi × modal efektif) |
| Modal aktif | Saldo awal + deposit IDR − withdraw IDR |
| Pencapaian periode (%) | (saldo sekarang − saldo awal periode − setoran bersih periode) ÷ saldo awal periode × 100 |
| Progress target (%) | Pencapaian periode ÷ target periode × 100 |

**Aturan**

- Fee beli sudah terkandung di modal efektif. Fee jual tidak dihitung di harga TP/SL, tetapi sudah terkandung di IDR diterima saat jual.
- Karena open posisi dinilai pada modal efektif, angka pencapaian hanya berubah setelah trade ditutup, bukan mengikuti naik-turun harga pasar.
- Seluruh perhitungan uang memakai tipe angka desimal presisi (bukan floating point). Jumlah koin sampai 8 desimal.

**Contoh nyata (ICNT, 5 November 2026)**

| Langkah | Data |
| --- | --- |
| Beli (market) 16:54 | Jumlah order 65.529 IDR, koin diterima 16,35667833, fee 133 |
| Modal efektif | 65.529 ÷ 16,35667833 ≈ 4.006,25 per koin (harga rata-rata 3.998; selisihnya karena fee) |
| TP 1% | ≈ 4.046 |
| SL 1% | ≈ 3.966 |
| Jual (limit) 17:06 | Total diterima 65.906 IDR, fee 207 |
| Hasil bersih | 65.906 − 65.529 = **+377 IDR (+0,58%)** |
| Total fee | 340 IDR (≈ 0,5% dari modal) |

Catatan: bila TP 1% tercapai, profit bersih ±0,7% karena fee jual. Bila SL 1% tercapai, rugi bersih ±1,3%.

## 9. Integrasi Indodax

Berdasarkan dokumentasi resmi Indodax (repo btcid/indodax-official-api-docs) per 6 November 2026. Seluruh poin harus diverifikasi lagi saat implementasi.

**Jalur realtime: WebSocket privat**

- Minta token lewat REST (POST generate\_token). Token berlaku 24 jam, jadi harus diminta ulang secara berkala. Permintaan token ditandatangani HMAC-SHA512.
- Terhubung ke WebSocket produksi, autentikasi dengan token, lalu subscribe ke channel pribadi.
- Event order\_update: NEW, FILL, DONE, CANCELLED, REJECTED. Event FILL membawa harga, jumlah koin, dan rincian biaya (fee, pajak, clearing).
- Hanya update order yang dikirim. Saldo tidak didorong lewat WebSocket.

**Jalur sinkron: REST TAPI v2** (https://api.indodax.com)

- GET /api/v2/myTrades: riwayat trade (harga, jumlah, nilai, total biaya, beli atau jual, waktu). Parameter symbol wajib, rentang maksimal 7 hari per permintaan, default 24 jam.
- GET /api/v2/account: saldo.
- GET /api/v2/fiat/orders: riwayat deposit dan withdraw IDR (rentang maksimal 30 hari).
- Batas laju umum: 300 permintaan per menit.
- Ditandatangani HMAC-SHA256 untuk REST v2, dengan header X-APIKEY dan Sign.

**Catatan penting**

- Metode lama tradeHistory dan orderHistory sudah dinonaktifkan sejak 7 April 2026. Jangan memakai library atau contoh yang masih memakainya.
- TAPI v2 butuh API key khusus v2 (dibuat di indodax.com/trade\_api). Key lama tidak bisa dipakai.
- Gunakan izin baca saja. Pembatasan IP untuk izin baca bersifat opsional. Karena IP jaringan seluler sering berubah, sebaiknya jangan diaktifkan agar koneksi tidak gagal.
- Rekonsiliasi riwayat harus berputar per koin dan per rentang 7 hari.
- Ketidakpastian yang harus divalidasi: (a) apakah fee dalam IDR atau dipotong dari koin yang diterima, (b) apakah total biaya di struk sama dengan nilai commission dari API, (c) apakah key v2 dapat dipakai untuk token WebSocket.

## 10. Model data

| Entitas | Isi utama |
| --- | --- |
| Setelan | TP%, SL%, target harian %, target mingguan %, zona waktu |
| Trade (pengisian) | ID trade (unik), ID order, koin, sisi (beli atau jual), harga, jumlah koin, biaya, waktu |
| Order | ID order, status akhir, total IDR, total koin |
| Posisi | koin, jumlah koin terbuka, total IDR modal, modal efektif, TP%, SL%, harga TP, harga SL, waktu buka, status |
| Trade tertutup | koin, modal, IDR diterima, profit atau loss, persen, waktu buka dan tutup |
| Snapshot | jenis (harian, mingguan, awal), waktu mulai, saldo |
| Arus kas | jenis (deposit atau withdraw IDR), jumlah, waktu, ID sumber (unik) |

Penyimpanan lokal di perangkat. Data disimpan sebagai catatan, bukan angka saldo mentah, sehingga mengoreksi satu catatan dapat menghitung ulang semuanya.

## 11. Kebutuhan non-fungsional

- **Keandalan**: koneksi realtime otomatis tersambung kembali, disusul rekonsiliasi REST agar tidak ada trade yang terlewat.
- **Kecepatan**: target notifikasi muncul dalam beberapa detik setelah order DONE (target awal, perlu diukur).
- **Baterai dan sistem**: layanan latar dengan satu notifikasi tetap yang bisa disenyapkan. Pengguna perlu mematikan optimasi baterai untuk aplikasi ini dan memberi izin notifikasi (Android 13 ke atas).
- **Keamanan**: kunci disimpan di penyimpanan aman perangkat, tidak masuk log, tidak masuk cadangan tanpa enkripsi, tidak dikirim ke pihak mana pun selain Indodax. Hanya izin baca.
- **Offline**: data lokal tetap bisa dilihat tanpa internet. Sinkron berjalan ulang saat koneksi kembali.
- **Ketepatan**: aplikasi menyediakan tombol koreksi saldo. Selisih terhadap saldo Indodax dicatat sebagai penyesuaian, bukan dihapus diam-diam.
- **Zona waktu dan periode**: WIB, minggu dimulai hari Senin (asumsi).

## 12. Kasus khusus

| Kasus | Penanganan yang diusulkan |
| --- | --- |
| Beli beberapa kali pada koin yang sama | Satu posisi, modal efektif dirata-rata, TP/SL dihitung ulang |
| Jual sebagian | Posisi tetap terbuka dengan sisa koin, profit dihitung proporsional |
| Order terisi beberapa kali | Dijumlahkan per order, dicatat saat DONE |
| Koneksi putus atau layanan dimatikan sistem | Rekonsiliasi saat tersambung kembali atau aplikasi dibuka |
| Pergantian hari tanpa layanan aktif | Snapshot diambil pada sinkron pertama setelahnya |
| Deposit atau withdraw IDR di tengah periode | Dikeluarkan dari pencapaian lewat setoran bersih |
| Menjual koin yang dibeli sebelum aplikasi dipasang | Tidak ada posisi. Dicatat sebagai penyesuaian dan dikeluarkan dari pencapaian (profit dianggap 0) kecuali pengguna mengisi modalnya |
| Withdraw atau deposit koin | Diabaikan di versi ini. Pengguna dapat mengoreksi lewat tombol koreksi saldo |
| Token WebSocket kedaluwarsa | Minta token baru dan hubungkan ulang secara otomatis |
| Kunci API dicabut atau tidak valid | Tampilkan status terputus dan minta kunci baru |

## 13. Metrik keberhasilan

Aplikasi pribadi, jadi ukurannya sederhana (target awal, bisa disesuaikan):

- Selisih saldo aplikasi dan saldo Indodax setelah sinkron: nol atau hanya penyesuaian yang tercatat.
- Tidak ada trade ganda dan tidak ada trade terlewat dalam satu minggu pemakaian.
- Notifikasi TP/SL muncul untuk setiap pembelian yang terisi.
- Layanan latar tetap hidup sepanjang jam trading tanpa dimatikan sistem.

## 14. Rencana rilis bertahap

| Fase | Isi | Hasil |
| --- | --- | --- |
| 0. Validasi data | Ambil trade nyata (misalnya ICNT) lewat myTrades dengan key baca saja. Cek format fee dan koin diterima, dan kecocokan dengan struk. | Rumus modal efektif terkonfirmasi |
| 1. MVP | Koneksi akun, WebSocket realtime, anti-dobel, posisi terbuka, TP/SL, notifikasi beli dan jual | Posisi dan notifikasi bekerja |
| 2. Target dan saldo | Setelan target, snapshot harian dan mingguan, riwayat deposit dan withdraw IDR, pencapaian dan progress | Target terukur |
| 3. Riwayat dan grafik | Daftar trade tertutup, grafik saldo dan modal aktif, tombol koreksi saldo | Pertumbuhan terlihat |
| 4. Statistik | Win rate, total fee, persen profit termakan fee | Evaluasi gaya trading |

## 15. Pertanyaan terbuka

1. Fee beli dipotong dari koin atau dari IDR di API? Ini menentukan detail rumus modal efektif (diselesaikan di Fase 0).
2. Apakah total biaya di struk sama dengan commission dari API (diselesaikan di Fase 0)?
3. Fungsi tombol Jual di Open Posisi: karena aplikasi tidak membuat order, apakah tombol hanya membuka aplikasi Indodax, atau dihapus dan penjualan murni terdeteksi otomatis?
4. Apakah baris kecil “estimasi hasil bersih” (memperhitungkan fee jual) ditampilkan di bawah TP dan SL, atau dibiarkan polos?
5. Setelan TP/SL baru: berlaku untuk posisi baru saja, atau boleh diubah per posisi yang sedang terbuka?
6. Koin yang sudah dipegang sebelum aplikasi dipasang: diabaikan atau diisi modalnya secara manual?
7. Teknologi pembuatan aplikasi belum diputuskan (syarat: layanan latar, WebSocket, penyimpanan lokal).
8. Apakah perlu cadangan data (ekspor), dan bila ya, ke mana?
9. Konfirmasi: hari mulai minggu (Senin) dan zona waktu (WIB).

## 16. Risiko dan asumsi

| Risiko atau asumsi | Dampak | Mitigasi |
| --- | --- | --- |
| Android mematikan layanan latar | Notifikasi terlambat | Pengecualian optimasi baterai, rekonsiliasi saat dibuka |
| Format fee di API berbeda dari struk | Modal efektif salah | Fase 0 membandingkan data nyata dengan struk |
| Perubahan atau penonaktifan endpoint Indodax | Sinkron gagal | Hanya memakai endpoint resmi TAPI v2, pantau changelog |
| Batas laju (300 per menit) | Rekonsiliasi lambat | Rekonsiliasi per koin yang aktif, jeda antar permintaan |
| Kebocoran kunci API | Data akun terbaca | Izin baca saja, simpan aman, tanpa izin trade atau withdraw |
| Pembatasan IP pada key | Koneksi gagal di jaringan seluler | Jangan aktifkan pembatasan IP untuk key baca |
| Asumsi: satu pengguna, satu akun Indodax, hanya pasangan IDR | Cakupan terbatas | Dicatat sebagai non-tujuan |
