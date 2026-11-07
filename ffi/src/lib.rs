//! Jembatan Rust -> Kotlin. Hanya lapisan tipis; logika ada di scalping-core.
//! Key dan secret tidak pernah dicatat ke log.
use scalping_core::calc::bulatkan_idr;
use scalping_core::{gabung_per_order, sinkron, IndodaxClient, Opsi, Penyimpanan, Sisi};

uniffi::setup_scaffolding!();

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ErrorApp {
    #[error("{pesan}")]
    Gagal { pesan: String },
}

fn gagal(e: impl std::fmt::Display) -> ErrorApp {
    ErrorApp::Gagal { pesan: e.to_string() }
}

/// Hasil tes koneksi (FR-02). Tidak memuat angka saldo.
#[derive(uniffi::Record)]
pub struct StatusKoneksi {
    pub jenis_akun: String,
    pub jumlah_aset: u32,
    pub idr_ada: bool,
    /// true bila key tidak punya izin trade maupun withdraw
    pub baca_saja: bool,
}

/// Hasil sinkron (T1.2).
#[derive(uniffi::Record)]
pub struct HasilSinkronUi {
    pub koin_dipindai: Vec<String>,
    pub trade_baru: u32,
    pub trade_total: u32,
    /// simbol yang ditolak Indodax (tidak ada pasangan IDR), dilewati
    pub koin_dilewati: Vec<String>,
    pub snapshot_awal_dibuat: bool,
}

/// Satu baris order (gabungan pengisian) untuk ditampilkan.
#[derive(uniffi::Record)]
pub struct BarisOrder {
    pub simbol: String,
    /// "beli" atau "jual"
    pub sisi: String,
    pub koin: String,
    /// beli: IDR keluar (nilai + fee); jual: IDR diterima (nilai - fee). Rupiah utuh.
    pub idr: String,
    pub waktu_ms: i64,
    pub pengisian: u32,
}

#[uniffi::export]
pub fn versi_core() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Memanggil GET /api/v2/account. Fungsi ini memblokir; panggil dari thread latar.
#[uniffi::export]
pub fn cek_koneksi(key: String, secret: String) -> Result<StatusKoneksi, ErrorApp> {
    let key = key.trim().to_string();
    let secret = secret.trim().to_string();
    if key.is_empty() || secret.is_empty() {
        return Err(ErrorApp::Gagal { pesan: "Key dan secret wajib diisi".into() });
    }
    let akun = IndodaxClient::new(key, secret).ambil_akun().map_err(gagal)?;
    Ok(StatusKoneksi {
        jenis_akun: akun.account_type.clone(),
        jumlah_aset: akun.balances.len() as u32,
        idr_ada: akun.saldo_aset("idr").is_some(),
        baca_saja: akun.kunci_baca_saja(),
    })
}

/// Rekonsiliasi REST (FR-10). Memblokir (bisa puluhan detik); panggil dari thread latar.
/// `path_db`: lokasi file database di penyimpanan privat aplikasi.
#[uniffi::export]
pub fn sinkron_sekarang(
    key: String,
    secret: String,
    path_db: String,
    hari_awal: u32,
    koin_tambahan: Vec<String>,
) -> Result<HasilSinkronUi, ErrorApp> {
    let key = key.trim().to_string();
    let secret = secret.trim().to_string();
    if key.is_empty() || secret.is_empty() {
        return Err(ErrorApp::Gagal { pesan: "Key dan secret wajib diisi".into() });
    }
    let mut simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    let opsi = Opsi { hari_awal: hari_awal.min(30), koin_tambahan, ..Opsi::default() };
    let h = sinkron(&IndodaxClient::new(key, secret), &mut simpan, &opsi).map_err(gagal)?;
    Ok(HasilSinkronUi {
        koin_dipindai: h.koin_dipindai,
        trade_baru: h.trade_baru as u32,
        koin_dilewati: h.koin_dilewati,
        trade_total: simpan.jumlah_trade().map_err(gagal)? as u32,
        snapshot_awal_dibuat: h.snapshot_awal_dibuat,
    })
}

/// Order terbaru dari database lokal (paling baru di atas). Tidak memakai jaringan.
#[uniffi::export]
pub fn order_terbaru(path_db: String, jumlah: u32) -> Result<Vec<BarisOrder>, ErrorApp> {
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    let trades = simpan.semua_trade().map_err(gagal)?;
    let mut order = gabung_per_order(&trades).map_err(gagal)?;
    order.sort_by_key(|o| std::cmp::Reverse(o.waktu_awal_ms));
    Ok(order
        .into_iter()
        .take(jumlah as usize)
        .map(|o| {
            let (sisi, uang) = match o.sisi {
                Sisi::Beli => ("beli", o.idr_keluar()),
                Sisi::Jual => ("jual", o.idr_diterima()),
            };
            BarisOrder {
                simbol: o.symbol.clone(),
                sisi: sisi.to_string(),
                koin: o.koin.normalize().to_string(),
                idr: bulatkan_idr(uang).to_string(),
                waktu_ms: o.waktu_awal_ms,
                pengisian: o.jumlah_pengisian as u32,
            }
        })
        .collect())
}
