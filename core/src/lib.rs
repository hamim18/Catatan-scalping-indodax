//! Inti logika Catatan Scalping. Hanya MEMBACA data Indodax; tidak pernah
//! membuat order, membatalkan order, atau memindahkan dana.
pub mod account;
pub mod calc;
pub mod client;
pub mod error;
pub mod model;
pub mod posisi;
pub mod sign;
pub mod store;
pub mod sync;
pub mod ws;

pub use account::{parse_akun, Akun, Saldo};
pub use calc::{gabung_per_order, OrderRingkas, Posisi, Sisi};
pub use client::IndodaxClient;
pub use error::{Error, Hasil};
pub use model::{parse_my_trades, Trade};
pub use posisi::{hitung, HasilHitung, Penyesuaian, PosisiTerbuka, RiwayatSetelan, TradeTertutup};
pub use store::Penyimpanan;
pub use sync::{sinkron, sinkron_koin, HasilSinkron, Opsi, SumberTrade};
pub use ws::{
    parse_pesan, parse_token, AlasanPemicu, EventOrder, PemicuSinkron, PesanMasuk, StatusOrder, TokenWs,
};
