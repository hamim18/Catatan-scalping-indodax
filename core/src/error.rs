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
    #[error("aset {0} tidak ada di daftar saldo akun")]
    AsetTidakAda(String),
    #[error("jumlah koin nol, tidak bisa dihitung")]
    KoinNol,
    #[error("jumlah koin dijual ({jual}) melebihi posisi ({posisi})")]
    JualMelebihiPosisi { jual: String, posisi: String },
    #[error("penyimpanan lokal bermasalah: {0}")]
    Penyimpanan(String),
    #[error("key punya izin trade/withdraw; sinkron ditolak. Pakai key BACA SAJA")]
    KunciBukanBacaSaja,
    #[error("setelan TP/SL belum diisi")]
    SetelanBelumDiisi,
    #[error("setelan tidak valid: {0}")]
    SetelanTidakValid(String),
    #[error("rentang waktu {simbol} terlalu padat: {jumlah} trade dalam rentang minimal")]
    RentangTerlaluPadat { simbol: String, jumlah: usize },
    #[error("belum ada snapshot saldo awal; jalankan sinkron trade dulu")]
    BelumAdaSnapshot,
}

pub type Hasil<T> = Result<T, Error>;
