//! T9.1: Screener pair IDR Indodax. HANYA informasi: tidak menyentuh posisi, profit, target,
//! grafik, statistik, maupun database trade. Modul ini murni (tanpa jaringan, tanpa database):
//! hanya mengurai teks JSON yang diberikan pemanggil, menyaring, menghitung, dan mengurutkan.
//! Pengambilan data (GET publik tanpa key) ditambahkan di T9.2 pada sisi klien.
//!
//! Bentuk respons (diverifikasi T9.0, 14 Nov 2026, lihat PROGRESS bagian 5):
//! - GET https://indodax.com/api/summaries: {"tickers":{"usdt_idr":{"buy","sell","high","low",
//!   "last","name","server_time","vol_idr",...}},"prices_24h":{"usdtidr":"..."},"prices_7d":{...}}.
//!   Kunci `tickers` memakai garis bawah, `prices_24h` TANPA garis bawah. `buy` = bid, `sell` = ask.
//!   `vol_idr` kemungkinan volume 24 jam bergulir (jangan dipakai sebagai laju transaksi).
//! - GET https://indodax.com/api/trades/<pair tanpa garis bawah>: 500 transaksi, terbaru dulu,
//!   item {"tid","type","price","amount","date"} (semua teks; date = epoch detik).
//! - GET https://indodax.com/api/depth/<pair tanpa garis bawah>: {"buy":[[harga,jumlah],..],
//!   "sell":[..]}, maksimal 150 level per sisi; jumlah dalam KOIN (dikonfirmasi pengguna).
//!
//! Semua hitungan memakai Decimal. Pembagian memakai checked_div agar tidak panik.
use crate::{Error, Hasil};
use rust_decimal::Decimal;
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::str::FromStr;

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
    Decimal::from_str(t).ok()
}

fn dec(v: &Value) -> Option<Decimal> {
    match v {
        Value::String(s) => dec_teks(s),
        Value::Number(n) => dec_teks(&n.to_string()),
        _ => None,
    }
}

fn bulat(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)),
        Value::String(s) => s.trim().parse::<i64>().ok(),
        _ => None,
    }
}

/// a / b x 100, atau None bila b nol atau terjadi luapan.
fn persen(a: Decimal, b: Decimal) -> Option<Decimal> {
    a.checked_mul(seratus())?.checked_div(b)
}

fn dalam_rentang(x: Decimal, min: Option<Decimal>, maks: Option<Decimal>) -> bool {
    if let Some(m) = min {
        if x < m {
            return false;
        }
    }
    if let Some(m) = maks {
        if x > m {
            return false;
        }
    }
    true
}

/// Nama pair di URL /api/trades dan /api/depth: tanpa garis bawah ("usdt_idr" -> "usdtidr").
pub fn pair_path(pair: &str) -> String {
    pair.replace('_', "")
}

// ---------------------------------------------------------------------------
// Konfigurasi
// ---------------------------------------------------------------------------

/// Urutan daftar hasil.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrutanScreener {
    /// Skor menurun (FR-32); tanpa skor di paling bawah, lalu trx/mnt.
    Skor,
    /// Trx/mnt menurun (sama dengan skrip Python pengguna).
    TrxPerMenit,
}

/// Semua ambang. `None` = filter dimatikan. Nilai awal = nilai skrip `screener_indodax.py`
/// (FR-31); filter tambahan (FR-32) mati secara bawaan sampai angka finalnya diuji di lapangan.
#[derive(Debug, Clone, PartialEq)]
pub struct KonfigScreener {
    pub harga_min: Option<Decimal>,
    pub harga_maks: Option<Decimal>,
    pub vol_min: Option<Decimal>,
    pub vol_maks: Option<Decimal>,
    pub trx_mnt_min: Option<Decimal>,
    /// lebar jendela trx/mnt dalam menit (minimal 1)
    pub jendela_menit: u32,
    /// dalam persen
    pub spread_maks: Option<Decimal>,
    /// range 24 jam (high-low)/low, dalam persen
    pub range24_min: Option<Decimal>,
    pub usia_trx_maks_detik: Option<i64>,
    // --- tambahan (usulan FR-32), mati bawaan ---
    /// gerak harga di jendela (high-low)/low, dalam persen
    pub gerak_min_persen: Option<Decimal>,
    pub menit_aktif_min_persen: Option<Decimal>,
    /// perubahan harga dari transaksi tertua ke terbaru di jendela, dalam persen (boleh negatif)
    pub arah_min_persen: Option<Decimal>,
    /// kelipatan "Modal trade ini"; aktif hanya bila modal diisi dan data kedalaman ada
    pub kedalaman_kelipatan: Option<Decimal>,
    /// jarak dari best bid/ask yang dihitung, dalam persen (bawaan 0,5)
    pub kedalaman_persen: Decimal,
    /// satu transaksi melebihi persen ini dari nilai jendela: hanya tanda peringatan
    pub dominasi_tanda_persen: Option<Decimal>,
    /// perkiraan biaya beli + jual, dalam persen (bawaan 0,41 = limit 0,10 + 0,31)
    pub biaya_roundtrip_persen: Decimal,
    pub urutan: UrutanScreener,
}

impl Default for KonfigScreener {
    fn default() -> Self {
        Self {
            harga_min: Some(Decimal::from(10_i64)),
            harga_maks: Some(Decimal::from(70_000_i64)),
            vol_min: Some(Decimal::from(10_000_000_i64)),
            vol_maks: Some(Decimal::from(10_000_000_000_i64)),
            trx_mnt_min: Some(Decimal::from(2_i64)),
            jendela_menit: 10,
            spread_maks: Some(Decimal::from(10_i64)),
            range24_min: Some(Decimal::from(4_i64)),
            usia_trx_maks_detik: Some(120),
            gerak_min_persen: None,
            menit_aktif_min_persen: None,
            arah_min_persen: None,
            kedalaman_kelipatan: None,
            kedalaman_persen: Decimal::new(5, 1),
            dominasi_tanda_persen: Some(Decimal::from(40_i64)),
            biaya_roundtrip_persen: Decimal::new(41, 2),
            urutan: UrutanScreener::Skor,
        }
    }
}

// ---------------------------------------------------------------------------
// Tahap 1: ringkasan pasar
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct RingkasanPasar {
    /// contoh "usdt_idr"
    pub pair: String,
    pub last: Decimal,
    /// `buy` di API (nol bila tidak ada)
    pub bid: Decimal,
    /// `sell` di API (nol bila tidak ada)
    pub ask: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub vol_idr: Decimal,
    /// dari `prices_24h` (kunci tanpa garis bawah)
    pub harga_24j: Option<Decimal>,
    /// epoch detik waktu server
    pub server_time: Option<i64>,
}

/// Urai /api/summaries. Hanya pair `*_idr` dengan `last` positif. Hasil diurutkan menurut nama pair.
/// Error bila bentuk respons tidak dikenali atau tidak ada pair IDR sama sekali.
pub fn parse_ringkasan(json: &str) -> Hasil<Vec<RingkasanPasar>> {
    let v: Value = serde_json::from_str(json).map_err(|e| Error::Json(e.to_string()))?;
    let tickers = v
        .get("tickers")
        .and_then(Value::as_object)
        .ok_or_else(|| Error::Json("respons summaries tidak memuat 'tickers'".into()))?;
    let p24 = v.get("prices_24h").and_then(Value::as_object);
    let mut hasil = Vec::new();
    for (pair, data) in tickers {
        if !pair.ends_with("_idr") {
            continue;
        }
        let Some(last) = data.get("last").and_then(dec).filter(|x| *x > Decimal::ZERO) else {
            continue;
        };
        let kunci24 = pair.replace('_', "");
        let harga_24j = p24
            .and_then(|m| m.get(kunci24.as_str()))
            .and_then(dec)
            .filter(|x| *x > Decimal::ZERO);
        hasil.push(RingkasanPasar {
            pair: pair.clone(),
            last,
            bid: data.get("buy").and_then(dec).unwrap_or(Decimal::ZERO),
            ask: data.get("sell").and_then(dec).unwrap_or(Decimal::ZERO),
            high: data.get("high").and_then(dec).unwrap_or(Decimal::ZERO),
            low: data.get("low").and_then(dec).unwrap_or(Decimal::ZERO),
            vol_idr: data.get("vol_idr").and_then(dec).unwrap_or(Decimal::ZERO),
            harga_24j,
            server_time: data.get("server_time").and_then(bulat),
        });
    }
    if hasil.is_empty() {
        return Err(Error::Json("summaries tidak memuat pair IDR satu pun".into()));
    }
    hasil.sort_by(|a, b| a.pair.cmp(&b.pair));
    Ok(hasil)
}

/// (ask - bid) / last x 100. None bila bid atau ask nol (artinya tidak lolos bila spread dibatasi).
pub fn hitung_spread(r: &RingkasanPasar) -> Option<Decimal> {
    if r.bid > Decimal::ZERO && r.ask > Decimal::ZERO {
        persen(r.ask - r.bid, r.last)
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KandidatScreener {
    pub ringkasan: RingkasanPasar,
    pub spread: Option<Decimal>,
    /// (high - low) / low x 100; nol bila low nol
    pub range24: Decimal,
    /// (last - harga 24 jam lalu) / harga 24 jam lalu x 100
    pub perubahan24: Option<Decimal>,
}

/// Tahap 1: harga, volume 24 jam, spread, range 24 jam. Satu request ringkasan saja.
pub fn saring_tahap1(k: &KonfigScreener, data: &[RingkasanPasar]) -> Vec<KandidatScreener> {
    let mut out = Vec::new();
    for r in data {
        if !dalam_rentang(r.last, k.harga_min, k.harga_maks) {
            continue;
        }
        if !dalam_rentang(r.vol_idr, k.vol_min, k.vol_maks) {
            continue;
        }
        let spread = hitung_spread(r);
        if let Some(m) = k.spread_maks {
            match spread {
                Some(s) if s <= m => {}
                _ => continue,
            }
        }
        let range24 = if r.low > Decimal::ZERO {
            persen(r.high - r.low, r.low).unwrap_or(Decimal::ZERO)
        } else {
            Decimal::ZERO
        };
        if let Some(m) = k.range24_min {
            if range24 < m {
                continue;
            }
        }
        let perubahan24 = r.harga_24j.and_then(|o| persen(r.last - o, o));
        out.push(KandidatScreener { ringkasan: r.clone(), spread, range24, perubahan24 });
    }
    out
}

// ---------------------------------------------------------------------------
// Tahap 2: riwayat transaksi publik
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JenisTransaksi {
    Beli,
    Jual,
    Lain,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransaksiPublik {
    pub tid: String,
    pub jenis: JenisTransaksi,
    pub harga: Decimal,
    /// jumlah koin
    pub jumlah: Decimal,
    /// epoch detik
    pub waktu: i64,
}

impl TransaksiPublik {
    fn urutan_tid(&self) -> u128 {
        self.tid.parse::<u128>().unwrap_or(0)
    }
}

/// Urai /api/trades/<pair>. Item rusak (harga atau jumlah tidak positif, tanpa waktu) dilewati.
/// Error bila respons bukan daftar (misalnya objek galat).
pub fn parse_transaksi(json: &str) -> Hasil<Vec<TransaksiPublik>> {
    let v: Value = serde_json::from_str(json).map_err(|e| Error::Json(e.to_string()))?;
    let daftar = v
        .as_array()
        .ok_or_else(|| Error::Json("respons trades bukan daftar".into()))?;
    let mut hasil = Vec::new();
    for item in daftar {
        let Some(harga) = item.get("price").and_then(dec).filter(|x| *x > Decimal::ZERO) else {
            continue;
        };
        let Some(jumlah) = item.get("amount").and_then(dec).filter(|x| *x > Decimal::ZERO) else {
            continue;
        };
        let Some(waktu) = item.get("date").and_then(bulat) else {
            continue;
        };
        let tid = match item.get("tid") {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Number(n)) => n.to_string(),
            _ => String::new(),
        };
        let jenis = match item.get("type").and_then(Value::as_str) {
            Some("buy") => JenisTransaksi::Beli,
            Some("sell") => JenisTransaksi::Jual,
            _ => JenisTransaksi::Lain,
        };
        hasil.push(TransaksiPublik { tid, jenis, harga, jumlah, waktu });
    }
    Ok(hasil)
}

/// Metrik dari riwayat transaksi dalam satu jendela.
#[derive(Debug, Clone, PartialEq)]
pub struct MetrikPair {
    /// transaksi per menit
    pub trx_per_menit: Decimal,
    /// jendela yang benar-benar dipakai (bisa lebih pendek dari yang diminta)
    pub jendela_detik: i64,
    pub jumlah_trx: i64,
    pub buy_persen: Decimal,
    /// detik sejak transaksi terbaru; None bila tidak ada transaksi
    pub usia_detik: Option<i64>,
    /// (high - low) / low x 100 harga transaksi di jendela
    pub gerak_persen: Option<Decimal>,
    /// menit yang punya minimal satu transaksi / jumlah menit jendela x 100 (maksimal 100)
    pub menit_aktif_persen: Decimal,
    /// (harga terbaru - harga tertua) / harga tertua x 100 di jendela
    pub arah_persen: Option<Decimal>,
    /// nilai transaksi terbesar / total nilai di jendela x 100
    pub dominasi_persen: Option<Decimal>,
}

impl MetrikPair {
    pub fn kosong(jendela_detik: i64) -> Self {
        Self {
            trx_per_menit: Decimal::ZERO,
            jendela_detik,
            jumlah_trx: 0,
            buy_persen: Decimal::ZERO,
            usia_detik: None,
            gerak_persen: None,
            menit_aktif_persen: Decimal::ZERO,
            arah_persen: None,
            dominasi_persen: None,
        }
    }
}

/// Hitung metrik. Aturan jendela sama dengan skrip Python: bila transaksi tertua yang dikembalikan
/// API lebih baru dari awal jendela, jendela diperpendek menjadi (sekarang - tertua), minimal 60 detik.
/// `sekarang` = epoch detik acuan (dianjurkan `server_time` dari ringkasan, bukan jam HP).
pub fn hitung_metrik(trx: &[TransaksiPublik], sekarang: i64, jendela_menit: u32) -> MetrikPair {
    let lebar = i64::from(jendela_menit.max(1)) * 60;
    if trx.is_empty() {
        return MetrikPair::kosong(lebar);
    }
    let mut urut: Vec<&TransaksiPublik> = trx.iter().collect();
    urut.sort_by(|a, b| a.waktu.cmp(&b.waktu).then_with(|| a.urutan_tid().cmp(&b.urutan_tid())));
    let tertua = urut[0].waktu;
    let terbaru = urut[urut.len() - 1].waktu;
    let awal = sekarang - lebar;
    let jendela_detik = if tertua <= awal { lebar } else { (sekarang - tertua).max(60) };
    let batas = sekarang - jendela_detik;
    let baru: Vec<&TransaksiPublik> = urut.iter().copied().filter(|t| t.waktu >= batas).collect();
    let usia_detik = Some((sekarang - terbaru).max(0));
    if baru.is_empty() {
        let mut m = MetrikPair::kosong(jendela_detik);
        m.usia_detik = usia_detik;
        return m;
    }
    let jumlah = baru.len() as i64;
    let jumlah_d = Decimal::from(jumlah);
    let trx_per_menit = persen(jumlah_d, Decimal::from(jendela_detik))
        .and_then(|x| x.checked_mul(Decimal::from(60_i64)))
        .and_then(|x| x.checked_div(seratus()))
        .unwrap_or(Decimal::ZERO);
    let beli = baru.iter().filter(|t| t.jenis == JenisTransaksi::Beli).count() as i64;
    let buy_persen = persen(Decimal::from(beli), jumlah_d).unwrap_or(Decimal::ZERO);

    let mut tinggi = baru[0].harga;
    let mut rendah = baru[0].harga;
    for t in &baru {
        if t.harga > tinggi {
            tinggi = t.harga;
        }
        if t.harga < rendah {
            rendah = t.harga;
        }
    }
    let gerak_persen = if rendah > Decimal::ZERO { persen(tinggi - rendah, rendah) } else { None };

    let pertama = baru[0];
    let terakhir = baru[baru.len() - 1];
    let arah_persen = if pertama.harga > Decimal::ZERO {
        persen(terakhir.harga - pertama.harga, pertama.harga)
    } else {
        None
    };

    let mut menit: BTreeSet<i64> = BTreeSet::new();
    for t in &baru {
        menit.insert((sekarang - t.waktu).max(0) / 60);
    }
    let jumlah_menit = (jendela_detik + 59) / 60;
    let menit_aktif_persen = persen(Decimal::from(menit.len() as i64), Decimal::from(jumlah_menit))
        .unwrap_or(Decimal::ZERO)
        .min(seratus());

    let mut total = Decimal::ZERO;
    let mut terbesar = Decimal::ZERO;
    for t in &baru {
        let nilai = t.harga.checked_mul(t.jumlah).unwrap_or(Decimal::ZERO);
        total += nilai;
        if nilai > terbesar {
            terbesar = nilai;
        }
    }
    let dominasi_persen = persen(terbesar, total);

    MetrikPair {
        trx_per_menit,
        jendela_detik,
        jumlah_trx: jumlah,
        buy_persen,
        usia_detik,
        gerak_persen,
        menit_aktif_persen,
        arah_persen,
        dominasi_persen,
    }
}

// ---------------------------------------------------------------------------
// Kedalaman order book
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct KedalamanPasar {
    /// (harga, jumlah koin); sisi beli
    pub bid: Vec<(Decimal, Decimal)>,
    /// (harga, jumlah koin); sisi jual
    pub ask: Vec<(Decimal, Decimal)>,
}

fn parse_level(v: &Value) -> Option<(Decimal, Decimal)> {
    let a = v.as_array()?;
    let harga = dec(a.first()?)?;
    let jumlah = dec(a.get(1)?)?;
    if harga > Decimal::ZERO && jumlah >= Decimal::ZERO {
        Some((harga, jumlah))
    } else {
        None
    }
}

/// Urai /api/depth/<pair>. Level rusak dilewati. Error bila `buy` atau `sell` tidak ada.
pub fn parse_kedalaman(json: &str) -> Hasil<KedalamanPasar> {
    let v: Value = serde_json::from_str(json).map_err(|e| Error::Json(e.to_string()))?;
    let sisi = |kunci: &str| -> Hasil<Vec<(Decimal, Decimal)>> {
        let daftar = v
            .get(kunci)
            .and_then(Value::as_array)
            .ok_or_else(|| Error::Json(format!("respons depth tidak memuat '{kunci}'")))?;
        Ok(daftar.iter().filter_map(parse_level).collect())
    };
    Ok(KedalamanPasar { bid: sisi("buy")?, ask: sisi("sell")? })
}

impl KedalamanPasar {
    /// (nilai Rupiah sisi beli, nilai Rupiah sisi jual) dalam `persen_jarak` dari best bid / best ask.
    /// Nilai = harga x jumlah koin. None bila salah satu sisi kosong.
    pub fn nilai_dalam(&self, persen_jarak: Decimal) -> Option<(Decimal, Decimal)> {
        let terbaik_bid = self.bid.iter().map(|(h, _)| *h).max()?;
        let terbaik_ask = self.ask.iter().map(|(h, _)| *h).min()?;
        let f = persen_jarak / seratus();
        let batas_bid = terbaik_bid * (Decimal::ONE - f);
        let batas_ask = terbaik_ask * (Decimal::ONE + f);
        let mut nilai_bid = Decimal::ZERO;
        for (h, j) in &self.bid {
            if *h >= batas_bid {
                nilai_bid += h.checked_mul(*j).unwrap_or(Decimal::ZERO);
            }
        }
        let mut nilai_ask = Decimal::ZERO;
        for (h, j) in &self.ask {
            if *h <= batas_ask {
                nilai_ask += h.checked_mul(*j).unwrap_or(Decimal::ZERO);
            }
        }
        Some((nilai_bid, nilai_ask))
    }
}

// ---------------------------------------------------------------------------
// Hasil akhir tahap 2, skor, urutan
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct BarisScreener {
    pub pair: String,
    pub harga: Decimal,
    pub vol_idr: Decimal,
    pub spread: Option<Decimal>,
    pub range24: Decimal,
    pub perubahan24: Option<Decimal>,
    pub metrik: MetrikPair,
    /// gerak jendela / (spread + biaya round-trip); None bila gerak atau spread tidak ada
    pub skor: Option<Decimal>,
    pub kedalaman_bid_idr: Option<Decimal>,
    pub kedalaman_ask_idr: Option<Decimal>,
    /// satu transaksi mendominasi nilai jendela (hanya tanda peringatan)
    pub dominasi_tinggi: bool,
}

/// Tahap 2 untuk satu kandidat. Mengembalikan None bila tidak lolos filter tahap 2.
/// - `sekarang_cadangan`: dipakai hanya bila ringkasan tidak memuat `server_time`.
/// - `kedalaman` dan `modal_idr`: filter kedalaman aktif HANYA bila keduanya ada, kelipatan diatur,
///   dan modal positif (FR-33: modal kosong = dilewati). Syarat: nilai sisi beli DAN sisi jual
///   dalam jarak yang diatur masing-masing minimal kelipatan x modal.
pub fn nilai_tahap2(
    k: &KonfigScreener,
    kand: &KandidatScreener,
    trx: &[TransaksiPublik],
    sekarang_cadangan: i64,
    kedalaman: Option<&KedalamanPasar>,
    modal_idr: Option<Decimal>,
) -> Option<BarisScreener> {
    let sekarang = kand.ringkasan.server_time.unwrap_or(sekarang_cadangan);
    let m = hitung_metrik(trx, sekarang, k.jendela_menit);

    if let Some(min) = k.trx_mnt_min {
        if m.trx_per_menit < min {
            return None;
        }
    }
    if let Some(maks) = k.usia_trx_maks_detik {
        match m.usia_detik {
            Some(u) if u <= maks => {}
            _ => return None,
        }
    }
    if let Some(min) = k.gerak_min_persen {
        match m.gerak_persen {
            Some(g) if g >= min => {}
            _ => return None,
        }
    }
    if let Some(min) = k.menit_aktif_min_persen {
        if m.menit_aktif_persen < min {
            return None;
        }
    }
    if let Some(min) = k.arah_min_persen {
        match m.arah_persen {
            Some(a) if a >= min => {}
            _ => return None,
        }
    }

    let nilai_dalam = kedalaman.and_then(|d| d.nilai_dalam(k.kedalaman_persen));
    if let (Some(kel), Some(modal), Some((bid, ask))) = (k.kedalaman_kelipatan, modal_idr, nilai_dalam) {
        if modal > Decimal::ZERO {
            let perlu = kel.checked_mul(modal).unwrap_or(Decimal::MAX);
            if bid.min(ask) < perlu {
                return None;
            }
        }
    }

    let skor = match (m.gerak_persen, kand.spread) {
        (Some(g), Some(s)) => {
            let penyebut = s + k.biaya_roundtrip_persen;
            if penyebut > Decimal::ZERO {
                g.checked_div(penyebut)
            } else {
                None
            }
        }
        _ => None,
    };
    let dominasi_tinggi = match (m.dominasi_persen, k.dominasi_tanda_persen) {
        (Some(d), Some(t)) => d > t,
        _ => false,
    };

    Some(BarisScreener {
        pair: kand.ringkasan.pair.clone(),
        harga: kand.ringkasan.last,
        vol_idr: kand.ringkasan.vol_idr,
        spread: kand.spread,
        range24: kand.range24,
        perubahan24: kand.perubahan24,
        metrik: m,
        skor,
        kedalaman_bid_idr: nilai_dalam.map(|(b, _)| b),
        kedalaman_ask_idr: nilai_dalam.map(|(_, a)| a),
        dominasi_tinggi,
    })
}

fn banding_skor(a: &Option<Decimal>, b: &Option<Decimal>) -> Ordering {
    match (a, b) {
        (Some(x), Some(y)) => y.cmp(x),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// Urutkan menurut `k.urutan`. Seri diputus dengan trx/mnt menurun, lalu nama pair.
pub fn urutkan(k: &KonfigScreener, baris: &mut [BarisScreener]) {
    baris.sort_by(|a, b| {
        let utama = match k.urutan {
            UrutanScreener::Skor => banding_skor(&a.skor, &b.skor),
            UrutanScreener::TrxPerMenit => Ordering::Equal,
        };
        utama
            .then_with(|| b.metrik.trx_per_menit.cmp(&a.metrik.trx_per_menit))
            .then_with(|| a.pair.cmp(&b.pair))
    });
}

// ---------------------------------------------------------------------------
// Tes
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_794_623_510;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    // usdt_idr: potongan nyata T9.0 (14 Nov 2026). abc_idr: rekaan untuk lolos filter bawaan.
    const SUMMARIES: &str = r#"{"tickers":{
      "usdt_idr":{"buy":"18266","high":"18350","last":"18267","low":"18205","name":"Tether USDt","sell":"18267","server_time":1794623510,"vol_idr":"126138422741","vol_usdt":"6906281.71726226"},
      "abc_idr":{"buy":"98","high":"104","last":"99","low":"96","name":"ABC","sell":"100","server_time":1794623510,"vol_idr":"50000000","vol_abc":"500000"},
      "eth_btc":{"last":"0.05"},
      "nol_idr":{"last":"0"},
      "rusak_idr":{"buy":"1"}},
      "prices_24h":{"usdtidr":"18200","abcidr":"100"},"prices_7d":{}}"#;

    fn kandidat_abc() -> KandidatScreener {
        let semua = parse_ringkasan(SUMMARIES).unwrap();
        let mut k = saring_tahap1(&KonfigScreener::default(), &semua);
        assert_eq!(k.len(), 1);
        k.remove(0)
    }

    fn t(jenis: JenisTransaksi, harga: &str, jumlah: &str, usia: i64) -> TransaksiPublik {
        TransaksiPublik { tid: usia.to_string(), jenis, harga: d(harga), jumlah: d(jumlah), waktu: NOW - usia }
    }

    /// 25 transaksi, usia 5, 30, 55, ... 605 detik; selang-seling beli/jual, harga 99.
    fn ramai() -> Vec<TransaksiPublik> {
        (0..25)
            .map(|i| {
                let jenis = if i % 2 == 0 { JenisTransaksi::Beli } else { JenisTransaksi::Jual };
                t(jenis, "99", "1", 5 + 25 * i)
            })
            .collect()
    }

    #[test]
    fn parse_ringkasan_hanya_pair_idr_valid() {
        let r = parse_ringkasan(SUMMARIES).unwrap();
        let nama: Vec<&str> = r.iter().map(|x| x.pair.as_str()).collect();
        assert_eq!(nama, vec!["abc_idr", "usdt_idr"]); // eth_btc, last nol, tanpa last dibuang
        let u = &r[1];
        assert_eq!(u.last, d("18267"));
        assert_eq!(u.bid, d("18266"));
        assert_eq!(u.ask, d("18267"));
        assert_eq!(u.vol_idr, d("126138422741"));
        assert_eq!(u.harga_24j, Some(d("18200"))); // kunci prices_24h tanpa garis bawah
        assert_eq!(u.server_time, Some(NOW));
    }

    #[test]
    fn parse_ringkasan_bentuk_tidak_dikenal_menjadi_error() {
        assert!(parse_ringkasan("{}").is_err());
        assert!(parse_ringkasan("bukan json").is_err());
        assert!(parse_ringkasan(r#"{"tickers":{"eth_btc":{"last":"1"}}}"#).is_err());
    }

    #[test]
    fn tahap1_menyaring_volume_dan_range_lalu_menghitung_spread() {
        let kand = kandidat_abc();
        assert_eq!(kand.ringkasan.pair, "abc_idr"); // usdt gugur: volume di atas batas dan range kecil
        let s = kand.spread.unwrap();
        assert!(s > d("2.02") && s < d("2.03")); // (100-98)/99
        assert!(kand.range24 > d("8.33") && kand.range24 < d("8.34")); // (104-96)/96
        assert_eq!(kand.perubahan24, Some(d("-1"))); // (99-100)/100
    }

    #[test]
    fn tahap1_filter_bisa_dimatikan() {
        let semua = parse_ringkasan(SUMMARIES).unwrap();
        let mut k = KonfigScreener::default();
        k.vol_maks = None;
        k.range24_min = None;
        assert_eq!(saring_tahap1(&k, &semua).len(), 2);
        k.spread_maks = Some(d("1")); // abc spread 2,02% tidak lolos lagi
        assert_eq!(saring_tahap1(&k, &semua).len(), 1);
    }

    #[test]
    fn tahap1_spread_tanpa_bid_ask_tidak_lolos_bila_dibatasi() {
        let mut r = parse_ringkasan(SUMMARIES).unwrap();
        r[0].bid = Decimal::ZERO; // abc
        assert!(saring_tahap1(&KonfigScreener::default(), &r).is_empty());
        let mut k = KonfigScreener::default();
        k.spread_maks = None;
        assert_eq!(saring_tahap1(&k, &r).len(), 1);
    }

    #[test]
    fn parse_transaksi_item_rusak_dilewati() {
        let json = r#"[
          {"tid":"144115188261258459","type":"buy","price":"18267","amount":"27.17058082","date":"1794623499"},
          {"tid":"x","type":"sell","price":"abc","amount":"1","date":"1"},
          {"tid":"2","type":"sell","price":"714","amount":"141.71369294","date":"1794623294"},
          {"tid":"3","type":"lain","price":"1","amount":"0","date":"5"}]"#;
        let r = parse_transaksi(json).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].jenis, JenisTransaksi::Beli);
        assert_eq!(r[0].waktu, 1_794_623_499);
        assert_eq!(r[1].jenis, JenisTransaksi::Jual);
        assert!(parse_transaksi(r#"{"error":"x"}"#).is_err());
    }

    #[test]
    fn metrik_jendela_penuh() {
        let m = hitung_metrik(&ramai(), NOW, 10);
        // transaksi usia 605 di luar jendela 600 detik: tersisa 24 (usia 5 sampai 580)
        assert_eq!(m.jendela_detik, 600);
        assert_eq!(m.jumlah_trx, 24);
        assert_eq!(m.trx_per_menit, d("2.4"));
        assert_eq!(m.buy_persen, d("50"));
        assert_eq!(m.usia_detik, Some(5));
        assert_eq!(m.gerak_persen, Some(Decimal::ZERO));
    }

    #[test]
    fn metrik_jendela_diperpendek_seperti_skrip() {
        let trx = vec![
            t(JenisTransaksi::Beli, "99", "1", 10),
            t(JenisTransaksi::Beli, "99", "1", 50),
            t(JenisTransaksi::Jual, "99", "1", 120),
        ];
        let m = hitung_metrik(&trx, NOW, 10);
        assert_eq!(m.jendela_detik, 120); // tertua 120 detik lalu
        assert_eq!(m.trx_per_menit, d("1.5")); // 3 transaksi / 2 menit
    }

    #[test]
    fn metrik_jendela_minimal_satu_menit() {
        let trx = vec![
            t(JenisTransaksi::Beli, "99", "1", 5),
            t(JenisTransaksi::Beli, "99", "1", 10),
            t(JenisTransaksi::Beli, "99", "1", 20),
        ];
        let m = hitung_metrik(&trx, NOW, 10);
        assert_eq!(m.jendela_detik, 60);
        assert_eq!(m.trx_per_menit, d("3"));
    }

    #[test]
    fn metrik_tanpa_transaksi() {
        let m = hitung_metrik(&[], NOW, 10);
        assert_eq!(m.trx_per_menit, Decimal::ZERO);
        assert_eq!(m.usia_detik, None);
        assert_eq!(m.jumlah_trx, 0);
    }

    #[test]
    fn metrik_gerak_arah_dan_dominasi() {
        let trx = vec![
            t(JenisTransaksi::Beli, "100", "1", 30),
            t(JenisTransaksi::Beli, "102", "9", 20),
            t(JenisTransaksi::Jual, "101", "1", 10),
        ];
        let m = hitung_metrik(&trx, NOW, 10);
        assert_eq!(m.jendela_detik, 60);
        assert_eq!(m.gerak_persen, Some(d("2"))); // (102-100)/100
        assert_eq!(m.arah_persen, Some(d("1"))); // tertua 100 ke terbaru 101
        // nilai: 100 + 918 + 101 = 1119; terbesar 918
        let dom = m.dominasi_persen.unwrap();
        assert!(dom > d("82.03") && dom < d("82.04")); // 918 / 1119
    }

    #[test]
    fn metrik_menit_aktif() {
        // jendela 600 detik (ada transaksi usia 700 yang menentukan jendela penuh, tetapi di luar jendela)
        let trx = vec![
            t(JenisTransaksi::Beli, "99", "1", 0),
            t(JenisTransaksi::Beli, "99", "1", 590),
            t(JenisTransaksi::Beli, "99", "1", 700),
        ];
        let m = hitung_metrik(&trx, NOW, 10);
        assert_eq!(m.jumlah_trx, 2);
        assert_eq!(m.menit_aktif_persen, d("20")); // menit ke-0 dan ke-9 dari 10 menit
    }

    #[test]
    fn tahap2_lolos_dan_gagal_sesuai_ambang() {
        let kand = kandidat_abc();
        let k = KonfigScreener::default();
        let baris = nilai_tahap2(&k, &kand, &ramai(), 0, None, None).unwrap();
        assert_eq!(baris.pair, "abc_idr");
        assert_eq!(baris.metrik.trx_per_menit, d("2.4"));
        assert_eq!(baris.skor, Some(Decimal::ZERO)); // gerak nol
        assert!(!baris.dominasi_tinggi);

        let mut ketat = k.clone();
        ketat.trx_mnt_min = Some(d("3"));
        assert!(nilai_tahap2(&ketat, &kand, &ramai(), 0, None, None).is_none());

        let mut usia = k.clone();
        usia.usia_trx_maks_detik = Some(3); // transaksi terbaru berusia 5 detik
        assert!(nilai_tahap2(&usia, &kand, &ramai(), 0, None, None).is_none());

        assert!(nilai_tahap2(&k, &kand, &[], 0, None, None).is_none()); // tanpa transaksi
    }

    #[test]
    fn tahap2_memakai_server_time_bukan_jam_cadangan() {
        let kand = kandidat_abc(); // server_time = NOW
        let k = KonfigScreener::default();
        // jam cadangan sangat meleset tidak boleh memengaruhi hasil
        assert!(nilai_tahap2(&k, &kand, &ramai(), 5, None, None).is_some());
    }

    #[test]
    fn tahap2_filter_tambahan() {
        let kand = kandidat_abc();
        let mut k = KonfigScreener::default();
        k.gerak_min_persen = Some(d("0.5")); // semua harga 99: gerak nol
        assert!(nilai_tahap2(&k, &kand, &ramai(), 0, None, None).is_none());
        k.gerak_min_persen = None;
        k.arah_min_persen = Some(d("0.1")); // arah nol
        assert!(nilai_tahap2(&k, &kand, &ramai(), 0, None, None).is_none());
        k.arah_min_persen = Some(d("-0.1"));
        assert!(nilai_tahap2(&k, &kand, &ramai(), 0, None, None).is_some());
        k.arah_min_persen = None;
        k.menit_aktif_min_persen = Some(d("100"));
        assert!(nilai_tahap2(&k, &kand, &ramai(), 0, None, None).is_some()); // 10 dari 10 menit terisi
    }

    #[test]
    fn skor_dan_tanda_dominasi() {
        let kand = kandidat_abc();
        let trx = vec![
            t(JenisTransaksi::Beli, "100", "1", 30),
            t(JenisTransaksi::Beli, "102", "9", 20),
            t(JenisTransaksi::Jual, "101", "1", 10),
        ];
        let mut k = KonfigScreener::default();
        k.trx_mnt_min = None;
        let baris = nilai_tahap2(&k, &kand, &trx, 0, None, None).unwrap();
        assert!(baris.dominasi_tinggi); // 82% > 40%
        // skor = 2 / (2,0202... + 0,41) ~ 0,82
        let skor = baris.skor.unwrap();
        assert!(skor > d("0.82") && skor < d("0.83"));
    }

    const DEPTH: &str = r#"{"buy":[["100","10"],["99.6","5"],["99","20"]],
                           "sell":[["101","2"],["101.4","3"],["102","1"]]}"#;

    #[test]
    fn kedalaman_nilai_dalam_jarak_dari_best() {
        let dp = parse_kedalaman(DEPTH).unwrap();
        let (bid, ask) = dp.nilai_dalam(d("0.5")).unwrap();
        assert_eq!(bid, d("1498")); // 100x10 + 99,6x5 ; 99 di luar 0,5%
        assert_eq!(ask, d("506.2")); // 101x2 + 101,4x3 ; 102 di luar 0,5%
        assert!(parse_kedalaman(r#"{"buy":[]}"#).is_err());
        assert!(parse_kedalaman(r#"{"buy":[],"sell":[]}"#).unwrap().nilai_dalam(d("0.5")).is_none());
    }

    #[test]
    fn filter_kedalaman_butuh_modal_dan_data() {
        let kand = kandidat_abc();
        let dp = parse_kedalaman(DEPTH).unwrap();
        let mut k = KonfigScreener::default();
        k.kedalaman_kelipatan = Some(Decimal::ONE);
        // modal Rp 1.000: sisi jual hanya 506,2 -> gagal
        assert!(nilai_tahap2(&k, &kand, &ramai(), 0, Some(&dp), Some(d("1000"))).is_none());
        // modal Rp 400: kedua sisi cukup
        let ok = nilai_tahap2(&k, &kand, &ramai(), 0, Some(&dp), Some(d("400"))).unwrap();
        assert_eq!(ok.kedalaman_bid_idr, Some(d("1498")));
        assert_eq!(ok.kedalaman_ask_idr, Some(d("506.2")));
        // modal kosong atau data kedalaman tidak ada: filter dilewati
        assert!(nilai_tahap2(&k, &kand, &ramai(), 0, Some(&dp), None).is_some());
        assert!(nilai_tahap2(&k, &kand, &ramai(), 0, None, Some(d("1000"))).is_some());
    }

    fn baris(pair: &str, skor: Option<&str>, tpm: &str) -> BarisScreener {
        let mut m = MetrikPair::kosong(600);
        m.trx_per_menit = d(tpm);
        BarisScreener {
            pair: pair.to_string(),
            harga: d("1"),
            vol_idr: d("1"),
            spread: None,
            range24: Decimal::ZERO,
            perubahan24: None,
            metrik: m,
            skor: skor.map(d),
            kedalaman_bid_idr: None,
            kedalaman_ask_idr: None,
            dominasi_tinggi: false,
        }
    }

    #[test]
    fn urutan_skor_dan_urutan_trx() {
        let mut v = vec![
            baris("a_idr", None, "9"),
            baris("b_idr", Some("1.5"), "2"),
            baris("c_idr", Some("3.0"), "3"),
            baris("d_idr", Some("1.5"), "5"),
        ];
        let mut k = KonfigScreener::default();
        urutkan(&k, &mut v);
        let nama: Vec<&str> = v.iter().map(|x| x.pair.as_str()).collect();
        assert_eq!(nama, vec!["c_idr", "d_idr", "b_idr", "a_idr"]); // skor, seri oleh trx/mnt, tanpa skor terakhir
        k.urutan = UrutanScreener::TrxPerMenit;
        urutkan(&k, &mut v);
        let nama: Vec<&str> = v.iter().map(|x| x.pair.as_str()).collect();
        assert_eq!(nama, vec!["a_idr", "d_idr", "c_idr", "b_idr"]);
    }

    #[test]
    fn pair_path_tanpa_garis_bawah() {
        assert_eq!(pair_path("usdt_idr"), "usdtidr");
    }
}
