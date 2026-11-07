# T1.2: panduan penempatan dan uji

Isi ZIP ini memakai struktur folder yang sama dengan repo `catatan-scalping/`.
Salin ke akar repo, timpa file yang sama.

| File di ZIP | Aksi |
| --- | --- |
| core/Cargo.toml | TIMPA (tambah rusqlite) |
| core/src/error.rs | TIMPA (tambah 3 varian error) |
| core/src/lib.rs | TIMPA (daftarkan modul store dan sync) |
| core/src/store.rs | BARU |
| core/src/sync.rs | BARU |
| core/README.md | TIMPA |
| ffi/src/lib.rs | TIMPA (tambah sinkron_sekarang dan order_terbaru) |
| app/aplikasi/src/main/kotlin/id/catatanscalping/app/MainActivity.kt | TIMPA (tombol Sinkron dan Lihat order) |
| PROGRESS.md | TIMPA (sudah diperbarui untuk T1.2) |

## Langkah
```
cd catatan-scalping
unzip -o catatan-scalping-T1.2.zip -d .      # atau salin manual sesuai tabel
cargo update                                  # ambil rusqlite
cargo test -p scalping-core                   # harus lolos: tes lama + 11 tes baru
bash tools/bangun_android.sh --pasang <serial-adb>
```

## Uji di HP
1. Buka aplikasi, kunci sudah tersimpan dari T1.1b.
2. Ketuk **Sinkron sekarang (T1.2)**. Tunggu sampai muncul "Sinkron selesai".
3. Cek: koin dipindai, trade baru, total tersimpan, dan daftar order di bawahnya (waktu WIB, beli/jual, koin, Rp).
4. Ketuk Sinkron sekali lagi: **Trade baru harus 0** dan total tidak berubah (bukti anti-dobel).
5. Cocokkan satu order dengan struk Indodax (beli = "Order Terisi", jual = "Total Diterima").

## Setelah lolos
1. Perbarui `PROGRESS.md`: ubah baris T1.2 dari "Kode siap, belum dibangun/diuji" menjadi **Selesai**, isi hash commit.
2. `git add . && git commit -m "T1.2: penyimpanan SQLite, rekonsiliasi REST, snapshot saldo awal"`

Bila build atau tes gagal, kirim pesan error persisnya; jangan commit dulu.
