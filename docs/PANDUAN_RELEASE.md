# Panduan Rilis Uji: Catatan Scalping 0.9.0-uji

Tujuan: aplikasi dipakai nyata sehari-hari beberapa hari untuk mencari bug, kekurangan, dan peningkatan. Ini rilis uji pribadi (satu pengguna, satu HP), bukan rilis publik.

Jenis build (dipastikan 15 Nov 2026): sampai 0.8.0-uji, `tools/bangun_android.sh` memanggil `assembleDebug` (baris 76) dan blok `release` di `build.gradle.kts` tidak punya `signingConfig`. Mulai 0.9.0-uji APK dibangun sebagai **release**, ditandatangani dengan kunci debug yang sama dengan APK 0.8.0-uji, sehingga bisa menimpa tanpa uninstall. Lihat bagian 3.

## 1. Sebelum menandai rilis

Semua harus terpenuhi:

- [ ] `cargo test -p scalping-core` lulus (166) dan `cargo test -p scalping-ffi` lulus (43)
- [ ] Build bersih lewat `bash tools/bangun_android.sh --pasang <serial>` dan skrip menampilkan `OK: kode Rust di APK sama ...`
- [ ] Uji HP lulus (Fase 9 dan 10 sudah lulus 14 Nov 2026; setelah build release, ulangi cek harian di bagian 5 sekali)
- [ ] Cadangan data diekspor dari Setelan dan disimpan di luar HP (komputer atau Drive)
- [ ] `PROGRESS.md` dan PRD v1.12 sudah diperbarui
- [ ] `.env` dan isi `data/` tidak ikut ter-commit (`git status` bersih dari keduanya)

## 2. Commit dan tandai versi

Satu task satu commit. Urutan:

```
cd catatan-scalping
git add .
git commit -m "Rilis 0.9.0-uji: build release dan versi 0.9.0-uji"
git tag -a v0.9.0-uji -m "Rilis uji 0.9.0-uji"
git log --oneline -5
```

Salin hash dari `git log --oneline` ke bagian 8 `PROGRESS.md`. Bila punya remote: `git push` lalu `git push origin v0.9.0-uji`. Tag dibuat setelah build release lulus uji HP (bagian 3 dan 4), bukan sebelumnya.

## 3. Build release dan simpan APK

Perubahan untuk 0.9.0-uji: `versionCode = 3`, `versionName = "0.9.0-uji"`, dan blok `release` memakai `signingConfig = signingConfigs.getByName("debug")`. `isMinifyEnabled` tetap `false` (jangan diaktifkan: JNA dan UniFFI memakai refleksi, perlu aturan keep dulu).

Skrip `tools/bangun_android.sh` sudah diubah (15 Nov 2026): memanggil `assembleRelease`, membaca APK dari `app/aplikasi/build/outputs/apk/release`, dan mencetak sidik jari tanda tangan (SHA-256) setelah build. Rust sudah dibangun dengan `--release` sejak awal; pemeriksaan "kode Rust di APK sama" tetap berlaku.

```
touch core/src/*.rs ffi/src/lib.rs app/aplikasi/src/main/kotlin/id/catatanscalping/app/*.kt
bash tools/bangun_android.sh --pasang <serial-adb>
```

- Bila ragu build memakai sumber lama: `cargo clean` dulu, lalu build ulang.
- Salin APK hasil build (lokasinya dicetak skrip) ke folder rilis dan beri nama `catatan-scalping-0.9.0-uji.apk`. Simpan juga APK 0.8.0-uji untuk rollback.
- **Periksa tanda tangan sebelum memasang** (harus sama dengan APK lama, kalau tidak Android menolak menimpa):
  `~/android-sdk/build-tools/36.0.0/apksigner verify --print-certs <apk-baru>`
  (Skrip sudah mencetak baris SHA-256 ini setelah build.) Bandingkan dengan APK 0.8.0-uji (perintah sama). Bila berbeda, jangan pasang: ekspor cadangan dulu, karena satu-satunya jalan adalah uninstall (data hilang kecuali sudah diekspor).
- `assembleRelease` menjalankan pemeriksaan lint ringan (`lintVital`). Bila gagal karena lint, kirim pesan galatnya; jangan mematikan pemeriksaan tanpa melihatnya.
- Perbedaan release dibanding debug: APK tidak lagi debuggable (`adb run-as` tidak berfungsi). Perilaku aplikasi tidak berubah.

## 4. Memasang di HP

1. Ekspor cadangan dulu (menu ☰, Setelan, kartu Cadangan data).
2. Jangan uninstall. Pasang lewat skrip di atas.
3. Pasang lewat adb mematikan pantauan: buka aplikasi dan nyalakan lagi manual.
4. Pastikan izin notifikasi aktif dan optimasi baterai untuk aplikasi ini dimatikan.
5. Menu ☰, Setelan, ketuk "Log pantauan": harus ada baris "Proses dimulai · versi 0.9.0-uji".
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
- [ ] menu ☰, Setelan, Log pantauan: tidak ada baris CRASH, dan tidak ada "Proses sebelumnya berakhir" yang tidak kamu duga
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
| Koin terjual saat aplikasi tertutup, lalu aplikasi dibuka | Posisi baru tersinkron sendiri (T10.5); bila tidak, log memuat "Pantauan seharusnya menyala tetapi tidak berjalan" |
| Layar Screener dibuka lama (Lonjakan dan Daftar bersamaan) | Baris Lonjakan muncul setelah masa pemanasan; tidak ada notifikasi; catat pemakaian data seluler |
| Harga global saat exchange luar terblokir DNS | Status per exchange tampil; layar Screener tetap jalan; setelah Private DNS `dns.google`, status menjadi tersambung |
| Impor cadangan di HP yang sama | Data kembali normal (uji sekali saja, setelah ekspor) |

## 7. Mencatat dan melaporkan bug

Segera setelah kejadian (log hanya menyimpan sekitar 256 KB terakhir):

1. Menu ☰, Setelan, ketuk "Log pantauan", ketuk "Bagikan log" (400 baris terakhir; tidak memuat kunci API atau saldo).
2. Kirim ke Claude lewat Telegram, Drive, atau email, dengan format:

```
Waktu kejadian (WIB):
Layar / fitur:
Yang dilakukan:
Yang diharapkan:
Yang terjadi:
Koin / order terkait (jika ada):
Angka struk Indodax vs angka aplikasi (jika angka beda):
Versi: 0.9.0-uji
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
3. Tiap perbaikan menjadi task baru (T11.1, T11.2, dan seterusnya): tes Rust, build bersih, uji HP, lalu satu commit per task.
4. Perbarui `PROGRESS.md` setelah tiap task lulus.
5. Naikkan versi di `app/aplikasi/build.gradle.kts`: `versionCode` +1 dan `versionName` berurutan (0.9.1-uji, 0.9.2-uji, ...); `versionCode` sekarang 3. `versionCode` tidak boleh turun.
6. Tandai tag baru (`git tag -a v0.9.1-uji -m "..."`) setiap kumpulan perbaikan dipasang.

## 9. Rollback

Android menolak memasang versi lebih rendah di atas yang lebih baru. Bila versi baru bermasalah berat:

1. Ekspor cadangan dari versi yang berjalan (bila aplikasi masih bisa dibuka).
2. Uninstall, pasang APK lama.
3. (APK lama punya `versionCode` lebih rendah, jadi memang harus uninstall dulu.) Masukkan kunci API, impor cadangan, "Sinkron sekarang" satu kali, baru nyalakan pantauan.

Impor cadangan lama (versi 1.x) tetap bisa dipakai.

## 10. Kriteria selesai uji (USULAN, belum dikonfirmasi)

- 7 hari pemakaian nyata tanpa bug kritis yang belum diperbaiki
- Tidak ada trade ganda atau terlewat dibanding struk
- Pantauan tidak mati diam-diam; semua kematian proses punya penjelasan di log
- Total aset dan profit per trade konsisten dengan Indodax

Setelah terpenuhi, versi yang berjalan ditandai stabil (misalnya `v1.0.0`) dan item peningkatan dipilih untuk fase berikutnya.

## 11. Keterbatasan yang diketahui saat rilis

- Withdraw nyata belum teruji (bentuk baris dan mode fee).
- ID order di notifikasi beli: dua beli koin sama yang selesai nyaris bersamaan bisa menampilkan order yang sama.
- `PenerimaBoot.kt` belum diperiksa terhadap penjaga pantauan: bila boot gagal menyalakan layanan, penjaga berhenti tanpa peringatan.
- Penjaga pantauan tidak menyalakan layanan sendiri; ia hanya memberi peringatan.
- Force stop menghapus alarm penjaga, jadi tidak ada peringatan (tampak di log berikutnya).
- Estimasi hasil bersih memakai fee jual 0,31%; jual market bisa sekitar 0,61%.
- Koin yang dipegang sebelum aplikasi dipasang tercatat sebagai penyesuaian saat dijual.
- Pemakaian data Screener dan exchange luar belum diukur (gzip belum aktif; siklus Daftar mengambil ringkasan pasar dua kali). Pantau pemakaian data seluler.
- Status pantauan di Setelan bisa menampilkan "Berjalan" sesaat setelah layanan dimatikan sistem, sampai pantauan dimulai ulang atau dihentikan.
- Filter Screener "spread terikat TP" belum ada.
- Arah harga global baru muncul sekitar N menit setelah layar Screener dibuka.
