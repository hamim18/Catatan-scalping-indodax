# Panduan T9.5: Harga global (tahap 1)

## Isi ZIP dan penempatan (salin ke akar repo `catatan-scalping/`, timpa bila sudah ada)
| Berkas | Status |
| --- | --- |
| `core/src/harga_global.rs` | BARU |
| `core/src/harga_global_setelan.rs` | BARU |
| `core/src/client.rs` | DIGANTI (hanya menambah `KlienGlobal`, `use`, 1 tes) |
| `core/src/lib.rs` | DIGANTI (2 `mod` + ekspor baru) |
| `ffi/src/global_ffi.rs` | BARU |
| `ffi/src/screener_ffi.rs` | DIGANTI (kait harga global; ~40 baris berubah) |
| `app/aplikasi/src/main/kotlin/id/catatanscalping/app/LayarScreener.kt` | DIGANTI |
| `tools/uji_koneksi_global.sh` | BARU (opsional) |

## Satu langkah manual: `ffi/src/lib.rs`
Saya tidak punya berkas ini, jadi tidak saya ubah. Tambahkan SATU baris di dekat `mod screener_ffi;`:
```
mod global_ffi;
```
Tanpa baris ini build gagal ("unresolved import `super::global_ffi`").

## `Cargo.toml` tidak berubah. Tidak ada dependensi baru.

## Tes dan build
```
cd catatan-scalping
touch core/src/*.rs ffi/src/*.rs app/aplikasi/src/main/kotlin/id/catatanscalping/app/*.kt
cargo test -p scalping-core        # harapan: 158
cargo test -p scalping-ffi         # harapan: 39
bash tools/bangun_android.sh --pasang <serial-adb>
```
Hitungan tes baru yang saya lihat di harness: harga_global 11, harga_global_setelan 4, client 1 (total core +16); global_ffi 7. Jadi harapan: core 158, ffi 39. Bila angkanya lain, kabari saya.
Jangan lupa `touch` (pelajaran T5.2) dan ekspor cadangan sebelum memasang ulang; pantauan harus dinyalakan lagi manual.

## Sebelum uji HP: cek jaringan (menentukan apakah fitur akan menampilkan data)
Di Termux, dengan Private DNS aktif: `bash tools/uji_koneksi_global.sh` lalu kirim hasilnya (kode HTTP, byte, detik, baris USDTIDR, ukuran gzip).

## Uji HP
1. Tab Screener: di bawah status muncul baris `Global: ✓ Binance ✓ Bybit ✓ OKX · kurs ... vs ... (...)`.
2. Tiap baris pair punya dua baris tambahan: `wajar ... (+x%)` dan harga tiap exchange.
3. Putuskan Private DNS: status berubah menjadi ✗ dengan alasan dan petunjuk DNS; daftar Screener tetap muncul (tidak macet).
4. Ambang > Harga global: matikan ("tidak") lalu Simpan ambang; baris global dan status hilang dan tidak ada request exchange luar.
5. Mode Lonjakan juga menampilkan baris global.
6. Bandingkan satu koin dengan skrip `harga_wajar_indodax.py` pada waktu yang sama (selisih persen harus nyaris sama).

## Setelah lulus
1. Tempel `PROGRESS_T9.5_tempel.md` ke `PROGRESS.md` (ubah status T9.5 menjadi selesai dan isi jumlah tes aktual).
2. `git add . && git commit -m "T9.5: harga global exchange luar (Binance, Bybit, OKX)"`
3. PRD v1.12 belum saya kerjakan (butuh isi PRD); sebut saja bila ingin saya selaraskan FR-38 sampai FR-40.
