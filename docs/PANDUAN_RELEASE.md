# Panduan Rilis Uji: Catatan Scalping 0.8.0-uji

Tujuan: aplikasi dipakai nyata sehari-hari beberapa hari untuk mencari bug, kekurangan, dan peningkatan. Ini rilis uji pribadi (satu pengguna, satu HP), bukan rilis publik.

Catatan: jenis build dan tanda tangan APK dari `tools/bangun_android.sh` (debug atau release) belum saya lihat. Lihat bagian 3 untuk cara memeriksanya.

## 1. Sebelum menandai rilis

Semua harus terpenuhi:

- [ ] `cargo test -p scalping-core` lulus (99) dan `cargo test -p scalping-ffi` lulus (24)
- [ ] Build bersih lewat `bash tools/bangun_android.sh --pasang <serial>` dan skrip menampilkan `OK: kode Rust di APK sama ...`
- [ ] Uji HP lulus (T8.1 sudah lulus 8 Nov 2026)
- [ ] Cadangan data diekspor dari Setelan dan disimpan di luar HP (komputer atau Drive)
- [ ] `PROGRESS.md` sudah diperbarui
- [ ] `.env` dan isi `data/` tidak ikut ter-commit (`git status` bersih dari keduanya)

## 2. Commit dan tandai versi

Satu task satu commit. Urutan:

```
cd catatan-scalping
git add .
git commit -m "T8.1: log diagnostik permanen dan penjaga pantauan"
git add .
git commit -m "Dokumen: README, panduan rilis uji, PROGRESS (T8.1 lulus)"
git tag -a v0.8.0-uji -m "Rilis uji 0.8.0-uji"
git log --oneline -5
```

Salin hash dari `git log --oneline` ke bagian 6 `PROGRESS.md`. Bila kamu punya remote, `git push` lalu `git push origin v0.8.0-uji`.

Bila perubahan T8.1 dan dokumen sudah terlanjur ada dalam satu commit, tidak masalah; yang penting tag dibuat setelah semuanya ter-commit.

## 3. Build dan simpan APK

```
touch core/src/*.rs ffi/src/lib.rs app/aplikasi/src/main/kotlin/id/catatanscalping/app/*.kt
bash tools/bangun_android.sh --pasang <serial-adb>
```

- Bila ragu build memakai sumber lama: `cargo clean` dulu, lalu build ulang.
- Salin APK hasil build (lokasinya dicetak skrip) ke folder rilis dan beri nama `catatan-scalping-0.8.0-uji.apk`. Simpan juga APK versi sebelumnya untuk rollback.
- **Periksa jenis build** (belum saya lihat): jalankan
  `grep -n "buildTypes\|signingConfig\|isMinify" app/aplikasi/build.gradle.kts`
  dan kirim hasilnya. Ini menentukan apakah APK baru bisa menimpa versi lama tanpa uninstall (tanda tangan harus sama). Selama build dari komputer yang sama, biasanya sama; kalau komputer atau keystore berganti, Android menolak pembaruan dan kamu harus uninstall (data hilang kecuali sudah diekspor).

## 4. Memasang di HP

1. Ekspor cadangan dulu (Setelan, kartu Cadangan data).
2. Jangan uninstall. Pasang lewat skrip di atas.
3. Pasang lewat adb mematikan pantauan: buka aplikasi dan nyalakan lagi manual.
4. Pastikan izin notifikasi aktif dan optimasi baterai untuk aplikasi ini dimatikan.
5. Setelan, ketuk "Log pantauan": harus ada baris "Proses dimulai · versi 0.8.0-uji".
6. Pasang saat tidak ada order limit menggantung, supaya tidak ada pantauan yang terputus saat order terisi.

## 5. Aturan selama masa uji

- Pakai seperti biasa, dengan modal yang siap kamu pertaruhkan. Aplikasi hanya membaca, tapi angka TP/SL dan estimasi hanya acuan; keputusan trading tetap milikmu.
- Struk Indodax adalah acuan kebenaran. Bila angka aplikasi beda dengan struk, itu bug yang perlu dicatat.
- Ekspor cadangan tiap malam (atau minimal tiap hari ada trade).
- Jangan uninstall atau hapus data aplikasi selama uji.
- Jangan ubah kunci API ke yang punya izin trade/withdraw (akan ditolak).
- Jangan matikan pantauan hanya untuk menghemat baterai; justru ketahanan pantauan yang sedang diuji.

### Cek harian (sekitar 1 menit)
- [ ] Chip Beranda "Terhubung" dan notifikasi pantauan masih ada
- [ ] Setelan, Log pantauan: tidak ada baris CRASH, dan tidak ada "Proses sebelumnya berakhir" yang tidak kamu duga
- [ ] Total aset kira-kira sama dengan di aplikasi Indodax
- [ ] Profit trade hari ini sama dengan struk
- [ ] Tidak ada notifikasi "Peringatan pantauan" yang terlewat

## 6. Skenario uji lapangan

Centang saat sudah ditemui secara alami (tidak perlu dipaksa semua):

| Skenario | Yang diperiksa |
| --- | --- |
| Beli market | Notifikasi beli muncul cepat; modal efektif, TP, SL sesuai struk |
| Beli limit | Sama; tercatat saat DONE |
| Beli tambahan koin yang sama | Notifikasi memuat order ini dan posisi keseluruhan; modal dirata-rata |
| Jual penuh | Hasil bersih cocok struk; posisi hilang dari tab Posisi |
| Jual sebagian | Posisi tersisa benar; profit proporsional |
| Order limit terisi sebagian lalu dibatalkan | Tercatat sesuai bagian terisi |
| Target harian tercapai | Penanda "TERCAPAI" muncul sekali |
| Pergantian hari (00:00 WIB) dan Senin | Profit harian dan mingguan reset benar |
| HP restart | Pantauan menyala lagi bila terakhir menyala; tetap mati bila sudah dihentikan |
| Mode pesawat sekitar 10 menit | Peringatan "terputus" muncul, hilang setelah tersambung; rekonsiliasi menangkap trade yang terlewat |
| Layar mati lama, hemat baterai | Pantauan tidak mati diam-diam; bila mati, log menjelaskan sebabnya |
| Wifi dan data seluler bergantian | Sambung ulang otomatis |
| Beli dan jual dalam hitungan menit | Tidak ada trade ganda atau terlewat |
| Impor cadangan di HP yang sama | Data kembali normal (uji sekali saja, setelah ekspor) |

## 7. Mencatat dan melaporkan bug

Segera setelah kejadian (log hanya menyimpan sekitar 256 KB terakhir):

1. Setelan, ketuk "Log pantauan", ketuk "Bagikan log" (400 baris terakhir; tidak memuat kunci API atau saldo).
2. Kirim ke Claude lewat Telegram, Drive, atau email, dengan format:

```
Waktu kejadian (WIB):
Layar / fitur:
Yang dilakukan:
Yang diharapkan:
Yang terjadi:
Koin / order terkait (jika ada):
Angka struk Indodax vs angka aplikasi (jika angka beda):
Versi: 0.8.0-uji
```

3. Sertakan screenshot bila perlu. Samarkan saldo dan jangan pernah kirim API key atau secret.

### Tingkat keparahan

| Tingkat | Contoh | Tindakan |
| --- | --- | --- |
| Kritis | Angka salah (profit, modal, total aset), trade terlewat atau ganda, pantauan mati diam-diam, crash, data hilang | Laporkan segera; perbaikan diutamakan |
| Sedang | Notifikasi terlambat atau salah tampil, layar tidak menyegarkan, peringatan palsu | Dikumpulkan, diperbaiki berkelompok |
| Rendah / peningkatan | Tata letak, kata-kata, ide fitur | Dicatat untuk versi berikutnya |

## 8. Alur perbaikan dan versi

1. Kumpulkan laporan (boleh beberapa sekaligus, kritis jangan ditunda).
2. Kirim ke Claude beserta log. Berkas kode yang relevan akan diminta, tidak diasumsikan.
3. Tiap perbaikan menjadi task baru (T8.2, T8.3, dan seterusnya): tes Rust, build bersih, uji HP, lalu satu commit per task.
4. Perbarui `PROGRESS.md` setelah tiap task lulus.
5. Naikkan versi di `app/aplikasi/build.gradle.kts`: `versionCode` +1 dan `versionName` berurutan (0.8.1-uji, 0.8.2-uji, ...). `versionCode` tidak boleh turun.
6. Tandai tag baru (`git tag -a v0.8.1-uji -m "..."`) setiap kumpulan perbaikan dipasang.

## 9. Rollback

Android menolak memasang versi lebih rendah di atas yang lebih baru. Bila versi baru bermasalah berat:

1. Ekspor cadangan dari versi yang berjalan (bila aplikasi masih bisa dibuka).
2. Uninstall, pasang APK lama.
3. Masukkan kunci API, impor cadangan, "Sinkron sekarang" satu kali, baru nyalakan pantauan.

Impor cadangan lama (versi 1.x) tetap bisa dipakai.

## 10. Kriteria selesai uji (USULAN, belum dikonfirmasi)

- 7 hari pemakaian nyata tanpa bug kritis yang belum diperbaiki
- Tidak ada trade ganda atau terlewat dibanding struk
- Pantauan tidak mati diam-diam; semua kematian proses punya penjelasan di log
- Total aset dan profit per trade konsisten dengan Indodax

Setelah terpenuhi, versi yang berjalan ditandai stabil (misalnya `v0.9.0`) dan item peningkatan dipilih untuk fase berikutnya.

## 11. Keterbatasan yang diketahui saat rilis

- Withdraw nyata belum teruji (bentuk baris dan mode fee).
- ID order di notifikasi beli: dua beli koin sama yang selesai nyaris bersamaan bisa menampilkan order yang sama.
- `PenerimaBoot.kt` belum diperiksa terhadap penjaga pantauan: bila boot gagal menyalakan layanan, penjaga berhenti tanpa peringatan.
- Penjaga pantauan tidak menyalakan layanan sendiri; ia hanya memberi peringatan.
- Force stop menghapus alarm penjaga, jadi tidak ada peringatan (tampak di log berikutnya).
- Estimasi hasil bersih memakai fee jual 0,31%; jual market bisa sekitar 0,61%.
- Koin yang dipegang sebelum aplikasi dipasang tercatat sebagai penyesuaian saat dijual.
