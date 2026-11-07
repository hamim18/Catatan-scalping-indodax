//! Jembatan Rust -> Kotlin. Hanya lapisan tipis; logika ada di scalping-core.
//! Key dan secret tidak pernah dicatat ke log.
use rust_decimal::Decimal;
use scalping_core::calc::bulatkan_idr;
use scalping_core::posisi::periksa_persen;
use scalping_core::{
    gabung_per_order, hitung, sinkron, IndodaxClient, Opsi, Penyimpanan, RiwayatSetelan, Sisi,
};
use std::str::FromStr;
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
            IsiNotifOrder {
                judul: format!("{nama} dijual: {hasil}"),
                teks: format!("Modal Rp {} · diterima Rp {}", fmt_rupiah(t.modal), fmt_rupiah(t.idr_diterima)),
                rinci: format!(
                    "Hasil bersih {hasil}\nModal Rp {}, diterima Rp {} (fee sudah termasuk).{sisa}",
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
