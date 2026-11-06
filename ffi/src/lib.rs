//! Jembatan Rust -> Kotlin. Hanya lapisan tipis; logika ada di scalping-core.
//! Key dan secret tidak pernah dicatat ke log.
use scalping_core::IndodaxClient;

uniffi::setup_scaffolding!();

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ErrorApp {
    #[error("{pesan}")]
    Gagal { pesan: String },
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
    let akun = IndodaxClient::new(key, secret)
        .ambil_akun()
        .map_err(|e| ErrorApp::Gagal { pesan: e.to_string() })?;
    Ok(StatusKoneksi {
        jenis_akun: akun.account_type.clone(),
        jumlah_aset: akun.balances.len() as u32,
        idr_ada: akun.saldo_aset("idr").is_some(),
        baca_saja: akun.kunci_baca_saja(),
    })
}
