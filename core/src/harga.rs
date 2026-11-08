//! T6.1: harga pasar publik Indodax dan total aset (IDR + koin dinilai harga terakhir).
//! Hanya membaca. Endpoint publik GET /api/ticker_all (tanpa key, tanpa tanda tangan).
//! Bentuk respons (diverifikasi dari curl pengguna, 8 Nov 2026):
//! {"tickers":{"btc_idr":{"buy":"..","high":"..","last":"1368000000","low":"..","sell":"..",...},...}}
//! Nilai berupa teks desimal; nama pasangan huruf kecil dengan garis bawah.
use crate::account::Akun;
use crate::{Error, Hasil};
use rust_decimal::Decimal;
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;

/// Harga terakhir (last) per koin terhadap IDR. Kunci: simbol koin huruf kecil, contoh "btc".
pub type HargaIdr = HashMap<String, Decimal>;

/// Ambil `last` semua pasangan yang berakhiran `_idr`. Pasangan tanpa `last` yang valid dan
/// positif dilewati (koin itu nanti ditandai "tanpa harga"). Error bila bentuk respons tidak dikenali.
pub fn parse_ticker_all(json: &str) -> Hasil<HargaIdr> {
    let v: Value = serde_json::from_str(json).map_err(|e| Error::Json(e.to_string()))?;
    let tickers = v
        .get("tickers")
        .and_then(Value::as_object)
        .ok_or_else(|| Error::Json("respons ticker_all tidak memuat 'tickers'".into()))?;
    let mut hasil = HargaIdr::new();
    for (pasangan, data) in tickers {
        let Some(koin) = pasangan.strip_suffix("_idr") else { continue };
        let last = match data.get("last") {
            Some(Value::String(s)) => Decimal::from_str(s.trim()).ok(),
            Some(Value::Number(n)) => Decimal::from_str(&n.to_string()).ok(),
            _ => None,
        };
        if let Some(h) = last {
            if h > Decimal::ZERO {
                hasil.insert(koin.to_lowercase(), h);
            }
        }
    }
    if hasil.is_empty() {
        return Err(Error::Json("ticker_all tidak memuat harga IDR satu pun".into()));
    }
    Ok(hasil)
}

/// Satu koin yang dipegang beserta nilainya.
#[derive(Debug, Clone, PartialEq)]
pub struct AsetNilai {
    /// huruf besar, contoh "DOGE"
    pub koin: String,
    /// tersedia + terkunci
    pub jumlah: Decimal,
    pub harga: Decimal,
    pub nilai: Decimal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TotalAset {
    pub idr: Decimal,
    /// diurutkan dari nilai terbesar
    pub koin: Vec<AsetNilai>,
    /// koin dengan saldo > 0 tetapi tanpa harga IDR: TIDAK dihitung dalam total
    pub tanpa_harga: Vec<String>,
    /// idr + jumlah nilai semua koin berharga
    pub total: Decimal,
}

impl TotalAset {
    pub fn nilai_koin(&self) -> Decimal {
        self.koin.iter().fold(Decimal::ZERO, |a, k| a + k.nilai)
    }
}

/// Total aset = saldo IDR (tersedia + terkunci) + Σ (jumlah koin × harga terakhir).
/// Tanpa jaringan. Error bila aset IDR tidak ada di akun (tidak dianggap nol).
pub fn hitung_total_aset(akun: &Akun, harga: &HargaIdr) -> Hasil<TotalAset> {
    let idr = akun.saldo_idr_total()?;
    let mut koin = Vec::new();
    let mut tanpa_harga = Vec::new();
    for s in &akun.balances {
        if s.asset.eq_ignore_ascii_case("idr") {
            continue;
        }
        let jumlah = s.total();
        if jumlah <= Decimal::ZERO {
            continue;
        }
        match harga.get(&s.asset.to_lowercase()) {
            Some(h) => koin.push(AsetNilai {
                koin: s.asset.to_uppercase(),
                jumlah,
                harga: *h,
                nilai: jumlah * *h,
            }),
            None => tanpa_harga.push(s.asset.to_uppercase()),
        }
    }
    koin.sort_by(|a, b| b.nilai.cmp(&a.nilai));
    tanpa_harga.sort();
    let total = idr + koin.iter().fold(Decimal::ZERO, |a, k| a + k.nilai);
    Ok(TotalAset { idr, koin, tanpa_harga, total })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::parse_akun;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    // Potongan bentuk nyata dari curl 8 Nov 2026 (angka dipertahankan), ditambah kasus tepi.
    const TICKER: &str = r#"{"tickers":{
      "btc_idr":{"buy":"1368000000","high":"1376808000","last":"1368000000","low":"1367430000","sell":"1369741000","server_time":1794113451,"vol_btc":"11.29708566","vol_idr":"15488714782"},
      "ab_idr":{"buy":"8.953","high":"9.188","last":"8.953","low":"8.95","sell":"9.181","server_time":1794113451},
      "ach_idr":{"buy":"78","high":"81","last":"79","low":"78","sell":"80","server_time":1794113451},
      "eth_btc":{"last":"0.05"},
      "mati_idr":{"last":"0"},
      "angka_idr":{"last":123.5},
      "rusak_idr":{"buy":"1"}}}"#;

    fn akun(balances: &str) -> Akun {
        parse_akun(&format!(r#"{{"balances":[{balances}]}}"#)).unwrap()
    }

    #[test]
    fn parse_ambil_last_pasangan_idr_saja() {
        let h = parse_ticker_all(TICKER).unwrap();
        assert_eq!(h["btc"], d("1368000000"));
        assert_eq!(h["ab"], d("8.953"));
        assert_eq!(h["ach"], d("79"));
        assert_eq!(h["angka"], d("123.5"));
        assert!(!h.contains_key("eth")); // pasangan bukan IDR
        assert!(!h.contains_key("mati")); // last nol dianggap tanpa harga
        assert!(!h.contains_key("rusak")); // tanpa last
    }

    #[test]
    fn parse_bentuk_tidak_dikenal_menjadi_error() {
        assert!(parse_ticker_all("{}").is_err());
        assert!(parse_ticker_all("bukan json").is_err());
        assert!(parse_ticker_all(r#"{"tickers":{"eth_btc":{"last":"1"}}}"#).is_err());
    }

    #[test]
    fn total_aset_idr_ditambah_nilai_koin() {
        let h = parse_ticker_all(TICKER).unwrap();
        let a = akun(
            r#"{"asset":"IDR","free":"100000","locked":"0"},
               {"asset":"BTC","free":"0.0005","locked":"0.0005"},
               {"asset":"AB","free":"100","locked":"0"}"#,
        );
        let t = hitung_total_aset(&a, &h).unwrap();
        // BTC 0,001 x 1.368.000.000 = 1.368.000 ; AB 100 x 8,953 = 895,3
        assert_eq!(t.idr, d("100000"));
        assert_eq!(t.nilai_koin(), d("1368895.3"));
        assert_eq!(t.total, d("1468895.3"));
        assert_eq!(t.koin[0].koin, "BTC"); // nilai terbesar di atas
        assert_eq!(t.koin[0].jumlah, d("0.001")); // tersedia + terkunci
        assert!(t.tanpa_harga.is_empty());
    }

    #[test]
    fn koin_tanpa_harga_ditandai_dan_tidak_dihitung_saldo_nol_dilewati() {
        let h = parse_ticker_all(TICKER).unwrap();
        let a = akun(
            r#"{"asset":"IDR","free":"5000","locked":"1000"},
               {"asset":"PXG","free":"3","locked":"0"},
               {"asset":"BTC","free":"0","locked":"0"},
               {"asset":"ACH","free":"10","locked":"0"}"#,
        );
        let t = hitung_total_aset(&a, &h).unwrap();
        assert_eq!(t.tanpa_harga, vec!["PXG".to_string()]);
        assert_eq!(t.koin.len(), 1); // BTC saldo nol tidak tampil
        assert_eq!(t.total, d("6790")); // 6000 + 10 x 79
    }

    #[test]
    fn idr_tidak_ada_menjadi_error_bukan_nol() {
        let h = parse_ticker_all(TICKER).unwrap();
        let a = akun(r#"{"asset":"BTC","free":"1","locked":"0"}"#);
        assert!(hitung_total_aset(&a, &h).is_err());
    }
}
