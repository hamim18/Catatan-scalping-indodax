//! Jembatan Rust -> Kotlin. Hanya lapisan tipis; logika ada di scalping-core.
//! Key dan secret tidak pernah dicatat ke log.
use rust_decimal::Decimal;
use scalping_core::calc::bulatkan_idr;
use scalping_core::posisi::periksa_persen;
use scalping_core::{
    gabung_per_order, hitung, sinkron, IndodaxClient, Opsi, Penyimpanan, RiwayatSetelan, Sisi,
};
use std::str::FromStr;
use scalping_core::{pantau, KejadianPantau};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

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

/// Posisi terbuka lengkap dengan TP/SL (T1.3). Semua angka sebagai teks.
#[derive(uniffi::Record)]
pub struct BarisPosisi {
    pub simbol: String,
    pub koin: String,
    /// IDR per koin, 2 desimal
    pub modal_efektif: String,
    /// total IDR modal untuk koin yang masih dipegang (rupiah utuh)
    pub total_modal: String,
    pub tp_persen: String,
    pub sl_persen: String,
    pub harga_tp: String,
    pub harga_sl: String,
    pub waktu_buka_ms: i64,
}

/// Satu penjualan yang cocok dengan posisi.
#[derive(uniffi::Record)]
pub struct BarisTertutup {
    pub simbol: String,
    pub koin: String,
    pub modal: String,
    pub diterima: String,
    pub profit: String,
    pub persen: String,
    pub waktu_buka_ms: i64,
    pub waktu_tutup_ms: i64,
}

#[derive(uniffi::Record)]
pub struct RingkasanHitung {
    pub posisi: Vec<BarisPosisi>,
    /// terbaru di atas
    pub tertutup: Vec<BarisTertutup>,
    pub total_profit: String,
    pub jumlah_penyesuaian: u32,
    /// trade dengan waktu di bawah ini diabaikan
    pub sejak_ms: i64,
}

fn desimal(teks: &str) -> Result<Decimal, ErrorApp> {
    // pengguna Indonesia boleh mengetik koma
    Decimal::from_str(&teks.trim().replace(',', "."))
        .map_err(|_| ErrorApp::Gagal { pesan: format!("angka tidak valid: {teks}") })
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

/// Simpan setelan TP/SL (persen). Hanya berlaku untuk posisi yang dibuka SETELAH ini
/// (kecuali belum ada setelan sebelumnya). Bila sama dengan setelan terakhir, tidak menambah catatan.
#[uniffi::export]
pub fn simpan_setelan(path_db: String, tp_persen: String, sl_persen: String) -> Result<(), ErrorApp> {
    let tp = desimal(&tp_persen)?;
    let sl = desimal(&sl_persen)?;
    periksa_persen(tp, sl).map_err(gagal)?;
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    if let Some((_, tp0, sl0)) = simpan.riwayat_setelan().map_err(gagal)?.last() {
        if *tp0 == tp && *sl0 == sl {
            return Ok(());
        }
    }
    let sekarang = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    simpan.tambah_setelan(sekarang, tp, sl).map_err(gagal)
}

/// Hitung ulang posisi, TP/SL, dan trade tertutup dari trade tersimpan. Tanpa jaringan.
/// `semua_riwayat` = true: hitung dari semua trade tersimpan (untuk uji).
/// false: hanya trade sejak snapshot saldo awal (aturan PRD).
#[uniffi::export]
pub fn hitung_posisi(path_db: String, semua_riwayat: bool) -> Result<RingkasanHitung, ErrorApp> {
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    let sejak_ms = if semua_riwayat {
        0
    } else {
        simpan
            .snapshot_awal()
            .map_err(gagal)?
            .map(|(w, _)| w)
            .ok_or_else(|| ErrorApp::Gagal { pesan: "Belum ada snapshot saldo awal; sinkron dulu".into() })?
    };
    let trades = simpan.semua_trade().map_err(gagal)?;
    let setelan = RiwayatSetelan::baru(simpan.riwayat_setelan().map_err(gagal)?);
    let h = hitung(&trades, sejak_ms, &setelan).map_err(gagal)?;

    let total_profit: Decimal = h.tertutup.iter().map(|t| t.profit).sum();
    let mut tertutup = h.tertutup;
    tertutup.sort_by_key(|t| std::cmp::Reverse(t.waktu_tutup_ms));

    Ok(RingkasanHitung {
        posisi: h
            .posisi
            .iter()
            .map(|p| BarisPosisi {
                simbol: p.simbol.clone(),
                koin: p.koin.normalize().to_string(),
                modal_efektif: p.modal_efektif.round_dp(2).to_string(),
                total_modal: bulatkan_idr(p.total_idr_keluar).to_string(),
                tp_persen: p.tp_persen.normalize().to_string(),
                sl_persen: p.sl_persen.normalize().to_string(),
                harga_tp: bulatkan_idr(p.harga_tp).to_string(),
                harga_sl: bulatkan_idr(p.harga_sl).to_string(),
                waktu_buka_ms: p.waktu_buka_ms,
            })
            .collect(),
        tertutup: tertutup
            .iter()
            .map(|t| BarisTertutup {
                simbol: t.simbol.clone(),
                koin: t.koin.normalize().to_string(),
                modal: bulatkan_idr(t.modal).to_string(),
                diterima: bulatkan_idr(t.idr_diterima).to_string(),
                profit: bulatkan_idr(t.profit).to_string(),
                persen: t.persen.round_dp(2).to_string(),
                waktu_buka_ms: t.waktu_buka_ms,
                waktu_tutup_ms: t.waktu_tutup_ms,
            })
            .collect(),
        total_profit: bulatkan_idr(total_profit).to_string(),
        jumlah_penyesuaian: h.penyesuaian.len() as u32,
        sejak_ms,
    })
}

// ---------------------------------------------------------------------------
// T1.4c: pantauan realtime (WebSocket privat + pemicu sinkron REST).
// Logika ada di scalping_core::pantau; di sini hanya pembungkus thread dan
// penerusan kejadian ke Kotlin lewat callback.
// ---------------------------------------------------------------------------

/// Kejadian pantauan untuk Kotlin (cermin KejadianPantau, semua angka u32).
#[derive(uniffi::Enum)]
pub enum KejadianUi {
    Tersambung,
    Terputus { alasan: String, coba_lagi_detik: u32 },
    /// `sisi`: "beli" atau "jual"
    OrderSelesai { simbol: String, order_id: String, sisi: String, trade_baru: u32 },
    Rekonsiliasi { trade_baru: u32 },
    Galat { pesan: String },
    /// Pantauan berhenti. `galat` = None bila dihentikan secara normal.
    Berhenti { galat: Option<String> },
}

/// Diimplementasikan di Kotlin. Dipanggil dari thread latar Rust, bukan thread UI.
#[uniffi::export(with_foreign)]
pub trait PendengarPantau: Send + Sync {
    fn kejadian(&self, k: KejadianUi);
}

// Asumsi: `match &k` aman baik k bernilai maupun referensi (match ergonomics).
fn ke_ui(k: &KejadianPantau) -> KejadianUi {
    match k {
        KejadianPantau::Tersambung => KejadianUi::Tersambung,
        KejadianPantau::Terputus { alasan, coba_lagi_detik } => KejadianUi::Terputus {
            alasan: alasan.to_string(),
            coba_lagi_detik: *coba_lagi_detik as u32,
        },
        KejadianPantau::OrderSelesai { simbol, order_id, sisi, trade_baru } => {
            KejadianUi::OrderSelesai {
                simbol: simbol.to_string(),
                order_id: order_id.to_string(),
                sisi: match sisi {
                    Sisi::Beli => "beli",
                    Sisi::Jual => "jual",
                }
                .to_string(),
                trade_baru: *trade_baru as u32,
            }
        }
        KejadianPantau::Rekonsiliasi { trade_baru } => {
            KejadianUi::Rekonsiliasi { trade_baru: *trade_baru as u32 }
        }
        KejadianPantau::Galat(p) => KejadianUi::Galat { pesan: p.to_string() },
    }
}

/// Pengendali pantauan. Satu objek dipakai bersama oleh layanan latar.
#[derive(uniffi::Object)]
pub struct Pemantau {
    /// bendera henti milik sesi yang sedang/terakhir berjalan
    berhenti: Mutex<Arc<AtomicBool>>,
    /// true selama thread pantau hidup (termasuk saat sedang berhenti)
    jalan: Arc<AtomicBool>,
}

#[uniffi::export]
impl Pemantau {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            berhenti: Mutex::new(Arc::new(AtomicBool::new(false))),
            jalan: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Mulai memantau di thread baru dan langsung kembali (tidak memblokir).
    /// Gagal bila pantauan sebelumnya masih berjalan atau masih berhenti.
    pub fn mulai(
        &self,
        key: String,
        secret: String,
        path_db: String,
        pendengar: Arc<dyn PendengarPantau>,
    ) -> Result<(), ErrorApp> {
        let key = key.trim().to_string();
        let secret = secret.trim().to_string();
        if key.is_empty() || secret.is_empty() {
            return Err(gagal("Key dan secret wajib diisi"));
        }
        // swap atomik: mencegah dua thread pantau berjalan bersamaan
        if self.jalan.swap(true, Ordering::SeqCst) {
            return Err(gagal("Pantauan masih berjalan atau belum selesai berhenti"));
        }
        // bendera baru per sesi, supaya henti lama tidak membunuh sesi baru
        let bendera = Arc::new(AtomicBool::new(false));
        *self.berhenti.lock().unwrap_or_else(|e| e.into_inner()) = bendera.clone();

        let jalan = self.jalan.clone();
        let hasil_thread = std::thread::Builder::new().name("pantau".into()).spawn(move || {
            let p = pendengar.clone();
            // catch_unwind: panic di dalam pantau tidak boleh meninggalkan `jalan` = true
            let hasil = catch_unwind(AssertUnwindSafe(|| {
                pantau(&key, &secret, &path_db, &bendera, move |k| p.kejadian(ke_ui(&k)))
            }));
            let galat = match hasil {
                Ok(Ok(())) => None,
                Ok(Err(e)) => Some(e.to_string()),
                Err(_) => Some("pantauan berhenti tak terduga".to_string()),
            };
            jalan.store(false, Ordering::SeqCst);
            pendengar.kejadian(KejadianUi::Berhenti { galat });
        });
        if let Err(e) = hasil_thread {
            self.jalan.store(false, Ordering::SeqCst);
            return Err(gagal(e));
        }
        Ok(())
    }

    /// Minta pantauan berhenti. Berhenti sungguhan terjadi sebentar lagi
    /// (kejadian `Berhenti` dikirim setelahnya).
    pub fn berhenti(&self) {
        self.berhenti.lock().unwrap_or_else(|e| e.into_inner()).store(true, Ordering::SeqCst);
    }

    pub fn sedang_jalan(&self) -> bool {
        self.jalan.load(Ordering::SeqCst)
    }
}
