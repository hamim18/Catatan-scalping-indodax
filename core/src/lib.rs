//! Inti logika Catatan Scalping. Hanya MEMBACA data Indodax; tidak pernah
//! membuat order, membatalkan order, atau memindahkan dana.
pub mod calc;
pub mod client;
pub mod error;
pub mod model;
pub mod sign;

pub use calc::{gabung_per_order, OrderRingkas, Posisi, Sisi};
pub use client::IndodaxClient;
pub use error::{Error, Hasil};
pub use model::{parse_my_trades, Trade};
