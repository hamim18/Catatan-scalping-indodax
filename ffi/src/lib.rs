//! Jembatan Rust -> Kotlin. Hanya lapisan tipis; logika ada di scalping-core.
//! Key dan secret tidak pernah dicatat ke log.
use rust_decimal::Decimal;
use scalping_core::calc::bulatkan_idr;
use scalping_core::posisi::{periksa_persen, periksa_target};
use scalping_core::{
    gabung_per_order, hitung, sinkron, IndodaxClient, Opsi, Penyimpanan, RiwayatSetelan, Sisi,
};
use std::str::FromStr;
use scalping_core::arus_kas::{sinkron_arus_kas as sinkron_arus_kas_inti, ArusKas};
use scalping_core::periode::{DataSaldo, Pencapaian, RingkasanTarget, SelisihSaldo};
use scalping_core::{awal_hari_wib, awal_minggu_wib, hitung_statistik as hitung_statistik_inti, TradeTertutup};
use scalping_core::{pantau, HasilHitung, KejadianPantau};
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

/// Target harian dan mingguan (persen) sebagai teks. None = belum diatur (T2.1).
#[derive(uniffi::Record)]
pub struct SetelanTarget {
    pub harian_persen: Option<String>,
    pub mingguan_persen: Option<String>,
}

/// Simpan target harian dan mingguan (FR-04). Menerima koma desimal. Tanpa jaringan.
#[uniffi::export]
pub fn simpan_target(path_db: String, harian_persen: String, mingguan_persen: String) -> Result<(), ErrorApp> {
    let harian = desimal(&harian_persen)?;
    let mingguan = desimal(&mingguan_persen)?;
    periksa_target(harian, mingguan).map_err(gagal)?;
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    simpan.atur_target(harian, mingguan).map_err(gagal)
}

/// Baca target yang tersimpan. Tanpa jaringan.
#[uniffi::export]
pub fn baca_target(path_db: String) -> Result<SetelanTarget, ErrorApp> {
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    let (h, m) = simpan.target().map_err(gagal)?;
    Ok(SetelanTarget {
        harian_persen: h.map(|d| d.normalize().to_string()),
        mingguan_persen: m.map(|d| d.normalize().to_string()),
    })
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

/// Setelan TP/SL terbaru (persen). None bila belum pernah disimpan.
#[derive(uniffi::Record)]
pub struct SetelanTpSl {
    pub tp_persen: Option<String>,
    pub sl_persen: Option<String>,
}

/// Baca setelan TP/SL terbaru untuk mengisi kolom di layar Setelan. Tanpa jaringan.
#[uniffi::export]
pub fn baca_setelan(path_db: String) -> Result<SetelanTpSl, ErrorApp> {
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    let riwayat = simpan.riwayat_setelan().map_err(gagal)?;
    Ok(match riwayat.last() {
        Some((_, tp, sl)) => SetelanTpSl {
            tp_persen: Some(tp.normalize().to_string()),
            sl_persen: Some(sl.normalize().to_string()),
        },
        None => SetelanTpSl { tp_persen: None, sl_persen: None },
    })
}

/// Satu posisi terbuka siap tampil di layar Posisi. Semua teks sudah berformat Indonesia
/// (titik ribuan, koma desimal) dan angkanya sama dengan notifikasi beli.
#[derive(uniffi::Record)]
pub struct PosisiUi {
    pub simbol: String,
    /// contoh "DOGE"
    pub nama: String,
    pub koin: String,
    pub total_modal: String,
    pub modal_efektif: String,
    pub harga_tp: String,
    pub harga_sl: String,
    pub tp_persen: String,
    pub sl_persen: String,
    /// estimasi hasil bersih bila TP tercapai (fee jual perkiraan), contoh "+0,69%" dan "+Rp 450"
    pub estimasi_tp_persen: String,
    pub estimasi_tp_rp: String,
    /// estimasi hasil bersih bila SL tercapai, contoh "−1,31%" dan "−Rp 856"
    pub estimasi_sl_persen: String,
    pub estimasi_sl_rp: String,
    pub waktu_buka_ms: i64,
}

/// Posisi terbuka (aturan PRD: trade sejak snapshot saldo awal), terbaru di atas. Tanpa jaringan.
#[uniffi::export]
pub fn daftar_posisi(path_db: String) -> Result<Vec<PosisiUi>, ErrorApp> {
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    let h = hitung_aplikasi(&simpan)?;
    let mut posisi: Vec<_> = h.posisi.iter().collect();
    posisi.sort_by_key(|p| std::cmp::Reverse(p.waktu_buka_ms));
    Ok(posisi
        .into_iter()
        .map(|p| {
            let (rp_tp, pers_tp) = estimasi_bersih(p.koin, p.total_idr_keluar, p.harga_tp);
            let (rp_sl, pers_sl) = estimasi_bersih(p.koin, p.total_idr_keluar, p.harga_sl);
            PosisiUi {
                simbol: p.simbol.clone(),
                nama: nama_koin(&p.simbol),
                koin: ribuan(&p.koin.normalize().to_string()),
                total_modal: fmt_rupiah(p.total_idr_keluar),
                modal_efektif: fmt_harga(p.modal_efektif),
                harga_tp: fmt_harga(p.harga_tp),
                harga_sl: fmt_harga(p.harga_sl),
                tp_persen: fmt_persen(p.tp_persen),
                sl_persen: fmt_persen(p.sl_persen),
                estimasi_tp_persen: fmt_persen_bertanda(pers_tp),
                estimasi_tp_rp: fmt_untung(rp_tp),
                estimasi_sl_persen: fmt_persen_bertanda(pers_sl),
                estimasi_sl_rp: fmt_untung(rp_sl),
                waktu_buka_ms: p.waktu_buka_ms,
            }
        })
        .collect())
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

// ---------------------------------------------------------------------------
// T1.5: isi notifikasi beli dan jual. Semua angka dihitung di Rust; Kotlin
// hanya menampilkan teks. Tanpa jaringan (hanya membaca database lokal).
// ---------------------------------------------------------------------------

/// Teks notifikasi order. `teks` = satu baris ringkas, `rinci` = teks panjang (dibuka saat notifikasi diperluas).
#[derive(uniffi::Record)]
pub struct IsiNotifOrder {
    pub judul: String,
    pub teks: String,
    pub rinci: String,
}

/// "4046" -> "4.046", "1234.5" -> "1.234,5", "-2940" -> "-2.940" (format Indonesia).
fn ribuan(angka: &str) -> String {
    let (tanda, sisa) = match angka.strip_prefix('-') {
        Some(r) => ("-", r),
        None => ("", angka),
    };
    let (bulat, desimal) = match sisa.split_once('.') {
        Some((b, d)) => (b, Some(d)),
        None => (sisa, None),
    };
    let n = bulat.len();
    let mut hasil = String::from(tanda);
    for (i, c) in bulat.chars().enumerate() {
        if i > 0 && (n - i) % 3 == 0 {
            hasil.push('.');
        }
        hasil.push(c);
    }
    if let Some(d) = desimal {
        hasil.push(',');
        hasil.push_str(d);
    }
    hasil
}

/// Harga per koin: koin murah (< 100) tampil 2 desimal, selebihnya rupiah utuh.
/// Hanya untuk tampilan (aturan tick harga per koin belum diperiksa, lihat PROGRESS).
fn fmt_harga(d: Decimal) -> String {
    if d.abs() < Decimal::from(100) {
        ribuan(&d.round_dp(2).normalize().to_string())
    } else {
        ribuan(&bulatkan_idr(d).to_string())
    }
}

fn fmt_rupiah(d: Decimal) -> String {
    ribuan(&bulatkan_idr(d).to_string())
}

/// Persen dengan koma desimal, contoh "0,58" atau "1".
fn fmt_persen(d: Decimal) -> String {
    d.round_dp(2).normalize().to_string().replace('.', ",")
}

/// "+Rp 380" atau "−Rp 2.940".
fn fmt_untung(d: Decimal) -> String {
    let tanda = if d < Decimal::ZERO { "−" } else { "+" };
    format!("{tanda}Rp {}", fmt_rupiah(d.abs()))
}

/// "uaiidr" -> "UAI"
fn nama_koin(simbol: &str) -> String {
    let s = simbol.to_lowercase();
    let s = s.strip_suffix("idr").unwrap_or(&s);
    s.trim_end_matches('_').to_uppercase()
}

/// Hitung ulang dari database dengan aturan PRD (trade sejak snapshot saldo awal).
fn hitung_aplikasi(simpan: &Penyimpanan) -> Result<HasilHitung, ErrorApp> {
    let riwayat = simpan.riwayat_setelan().map_err(gagal)?;
    if riwayat.is_empty() {
        return Err(gagal("Setelan TP/SL belum disimpan"));
    }
    let sejak_ms = simpan
        .snapshot_awal()
        .map_err(gagal)?
        .map(|(w, _)| w)
        .ok_or_else(|| gagal("Belum ada snapshot saldo awal; sinkron dulu"))?;
    let trades = simpan.semua_trade().map_err(gagal)?;
    hitung(&trades, sejak_ms, &RiwayatSetelan::baru(riwayat)).map_err(gagal)
}

/// Perkiraan fee jual untuk estimasi hasil bersih. Sama dengan jual limit pada data nyata
/// (PRD: ±0,31%); fee sebenarnya tetap dari `commission` API saat jual terjadi.
fn fee_jual_perkiraan() -> Decimal {
    Decimal::new(31, 4) // 0,0031
}

/// Estimasi hasil bersih bila koin dijual di `harga`: (profit Rp, persen terhadap modal).
/// Fee beli sudah ada di `modal`; fee jual diperkirakan.
fn estimasi_bersih(koin: Decimal, modal: Decimal, harga: Decimal) -> (Decimal, Decimal) {
    let diterima = koin * harga * (Decimal::ONE - fee_jual_perkiraan());
    let profit = diterima - modal;
    let persen = if modal.is_zero() { Decimal::ZERO } else { profit / modal * Decimal::from(100) };
    (profit, persen)
}

/// "+0,58%" atau "−1,31%"
fn fmt_persen_bertanda(d: Decimal) -> String {
    let tanda = if d < Decimal::ZERO { "−" } else { "+" };
    format!("{tanda}{}%", fmt_persen(d.abs()))
}

/// Harga TP dan SL dari modal efektif (rumus PRD bagian 8).
fn harga_tp_sl(modal_efektif: Decimal, tp_persen: Decimal, sl_persen: Decimal) -> (Decimal, Decimal) {
    let seratus = Decimal::from(100);
    (
        modal_efektif * (Decimal::ONE + tp_persen / seratus),
        modal_efektif * (Decimal::ONE - sl_persen / seratus),
    )
}

/// Satu blok TP/SL (dua baris) untuk `koin` dengan total modal `modal`.
fn blok_tp_sl(koin: Decimal, modal: Decimal, tp: Decimal, sl: Decimal, tp_persen: Decimal, sl_persen: Decimal) -> String {
    let (e_tp, pers_tp) = estimasi_bersih(koin, modal, tp);
    let (e_sl, pers_sl) = estimasi_bersih(koin, modal, sl);
    format!(
        "TP {} (+{}%) → bersih ≈ {} ({})\nSL {} (−{}%) → bersih ≈ {} ({})",
        fmt_harga(tp),
        fmt_persen(tp_persen),
        fmt_untung(e_tp),
        fmt_persen_bertanda(pers_tp),
        fmt_harga(sl),
        fmt_persen(sl_persen),
        fmt_untung(e_sl),
        fmt_persen_bertanda(pers_sl)
    )
}

/// Satu baris ringkas: "TP 4.046 (≈+0,69%) · SL 3.966 (≈−1,31%)" (persen = estimasi bersih).
fn ringkas_tp_sl(koin: Decimal, modal: Decimal, tp: Decimal, sl: Decimal) -> String {
    let (_, pers_tp) = estimasi_bersih(koin, modal, tp);
    let (_, pers_sl) = estimasi_bersih(koin, modal, sl);
    format!(
        "TP {} (≈{}) · SL {} (≈{})",
        fmt_harga(tp),
        fmt_persen_bertanda(pers_tp),
        fmt_harga(sl),
        fmt_persen_bertanda(pers_sl)
    )
}

const CATATAN_FEE: &str = "Bersih ≈ setelah fee beli (sudah di modal) dan fee jual perkiraan 0,31%; fee sebenarnya bisa berbeda.";

/// Isi notifikasi untuk satu order yang baru selesai (FR-15, FR-16).
/// `sisi`: "beli" atau "jual". Progress target harian belum ada (Fase 2).
/// Beli tambahan pada koin yang masih dipegang: tampil TP/SL order ini DAN posisi keseluruhan.
#[uniffi::export]
pub fn isi_notifikasi(path_db: String, simbol: String, sisi: String) -> Result<IsiNotifOrder, ErrorApp> {
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    let h = hitung_aplikasi(&simpan)?;
    let nama = nama_koin(&simbol);
    let posisi = h.posisi.iter().find(|p| p.simbol.eq_ignore_ascii_case(&simbol));
    let trades = simpan.semua_trade().map_err(gagal)?;

    if sisi == "beli" {
        let Some(p) = posisi else {
            let t = "Tercatat, tapi posisinya tidak ditemukan (mungkin sudah terjual).".to_string();
            return Ok(IsiNotifOrder { judul: format!("{nama} dibeli"), teks: t.clone(), rinci: t });
        };
        // Order beli terbaru koin ini (Asumsi: dua beli yang selesai nyaris bersamaan menampilkan order terbaru yang sama)
        let order = gabung_per_order(&trades)
            .map_err(gagal)?
            .into_iter()
            .filter(|o| matches!(o.sisi, Sisi::Beli) && o.symbol.eq_ignore_ascii_case(&simbol))
            .max_by_key(|o| o.waktu_awal_ms);
        // Koin yang sudah dipegang SEBELUM order ini (posisi sudah menyertakan order ini)
        let sebelumnya = order.as_ref().map(|o| p.koin - o.koin).filter(|d| *d > Decimal::ZERO);

        return Ok(match (order, sebelumnya) {
            (Some(o), Some(_)) if o.koin > Decimal::ZERO => {
                let modal_o = o.idr_keluar();
                let ef_o = modal_o / o.koin;
                let (tp_o, sl_o) = harga_tp_sl(ef_o, p.tp_persen, p.sl_persen);
                let blok_o = blok_tp_sl(o.koin, modal_o, tp_o, sl_o, p.tp_persen, p.sl_persen);
                let blok_p = blok_tp_sl(p.koin, p.total_idr_keluar, p.harga_tp, p.harga_sl, p.tp_persen, p.sl_persen);
                IsiNotifOrder {
                    judul: format!("{nama} dibeli (tambah posisi)"),
                    teks: format!(
                        "Order ini: {} | Total: {}",
                        ringkas_tp_sl(o.koin, modal_o, tp_o, sl_o),
                        ringkas_tp_sl(p.koin, p.total_idr_keluar, p.harga_tp, p.harga_sl)
                    ),
                    rinci: format!(
                        "ORDER INI: {} koin, modal Rp {} (efektif {})\n{blok_o}\n\nPOSISI KESELURUHAN: {} koin, modal Rp {} (efektif rata-rata {})\n{blok_p}\n\n{CATATAN_FEE}",
                        o.koin.normalize(),
                        fmt_rupiah(modal_o),
                        fmt_harga(ef_o),
                        p.koin.normalize(),
                        fmt_rupiah(p.total_idr_keluar),
                        fmt_harga(p.modal_efektif),
                    ),
                }
            }
            _ => IsiNotifOrder {
                judul: format!("{nama} dibeli"),
                teks: format!(
                    "Modal {} · {}",
                    fmt_harga(p.modal_efektif),
                    ringkas_tp_sl(p.koin, p.total_idr_keluar, p.harga_tp, p.harga_sl)
                ),
                rinci: format!(
                    "Modal efektif {} per koin ({} koin, total Rp {})\n{}\n\n{CATATAN_FEE}",
                    fmt_harga(p.modal_efektif),
                    p.koin.normalize(),
                    fmt_rupiah(p.total_idr_keluar),
                    blok_tp_sl(p.koin, p.total_idr_keluar, p.harga_tp, p.harga_sl, p.tp_persen, p.sl_persen)
                ),
            },
        });
    }

    // Jual: cari penjualan terbaru koin ini yang cocok dengan posisi.
    let waktu_jual_terbaru = gabung_per_order(&trades)
        .map_err(gagal)?
        .iter()
        .filter(|o| matches!(o.sisi, Sisi::Jual) && o.symbol.eq_ignore_ascii_case(&simbol))
        .map(|o| o.waktu_awal_ms)
        .max();
    let tutup = h
        .tertutup
        .iter()
        .filter(|t| t.simbol.eq_ignore_ascii_case(&simbol))
        .max_by_key(|t| t.waktu_tutup_ms)
        // Jual tanpa modal di aplikasi menjadi penyesuaian, jangan tampilkan jual lama sebagai hasilnya.
        .filter(|t| waktu_jual_terbaru.map_or(true, |w| t.waktu_tutup_ms >= w));

    Ok(match tutup {
        Some(t) => {
            let hasil = format!("{} ({})", fmt_untung(t.profit), fmt_persen_bertanda(t.persen));
            let sisa = posisi
                .map(|p| format!("\nSisa posisi: {} koin masih terbuka.", p.koin.normalize()))
                .unwrap_or_default();
            // T2.4 (FR-16, FR-18): progress target. Bila tidak bisa dihitung (mis. snapshot belum ada),
            // notifikasi tetap tampil tanpa bagian ini.
            let (progres_ringkas, progres_rinci) = progres_untuk_notif(&simpan, &h, t.waktu_tutup_ms, t.profit);
            IsiNotifOrder {
                judul: format!("{nama} dijual: {hasil}"),
                teks: format!(
                    "Modal Rp {} · diterima Rp {}{progres_ringkas}",
                    fmt_rupiah(t.modal),
                    fmt_rupiah(t.idr_diterima)
                ),
                rinci: format!(
                    "Hasil bersih {hasil}\nModal Rp {}, diterima Rp {} (fee sudah termasuk).{sisa}{progres_rinci}",
                    fmt_rupiah(t.modal),
                    fmt_rupiah(t.idr_diterima)
                ),
            }
        }
        None => IsiNotifOrder {
            judul: format!("{nama} dijual"),
            teks: "Tercatat, tidak dihitung: koin ini tidak punya modal di aplikasi.".into(),
            rinci: "Tercatat sebagai penyesuaian (profit dianggap 0) karena koin ini dibeli sebelum aplikasi dipasang.".into(),
        },
    })
}

/// Ringkasan posisi terbuka, dipakai saat sinkron menemukan trade yang terlewat
/// dan koinnya tidak diketahui (jadi TP/SL semua posisi ditampilkan sekaligus).
#[uniffi::export]
pub fn ringkasan_posisi_notif(path_db: String, trade_baru: u32) -> Result<IsiNotifOrder, ErrorApp> {
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    let h = hitung_aplikasi(&simpan)?;
    let judul = format!("{trade_baru} trade baru dicatat saat sinkron");
    if h.posisi.is_empty() {
        let t = "Tidak ada posisi terbuka.".to_string();
        return Ok(IsiNotifOrder { judul, teks: t.clone(), rinci: t });
    }
    let ringkas: Vec<String> = h
        .posisi
        .iter()
        .map(|p| format!("{}: {}", nama_koin(&p.simbol), ringkas_tp_sl(p.koin, p.total_idr_keluar, p.harga_tp, p.harga_sl)))
        .collect();
    let rinci: Vec<String> = h
        .posisi
        .iter()
        .map(|p| {
            format!(
                "{} ({} koin, modal efektif {})\n{}",
                nama_koin(&p.simbol),
                p.koin.normalize(),
                fmt_harga(p.modal_efektif),
                blok_tp_sl(p.koin, p.total_idr_keluar, p.harga_tp, p.harga_sl, p.tp_persen, p.sl_persen)
            )
        })
        .collect();
    Ok(IsiNotifOrder {
        judul,
        teks: ringkas.join("; "),
        rinci: format!("{}\n\n{CATATAN_FEE}", rinci.join("\n\n")),
    })
}

// ---------------------------------------------------------------------------
// T2.2 sampai T2.4: arus kas (deposit/withdraw IDR), pencapaian dan progress target.
// Semua hitungan ada di scalping_core (arus_kas, periode); di sini hanya pembungkus.
// ---------------------------------------------------------------------------

fn sekarang_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Hasil sinkron arus kas.
#[derive(uniffi::Record)]
pub struct HasilArusKasUi {
    pub baru: u32,
    pub total: u32,
    /// baris yang bukan sukses atau bukan IDR
    pub dilewati: u32,
}

/// Satu deposit/withdraw tersimpan.
#[derive(uniffi::Record)]
pub struct BarisArusKas {
    /// "deposit" atau "withdraw"
    pub jenis: String,
    /// `amount` dari Indodax, rupiah utuh
    pub jumlah: String,
    /// `totalFee` dari Indodax
    pub biaya: String,
    /// perubahan saldo menurut mode fee yang aktif (negatif = keluar)
    pub efek_saldo: String,
    pub waktu_ms: i64,
}

/// Pencapaian satu periode (harian atau mingguan).
#[derive(uniffi::Record)]
pub struct PencapaianUi {
    pub mulai_ms: i64,
    pub saldo_awal: String,
    pub profit: String,
    pub setoran_bersih: String,
    /// 2 desimal
    pub persen: String,
    pub target_persen: Option<String>,
    /// persen terhadap target, dibulatkan; bisa negatif atau lebih dari 100
    pub progress_persen: Option<String>,
    pub tercapai: bool,
}

#[derive(uniffi::Record)]
pub struct RingkasanTargetUi {
    pub snapshot_awal_ms: i64,
    pub saldo_awal: String,
    /// saldo versi aplikasi (IDR + posisi pada modal efektif), dihitung dari catatan
    pub saldo_sekarang: String,
    /// saldo awal + setoran bersih
    pub modal_aktif: String,
    /// saldo sekarang - modal aktif = profit sejak aplikasi dipasang
    pub selisih_modal: String,
    pub harian: PencapaianUi,
    pub mingguan: PencapaianUi,
    pub amount_sudah_bersih: bool,
    pub jumlah_arus_kas: u32,
    /// (saldo sekarang - modal aktif) / modal aktif x 100, 2 desimal; "0" bila modal aktif nol (layar Beranda)
    pub hasil_persen: String,
}

fn pencapaian_ui(p: &Pencapaian) -> PencapaianUi {
    PencapaianUi {
        mulai_ms: p.mulai_ms,
        saldo_awal: bulatkan_idr(p.saldo_awal_periode).to_string(),
        profit: bulatkan_idr(p.profit).to_string(),
        setoran_bersih: bulatkan_idr(p.setoran_bersih).to_string(),
        persen: p.persen.round_dp(2).to_string(),
        target_persen: p.target_persen.map(|t| t.normalize().to_string()),
        progress_persen: p.progress_persen.map(|t| t.round_dp(0).normalize().to_string()),
        tercapai: p.tercapai,
    }
}

/// Kumpulkan bahan hitung dari database: snapshot awal, profit trade tertutup (dari `h`),
/// arus kas, dan mode fee.
fn muat_data_saldo(simpan: &Penyimpanan, h: &HasilHitung) -> Result<DataSaldo, ErrorApp> {
    let (awal_ms, saldo_awal) = simpan
        .snapshot_awal()
        .map_err(gagal)?
        .ok_or_else(|| gagal("Belum ada snapshot saldo awal; sinkron dulu"))?;
    Ok(DataSaldo {
        snapshot_awal_ms: awal_ms,
        saldo_awal,
        profit_tertutup: h.tertutup.iter().map(|t| (t.waktu_tutup_ms, t.profit)).collect(),
        arus_kas: simpan.semua_arus_kas().map_err(gagal)?,
        amount_sudah_bersih: simpan.amount_sudah_bersih().map_err(gagal)?,
        penyesuaian: simpan.semua_koreksi().map_err(gagal)?.iter().map(|k| (k.waktu_ms, k.jumlah)).collect(),
    })
}

fn ringkasan_target_ui(r: &RingkasanTarget, data: &DataSaldo) -> RingkasanTargetUi {
    RingkasanTargetUi {
        snapshot_awal_ms: r.snapshot_awal_ms,
        saldo_awal: bulatkan_idr(r.saldo_awal).to_string(),
        saldo_sekarang: bulatkan_idr(r.saldo_sekarang).to_string(),
        modal_aktif: bulatkan_idr(r.modal_aktif).to_string(),
        selisih_modal: bulatkan_idr(r.saldo_sekarang - r.modal_aktif).to_string(),
        harian: pencapaian_ui(&r.harian),
        mingguan: pencapaian_ui(&r.mingguan),
        amount_sudah_bersih: data.amount_sudah_bersih,
        jumlah_arus_kas: data.arus_kas.len() as u32,
        hasil_persen: if r.modal_aktif.is_zero() {
            "0".to_string()
        } else {
            ((r.saldo_sekarang - r.modal_aktif) / r.modal_aktif * Decimal::from(100))
                .round_dp(2)
                .normalize()
                .to_string()
        },
    }
}

/// Tarik deposit dan withdraw IDR dari Indodax (GET /api/v2/fiat/orders, baca saja) dan simpan
/// tanpa dobel. Butuh snapshot saldo awal (sinkron trade dulu). Memblokir; panggil dari thread latar.
#[uniffi::export]
pub fn sinkron_arus_kas(key: String, secret: String, path_db: String) -> Result<HasilArusKasUi, ErrorApp> {
    let key = key.trim().to_string();
    let secret = secret.trim().to_string();
    if key.is_empty() || secret.is_empty() {
        return Err(gagal("Key dan secret wajib diisi"));
    }
    let mut simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    let h = sinkron_arus_kas_inti(&IndodaxClient::new(key, secret), &mut simpan, sekarang_ms()).map_err(gagal)?;
    Ok(HasilArusKasUi { baru: h.baru as u32, total: h.total as u32, dilewati: h.dilewati as u32 })
}

/// Arus kas tersimpan, terbaru di atas. Tanpa jaringan.
#[uniffi::export]
pub fn daftar_arus_kas(path_db: String, jumlah: u32) -> Result<Vec<BarisArusKas>, ErrorApp> {
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    let bersih = simpan.amount_sudah_bersih().map_err(gagal)?;
    let mut arus: Vec<ArusKas> = simpan.semua_arus_kas().map_err(gagal)?;
    arus.sort_by_key(|a| std::cmp::Reverse(a.waktu_ms));
    Ok(arus
        .iter()
        .take(jumlah as usize)
        .map(|a| BarisArusKas {
            jenis: a.jenis.teks().to_string(),
            jumlah: bulatkan_idr(a.jumlah).to_string(),
            biaya: bulatkan_idr(a.biaya).to_string(),
            efek_saldo: bulatkan_idr(a.efek_saldo(bersih)).to_string(),
            waktu_ms: a.waktu_ms,
        })
        .collect())
}

/// Atur apakah `amount` deposit/withdraw sudah bersih (true: fee tidak dikurangkan lagi).
/// Belum terkonfirmasi dari data nyata; bawaan false (deposit = amount - fee).
#[uniffi::export]
pub fn atur_amount_sudah_bersih(path_db: String, ya: bool) -> Result<(), ErrorApp> {
    Penyimpanan::buka(&path_db).map_err(gagal)?.atur_amount_sudah_bersih(ya).map_err(gagal)
}

/// Saldo versi aplikasi, modal aktif, serta pencapaian dan progress harian/mingguan (WIB, minggu
/// mulai Senin). Dihitung dari catatan lokal, tanpa jaringan. Sekalian mencatat snapshot awal
/// hari/minggu ke tabel snapshot (catatan untuk grafik Fase 3; tidak dipakai menghitung).
#[uniffi::export]
pub fn hitung_target(path_db: String) -> Result<RingkasanTargetUi, ErrorApp> {
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    let h = hitung_aplikasi(&simpan)?;
    let data = muat_data_saldo(&simpan, &h)?;
    let target = simpan.target().map_err(gagal)?;
    let r = data.ringkasan(sekarang_ms(), target);
    let _ = simpan.tambah_snapshot("harian", r.harian.mulai_ms, r.harian.saldo_awal_periode);
    let _ = simpan.tambah_snapshot("mingguan", r.mingguan.mulai_ms, r.mingguan.saldo_awal_periode);
    Ok(ringkasan_target_ui(&r, &data))
}

/// "Harian: +0,90% dari target 1,5% (60% tercapai)" / "Harian: −0,40% (target 1,5%)" /
/// "Harian: +0,90% (target belum diatur)".
fn teks_progres(nama: &str, p: &Pencapaian) -> String {
    let persen = fmt_persen_bertanda(p.persen);
    match (p.target_persen, p.progress_persen) {
        (Some(t), Some(pr)) if p.persen >= Decimal::ZERO => {
            format!("{nama}: {persen} dari target {}% ({}% tercapai)", fmt_persen(t), fmt_persen(pr.round_dp(0)))
        }
        (Some(t), _) => format!("{nama}: {persen} (target {}%)", fmt_persen(t)),
        _ => format!("{nama}: {persen} (target belum diatur)"),
    }
}

/// Bagian progress untuk notifikasi jual: (tambahan teks ringkas, tambahan teks rinci).
/// Target dianggap BARU tercapai bila sebelum penjualan ini belum tercapai dan sesudahnya sudah
/// (dihitung dengan membuang trade ini dari catatan, jadi tanpa status tambahan).
fn progres_untuk_notif(simpan: &Penyimpanan, h: &HasilHitung, waktu_tutup: i64, profit: Decimal) -> (String, String) {
    let kosong = (String::new(), String::new());
    let Ok(data) = muat_data_saldo(simpan, h) else { return kosong };
    let Ok(target) = simpan.target() else { return kosong };
    let sekarang = sekarang_ms();
    let sesudah = data.ringkasan(sekarang, target);
    let mut tanpa = data.clone();
    if let Some(i) = tanpa.profit_tertutup.iter().position(|(w, p)| *w == waktu_tutup && *p == profit) {
        tanpa.profit_tertutup.remove(i);
    }
    let sebelum = tanpa.ringkasan(sekarang, target);

    let mut ringkas = format!(" · {}", teks_progres("Harian", &sesudah.harian));
    let mut rinci = format!(
        "\n\n{}\n{}",
        teks_progres("Harian", &sesudah.harian),
        teks_progres("Mingguan", &sesudah.mingguan)
    );
    if sesudah.harian.tercapai && !sebelum.harian.tercapai {
        ringkas.push_str(" · Target harian TERCAPAI");
        rinci.push_str("\nTarget harian TERCAPAI.");
    }
    if sesudah.mingguan.tercapai && !sebelum.mingguan.tercapai {
        ringkas.push_str(" · Target mingguan TERCAPAI");
        rinci.push_str("\nTarget mingguan TERCAPAI.");
    }
    (ringkas, rinci)
}

// ---------------------------------------------------------------------------
// Fase 3: riwayat trade tertutup (FR-23), grafik saldo dan modal aktif (FR-22),
// koreksi saldo terhadap saldo asli Indodax (T3.3). Hitungan di scalping_core.
// ---------------------------------------------------------------------------

/// Titik grafik: saldo versi aplikasi dan modal aktif (rupiah utuh). Jarak keduanya = profit.
#[derive(uniffi::Record)]
pub struct TitikGrafikUi {
    pub waktu_ms: i64,
    pub saldo: String,
    pub modal_aktif: String,
}

/// Deret grafik sejak snapshot saldo awal sampai sekarang, paling banyak `maks_titik` titik.
/// Tanpa jaringan.
#[uniffi::export]
pub fn data_grafik(path_db: String, maks_titik: u32) -> Result<Vec<TitikGrafikUi>, ErrorApp> {
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    let h = hitung_aplikasi(&simpan)?;
    let data = muat_data_saldo(&simpan, &h)?;
    Ok(data
        .deret(sekarang_ms(), maks_titik as usize)
        .into_iter()
        .map(|t| TitikGrafikUi {
            waktu_ms: t.waktu_ms,
            saldo: bulatkan_idr(t.saldo).to_string(),
            modal_aktif: bulatkan_idr(t.modal_aktif).to_string(),
        })
        .collect())
}

fn baris_tertutup(t: &TradeTertutup) -> BarisTertutup {
    BarisTertutup {
        simbol: t.simbol.clone(),
        koin: t.koin.normalize().to_string(),
        modal: bulatkan_idr(t.modal).to_string(),
        diterima: bulatkan_idr(t.idr_diterima).to_string(),
        profit: bulatkan_idr(t.profit).to_string(),
        persen: t.persen.round_dp(2).to_string(),
        waktu_buka_ms: t.waktu_buka_ms,
        waktu_tutup_ms: t.waktu_tutup_ms,
    }
}

/// Riwayat trade tertutup (FR-23) dengan ringkasan hari ini (WIB).
#[derive(uniffi::Record)]
pub struct RiwayatUi {
    pub total_trade: u32,
    pub total_profit: String,
    pub hari_ini_jumlah: u32,
    pub hari_ini_untung: u32,
    pub hari_ini_profit: String,
    /// terbaru di atas, paling banyak `jumlah`
    pub baris: Vec<BarisTertutup>,
}

/// Trade tertutup sejak snapshot saldo awal. Tanpa jaringan.
#[uniffi::export]
pub fn riwayat_tertutup(path_db: String, jumlah: u32) -> Result<RiwayatUi, ErrorApp> {
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    let h = hitung_aplikasi(&simpan)?;
    let mut tertutup: Vec<&TradeTertutup> = h.tertutup.iter().collect();
    tertutup.sort_by_key(|t| std::cmp::Reverse(t.waktu_tutup_ms));
    let awal_hari = awal_hari_wib(sekarang_ms());
    let hari_ini: Vec<&&TradeTertutup> = tertutup.iter().filter(|t| t.waktu_tutup_ms >= awal_hari).collect();
    let total: Decimal = tertutup.iter().map(|t| t.profit).sum();
    let profit_hari: Decimal = hari_ini.iter().map(|t| t.profit).sum();
    Ok(RiwayatUi {
        total_trade: tertutup.len() as u32,
        total_profit: bulatkan_idr(total).to_string(),
        hari_ini_jumlah: hari_ini.len() as u32,
        hari_ini_untung: hari_ini.iter().filter(|t| t.profit > Decimal::ZERO).count() as u32,
        hari_ini_profit: bulatkan_idr(profit_hari).to_string(),
        baris: tertutup.iter().take(jumlah as usize).map(|t| baris_tertutup(t)).collect(),
    })
}

/// T4.1: statistik trade tertutup (FR-25). Semua angka sudah berupa teks siap tampil.
#[derive(uniffi::Record)]
pub struct StatistikUi {
    /// "hari", "minggu", atau "semua"
    pub rentang: String,
    pub jumlah: u32,
    pub untung: u32,
    pub rugi: u32,
    pub impas: u32,
    /// persen, 2 desimal; "-" bila belum ada trade
    pub win_rate: String,
    /// rata-rata hasil trade untung (Rp); "-" bila tidak ada
    pub rata_untung: String,
    /// rata-rata hasil trade rugi (Rp, negatif); "-" bila tidak ada
    pub rata_rugi: String,
    pub hasil_bersih: String,
    pub total_fee: String,
    pub profit_kotor: String,
    /// total fee / profit kotor x 100; "-" bila profit kotor tidak positif
    pub persen_fee: String,
}

fn teks_opsi_rp(v: Option<Decimal>) -> String {
    v.map(|x| bulatkan_idr(x).to_string()).unwrap_or_else(|| "-".to_string())
}

fn teks_opsi_persen(v: Option<Decimal>) -> String {
    v.map(|x| x.round_dp(2).to_string()).unwrap_or_else(|| "-".to_string())
}

/// Statistik trade tertutup untuk rentang "hari" / "minggu" / "semua" (WIB). Tanpa jaringan.
#[uniffi::export]
pub fn statistik_trading(path_db: String, rentang: String) -> Result<StatistikUi, ErrorApp> {
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    let h = hitung_aplikasi(&simpan)?;
    let sekarang = sekarang_ms();
    let (nama, dari_ms) = match rentang.trim().to_lowercase().as_str() {
        "hari" => ("hari", awal_hari_wib(sekarang)),
        "minggu" => ("minggu", awal_minggu_wib(sekarang)),
        "semua" => ("semua", i64::MIN),
        lain => return Err(gagal(format!("rentang tidak dikenal: {lain}"))),
    };
    let s = hitung_statistik_inti(&h.tertutup, dari_ms);
    Ok(StatistikUi {
        rentang: nama.to_string(),
        jumlah: s.jumlah as u32,
        untung: s.untung as u32,
        rugi: s.rugi as u32,
        impas: s.impas as u32,
        win_rate: teks_opsi_persen(s.win_rate_persen),
        rata_untung: teks_opsi_rp(s.rata_untung),
        rata_rugi: teks_opsi_rp(s.rata_rugi),
        hasil_bersih: bulatkan_idr(s.hasil_bersih).to_string(),
        total_fee: bulatkan_idr(s.total_fee).to_string(),
        profit_kotor: bulatkan_idr(s.profit_kotor).to_string(),
        persen_fee: teks_opsi_persen(s.persen_fee_dari_profit),
    })
}

/// Selisih saldo aplikasi dengan saldo IDR asli di Indodax.
#[derive(uniffi::Record)]
pub struct SelisihSaldoUi {
    pub idr_asli: String,
    pub idr_menurut_aplikasi: String,
    pub nilai_posisi: String,
    pub saldo_aplikasi: String,
    /// idr asli - idr menurut aplikasi, maksimal 2 desimal. Positif: IDR asli lebih besar.
    pub selisih: String,
    /// true bila selisih sedikitnya Rp 1
    pub perlu_koreksi: bool,
}

/// Selisih di bawah ini dianggap pembulatan dan tidak dikoreksi.
fn batas_koreksi() -> Decimal {
    Decimal::ONE
}

fn selisih_ui(s: &SelisihSaldo) -> SelisihSaldoUi {
    SelisihSaldoUi {
        idr_asli: s.idr_asli.round_dp(2).normalize().to_string(),
        idr_menurut_aplikasi: s.idr_menurut_aplikasi.round_dp(2).normalize().to_string(),
        nilai_posisi: bulatkan_idr(s.nilai_posisi).to_string(),
        saldo_aplikasi: bulatkan_idr(s.saldo_aplikasi).to_string(),
        selisih: s.selisih.round_dp(2).normalize().to_string(),
        perlu_koreksi: s.selisih.abs() >= batas_koreksi(),
    }
}

/// Ambil saldo IDR asli (tersedia + terkunci) lewat GET /api/v2/account. Hanya membaca;
/// menolak key yang punya izin trade/withdraw.
fn ambil_idr_asli(key: String, secret: String) -> Result<Decimal, ErrorApp> {
    let key = key.trim().to_string();
    let secret = secret.trim().to_string();
    if key.is_empty() || secret.is_empty() {
        return Err(gagal("Key dan secret wajib diisi"));
    }
    let akun = IndodaxClient::new(key, secret).ambil_akun().map_err(gagal)?;
    if !akun.kunci_baca_saja() {
        return Err(gagal("Key punya izin trade/withdraw; pakai key BACA SAJA"));
    }
    akun.saldo_idr_total().map_err(gagal)
}

fn hitung_selisih(simpan: &Penyimpanan, idr_asli: Decimal, sekarang: i64) -> Result<SelisihSaldo, ErrorApp> {
    let h = hitung_aplikasi(simpan)?;
    let data = muat_data_saldo(simpan, &h)?;
    let nilai_posisi: Decimal = h.posisi.iter().map(|p| p.total_idr_keluar).sum();
    Ok(data.selisih_saldo(idr_asli, nilai_posisi, sekarang))
}

/// Catat koreksi sebesar selisih bila minimal Rp 1; mengembalikan jumlah yang dicatat.
/// Koreksi diperlakukan seperti setoran: mengubah saldo dan modal aktif, bukan profit.
fn catat_koreksi(simpan: &Penyimpanan, idr_asli: Decimal, sekarang: i64, catatan: &str) -> Result<Option<Decimal>, ErrorApp> {
    let s = hitung_selisih(simpan, idr_asli, sekarang)?;
    if s.selisih.abs() < batas_koreksi() {
        return Ok(None);
    }
    let catatan = if catatan.trim().is_empty() { "Koreksi saldo" } else { catatan.trim() };
    simpan.tambah_koreksi(sekarang, s.selisih, catatan).map_err(gagal)?;
    Ok(Some(s.selisih))
}

/// Bandingkan saldo aplikasi dengan saldo asli Indodax (baca saja, tidak mengubah apa pun).
/// Sinkron trade dulu dan jangan memeriksa saat ada order terisi sebagian. Memblokir.
#[uniffi::export]
pub fn cek_selisih_saldo(key: String, secret: String, path_db: String) -> Result<SelisihSaldoUi, ErrorApp> {
    let idr_asli = ambil_idr_asli(key, secret)?;
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    Ok(selisih_ui(&hitung_selisih(&simpan, idr_asli, sekarang_ms())?))
}

/// Catat koreksi saldo (T3.3). Mengambil saldo asli lagi, lalu mencatat selisihnya sebagai
/// penyesuaian. None bila selisih kurang dari Rp 1. Memblokir.
#[uniffi::export]
pub fn terapkan_koreksi_saldo(key: String, secret: String, path_db: String, catatan: String) -> Result<Option<String>, ErrorApp> {
    let idr_asli = ambil_idr_asli(key, secret)?;
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    let dicatat = catat_koreksi(&simpan, idr_asli, sekarang_ms(), &catatan)?;
    Ok(dicatat.map(|d| d.round_dp(2).normalize().to_string()))
}

#[derive(uniffi::Record)]
pub struct BarisKoreksi {
    pub waktu_ms: i64,
    /// positif = saldo dinaikkan, negatif = diturunkan
    pub jumlah: String,
    pub catatan: String,
}

/// Koreksi saldo tersimpan, terbaru di atas. Tanpa jaringan.
#[uniffi::export]
pub fn daftar_koreksi_saldo(path_db: String, jumlah: u32) -> Result<Vec<BarisKoreksi>, ErrorApp> {
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    let mut k = simpan.semua_koreksi().map_err(gagal)?;
    k.sort_by_key(|x| std::cmp::Reverse((x.waktu_ms, x.id)));
    Ok(k
        .into_iter()
        .take(jumlah as usize)
        .map(|x| BarisKoreksi {
            waktu_ms: x.waktu_ms,
            jumlah: x.jumlah.round_dp(2).normalize().to_string(),
            catatan: x.catatan,
        })
        .collect())
}

/// Ringkasan isi cadangan (U5). Hanya jumlah; tanpa angka uang.
#[derive(uniffi::Record)]
pub struct RingkasanCadanganUi {
    pub jumlah_trade: u32,
    pub jumlah_arus_kas: u32,
    pub jumlah_koreksi: u32,
    /// waktu snapshot saldo awal (ms), kosong bila tidak ada
    pub snapshot_awal_ms: Option<i64>,
}

fn ringkasan_cadangan(r: (usize, usize, usize, Option<i64>)) -> RingkasanCadanganUi {
    RingkasanCadanganUi {
        jumlah_trade: r.0 as u32,
        jumlah_arus_kas: r.1 as u32,
        jumlah_koreksi: r.2 as u32,
        snapshot_awal_ms: r.3,
    }
}

/// U5: ekspor salinan database ke berkas BARU `path_tujuan` (biasanya di cache aplikasi; Kotlin lalu
/// menyalinnya ke lokasi pilihan pengguna). Hasilnya diperiksa ulang sebelum dikembalikan.
/// Kunci API tidak ada di database, jadi tidak ikut.
#[uniffi::export]
pub fn ekspor_cadangan(path_db: String, path_tujuan: String) -> Result<RingkasanCadanganUi, ErrorApp> {
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    simpan.ekspor_ke(&path_tujuan).map_err(gagal)?;
    match Penyimpanan::periksa_cadangan(&path_tujuan) {
        Ok(r) => Ok(ringkasan_cadangan(r)),
        Err(e) => {
            let _ = std::fs::remove_file(&path_tujuan);
            Err(gagal(e))
        }
    }
}

/// U5: periksa berkas cadangan tanpa mengubah apa pun (untuk dialog konfirmasi impor).
#[uniffi::export]
pub fn periksa_cadangan(path_sumber: String) -> Result<RingkasanCadanganUi, ErrorApp> {
    Penyimpanan::periksa_cadangan(&path_sumber).map(ringkasan_cadangan).map_err(gagal)
}

/// U5: ganti database dengan isi cadangan. Pantauan HARUS sudah mati. Database lama disimpan
/// sebagai `<path_db>.sebelum-impor`. Gagal di tengah jalan tidak merusak database yang ada.
#[uniffi::export]
pub fn pulihkan_cadangan(path_db: String, path_sumber: String) -> Result<RingkasanCadanganUi, ErrorApp> {
    Penyimpanan::pulihkan(&path_db, &path_sumber).map(ringkasan_cadangan).map_err(gagal)
}

#[cfg(test)]
mod tes_fase3 {
    use super::*;
    use scalping_core::Trade;

    const SABTU: i64 = 1_793_984_400_000; // Sabtu 7 Nov 2026 00:00 WIB
    const JAM: i64 = 3_600_000;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn trade(id: &str, order: &str, beli: bool, quote: &str, fee: &str, waktu: i64) -> Trade {
        Trade {
            trade_id: id.into(),
            order_id: order.into(),
            symbol: "icntidr".into(),
            price: d("3998"),
            qty: d("16.35667833"),
            quote_qty: d(quote),
            commission: d(fee),
            commission_asset: "idr".into(),
            is_buyer: beli,
            is_maker: false,
            time: waktu,
        }
    }

    /// Snapshot awal Rp 1.000.000, satu trade ICNT tertutup (+380) pada Sabtu 10:00 WIB.
    fn siapkan(nama: &str) -> (String, Penyimpanan) {
        let p = std::env::temp_dir().join(format!("catatan_tes_{nama}_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&p);
        let db = p.to_string_lossy().to_string();
        let mut simpan = Penyimpanan::buka(&db).unwrap();
        simpan.tambah_snapshot("awal", SABTU - 20 * JAM, d("1000000")).unwrap();
        simpan.tambah_setelan(0, d("1"), d("1")).unwrap();
        simpan
            .simpan_banyak(&[
                trade("t1", "o1", true, "65393", "133", SABTU + 10 * JAM),
                trade("t2", "o2", false, "66113", "207", SABTU + 10 * JAM + 600_000),
            ])
            .unwrap();
        (db, simpan)
    }

    #[test]
    fn koreksi_dicatat_sekali_dan_tidak_mengubah_profit() {
        let (db, simpan) = siapkan("koreksi");
        let sekarang = SABTU + 12 * JAM;
        // menurut catatan IDR = 1.000.380; asli Indodax 1.000.400
        let s = hitung_selisih(&simpan, d("1000400"), sekarang).unwrap();
        assert_eq!(s.selisih.round_dp(0), d("20"));
        assert!(selisih_ui(&s).perlu_koreksi);

        let dicatat = catat_koreksi(&simpan, d("1000400"), sekarang, "  ").unwrap().unwrap();
        assert_eq!(dicatat.round_dp(0), d("20"));
        let k = simpan.semua_koreksi().unwrap();
        assert_eq!(k.len(), 1);
        assert_eq!(k[0].catatan, "Koreksi saldo"); // catatan kosong diganti bawaan

        // diulang dengan saldo asli yang sama: tidak ada selisih lagi, tidak ada catatan ganda
        assert_eq!(catat_koreksi(&simpan, d("1000400"), sekarang, "").unwrap(), None);
        assert_eq!(simpan.semua_koreksi().unwrap().len(), 1);

        // profit tetap +380; saldo naik 20 dan modal aktif ikut naik 20
        let h = hitung_aplikasi(&simpan).unwrap();
        let data = muat_data_saldo(&simpan, &h).unwrap();
        let r = data.ringkasan(sekarang, (None, None));
        assert_eq!(r.harian.profit.round_dp(0), d("380"));
        assert_eq!(r.saldo_sekarang.round_dp(0), d("1000400"));
        assert_eq!(r.modal_aktif.round_dp(0), d("1000020"));
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn selisih_di_bawah_satu_rupiah_diabaikan() {
        let (db, simpan) = siapkan("kecil");
        let sekarang = SABTU + 12 * JAM;
        assert_eq!(catat_koreksi(&simpan, d("1000380.4"), sekarang, "").unwrap(), None);
        assert!(simpan.semua_koreksi().unwrap().is_empty());
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn koreksi_negatif_menurunkan_saldo() {
        let (db, simpan) = siapkan("negatif");
        let sekarang = SABTU + 12 * JAM;
        let dicatat = catat_koreksi(&simpan, d("1000300"), sekarang, "selisih fee").unwrap().unwrap();
        assert_eq!(dicatat.round_dp(0), d("-80"));
        let ringkas = daftar_koreksi_saldo(db.clone(), 10).unwrap();
        assert_eq!(ringkas.len(), 1);
        assert_eq!(ringkas[0].jumlah, "-80");
        assert_eq!(ringkas[0].catatan, "selisih fee");
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn riwayat_dan_grafik_dari_database() {
        let (db, simpan) = siapkan("riwayat");
        simpan.tambah_koreksi(SABTU + 11 * JAM, d("20"), "uji").unwrap();
        let r = riwayat_tertutup(db.clone(), 10).unwrap();
        assert_eq!(r.total_trade, 1);
        assert_eq!(r.total_profit, "380");
        assert_eq!(r.baris.len(), 1);
        assert_eq!(r.baris[0].simbol, "icntidr");
        assert_eq!(r.baris[0].profit, "380");

        // T4.1: fee 133 + 207 = 340; profit kotor 720; 340/720 = 47,22%
        let st = statistik_trading(db.clone(), "semua".into()).unwrap();
        assert_eq!(st.jumlah, 1);
        assert_eq!(st.untung, 1);
        assert_eq!(st.win_rate, "100");
        assert_eq!(st.total_fee, "340");
        assert_eq!(st.profit_kotor, "720");
        assert_eq!(st.persen_fee, "47.22");
        assert!(statistik_trading(db.clone(), "bulan".into()).is_err());

        let g = data_grafik(db.clone(), 50).unwrap();
        assert!(g.len() >= 3); // awal, trade tutup, koreksi, sekarang
        assert_eq!(g.first().unwrap().saldo, "1000000");
        assert_eq!(g.first().unwrap().modal_aktif, "1000000");
        assert_eq!(g.last().unwrap().saldo, "1000400");
        assert_eq!(g.last().unwrap().modal_aktif, "1000020");
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn data_layar_ui_setelan_hasil_persen_dan_posisi() {
        let (db, mut simpan) = siapkan("ui");
        // belum ada posisi terbuka (ICNT sudah terjual), setelan terbaca apa adanya
        assert!(daftar_posisi(db.clone()).unwrap().is_empty());
        let s = baca_setelan(db.clone()).unwrap();
        assert_eq!(s.tp_persen.as_deref(), Some("1"));
        assert_eq!(s.sl_persen.as_deref(), Some("1"));

        // hasil 380 dari modal aktif 1.000.000 = 0,038% -> 0,04
        assert_eq!(hitung_target(db.clone()).unwrap().hasil_persen, "0.04");

        // beli ICNT lagi: posisi terbuka dengan angka yang sama seperti contoh notifikasi
        simpan.simpan_banyak(&[trade("t3", "o3", true, "65393", "133", SABTU + 11 * JAM)]).unwrap();
        let p = daftar_posisi(db.clone()).unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].nama, "ICNT");
        assert_eq!(p[0].koin, "16,35667833");
        assert_eq!(p[0].total_modal, "65.526");
        assert_eq!(p[0].modal_efektif, "4.006");
        assert_eq!(p[0].harga_tp, "4.046");
        assert_eq!(p[0].harga_sl, "3.966");
        assert_eq!(p[0].tp_persen, "1");
        assert_eq!(p[0].estimasi_tp_persen, "+0,69%");
        assert_eq!(p[0].estimasi_sl_persen, "−1,31%");
        let _ = std::fs::remove_file(&db);
    }
}

#[cfg(test)]
mod tes_target_progres {
    use super::*;
    use scalping_core::arus_kas::JenisArusKas;
    use scalping_core::Trade;

    const SABTU: i64 = 1_793_984_400_000; // Sabtu 7 Nov 2026 00:00 WIB
    const JAM: i64 = 3_600_000;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn trade(id: &str, order: &str, beli: bool, quote: &str, fee: &str, waktu: i64) -> Trade {
        Trade {
            trade_id: id.into(),
            order_id: order.into(),
            symbol: "icntidr".into(),
            price: d("3998"),
            qty: d("16.35667833"),
            quote_qty: d(quote),
            commission: d(fee),
            commission_asset: "idr".into(),
            is_buyer: beli,
            is_maker: false,
            time: waktu,
        }
    }

    fn db_sementara(nama: &str) -> String {
        let p = std::env::temp_dir().join(format!("catatan_tes_{nama}_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&p);
        p.to_string_lossy().to_string()
    }

    /// Contoh ICNT 5 Nov (+380) pada hari Sabtu, plus deposit Rp 100.000 yang harus dikeluarkan.
    #[test]
    fn progres_dari_database_mengeluarkan_deposit() {
        let db = db_sementara("progres");
        let mut simpan = Penyimpanan::buka(&db).unwrap();
        simpan.tambah_snapshot("awal", SABTU - 20 * JAM, d("1000000")).unwrap();
        simpan.tambah_setelan(0, d("1"), d("1")).unwrap();
        simpan
            .simpan_banyak(&[
                trade("t1", "o1", true, "65393", "133", SABTU + 10 * JAM),
                trade("t2", "o2", false, "66113", "207", SABTU + 10 * JAM + 600_000),
            ])
            .unwrap();
        simpan
            .simpan_arus_kas(&[ArusKas {
                id_sumber: "deposit:1".into(),
                jenis: JenisArusKas::Deposit,
                jumlah: d("100000"),
                biaya: Decimal::ZERO,
                waktu_ms: SABTU + 11 * JAM,
            }])
            .unwrap();
        simpan.atur_amount_sudah_bersih(true).unwrap();

        let h = hitung_aplikasi(&simpan).unwrap();
        let data = muat_data_saldo(&simpan, &h).unwrap();
        let r = data.ringkasan(SABTU + 12 * JAM, (Some(d("1.5")), Some(d("5"))));

        assert_eq!(r.harian.profit.round_dp(0), d("380"));
        assert_eq!(r.harian.setoran_bersih, d("100000"));
        assert_eq!(r.harian.persen.round_dp(3), d("0.038"));
        assert_eq!(r.saldo_sekarang.round_dp(0), d("1100380"));
        assert_eq!(r.modal_aktif, d("1100000"));
        assert!(!r.harian.tercapai);
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn teks_progres_tiga_kasus() {
        let mk = |persen: &str, target: Option<&str>| {
            let t = target.map(d);
            Pencapaian {
                mulai_ms: 0,
                saldo_awal_periode: d("1000000"),
                saldo_sekarang: d("1000000"),
                setoran_bersih: Decimal::ZERO,
                profit: Decimal::ZERO,
                persen: d(persen),
                target_persen: t,
                progress_persen: t.map(|x| d(persen) / x * Decimal::from(100)),
                tercapai: false,
            }
        };
        assert_eq!(teks_progres("Harian", &mk("0.9", Some("1.5"))), "Harian: +0,9% dari target 1,5% (60% tercapai)");
        assert_eq!(teks_progres("Harian", &mk("-0.4", Some("1.5"))), "Harian: −0,4% (target 1,5%)");
        assert_eq!(teks_progres("Harian", &mk("0.9", None)), "Harian: +0,9% (target belum diatur)");
    }
}

#[cfg(test)]
mod tes_target {
    use super::*;

    fn db_sementara(nama: &str) -> String {
        let p = std::env::temp_dir().join(format!("catatan_tes_{nama}_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&p);
        p.to_string_lossy().to_string()
    }

    #[test]
    fn target_disimpan_dibaca_dan_menerima_koma() {
        let db = db_sementara("target");
        let kosong = baca_target(db.clone()).unwrap();
        assert!(kosong.harian_persen.is_none() && kosong.mingguan_persen.is_none());
        simpan_target(db.clone(), "1,5".into(), "5".into()).unwrap();
        let t = baca_target(db.clone()).unwrap();
        assert_eq!(t.harian_persen.as_deref(), Some("1.5"));
        assert_eq!(t.mingguan_persen.as_deref(), Some("5"));
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn target_tidak_valid_ditolak_dan_tidak_menimpa() {
        let db = db_sementara("tolak");
        simpan_target(db.clone(), "1".into(), "4".into()).unwrap();
        assert!(simpan_target(db.clone(), "0".into(), "4".into()).is_err());
        assert!(simpan_target(db.clone(), "abc".into(), "4".into()).is_err());
        let t = baca_target(db.clone()).unwrap();
        assert_eq!(t.harian_persen.as_deref(), Some("1"));
        let _ = std::fs::remove_file(&db);
    }
}

#[cfg(test)]
mod tes_notifikasi {
    use super::*;

    #[test]
    fn ribuan_format_indonesia() {
        assert_eq!(ribuan("0"), "0");
        assert_eq!(ribuan("380"), "380");
        assert_eq!(ribuan("4046"), "4.046");
        assert_eq!(ribuan("1234567.89"), "1.234.567,89");
        assert_eq!(ribuan("-2940"), "-2.940");
        assert_eq!(ribuan("12.3"), "12,3");
    }

    #[test]
    fn harga_koin_mahal_dan_murah() {
        assert_eq!(fmt_harga(Decimal::from_str("4006.07").unwrap()), "4.006");
        assert_eq!(fmt_harga(Decimal::from_str("2950000").unwrap()), "2.950.000");
        assert_eq!(fmt_harga(Decimal::from_str("12.3456").unwrap()), "12,35");
    }

    #[test]
    fn untung_dan_persen() {
        assert_eq!(fmt_untung(Decimal::from(380)), "+Rp 380");
        assert_eq!(fmt_untung(Decimal::from(-2940)), "−Rp 2.940");
        assert_eq!(fmt_persen(Decimal::from_str("0.5800").unwrap()), "0,58");
        assert_eq!(fmt_persen(Decimal::from(1)), "1");
    }

    #[test]
    fn nama_koin_dari_simbol() {
        assert_eq!(nama_koin("uaiidr"), "UAI");
        assert_eq!(nama_koin("ICNTIDR"), "ICNT");
        assert_eq!(nama_koin("doge_idr"), "DOGE");
    }

    #[test]
    fn estimasi_bersih_sesuai_contoh_prd() {
        // ICNT 5 Nov: modal 65.526 untuk 16,35667833 koin. TP 1% -> sekitar +0,7%, SL 1% -> sekitar -1,3% (PRD bagian 8).
        let koin = Decimal::from_str("16.35667833").unwrap();
        let modal = Decimal::from(65526);
        let ef = modal / koin;
        let (tp, sl) = harga_tp_sl(ef, Decimal::from(1), Decimal::from(1));
        let (u_tp, p_tp) = estimasi_bersih(koin, modal, tp);
        let (u_sl, p_sl) = estimasi_bersih(koin, modal, sl);
        assert_eq!(fmt_persen(p_tp), "0,69");
        assert_eq!(fmt_persen(p_sl), "-1,31");
        assert_eq!(fmt_untung(u_tp), "+Rp 450");
        assert_eq!(fmt_untung(u_sl), "−Rp 856");
        assert_eq!(fmt_persen_bertanda(p_sl), "−1,31%");
    }

    #[test]
    fn harga_tp_sl_icnt() {
        let ef = Decimal::from(65526) / Decimal::from_str("16.35667833").unwrap();
        let (tp, sl) = harga_tp_sl(ef, Decimal::from(1), Decimal::from(1));
        assert_eq!(fmt_harga(tp), "4.046");
        assert_eq!(fmt_harga(sl), "3.966");
    }
}

#[cfg(test)]
mod tes_cadangan {
    use super::*;
    use scalping_core::Trade;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn path(nama: &str) -> String {
        std::env::temp_dir()
            .join(format!("ffi_cadangan_{nama}_{}.db", std::process::id()))
            .to_string_lossy()
            .to_string()
    }

    fn bersih(p: &str) {
        for a in ["", ".impor.tmp", ".sebelum-impor"] {
            let _ = std::fs::remove_file(format!("{p}{a}"));
        }
    }

    #[test]
    fn ekspor_periksa_dan_pulihkan_lewat_ffi() {
        let asal = path("asal");
        let salinan = path("salinan");
        let tujuan = path("tujuan");
        for p in [&asal, &salinan, &tujuan] {
            bersih(p);
        }
        let mut s = Penyimpanan::buka(&asal).unwrap();
        s.simpan_banyak(&[Trade {
            trade_id: "t1".into(),
            order_id: "o1".into(),
            symbol: "icntidr".into(),
            price: d("3998"),
            qty: d("16.35667833"),
            quote_qty: d("65393"),
            commission: d("133"),
            commission_asset: "idr".into(),
            is_buyer: true,
            is_maker: false,
            time: 1000,
        }])
        .unwrap();
        s.tambah_snapshot("awal", 500, d("1000000")).unwrap();
        drop(s);

        let r = ekspor_cadangan(asal.clone(), salinan.clone()).unwrap();
        assert_eq!((r.jumlah_trade, r.jumlah_arus_kas, r.snapshot_awal_ms), (1, 0, Some(500)));
        // ekspor tidak boleh menimpa
        assert!(ekspor_cadangan(asal.clone(), salinan.clone()).is_err());

        let p = periksa_cadangan(salinan.clone()).unwrap();
        assert_eq!(p.jumlah_trade, 1);
        assert!(periksa_cadangan("/tidak/ada.db".into()).is_err());

        let h = pulihkan_cadangan(tujuan.clone(), salinan.clone()).unwrap();
        assert_eq!(h.jumlah_trade, 1);
        assert_eq!(Penyimpanan::buka(&tujuan).unwrap().jumlah_trade().unwrap(), 1);
        for p in [&asal, &salinan, &tujuan] {
            bersih(p);
        }
    }
}
