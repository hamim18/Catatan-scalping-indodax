use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("simbol tidak valid: {0} (hanya huruf kecil dan angka)")]
    SimbolTidakValid(String),
    #[error("gagal tersambung: {0}")]
    Jaringan(String),
    #[error("Indodax membalas HTTP {kode}: {isi_awal}")]
    Http { kode: u16, isi_awal: String },
    #[error("respons bukan JSON yang dikenali: {0}")]
    Json(String),
    #[error("biaya trade {trade_id} bukan dalam IDR (aset: {aset}); belum didukung")]
    FeeBukanIdr { trade_id: String, aset: String },
    #[error("jumlah koin nol, tidak bisa dihitung")]
    KoinNol,
    #[error("jumlah koin dijual ({jual}) melebihi posisi ({posisi})")]
    JualMelebihiPosisi { jual: String, posisi: String },
}

pub type Hasil<T> = Result<T, Error>;
