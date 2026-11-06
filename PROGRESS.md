# PROGRESS: Catatan Scalping

Terakhir diperbarui: 6 November 2026

## Aturan kerja
- Satu task selesai = satu `git commit`.
- Setelah tiap task: perbarui file ini, lalu commit.
- Jangan pernah commit `.env` (sudah ada di `.gitignore`).

## Keputusan
| Hal | Status |
| --- | --- |
| Teknologi aplikasi (Kotlin / Flutter / React Native) | BELUM diputuskan |
| Tombol "Jual di Indodax" (buka app Indodax atau dihapus) | BELUM diputuskan |
| Estimasi hasil bersih di bawah TP/SL | BELUM diputuskan |
| Setelan TP/SL baru berlaku untuk posisi lama? | BELUM diputuskan |
| Koin yang dipegang sebelum app dipasang | BELUM diputuskan |
| Cadangan data (ekspor) | BELUM diputuskan |
| Zona waktu WIB, minggu mulai Senin | Perlu konfirmasi |

## Fase 0: Validasi data
- [x] T0.1 Struktur repo, PRD, prototype, `.gitignore`, `.env.example`
- [x] T0.2 Skrip `tools/fase0_validasi.py` (dibuat, BELUM dijalankan)
- [ ] T0.3 Jalankan skrip dengan key baca saja untuk trade ICNT 5 Nov
- [ ] T0.4 Jawab: fee dari IDR atau koin? commission API = biaya struk?
- [ ] T0.5 Konfirmasi rumus modal efektif (PRD bagian 8)

## Fase 1 sampai 4
Belum dimulai (menunggu Fase 0 dan keputusan teknologi).

## Masalah terbuka
- Dokumen resmi simpang siur soal tanda tangan TAPI v2 (SHA256 atau SHA512). Skrip mencoba keduanya, catat mana yang berhasil.
- Format simbol (`icntidr` atau `icnt_idr`) perlu dipastikan.
- Nama parameter rentang waktu (`startTime`/`endTime`) perlu dipastikan.

## Log commit
(isi hash dan pesan commit di sini)
