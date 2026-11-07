# scalping-core

Inti logika Catatan Scalping (Rust). Hanya membaca data Indodax, tidak pernah membuat order.

- `sign.rs` HMAC-SHA256 TAPI v2
- `client.rs` klien REST (GET saja): `/api/v2/account`, `/api/v2/myTrades`
- `model.rs` bentuk data `myTrades`
- `calc.rs` gabung per order, anti-dobel `tradeId`, modal efektif, TP/SL, profit jual (semua `Decimal`)
- `store.rs` penyimpanan SQLite lokal (anti-dobel `trade_id`)
- `sync.rs` rekonsiliasi REST per koin, snapshot saldo awal

Tes: `cargo test -p scalping-core`
