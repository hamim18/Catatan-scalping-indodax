//! T9.5: Harga global tahap 1. HANYA informasi pembanding: tidak menyentuh posisi, profit, target,
//! grafik, statistik, maupun database trade. Modul MURNI (tanpa jaringan, tanpa database): hanya
//! mengurai teks JSON yang diberikan pemanggil lalu menghitung. Pengambilan data ada di
//! `client::KlienGlobal`.
//!
//! Acuan: skrip pengguna `harga_wajar_indodax.py` (alur dan rumus sama):
//! - harga wajar (IDR) = median harga USDT dari exchange sumber x kurs USDT/IDR;
//! - selisih (%) = (harga Indodax / harga wajar - 1) x 100;
//! - sebaran sumber (%) = (maks - min) / median x 100.
//!
//! Beda dari skrip (keputusan 14 Nov 2026): sumber hanya Binance, Bybit, OKX (tanpa Gate, MEXC,
//! KuCoin), sehingga label kualitas memakai satu tingkat sumber saja.
//! Kurs harga wajar = `usdt_idr` Indodax (selisih kurs terhadap Binance USDTIDR hanya ditampilkan).
//!
//! Bentuk respons (dari skrip pengguna; BELUM diverifikasi ulang oleh aplikasi):
//! - Binance GET /api/v3/ticker/price: [{"symbol":"BTCUSDT","price":"..."}, ...]
//! - Bybit GET /v5/market/tickers?category=spot: {"retCode":0,"result":{"list":[{"symbol","lastPrice"}]}}
//! - OKX GET /api/v5/market/tickers?instType=SPOT: {"code":"0","data":[{"instId":"BTC-USDT","last"}]}
//!
//! Semua hitungan memakai Decimal. Pembagian memakai checked_div agar tidak panik.
use crate::screener::RingkasanPasar;
use crate::{Error, Hasil};
use rust_decimal::Decimal;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::str::FromStr;

// ---------------------------------------------------------------------------
// Exchange
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Exchange {
    Binance,
    Bybit,
    Okx,
}

impl Exchange {
    /// Urutan tampil dan urutan indeks cache.
    pub const SEMUA: [Exchange; 3] = [Exchange::Binance, Exchange::Bybit, Exchange::Okx];

    pub fn nama(self) -> &'static str {
        match self {
            Exchange::Binance => "Binance",
            Exchange::Bybit => "Bybit",
            Exchange::Okx => "OKX",
        }
    }

    /// Endpoint publik (tanpa key) yang mengembalikan harga terakhir semua pair.
    pub fn url(self) -> &'static str {
        match self {
            Exchange::Binance => "https://api.binance.com/api/v3/ticker/price",
            Exchange::Bybit => "https://api.bybit.com/v5/market/tickers?category=spot",
            Exchange::Okx => "https://www.okx.com/api/v5/market/tickers?instType=SPOT",
        }
    }
}

/// Simbol koin HURUF BESAR (contoh "BTC") -> harga terakhir dalam USDT.
pub type HargaUsdt = BTreeMap<String, Decimal>;

// ---------------------------------------------------------------------------
// Pembantu angka
// ---------------------------------------------------------------------------

fn seratus() -> Decimal {
    Decimal::from(100_i64)
}

fn dec_teks(s: &str) -> Option<Decimal> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    // Harga sangat kecil kadang ditulis ilmiah ("1e-7"); from_str menolaknya.
    Decimal::from_str(t).ok().or_else(|| Decimal::from_scientific(t).ok())
}

fn dec(v: &Value) -> Option<Decimal> {
    match v {
        Value::String(s) => dec_teks(s),
        Value::Number(n) => dec_teks(&n.to_string()),
        _ => None,
    }
}

fn positif(v: Option<&Value>) -> Option<Decimal> {
    v.and_then(dec).filter(|x| *x > Decimal::ZERO)
}

/// a / b x 100, atau None bila b nol atau terjadi luapan.
fn persen(a: Decimal, b: Decimal) -> Option<Decimal> {
    a.checked_mul(seratus())?.checked_div(b)
}

fn median(mut v: Vec<Decimal>) -> Option<Decimal> {
    if v.is_empty() {
        return None;
    }
    v.sort();
    let n = v.len();
    if n % 2 == 1 {
        Some(v[n / 2])
    } else {
        v[n / 2 - 1].checked_add(v[n / 2])?.checked_div(Decimal::from(2_i64))
    }
}

fn json(teks: &str) -> Hasil<Value> {
    serde_json::from_str(teks).map_err(|e| Error::Json(e.to_string()))
}

// ---------------------------------------------------------------------------
// Parser tiap exchange
// ---------------------------------------------------------------------------

/// Hasil Binance: harga semua pair `*USDT` dan, bila ada di respons yang sama, kurs USDTIDR.
#[derive(Debug, Clone, PartialEq)]
pub struct HasilBinance {
    pub harga: HargaUsdt,
    pub kurs_usdt_idr: Option<Decimal>,
}

/// Urai Binance. Error bila respons bukan daftar (misalnya objek galat atau pembatasan wilayah)
/// atau tidak ada pair USDT satu pun.
pub fn parse_binance(teks: &str) -> Hasil<HasilBinance> {
    let v = json(teks)?;
    let daftar = v
        .as_array()
        .ok_or_else(|| Error::Json("respons Binance bukan daftar harga".into()))?;
    let mut harga = HargaUsdt::new();
    let mut kurs = None;
    for item in daftar {
        let Some(simbol) = item.get("symbol").and_then(Value::as_str) else {
            continue;
        };
        let Some(p) = positif(item.get("price")) else {
            continue;
        };
        if simbol == "USDTIDR" {
            kurs = Some(p);
        } else if let Some(dasar) = simbol.strip_suffix("USDT") {
            if !dasar.is_empty() {
                harga.insert(dasar.to_uppercase(), p);
            }
        }
    }
    if harga.is_empty() {
        return Err(Error::Json("respons Binance tidak memuat pair USDT satu pun".into()));
    }
    Ok(HasilBinance { harga, kurs_usdt_idr: kurs })
}

fn kode_galat(v: &Value, kunci: &str) -> Option<i64> {
    v.get(kunci).and_then(|c| c.as_i64().or_else(|| c.as_str().and_then(|s| s.trim().parse().ok())))
}

/// Urai Bybit. Error bila `retCode` bukan 0 atau daftar tidak ada.
pub fn parse_bybit(teks: &str) -> Hasil<HargaUsdt> {
    let v = json(teks)?;
    if let Some(k) = kode_galat(&v, "retCode") {
        if k != 0 {
            let pesan = v.get("retMsg").and_then(Value::as_str).unwrap_or("");
            return Err(Error::Json(format!("Bybit menolak (kode {k}) {pesan}").trim().to_string()));
        }
    }
    let daftar = v
        .get("result")
        .and_then(|r| r.get("list"))
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Json("respons Bybit tidak memuat result.list".into()))?;
    let mut harga = HargaUsdt::new();
    for item in daftar {
        let Some(simbol) = item.get("symbol").and_then(Value::as_str) else {
            continue;
        };
        let Some(p) = positif(item.get("lastPrice")) else {
            continue;
        };
        if let Some(dasar) = simbol.strip_suffix("USDT") {
            if !dasar.is_empty() {
                harga.insert(dasar.to_uppercase(), p);
            }
        }
    }
    if harga.is_empty() {
        return Err(Error::Json("respons Bybit tidak memuat pair USDT satu pun".into()));
    }
    Ok(harga)
}

/// Urai OKX. Error bila `code` bukan "0" atau daftar tidak ada.
pub fn parse_okx(teks: &str) -> Hasil<HargaUsdt> {
    let v = json(teks)?;
    if let Some(k) = kode_galat(&v, "code") {
        if k != 0 {
            let pesan = v.get("msg").and_then(Value::as_str).unwrap_or("");
            return Err(Error::Json(format!("OKX menolak (kode {k}) {pesan}").trim().to_string()));
        }
    }
    let daftar = v
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Json("respons OKX tidak memuat data".into()))?;
    let mut harga = HargaUsdt::new();
    for item in daftar {
        let Some(id) = item.get("instId").and_then(Value::as_str) else {
            continue;
        };
        let Some(p) = positif(item.get("last")) else {
            continue;
        };
        if let Some(dasar) = id.strip_suffix("-USDT") {
            if !dasar.is_empty() {
                harga.insert(dasar.to_uppercase(), p);
            }
        }
    }
    if harga.is_empty() {
        return Err(Error::Json("respons OKX tidak memuat pair USDT satu pun".into()));
    }
    Ok(harga)
}

// ---------------------------------------------------------------------------
// Konfigurasi
// ---------------------------------------------------------------------------

/// Ambang Harga global. Hanya memengaruhi tanda dan label di layar; tidak ada yang menyaring pair.
#[derive(Debug, Clone, PartialEq)]
pub struct KonfigGlobal {
    /// false = tidak mengambil data exchange luar sama sekali
    pub aktif: bool,
    /// sebaran antar exchange di atas ini = kualitas "sebaran tinggi" (skrip: 2)
    pub sebaran_maks_persen: Decimal,
    /// |selisih| di atas ini = tanda curiga (ticker sama bisa koin lain atau token berskala)
    pub curiga_persen: Decimal,
    /// |selisih kurs Indodax vs Binance| di atas ini = tanda
    pub kurs_tanda_persen: Decimal,
    /// jarak minimal antar-pengambilan data exchange luar (detik)
    pub interval_detik: u32,
    /// T9.6: lebar jendela arah harga, menit (5 sampai 30)
    pub arah_menit: u32,
    /// T9.6: |arah global - arah Indodax| (poin persen) di atas atau sama dengan ini = diberi tanda
    pub arah_ambang_persen: Decimal,
}

impl Default for KonfigGlobal {
    /// Angka SEMENTARA (tebakan; tentukan dari uji lapangan), kecuali sebaran 2% dari skrip.
    fn default() -> Self {
        Self {
            aktif: true,
            sebaran_maks_persen: Decimal::from(2_i64),
            curiga_persen: Decimal::from(10_i64),
            kurs_tanda_persen: Decimal::from(1_i64),
            interval_detik: 60,
            arah_menit: 10,
            arah_ambang_persen: Decimal::new(5, 1), // 0,5 (sedikit di atas biaya round-trip 0,41%)
        }
    }
}

impl KonfigGlobal {
    /// Toleransi waktu (detik) untuk arah: titik terakhir tidak boleh lebih tua dari ini, dan titik
    /// acuan N menit lalu dicari paling jauh sejauh ini dari waktu targetnya. Data exchange luar
    /// masuk paling cepat tiap `interval_detik` (dan layar memolling tiap 20 sampai 45 detik),
    /// jadi toleransi mengikuti interval, minimal 90 detik.
    pub fn toleransi_arah_detik(&self) -> i64 {
        (i64::from(self.interval_detik) * 3 / 2).max(90)
    }
}

// ---------------------------------------------------------------------------
// Perhitungan
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kualitas {
    /// minimal 2 exchange punya koin ini dan sebarannya wajar
    Tinggi,
    /// hanya 1 exchange punya koin ini
    Rendah,
    /// minimal 2 exchange, tetapi harga antar exchange berbeda jauh
    SebaranTinggi,
    TanpaPembanding,
}

impl Kualitas {
    pub fn teks(self) -> &'static str {
        match self {
            Kualitas::Tinggi => "tinggi",
            Kualitas::Rendah => "rendah",
            Kualitas::SebaranTinggi => "sebaran tinggi",
            Kualitas::TanpaPembanding => "tanpa pembanding",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RincianExchange {
    pub exchange: Exchange,
    /// harga USDT x kurs; None = exchange ini tidak punya koin atau gagal diambil
    pub harga_idr: Option<Decimal>,
    /// (harga Indodax / harga exchange - 1) x 100
    pub selisih_persen: Option<Decimal>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HargaGlobal {
    /// median harga USDT x kurs; None bila tanpa pembanding
    pub wajar_idr: Option<Decimal>,
    /// (harga Indodax / harga wajar - 1) x 100
    pub selisih_persen: Option<Decimal>,
    /// (maks - min) / median x 100; None bila kurang dari 2 sumber
    pub sebaran_persen: Option<Decimal>,
    pub kualitas: Kualitas,
    /// |selisih| melebihi `curiga_persen`
    pub curiga: bool,
    /// selalu tiga baris, urut `Exchange::SEMUA`
    pub rincian: Vec<RincianExchange>,
}

/// Hitung harga global satu koin.
/// - `sumber`: untuk tiap exchange, `Some(daftar harga)` bila berhasil diambil, `None` bila gagal.
/// - `kurs_idr`: kurs USDT/IDR untuk mengonversi (bawaan: `usdt_idr` Indodax). Harus positif.
pub fn hitung_harga_global(
    k: &KonfigGlobal,
    simbol: &str,
    harga_indodax: Decimal,
    kurs_idr: Decimal,
    sumber: &[(Exchange, Option<&HargaUsdt>)],
) -> HargaGlobal {
    let simbol = simbol.to_uppercase();
    let mut rincian = Vec::new();
    let mut harga_usdt: Vec<Decimal> = Vec::new();
    for (ex, data) in sumber {
        let p = data.and_then(|m| m.get(&simbol).copied());
        if let Some(p) = p {
            harga_usdt.push(p);
        }
        let harga_idr = if kurs_idr > Decimal::ZERO { p.and_then(|p| p.checked_mul(kurs_idr)) } else { None };
        let selisih_persen = harga_idr.and_then(|h| persen(harga_indodax - h, h));
        rincian.push(RincianExchange { exchange: *ex, harga_idr, selisih_persen });
    }

    let med = if kurs_idr > Decimal::ZERO && harga_indodax > Decimal::ZERO { median(harga_usdt.clone()) } else { None };
    let Some(med) = med else {
        return HargaGlobal {
            wajar_idr: None,
            selisih_persen: None,
            sebaran_persen: None,
            kualitas: Kualitas::TanpaPembanding,
            curiga: false,
            rincian,
        };
    };

    let wajar = med.checked_mul(kurs_idr);
    let selisih = wajar.and_then(|w| persen(harga_indodax - w, w));
    let sebaran = if harga_usdt.len() > 1 {
        let maks = harga_usdt.iter().copied().max().unwrap_or(med);
        let min = harga_usdt.iter().copied().min().unwrap_or(med);
        persen(maks - min, med)
    } else {
        None
    };
    let kualitas = match (harga_usdt.len(), sebaran) {
        (n, Some(s)) if n > 1 && s > k.sebaran_maks_persen => Kualitas::SebaranTinggi,
        (n, _) if n >= 2 => Kualitas::Tinggi,
        _ => Kualitas::Rendah,
    };
    let curiga = selisih.map(|s| s.abs() > k.curiga_persen).unwrap_or(false);
    HargaGlobal { wajar_idr: wajar, selisih_persen: selisih, sebaran_persen: sebaran, kualitas, curiga, rincian }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelisihKurs {
    pub indodax: Decimal,
    pub binance: Decimal,
    /// (kurs Indodax / kurs Binance - 1) x 100
    pub selisih_persen: Decimal,
    /// |selisih| melebihi `kurs_tanda_persen`
    pub tanda: bool,
}

/// Bandingkan `usdt_idr` Indodax dengan USDTIDR Binance. None bila salah satu tidak ada atau nol.
pub fn bandingkan_kurs(k: &KonfigGlobal, indodax: Decimal, binance: Option<Decimal>) -> Option<SelisihKurs> {
    let binance = binance.filter(|b| *b > Decimal::ZERO)?;
    if indodax <= Decimal::ZERO {
        return None;
    }
    let selisih = persen(indodax - binance, binance)?;
    Some(SelisihKurs { indodax, binance, selisih_persen: selisih, tanda: selisih.abs() > k.kurs_tanda_persen })
}

// ---------------------------------------------------------------------------
// T9.6: arah harga global (perubahan N menit terakhir, global dibanding Indodax)
// ---------------------------------------------------------------------------

/// Titik yang lebih tua dari ini (detik) dibuang. 45 menit: cukup untuk N maksimum 30 menit
/// ditambah toleransi pencarian titik acuan.
pub const MAKS_RIWAYAT_ARAH_DETIK: i64 = 2700;

type Antrean = VecDeque<(i64, Decimal)>;

/// Tambah satu titik (epoch detik, harga). Waktu yang tidak lebih baru dari titik terakhir
/// dilewati. Titik lebih tua dari `MAKS_RIWAYAT_ARAH_DETIK` dibuang.
fn tambah_titik(antrean: &mut Antrean, waktu: i64, harga: Decimal) {
    if let Some(&(w, _)) = antrean.back() {
        if waktu <= w {
            return;
        }
    }
    antrean.push_back((waktu, harga));
    while let Some(&(w, _)) = antrean.front() {
        if w < waktu - MAKS_RIWAYAT_ARAH_DETIK {
            antrean.pop_front();
        } else {
            break;
        }
    }
}

/// Perubahan (%) dari titik acuan sampai titik terakhir. Titik terakhir tidak boleh basi
/// (`sekarang - waktu > toleransi`); titik acuan = titik terlama dari titik terakhir yang paling
/// dekat ke `menit` menit sebelum titik terakhir, dan harus dalam `toleransi` dari waktu target itu.
/// None bila data tidak cukup (celah, belum lama terkumpul) atau terjadi luapan.
fn perubahan(antrean: &Antrean, sekarang: i64, menit: u32, toleransi: i64) -> Option<Decimal> {
    let (w_akhir, h_akhir) = *antrean.back()?;
    if sekarang - w_akhir > toleransi {
        return None;
    }
    let target = w_akhir - i64::from(menit) * 60;
    let (w0, h0) = antrean
        .iter()
        .copied()
        .filter(|&(w, _)| w < w_akhir)
        .min_by_key(|&(w, _)| (w - target).abs())?;
    if (w0 - target).abs() > toleransi || h0 <= Decimal::ZERO {
        return None;
    }
    persen(h_akhir.checked_sub(h0)?, h0)
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArahHarga {
    /// lebar jendela, menit
    pub menit: u32,
    /// median perubahan harga USDT dari exchange luar yang datanya cukup, persen BERTANDA
    pub global_persen: Decimal,
    /// perubahan harga Indodax dalam USDT (harga IDR / `usdt_idr` pada tiap titik), persen BERTANDA;
    /// dihitung dalam USDT agar naik-turun kurs tidak terhitung sebagai gerak harga koin
    pub indodax_persen: Decimal,
    /// global - Indodax, poin persen. Positif = global naik lebih banyak (atau turun lebih sedikit)
    pub selisih_poin: Decimal,
    /// jumlah exchange luar yang datanya cukup
    pub jumlah_exchange: usize,
    /// |selisih_poin| >= `arah_ambang_persen`
    pub tanda: bool,
}

/// Riwayat titik harga untuk arah. HANYA di memori (pemanggil menyimpannya di statik; tidak pernah
/// masuk database). Murni: waktu dan harga diberikan pemanggil. Hanya koin yang dilacak
/// (koin pasangan IDR Indodax) yang dicatat.
#[derive(Debug, Default, Clone)]
pub struct RiwayatArah {
    indodax: BTreeMap<String, Antrean>,
    luar: BTreeMap<(Exchange, String), Antrean>,
}

impl RiwayatArah {
    pub fn baru() -> Self {
        Self::default()
    }

    pub fn kosongkan(&mut self) {
        self.indodax.clear();
        self.luar.clear();
    }

    /// Catat satu titik harga Indodax dalam USDT untuk `simbol` (huruf besar).
    pub fn catat_titik_indodax(&mut self, simbol: &str, waktu: i64, harga_usdt: Decimal) {
        if harga_usdt <= Decimal::ZERO {
            return;
        }
        tambah_titik(self.indodax.entry(simbol.to_uppercase()).or_default(), waktu, harga_usdt);
    }

    /// Catat harga `last` semua pair `*_idr` dari ringkasan pasar, dikonversi ke USDT dengan `kurs`
    /// (`usdt_idr` Indodax pada ringkasan yang sama). Kurs tidak positif = tidak mencatat apa pun.
    pub fn catat_indodax(&mut self, waktu: i64, ringkasan: &[RingkasanPasar], kurs: Decimal) {
        if kurs <= Decimal::ZERO {
            return;
        }
        for r in ringkasan {
            let Some(simbol) = r.pair.strip_suffix("_idr") else {
                continue;
            };
            if simbol.is_empty() || r.last <= Decimal::ZERO {
                continue;
            }
            if let Some(usdt) = r.last.checked_div(kurs) {
                self.catat_titik_indodax(simbol, waktu, usdt);
            }
        }
    }

    /// Catat harga USDT satu exchange luar, hanya untuk koin di `dilacak` (huruf besar).
    pub fn catat_global(&mut self, waktu: i64, ex: Exchange, data: &HargaUsdt, dilacak: &BTreeSet<String>) {
        for simbol in dilacak {
            if let Some(&p) = data.get(simbol) {
                if p > Decimal::ZERO {
                    tambah_titik(self.luar.entry((ex, simbol.clone())).or_default(), waktu, p);
                }
            }
        }
    }

    /// Arah satu koin pada waktu `sekarang`. None bila Indodax atau exchange luar belum punya data
    /// yang cukup (belum terkumpul N menit, ada celah, atau data basi): tidak ditebak.
    pub fn hitung_arah(&self, k: &KonfigGlobal, simbol: &str, sekarang: i64) -> Option<ArahHarga> {
        let simbol = simbol.to_uppercase();
        let tol = k.toleransi_arah_detik();
        let indodax = perubahan(self.indodax.get(&simbol)?, sekarang, k.arah_menit, tol)?;
        let mut luar: Vec<Decimal> = Vec::new();
        for ex in Exchange::SEMUA.iter() {
            if let Some(a) = self.luar.get(&(*ex, simbol.clone())) {
                if let Some(p) = perubahan(a, sekarang, k.arah_menit, tol) {
                    luar.push(p);
                }
            }
        }
        let jumlah_exchange = luar.len();
        let global = median(luar)?;
        let selisih = global.checked_sub(indodax)?;
        Some(ArahHarga {
            menit: k.arah_menit,
            global_persen: global,
            indodax_persen: indodax,
            selisih_poin: selisih,
            jumlah_exchange,
            tanda: selisih.abs() >= k.arah_ambang_persen,
        })
    }
}

// ---------------------------------------------------------------------------
// Tes
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn peta(isi: &[(&str, &str)]) -> HargaUsdt {
        isi.iter().map(|(k, v)| (k.to_string(), d(v))).collect()
    }

    const BINANCE: &str = r#"[
      {"symbol":"BTCUSDT","price":"65000.10"},
      {"symbol":"ABCUSDT","price":"0.2000"},
      {"symbol":"ETHBTC","price":"0.05"},
      {"symbol":"USDTIDR","price":"17300"},
      {"symbol":"NOLUSDT","price":"0"},
      {"symbol":"RUSAKUSDT","price":"abc"},
      {"symbol":"KECILUSDT","price":"1e-7"},
      {"price":"1"}]"#;

    #[test]
    fn parse_binance_usdt_dan_kurs() {
        let h = parse_binance(BINANCE).unwrap();
        assert_eq!(h.harga.get("BTC"), Some(&d("65000.10")));
        assert_eq!(h.harga.get("ABC"), Some(&d("0.2")));
        assert_eq!(h.harga.get("KECIL"), Some(&d("0.0000001"))); // notasi ilmiah diterima
        assert!(!h.harga.contains_key("ETH")); // ETHBTC bukan USDT
        assert!(!h.harga.contains_key("NOL")); // harga nol dibuang
        assert!(!h.harga.contains_key("RUSAK"));
        assert!(!h.harga.contains_key("USDTIDR"));
        assert_eq!(h.kurs_usdt_idr, Some(d("17300")));
    }

    #[test]
    fn parse_binance_tanpa_kurs_dan_bentuk_salah() {
        let h = parse_binance(r#"[{"symbol":"BTCUSDT","price":"1"}]"#).unwrap();
        assert_eq!(h.kurs_usdt_idr, None);
        assert!(parse_binance(r#"{"code":0,"msg":"Service unavailable from a restricted location"}"#).is_err());
        assert!(parse_binance("[]").is_err());
        assert!(parse_binance("bukan json").is_err());
    }

    #[test]
    fn parse_bybit_dan_galat() {
        let ok = r#"{"retCode":0,"retMsg":"OK","result":{"category":"spot","list":[
          {"symbol":"BTCUSDT","lastPrice":"65001"},
          {"symbol":"ABCUSDT","lastPrice":"0.2010"},
          {"symbol":"ABCUSDC","lastPrice":"0.2"},
          {"symbol":"NOLUSDT","lastPrice":"0"},
          {"symbol":"KOSONGUSDT","lastPrice":""}]}}"#;
        let h = parse_bybit(ok).unwrap();
        assert_eq!(h.len(), 2);
        assert_eq!(h.get("ABC"), Some(&d("0.201")));
        let galat = r#"{"retCode":10001,"retMsg":"params error","result":{}}"#;
        assert!(parse_bybit(galat).unwrap_err().to_string().contains("10001"));
        assert!(parse_bybit(r#"{"retCode":0}"#).is_err());
    }

    #[test]
    fn parse_okx_dan_galat() {
        let ok = r#"{"code":"0","msg":"","data":[
          {"instId":"BTC-USDT","last":"65002"},
          {"instId":"ABC-USDT","last":"0.1990"},
          {"instId":"ABC-USDC","last":"0.2"},
          {"instId":"KOSONG-USDT","last":""}]}"#;
        let h = parse_okx(ok).unwrap();
        assert_eq!(h.len(), 2);
        assert_eq!(h.get("ABC"), Some(&d("0.199")));
        assert!(parse_okx(r#"{"code":"50011","msg":"Too many requests","data":[]}"#).is_err());
        assert!(parse_okx(r#"{"code":"0"}"#).is_err());
    }

    #[test]
    fn median_ganjil_dan_genap() {
        assert_eq!(median(vec![d("3"), d("1"), d("2")]), Some(d("2")));
        assert_eq!(median(vec![d("4"), d("1"), d("2"), d("3")]), Some(d("2.5")));
        assert_eq!(median(vec![d("7")]), Some(d("7")));
        assert_eq!(median(vec![]), None);
    }

    fn tiga() -> (HargaUsdt, HargaUsdt, HargaUsdt) {
        (peta(&[("ABC", "0.2000")]), peta(&[("ABC", "0.2010")]), peta(&[("ABC", "0.1990")]))
    }

    #[test]
    fn harga_wajar_median_tiga_sumber_cocok_dengan_rumus_skrip() {
        let (b, y, o) = tiga();
        let sumber = [(Exchange::Binance, Some(&b)), (Exchange::Bybit, Some(&y)), (Exchange::Okx, Some(&o))];
        let k = KonfigGlobal::default();
        // Indodax 3.480 IDR, kurs 17.000: wajar = 0,2 x 17.000 = 3.400
        let g = hitung_harga_global(&k, "abc", d("3480"), d("17000"), &sumber);
        assert_eq!(g.wajar_idr, Some(d("3400")));
        let s = g.selisih_persen.unwrap(); // (3480 / 3400 - 1) x 100 = 2,3529...
        assert!(s > d("2.352") && s < d("2.353"));
        assert_eq!(g.kualitas, Kualitas::Tinggi);
        assert!(!g.curiga);
        let sb = g.sebaran_persen.unwrap(); // (0,2010 - 0,1990) / 0,2 x 100 = 1
        assert_eq!(sb, d("1"));
        assert_eq!(g.rincian.len(), 3);
        assert_eq!(g.rincian[0].exchange, Exchange::Binance);
        assert_eq!(g.rincian[0].harga_idr, Some(d("3400")));
        assert_eq!(g.rincian[1].harga_idr, Some(d("3417"))); // 0,2010 x 17.000
        let sy = g.rincian[1].selisih_persen.unwrap(); // (3480 / 3417 - 1) x 100 ~ 1,8437
        assert!(sy > d("1.84") && sy < d("1.85"));
    }

    #[test]
    fn kualitas_menurut_jumlah_sumber_dan_sebaran() {
        let (b, y, o) = tiga();
        let k = KonfigGlobal::default();
        // dua sumber
        let g = hitung_harga_global(&k, "ABC", d("3400"), d("17000"), &[(Exchange::Binance, Some(&b)), (Exchange::Bybit, Some(&y)), (Exchange::Okx, None)]);
        assert_eq!(g.kualitas, Kualitas::Tinggi);
        assert_eq!(g.wajar_idr, Some(d("3408.5"))); // median genap = (0,2 + 0,201) / 2 x 17.000
        assert_eq!(g.rincian[2].harga_idr, None);
        // satu sumber
        let g = hitung_harga_global(&k, "ABC", d("3400"), d("17000"), &[(Exchange::Binance, Some(&b)), (Exchange::Bybit, None), (Exchange::Okx, None)]);
        assert_eq!(g.kualitas, Kualitas::Rendah);
        assert_eq!(g.sebaran_persen, None);
        // tanpa pembanding: koin tidak ada di exchange mana pun, atau semua gagal
        let g = hitung_harga_global(&k, "XYZ", d("3400"), d("17000"), &[(Exchange::Binance, Some(&b)), (Exchange::Bybit, Some(&y)), (Exchange::Okx, Some(&o))]);
        assert_eq!(g.kualitas, Kualitas::TanpaPembanding);
        assert_eq!(g.wajar_idr, None);
        assert_eq!(g.selisih_persen, None);
        assert_eq!(g.rincian.len(), 3);
        let g = hitung_harga_global(&k, "ABC", d("3400"), d("17000"), &[(Exchange::Binance, None), (Exchange::Bybit, None), (Exchange::Okx, None)]);
        assert_eq!(g.kualitas, Kualitas::TanpaPembanding);
        // sebaran di atas 2%
        let jauh = peta(&[("ABC", "0.2300")]);
        let g = hitung_harga_global(&k, "ABC", d("3400"), d("17000"), &[(Exchange::Binance, Some(&b)), (Exchange::Bybit, Some(&jauh)), (Exchange::Okx, None)]);
        assert_eq!(g.kualitas, Kualitas::SebaranTinggi);
        // ambang sebaran bisa diubah
        let mut longgar = k.clone();
        longgar.sebaran_maks_persen = d("50");
        let g = hitung_harga_global(&longgar, "ABC", d("3400"), d("17000"), &[(Exchange::Binance, Some(&b)), (Exchange::Bybit, Some(&jauh)), (Exchange::Okx, None)]);
        assert_eq!(g.kualitas, Kualitas::Tinggi);
    }

    #[test]
    fn tanda_curiga_dan_selisih_negatif() {
        let (b, _, _) = tiga();
        let k = KonfigGlobal::default();
        let sumber = [(Exchange::Binance, Some(&b))];
        // Indodax 3.000 vs wajar 3.400 = -11,76%: curiga (> 10%)
        let g = hitung_harga_global(&k, "ABC", d("3000"), d("17000"), &sumber);
        assert!(g.selisih_persen.unwrap() < d("-11.7"));
        assert!(g.curiga);
        let g = hitung_harga_global(&k, "ABC", d("3300"), d("17000"), &sumber); // -2,94%
        assert!(!g.curiga);
    }

    #[test]
    fn kurs_nol_atau_harga_nol_tidak_panik() {
        let (b, _, _) = tiga();
        let k = KonfigGlobal::default();
        let sumber = [(Exchange::Binance, Some(&b))];
        let g = hitung_harga_global(&k, "ABC", d("3400"), Decimal::ZERO, &sumber);
        assert_eq!(g.kualitas, Kualitas::TanpaPembanding);
        assert_eq!(g.rincian[0].harga_idr, None);
        let g = hitung_harga_global(&k, "ABC", Decimal::ZERO, d("17000"), &sumber);
        assert_eq!(g.kualitas, Kualitas::TanpaPembanding);
    }

    #[test]
    fn selisih_kurs_dan_tanda() {
        let k = KonfigGlobal::default();
        let s = bandingkan_kurs(&k, d("17400"), Some(d("17300"))).unwrap(); // +0,578%
        assert!(s.selisih_persen > d("0.57") && s.selisih_persen < d("0.58"));
        assert!(!s.tanda);
        let s = bandingkan_kurs(&k, d("17600"), Some(d("17300"))).unwrap(); // +1,734%
        assert!(s.tanda);
        let s = bandingkan_kurs(&k, d("17000"), Some(d("17300"))).unwrap(); // negatif juga ditandai
        assert!(s.selisih_persen < Decimal::ZERO && s.tanda);
        assert!(bandingkan_kurs(&k, d("17400"), None).is_none());
        assert!(bandingkan_kurs(&k, d("17400"), Some(Decimal::ZERO)).is_none());
        assert!(bandingkan_kurs(&k, Decimal::ZERO, Some(d("17300"))).is_none());
    }

    #[test]
    fn exchange_urutan_dan_nama() {
        let nama: Vec<&str> = Exchange::SEMUA.iter().map(|e| e.nama()).collect();
        assert_eq!(nama, vec!["Binance", "Bybit", "OKX"]);
        assert!(Exchange::SEMUA.iter().all(|e| e.url().starts_with("https://")));
    }

    // ----- T9.6: arah -----

    fn ringkas(pair: &str, last: &str) -> RingkasanPasar {
        RingkasanPasar {
            pair: pair.to_string(),
            last: d(last),
            bid: d("1"),
            ask: d("2"),
            high: d("3"),
            low: d("1"),
            vol_idr: d("50000000"),
            harga_24j: None,
            server_time: Some(0),
        }
    }

    fn dilacak() -> BTreeSet<String> {
        ["ABC".to_string()].into_iter().collect()
    }

    /// Dua titik (t=0 dan t=600): Binance dan Bybit naik 1%, OKX datar. Indodax di `last_akhir`.
    fn riwayat_contoh(last_akhir: &str) -> RiwayatArah {
        let mut r = RiwayatArah::baru();
        let dl = dilacak();
        r.catat_indodax(0, &[ringkas("abc_idr", "3400")], d("17000"));
        r.catat_indodax(600, &[ringkas("abc_idr", last_akhir)], d("17000"));
        for (ex, akhir) in [(Exchange::Binance, "0.2020"), (Exchange::Bybit, "0.2020"), (Exchange::Okx, "0.2000")] {
            r.catat_global(0, ex, &peta(&[("ABC", "0.2000")]), &dl);
            r.catat_global(600, ex, &peta(&[("ABC", akhir)]), &dl);
        }
        r
    }

    #[test]
    fn arah_global_naik_indodax_belum_diberi_tanda() {
        let r = riwayat_contoh("3400"); // Indodax datar, median global +1%
        let k = KonfigGlobal::default(); // 10 menit, ambang 0,5
        let a = r.hitung_arah(&k, "abc", 600).unwrap();
        assert_eq!(a.menit, 10);
        assert_eq!(a.global_persen, d("1"));
        assert_eq!(a.indodax_persen, d("0"));
        assert_eq!(a.selisih_poin, d("1"));
        assert_eq!(a.jumlah_exchange, 3);
        assert!(a.tanda);
    }

    #[test]
    fn arah_indodax_ikut_naik_tidak_diberi_tanda() {
        let r = riwayat_contoh("3434"); // 3434 / 17000 = 0,202 -> +1% dalam USDT
        let a = r.hitung_arah(&KonfigGlobal::default(), "ABC", 600).unwrap();
        assert_eq!(a.indodax_persen, d("1"));
        assert_eq!(a.selisih_poin, d("0"));
        assert!(!a.tanda);
        // ambang bisa diubah: selisih 0 tetap tidak melewati ambang positif sekecil apa pun
        let mut k = KonfigGlobal::default();
        k.arah_ambang_persen = d("0.01");
        assert!(!r.hitung_arah(&k, "ABC", 600).unwrap().tanda);
    }

    #[test]
    fn arah_negatif_global_turun() {
        let mut r = RiwayatArah::baru();
        let dl = dilacak();
        r.catat_titik_indodax("ABC", 0, d("0.2"));
        r.catat_titik_indodax("ABC", 600, d("0.2"));
        r.catat_global(0, Exchange::Binance, &peta(&[("ABC", "0.2000")]), &dl);
        r.catat_global(600, Exchange::Binance, &peta(&[("ABC", "0.1980")]), &dl); // -1%
        let a = r.hitung_arah(&KonfigGlobal::default(), "ABC", 600).unwrap();
        assert_eq!(a.global_persen, d("-1"));
        assert_eq!(a.selisih_poin, d("-1"));
        assert_eq!(a.jumlah_exchange, 1);
        assert!(a.tanda); // |-1| >= 0,5
    }

    #[test]
    fn arah_kosong_bila_data_kurang_basi_atau_ada_celah() {
        let k = KonfigGlobal::default(); // toleransi 90 detik
        assert_eq!(k.toleransi_arah_detik(), 90);
        let r = riwayat_contoh("3400");
        assert!(r.hitung_arah(&k, "XYZ", 600).is_none()); // koin tidak dilacak
        assert!(r.hitung_arah(&k, "ABC", 800).is_none()); // titik terakhir basi (200 detik)
        assert!(r.hitung_arah(&k, "ABC", 680).is_some()); // masih dalam toleransi

        // baru terkumpul 300 detik: acuan 10 menit lalu tidak ada
        let mut pendek = RiwayatArah::baru();
        let dl = dilacak();
        pendek.catat_titik_indodax("ABC", 300, d("0.2"));
        pendek.catat_titik_indodax("ABC", 600, d("0.2"));
        pendek.catat_global(300, Exchange::Binance, &peta(&[("ABC", "0.2")]), &dl);
        pendek.catat_global(600, Exchange::Binance, &peta(&[("ABC", "0.2")]), &dl);
        assert!(pendek.hitung_arah(&k, "ABC", 600).is_none());

        // celah: titik di 0 dan 600 saja, tetapi jendela 5 menit tidak punya acuan di t=300
        let mut k5 = k.clone();
        k5.arah_menit = 5;
        assert!(r.hitung_arah(&k5, "ABC", 600).is_none());

        // exchange luar tanpa data sama sekali = tanpa arah
        let mut hanya_indodax = RiwayatArah::baru();
        hanya_indodax.catat_titik_indodax("ABC", 0, d("0.2"));
        hanya_indodax.catat_titik_indodax("ABC", 600, d("0.2"));
        assert!(hanya_indodax.hitung_arah(&k, "ABC", 600).is_none());
    }

    #[test]
    fn riwayat_arah_membuang_titik_tua_melewati_waktu_mundur_dan_kurs_nol() {
        let mut r = RiwayatArah::baru();
        r.catat_titik_indodax("ABC", 0, d("1"));
        r.catat_titik_indodax("ABC", 3000, d("1")); // titik di 0 lebih tua dari 2.700 detik: dibuang
        assert_eq!(r.indodax["ABC"].len(), 1);
        r.catat_titik_indodax("ABC", 3000, d("9")); // waktu sama
        r.catat_titik_indodax("ABC", 2000, d("9")); // waktu mundur
        r.catat_titik_indodax("ABC", 3100, d("0")); // harga nol
        assert_eq!(r.indodax["ABC"].len(), 1);

        let mut k = RiwayatArah::baru();
        k.catat_indodax(10, &[ringkas("abc_idr", "3400")], Decimal::ZERO); // kurs nol: tidak mencatat
        assert!(k.indodax.is_empty());
        k.catat_indodax(10, &[ringkas("abc_idr", "3400"), ringkas("btc_usdt", "5"), ringkas("nol_idr", "0")], d("17000"));
        assert_eq!(k.indodax.len(), 1); // hanya *_idr dengan harga positif
        assert_eq!(k.indodax["ABC"][0], (10, d("0.2")));

        k.kosongkan();
        assert!(k.indodax.is_empty() && k.luar.is_empty());
    }

    #[test]
    fn catat_global_hanya_koin_dilacak() {
        let mut r = RiwayatArah::baru();
        r.catat_global(0, Exchange::Binance, &peta(&[("ABC", "0.2"), ("BTC", "65000")]), &dilacak());
        assert_eq!(r.luar.len(), 1);
        assert!(r.luar.contains_key(&(Exchange::Binance, "ABC".to_string())));
    }

    #[test]
    fn toleransi_mengikuti_interval() {
        let mut k = KonfigGlobal::default();
        k.interval_detik = 600;
        assert_eq!(k.toleransi_arah_detik(), 900);
        k.interval_detik = 15;
        assert_eq!(k.toleransi_arah_detik(), 90); // minimal 90
    }
}
