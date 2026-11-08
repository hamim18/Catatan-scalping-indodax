//! Inti logika Catatan Scalping. Hanya MEMBACA data Indodax; tidak pernah
//! membuat order, membatalkan order, atau memindahkan dana.
pub mod account;
pub mod arus_kas;
pub mod calc;
pub mod client;
pub mod error;
pub mod harga;
pub mod model;
pub mod periode;
pub mod posisi;
pub mod statistik;
pub mod sign;
pub mod store;
pub mod sync;
pub mod ws;
pub mod ws_klien;

pub use account::{parse_akun, Akun, Saldo};
pub use arus_kas::{
    parse_fiat_orders, setoran_bersih, sinkron_arus_kas, ArusKas, HasilArusKas, JenisArusKas,
};
pub use calc::{gabung_per_order, OrderRingkas, Posisi, Sisi};
pub use client::IndodaxClient;
pub use error::{Error, Hasil};
pub use harga::{hitung_total_aset, parse_ticker_all, AsetNilai, HargaIdr, TotalAset};
pub use model::{parse_my_trades, Trade};
pub use periode::{awal_hari_wib, awal_minggu_wib, DataSaldo, Pencapaian, RingkasanTarget, SelisihSaldo, TitikGrafik};
pub use posisi::{hitung, HasilHitung, Penyesuaian, PosisiTerbuka, RiwayatSetelan, TradeTertutup};
pub use statistik::{hitung_statistik, Statistik};
pub use store::{KoreksiSaldo, Penyimpanan};
pub use sync::{sinkron, sinkron_koin, HasilSinkron, Opsi, SumberTrade};
pub use ws::{
    parse_pesan, parse_token, AlasanPemicu, EventOrder, PemicuSinkron, PesanMasuk, StatusOrder, TokenWs,
};
pub use ws_klien::{pantau, KejadianPantau};
