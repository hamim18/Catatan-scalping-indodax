# Catatan Scalping

Aplikasi mobile pribadi pencatat trading crypto Indodax (pasangan IDR). Hanya membaca data, tidak pernah membuat order.

- Konsep: `docs/PRD_Aplikasi_Catatan_Trading_Crypto.md`
- Tampilan: buka `docs/prototype/Catatan_trading__prototype_UI.html` di browser
- Status kerja: `PROGRESS.md`

## Menjalankan Fase 0
1. Pasang Python 3.8+ (cek: `python --version`).
2. Buat API key di indodax.com/trade_api: key v2, izin BACA SAJA, tanpa pembatasan IP.
3. Salin `.env.example` menjadi `.env`, isi key dan secret.
4. Jalankan: `python tools/fase0_validasi.py icntidr --hari 3`
5. Kirim hasil cetakannya ke Claude.
