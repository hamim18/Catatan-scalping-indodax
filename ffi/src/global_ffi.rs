//! T9.5: jembatan Harga global. HANYA informasi pembanding: membaca harga publik exchange luar
//! (tanpa key), menghitung harga wajar, dan mengembalikan teks siap tampil. Tidak menyentuh posisi,
//! profit, target, grafik, statistik, atau database trade (database hanya dibaca/ditulis untuk
//! setelan `global_*` di tabel `meta`). Logika ada di `scalping_core::harga_global`.
//! Pengambilan data memblokir; dipanggil dari thread latar (dijalankan paralel dengan pengambilan
//! `trades` Screener). Data di-cache di memori selama proses hidup, selama `interval_detik`.
use super::{desimal, fmt_harga, gagal, nama_koin, sekarang_ms, ErrorApp};
use rust_decimal::Decimal;
use scalping_core::{
    bandingkan_kurs, hitung_harga_global, muat_konfig_global, parse_binance, parse_bybit, parse_okx,
    simpan_konfig_global, Exchange, Hasil, HargaUsdt, KlienGlobal, KonfigGlobal, Penyimpanan, RingkasanPasar,
    RiwayatArah,
};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Exchange yang GAGAL diambil dicoba lagi paling cepat sekian detik kemudian (atau
/// `interval_detik` bila lebih kecil), supaya jaringan yang diblokir tidak membuat tiap
/// pemindaian menunggu batas waktu.
const TTL_GAGAL_DETIK: u64 = 20;

// ---------------------------------------------------------------------------
// Setelan
// ---------------------------------------------------------------------------

/// Ambang Harga global sebagai teks (Kotlin tidak menghitung apa pun). Semua isian wajib diisi.
#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct SetelanGlobalUi {
    pub aktif: bool,
    /// persen, contoh "2"
    pub sebaran_maks: String,
    pub curiga: String,
    pub kurs_tanda: String,
    pub interval_detik: String,
    /// T9.6: lebar jendela arah harga, menit (5 sampai 30)
    pub arah_menit: String,
    /// T9.6: persen, selisih arah global vs Indodax yang diberi tanda
    pub arah_ambang: String,
}

fn ke_ui(k: &KonfigGlobal) -> SetelanGlobalUi {
    SetelanGlobalUi {
        aktif: k.aktif,
        sebaran_maks: k.sebaran_maks_persen.normalize().to_string(),
        curiga: k.curiga_persen.normalize().to_string(),
        kurs_tanda: k.kurs_tanda_persen.normalize().to_string(),
        interval_detik: k.interval_detik.to_string(),
        arah_menit: k.arah_menit.to_string(),
        arah_ambang: k.arah_ambang_persen.normalize().to_string(),
    }
}

fn wajib_dec(nama: &str, t: &str) -> Result<Decimal, ErrorApp> {
    let t = t.trim();
    if t.is_empty() {
        return Err(ErrorApp::Gagal { pesan: format!("{nama} wajib diisi") });
    }
    desimal(t)
}

fn dari_ui(s: &SetelanGlobalUi) -> Result<KonfigGlobal, ErrorApp> {
    let interval_detik = s
        .interval_detik
        .trim()
        .parse::<u32>()
        .map_err(|_| ErrorApp::Gagal { pesan: "Interval harus bilangan bulat (detik)".into() })?;
    let arah_menit = s
        .arah_menit
        .trim()
        .parse::<u32>()
        .map_err(|_| ErrorApp::Gagal { pesan: "Jendela arah harus bilangan bulat (menit)".into() })?;
    Ok(KonfigGlobal {
        aktif: s.aktif,
        sebaran_maks_persen: wajib_dec("Sebaran maksimum", &s.sebaran_maks)?,
        curiga_persen: wajib_dec("Batas curiga selisih", &s.curiga)?,
        kurs_tanda_persen: wajib_dec("Batas tanda kurs", &s.kurs_tanda)?,
        interval_detik,
        arah_menit,
        arah_ambang_persen: wajib_dec("Batas tanda arah", &s.arah_ambang)?,
    })
}

/// Setelan nilai awal (angka sementara). Tanpa database.
#[uniffi::export]
pub fn setelan_global_bawaan() -> SetelanGlobalUi {
    ke_ui(&KonfigGlobal::default())
}

/// Baca ambang tersimpan (atau bawaan). Tanpa jaringan.
#[uniffi::export]
pub fn baca_setelan_global(path_db: String) -> Result<SetelanGlobalUi, ErrorApp> {
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    Ok(ke_ui(&muat_konfig_global(&simpan).map_err(gagal)?))
}

/// Periksa dan simpan ambang. Galat berisi pesan Indonesia siap tampil. Tanpa jaringan.
#[uniffi::export]
pub fn simpan_setelan_global(path_db: String, setelan: SetelanGlobalUi) -> Result<(), ErrorApp> {
    let konfig = dari_ui(&setelan)?;
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    simpan_konfig_global(&simpan, &konfig).map_err(gagal)
}

// ---------------------------------------------------------------------------
// Bentuk keluaran untuk layar
// ---------------------------------------------------------------------------

#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct HargaExchangeUi {
    /// contoh "Binance"
    pub nama: String,
    /// harga dalam Rupiah (harga USDT x kurs), berformat Indonesia; kosong bila tidak ada
    pub harga: String,
    /// selisih harga Indodax terhadap exchange ini, persen 2 desimal BERTANDA ("-1.25"); kosong bila tidak ada
    pub selisih: String,
}

/// T9.6: arah harga dalam N menit terakhir. Semua persen BERTANDA ("-1.25"), 2 desimal.
#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct ArahGlobalUi {
    pub menit: i32,
    /// median perubahan harga di exchange luar
    pub global: String,
    /// perubahan harga Indodax dalam USDT (kurs tidak ikut terhitung)
    pub indodax: String,
    /// global - Indodax (poin persen)
    pub selisih: String,
    /// |selisih| melewati ambang
    pub tanda: bool,
    /// jumlah exchange luar yang datanya cukup
    pub jumlah_exchange: i32,
}

#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct InfoGlobalUi {
    /// harga wajar (median x kurs), berformat Indonesia; kosong bila tanpa pembanding
    pub wajar: String,
    /// selisih Indodax terhadap harga wajar, persen 2 desimal BERTANDA; kosong bila tanpa pembanding
    pub selisih: String,
    /// "tinggi", "rendah", "sebaran tinggi", atau "tanpa pembanding"
    pub kualitas: String,
    /// selisih terlalu besar: mungkin koin berbeda dengan ticker sama atau token berskala
    pub curiga: bool,
    /// selalu tiga baris: Binance, Bybit, OKX
    pub rincian: Vec<HargaExchangeUi>,
    /// T9.6: arah harga; None bila data riwayat belum cukup (belum terkumpul N menit, ada celah)
    pub arah: Option<ArahGlobalUi>,
}

#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct StatusExchangeUi {
    pub nama: String,
    pub ok: bool,
    /// alasan singkat bila gagal (maks 80 karakter); kosong bila ok
    pub pesan: String,
}

#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct StatusGlobalUi {
    pub exchange: Vec<StatusExchangeUi>,
    /// kurs `usdt_idr` Indodax (dipakai untuk harga wajar), berformat Indonesia; kosong bila tidak ada
    pub kurs_indodax: String,
    /// kurs USDTIDR Binance; kosong bila tidak ada
    pub kurs_binance: String,
    /// selisih kurs Indodax terhadap Binance, persen 2 desimal BERTANDA; kosong bila tidak ada
    pub selisih_kurs: String,
    pub kurs_tanda: bool,
    /// gagal karena nama host tidak ditemukan (kemungkinan diblokir DNS): tampilkan petunjuk Private DNS
    pub petunjuk_dns: bool,
}

fn persen_bertanda(d: Decimal) -> String {
    d.round_dp(2).normalize().to_string()
}

fn opsi_persen(v: Option<Decimal>) -> String {
    v.map(persen_bertanda).unwrap_or_default()
}

fn pendek(mut t: String) -> String {
    t = t.replace('\n', " ");
    if t.chars().count() > 80 {
        t = t.chars().take(80).collect::<String>() + "…";
    }
    t
}

fn mirip_masalah_dns(pesan: &str) -> bool {
    let p = pesan.to_lowercase();
    ["dns", "lookup", "resolve", "no address associated", "name or service", "not known"]
        .iter()
        .any(|k| p.contains(k))
}

// ---------------------------------------------------------------------------
// Pengambilan data dan cache
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Entri {
    waktu: Instant,
    data: Result<Arc<HargaUsdt>, String>,
    /// kurs USDTIDR (hanya Binance)
    kurs: Option<Decimal>,
}

static CACHE: OnceLock<Mutex<Vec<Option<Entri>>>> = OnceLock::new();

/// T9.6: riwayat titik harga untuk arah. Hanya di memori selama proses hidup (titik > 45 menit
/// dibuang); tidak pernah masuk database.
static RIWAYAT_ARAH: OnceLock<Arc<Mutex<RiwayatArah>>> = OnceLock::new();

fn riwayat_arah() -> Arc<Mutex<RiwayatArah>> {
    RIWAYAT_ARAH.get_or_init(|| Arc::new(Mutex::new(RiwayatArah::baru()))).clone()
}

fn detik_sekarang() -> i64 {
    sekarang_ms() / 1000
}

fn cache() -> &'static Mutex<Vec<Option<Entri>>> {
    CACHE.get_or_init(|| Mutex::new((0..Exchange::SEMUA.len()).map(|_| None).collect()))
}

fn ambil_satu(klien: &KlienGlobal, ex: Exchange) -> Entri {
    let hasil: Hasil<(HargaUsdt, Option<Decimal>)> = klien.ambil_mentah(ex).and_then(|t| match ex {
        Exchange::Binance => parse_binance(&t).map(|p| (p.harga, p.kurs_usdt_idr)),
        Exchange::Bybit => parse_bybit(&t).map(|h| (h, None)),
        Exchange::Okx => parse_okx(&t).map(|h| (h, None)),
    });
    match hasil {
        Ok((h, kurs)) => Entri { waktu: Instant::now(), data: Ok(Arc::new(h)), kurs },
        Err(e) => Entri { waktu: Instant::now(), data: Err(pendek(e.to_string())), kurs: None },
    }
}

/// Ambil (atau pakai cache) harga tiga exchange. Yang perlu diambil dikerjakan paralel.
/// Kunci cache tidak dipegang saat jaringan berjalan.
/// Data yang baru berhasil diambil juga dicatat ke `riwayat` (hanya koin di `dilacak`).
fn ambil_snapshot(interval_detik: u32, riwayat: &Mutex<RiwayatArah>, dilacak: &BTreeSet<String>) -> Vec<(Exchange, Entri)> {
    let sekarang = Instant::now();
    let ttl_ok = Duration::from_secs(u64::from(interval_detik));
    let ttl_gagal = Duration::from_secs(u64::from(interval_detik).min(TTL_GAGAL_DETIK));
    let perlu: Vec<usize> = {
        let g = cache().lock().unwrap_or_else(|e| e.into_inner());
        (0..Exchange::SEMUA.len())
            .filter(|&i| match &g[i] {
                None => true,
                Some(e) => {
                    let umur = sekarang.saturating_duration_since(e.waktu);
                    umur >= if e.data.is_ok() { ttl_ok } else { ttl_gagal }
                }
            })
            .collect()
    };
    if !perlu.is_empty() {
        let klien = KlienGlobal::baru();
        let baru: Vec<(usize, Entri)> = std::thread::scope(|s| {
            let kerja: Vec<_> = perlu
                .iter()
                .map(|&i| {
                    let k = &klien;
                    s.spawn(move || (i, ambil_satu(k, Exchange::SEMUA[i])))
                })
                .collect();
            kerja.into_iter().filter_map(|h| h.join().ok()).collect()
        });
        // Catat titik harga untuk arah SEBELUM kunci cache diambil (kunci tidak pernah bersarang).
        {
            let waktu = detik_sekarang();
            let mut r = riwayat.lock().unwrap_or_else(|e| e.into_inner());
            for (i, e) in &baru {
                if let Ok(h) = &e.data {
                    r.catat_global(waktu, Exchange::SEMUA[*i], &**h, dilacak);
                }
            }
        }
        let mut g = cache().lock().unwrap_or_else(|e| e.into_inner());
        for (i, e) in baru {
            g[i] = Some(e);
        }
    }
    let g = cache().lock().unwrap_or_else(|e| e.into_inner());
    (0..Exchange::SEMUA.len()).filter_map(|i| g[i].clone().map(|e| (Exchange::SEMUA[i], e))).collect()
}

/// Pengambilan yang sedang berjalan di thread latar. Panggil `selesai` untuk menunggu hasilnya.
pub(crate) struct TugasGlobal {
    konfig: KonfigGlobal,
    kurs_indodax: Option<Decimal>,
    riwayat: Arc<Mutex<RiwayatArah>>,
    handle: Option<JoinHandle<Vec<(Exchange, Entri)>>>,
}

/// Mulai pengambilan di thread terpisah (paralel dengan pengambilan `trades`). Bila fitur
/// dimatikan di setelan, tidak ada permintaan sama sekali. Kurs Indodax diambil dari
/// `usdt_idr` pada ringkasan LENGKAP (bukan hanya kandidat yang lolos tahap 1).
pub(crate) fn mulai_global(konfig: KonfigGlobal, ringkasan: &[RingkasanPasar]) -> TugasGlobal {
    let kurs_indodax = ringkasan.iter().find(|r| r.pair == "usdt_idr").map(|r| r.last);
    let riwayat = riwayat_arah();
    let handle = if konfig.aktif {
        // T9.6: harga Indodax semua pair dicatat dari ringkasan yang sama (tanpa request baru),
        // dalam USDT memakai kurs `usdt_idr` ringkasan itu; berlaku di mode Daftar dan Lonjakan.
        if let Some(kurs) = kurs_indodax {
            let mut r = riwayat.lock().unwrap_or_else(|e| e.into_inner());
            r.catat_indodax(detik_sekarang(), ringkasan, kurs);
        }
        let dilacak: BTreeSet<String> =
            ringkasan.iter().filter(|r| r.pair.ends_with("_idr")).map(|r| nama_koin(&r.pair)).collect();
        let interval = konfig.interval_detik;
        let untuk_thread = riwayat.clone();
        Some(std::thread::spawn(move || ambil_snapshot(interval, &untuk_thread, &dilacak)))
    } else {
        None
    };
    TugasGlobal { konfig, kurs_indodax, riwayat, handle }
}

impl TugasGlobal {
    pub(crate) fn selesai(self) -> HasilGlobal {
        let entri = self.handle.and_then(|h| h.join().ok()).unwrap_or_default();
        HasilGlobal {
            konfig: self.konfig,
            kurs_indodax: self.kurs_indodax,
            entri,
            riwayat: self.riwayat,
            sekarang: detik_sekarang(),
        }
    }
}

pub(crate) struct HasilGlobal {
    konfig: KonfigGlobal,
    kurs_indodax: Option<Decimal>,
    entri: Vec<(Exchange, Entri)>,
    /// T9.6: riwayat titik harga (memori) dan waktu penilaian arah (epoch detik)
    riwayat: Arc<Mutex<RiwayatArah>>,
    sekarang: i64,
}

impl HasilGlobal {
    fn entri_untuk(&self, ex: Exchange) -> Option<&Entri> {
        self.entri.iter().find(|(e, _)| *e == ex).map(|(_, en)| en)
    }

    /// Info satu baris. None bila fitur mati, kurs Indodax tidak ada, atau pengambilan terputus.
    pub(crate) fn info(&self, pair: &str, harga: Decimal) -> Option<InfoGlobalUi> {
        if !self.konfig.aktif || self.entri.is_empty() {
            return None;
        }
        let kurs = self.kurs_indodax?;
        let sumber: Vec<(Exchange, Option<&HargaUsdt>)> = Exchange::SEMUA
            .iter()
            .map(|ex| {
                let data = self.entri_untuk(*ex).and_then(|e| e.data.as_ref().ok()).map(|a| &**a);
                (*ex, data)
            })
            .collect();
        let simbol = nama_koin(pair);
        let g = hitung_harga_global(&self.konfig, &simbol, harga, kurs, &sumber);
        let arah = {
            let r = self.riwayat.lock().unwrap_or_else(|e| e.into_inner());
            r.hitung_arah(&self.konfig, &simbol, self.sekarang)
        }
        .map(|a| ArahGlobalUi {
            menit: a.menit as i32,
            global: persen_bertanda(a.global_persen),
            indodax: persen_bertanda(a.indodax_persen),
            selisih: persen_bertanda(a.selisih_poin),
            tanda: a.tanda,
            jumlah_exchange: a.jumlah_exchange as i32,
        });
        Some(InfoGlobalUi {
            wajar: g.wajar_idr.map(fmt_harga).unwrap_or_default(),
            selisih: opsi_persen(g.selisih_persen),
            kualitas: g.kualitas.teks().to_string(),
            curiga: g.curiga,
            rincian: g
                .rincian
                .iter()
                .map(|r| HargaExchangeUi {
                    nama: r.exchange.nama().to_string(),
                    harga: r.harga_idr.map(fmt_harga).unwrap_or_default(),
                    selisih: opsi_persen(r.selisih_persen),
                })
                .collect(),
            arah,
        })
    }

    /// Status koneksi per exchange dan perbandingan kurs. None bila fitur dimatikan.
    pub(crate) fn status(&self) -> Option<StatusGlobalUi> {
        if !self.konfig.aktif {
            return None;
        }
        let mut dns = false;
        let exchange: Vec<StatusExchangeUi> = Exchange::SEMUA
            .iter()
            .map(|ex| {
                let (ok, pesan) = match self.entri_untuk(*ex) {
                    Some(Entri { data: Ok(_), .. }) => (true, String::new()),
                    Some(Entri { data: Err(p), .. }) => {
                        if mirip_masalah_dns(p) {
                            dns = true;
                        }
                        (false, p.clone())
                    }
                    None => (false, "tidak tersedia".to_string()),
                };
                StatusExchangeUi { nama: ex.nama().to_string(), ok, pesan }
            })
            .collect();
        let kurs_binance = self.entri_untuk(Exchange::Binance).and_then(|e| e.kurs);
        let selisih = self.kurs_indodax.and_then(|k| bandingkan_kurs(&self.konfig, k, kurs_binance));
        Some(StatusGlobalUi {
            exchange,
            kurs_indodax: self.kurs_indodax.map(fmt_harga).unwrap_or_default(),
            kurs_binance: kurs_binance.map(fmt_harga).unwrap_or_default(),
            selisih_kurs: opsi_persen(selisih.as_ref().map(|s| s.selisih_persen)),
            kurs_tanda: selisih.map(|s| s.tanda).unwrap_or(false),
            petunjuk_dns: dns,
        })
    }
}

// ---------------------------------------------------------------------------
// Tes (tanpa jaringan)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tes_global_ffi {
    use super::*;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn ok(harga: &[(&str, &str)], kurs: Option<&str>) -> Entri {
        let m: HargaUsdt = harga.iter().map(|(k, v)| (k.to_string(), d(v))).collect();
        Entri { waktu: Instant::now(), data: Ok(Arc::new(m)), kurs: kurs.map(d) }
    }

    fn galat(p: &str) -> Entri {
        Entri { waktu: Instant::now(), data: Err(p.to_string()), kurs: None }
    }

    fn hasil(entri: Vec<(Exchange, Entri)>) -> HasilGlobal {
        HasilGlobal {
            konfig: KonfigGlobal::default(),
            kurs_indodax: Some(d("17000")),
            entri,
            riwayat: Arc::new(Mutex::new(RiwayatArah::baru())),
            sekarang: 600,
        }
    }

    #[test]
    fn setelan_ui_bolak_balik_sama_dengan_bawaan() {
        let ui = setelan_global_bawaan();
        assert!(ui.aktif);
        assert_eq!(ui.sebaran_maks, "2");
        assert_eq!(ui.curiga, "10");
        assert_eq!(ui.kurs_tanda, "1");
        assert_eq!(ui.interval_detik, "60");
        assert_eq!(ui.arah_menit, "10");
        assert_eq!(ui.arah_ambang, "0.5");
        assert_eq!(dari_ui(&ui).unwrap(), KonfigGlobal::default());
    }

    #[test]
    fn dari_ui_menerima_koma_dan_menolak_kosong_atau_rusak() {
        let mut ui = setelan_global_bawaan();
        ui.sebaran_maks = "2,5".to_string();
        assert_eq!(dari_ui(&ui).unwrap().sebaran_maks_persen, Decimal::new(25, 1));
        let mut ui = setelan_global_bawaan();
        ui.curiga = String::new();
        assert!(dari_ui(&ui).is_err());
        let mut ui = setelan_global_bawaan();
        ui.interval_detik = "abc".to_string();
        assert!(dari_ui(&ui).is_err());
        let mut ui = setelan_global_bawaan();
        ui.arah_menit = "sepuluh".to_string();
        assert!(dari_ui(&ui).is_err());
        let mut ui = setelan_global_bawaan();
        ui.arah_ambang = "0,8".to_string();
        assert_eq!(dari_ui(&ui).unwrap().arah_ambang_persen, Decimal::new(8, 1));
    }

    #[test]
    fn info_baris_berformat_dan_selisih_bertanda() {
        let h = hasil(vec![
            (Exchange::Binance, ok(&[("ABC", "0.2")], Some("17300"))),
            (Exchange::Bybit, ok(&[("ABC", "0.201")], None)),
            (Exchange::Okx, galat("gagal")),
        ]);
        let i = h.info("abc_idr", d("3400")).unwrap();
        assert_eq!(i.kualitas, "tinggi"); // dua exchange punya koin ini
        assert!(!i.curiga);
        assert_eq!(i.rincian.len(), 3);
        assert_eq!(i.rincian[0].nama, "Binance");
        assert_eq!(i.rincian[2].harga, ""); // OKX gagal
        assert_eq!(i.rincian[2].selisih, "");
        assert!(i.rincian[0].selisih == "0"); // Indodax 3.400 = Binance 0,2 x 17.000
        assert!(i.rincian[1].selisih.starts_with('-')); // Indodax lebih murah dari Bybit
        assert!(!i.wajar.is_empty());
    }

    #[test]
    fn info_memuat_arah_hanya_bila_riwayat_cukup() {
        let h = hasil(vec![(Exchange::Binance, ok(&[("ABC", "0.2020")], Some("17300")))]);
        assert!(h.info("abc_idr", d("3400")).unwrap().arah.is_none()); // riwayat kosong
        {
            let mut r = h.riwayat.lock().unwrap();
            let dl: BTreeSet<String> = ["ABC".to_string()].into_iter().collect();
            let awal: HargaUsdt = [("ABC".to_string(), d("0.2000"))].into_iter().collect();
            let akhir: HargaUsdt = [("ABC".to_string(), d("0.2020"))].into_iter().collect();
            r.catat_titik_indodax("ABC", 0, d("0.2"));
            r.catat_titik_indodax("ABC", 600, d("0.2"));
            r.catat_global(0, Exchange::Binance, &awal, &dl);
            r.catat_global(600, Exchange::Binance, &akhir, &dl);
        }
        let a = h.info("abc_idr", d("3400")).unwrap().arah.unwrap();
        assert_eq!(a.menit, 10);
        assert_eq!(a.global, "1"); // +1% di Binance
        assert_eq!(a.indodax, "0"); // Indodax datar dalam USDT
        assert_eq!(a.selisih, "1");
        assert!(a.tanda); // 1 >= ambang 0,5
        assert_eq!(a.jumlah_exchange, 1);
        // koin lain tidak punya riwayat
        assert!(h.info("xyz_idr", d("3400")).unwrap().arah.is_none());
    }

    #[test]
    fn info_koin_tanpa_pembanding_dan_fitur_mati() {
        let h = hasil(vec![(Exchange::Binance, ok(&[("ABC", "0.2")], None))]);
        let i = h.info("xyz_idr", d("3400")).unwrap();
        assert_eq!(i.kualitas, "tanpa pembanding");
        assert_eq!(i.wajar, "");
        assert_eq!(i.selisih, "");
        // pengambilan terputus: tidak ada entri sama sekali
        assert!(hasil(vec![]).info("abc_idr", d("3400")).is_none());
        // fitur dimatikan
        let mut mati = hasil(vec![(Exchange::Binance, ok(&[("ABC", "0.2")], None))]);
        mati.konfig.aktif = false;
        assert!(mati.info("abc_idr", d("3400")).is_none());
        assert!(mati.status().is_none());
        // kurs Indodax tidak ada
        let mut tanpa_kurs = hasil(vec![(Exchange::Binance, ok(&[("ABC", "0.2")], None))]);
        tanpa_kurs.kurs_indodax = None;
        assert!(tanpa_kurs.info("abc_idr", d("3400")).is_none());
    }

    #[test]
    fn status_koneksi_kurs_dan_petunjuk_dns() {
        let h = hasil(vec![
            (Exchange::Binance, ok(&[("ABC", "0.2")], Some("17300"))),
            (Exchange::Bybit, galat("https://api.bybit.com: Dns Failed: failed to lookup address")),
        ]);
        let s = h.status().unwrap();
        assert_eq!(s.exchange.len(), 3);
        assert!(s.exchange[0].ok);
        assert!(!s.exchange[1].ok && s.exchange[1].pesan.contains("Dns"));
        assert!(!s.exchange[2].ok && s.exchange[2].pesan == "tidak tersedia"); // OKX tidak ada entri
        assert!(s.petunjuk_dns);
        assert_eq!(s.kurs_binance, fmt_harga(d("17300")));
        // 17.000 vs 17.300 = -1,73%: bertanda dan melewati ambang 1%
        assert_eq!(s.selisih_kurs, "-1.73");
        assert!(s.kurs_tanda);
    }

    #[test]
    fn status_tanpa_dns_dan_tanpa_kurs_binance() {
        let h = hasil(vec![
            (Exchange::Binance, ok(&[("ABC", "0.2")], None)),
            (Exchange::Bybit, galat("http 403: forbidden")),
            (Exchange::Okx, ok(&[("ABC", "0.2")], None)),
        ]);
        let s = h.status().unwrap();
        assert!(!s.petunjuk_dns);
        assert_eq!(s.kurs_binance, "");
        assert_eq!(s.selisih_kurs, "");
        assert!(!s.kurs_tanda);
    }

    #[test]
    fn pesan_dipendekkan() {
        let panjang = "x".repeat(200);
        assert!(pendek(panjang).chars().count() <= 81);
        assert_eq!(pendek("a\nb".to_string()), "a b");
    }
}
