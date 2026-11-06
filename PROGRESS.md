# PROGRESS: Catatan Scalping

Terakhir diperbarui: 6 November 2026
Status: **Fase 0 (Validasi data) selesai.** Fase 1 menunggu keputusan teknologi.

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
| Selisih 1 rupiah antara hasil jual dan modal beli berikutnya (data 3 Nov) | Diduga pembulatan | Belum dikonfirmasi, tidak berdampak |
| Apakah sisa 3 IDR (anggaran tak terpakai) kembali ke saldo | Belum dicek di saldo | Belum dikonfirmasi. Tidak memengaruhi rumus karena saldo diambil dari API |

---

## 3. Tahap selanjutnya

### Satu-satunya hal yang masih mengganjal Fase 1
**Teknologi aplikasi belum diputuskan.** Syaratnya foreground service, WebSocket, dan penyimpanan lokal di Android.
- Kotlin native: paling andal untuk layanan latar, paling berat dipelajari.
- Flutter: satu bahasa (Dart), prototype mudah dipindah, layanan latar butuh plugin.
- React Native (Expo): cocok jika akrab JavaScript, layanan latar paling rapuh.

### Task berikutnya (urut)
| Task | Isi | Syarat |
| --- | --- | --- |
| T0.6 | Skrip tes token WebSocket privat dengan key v2 (`generate_token`). Ini satu-satunya ketidakpastian PRD yang tersisa | Key sudah ada |
| T1.1 | Inisialisasi proyek aplikasi dan koneksi akun (FR-01 sampai FR-03) | Teknologi diputuskan |
| T1.2 | Rekonsiliasi REST dengan anti-dobel berdasarkan `tradeId` (FR-09, FR-10) | T1.1 |
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

Kebutuhan: Python 3.8+ dan git. Skrip tidak butuh `pip install`.

1. Masuk ke folder proyek: `cd catatan-scalping`
2. Pastikan skrip versi terbaru: `python tools/fase0_validasi.py -h` harus menampilkan `--diagnosa`.
3. Buat key di indodax.com/trade_api: key **TAPI v2**, izin **baca saja**, tanpa pembatasan IP.
4. Salin `.env.example` menjadi `.env`, isi `INDODAX_API_KEY` dan `INDODAX_API_SECRET`.
5. Tes koneksi tanpa key: `python tools/fase0_validasi.py --diagnosa`
6. Tarik trade: `python tools/fase0_validasi.py icntidr --hari 7`
7. Hasil mentah tersimpan di `data/raw_<simbol>_<waktu>.json`.

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
├── docs/
│   ├── PRD_Aplikasi_Catatan_Trading_Crypto.md     (v1.1)
│   └── prototype/
│       └── Catatan_trading__prototype_UI.html
└── tools/
    └── fase0_validasi.py            (hanya baca; SHA256; --diagnosa)
```

### Rencana (bergantung teknologi yang dipilih)
```
app/        kode aplikasi mobile (folder dan bahasa mengikuti teknologi)
docs/       PRD, prototype, catatan hasil validasi
tools/      skrip bantu (validasi data, tes WebSocket)
```
Rincian `app/` dibuat setelah teknologi diputuskan.

---

## 6. Log commit
(isi hash dan pesan commit di sini)
