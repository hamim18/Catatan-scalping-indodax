//! Bentuk respons GET /api/v2/account (bentuk dilihat lewat tools/fase0_akun.py,
//! 7 November 2026): canTrade, canWithdraw, canWithdrawFiat, accountType,
//! balances[{asset, free, locked}], uid. Saldo berupa teks desimal.
use crate::{Error, Hasil};
use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Saldo {
    pub asset: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub free: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub locked: Decimal,
}

impl Saldo {
    /// tersedia + terkunci
    pub fn total(&self) -> Decimal {
        self.free + self.locked
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Akun {
    #[serde(default)]
    pub can_trade: bool,
    #[serde(default)]
    pub can_withdraw: bool,
    #[serde(default)]
    pub can_withdraw_fiat: bool,
    #[serde(default)]
    pub account_type: String,
    pub balances: Vec<Saldo>,
}

pub fn parse_akun(json: &str) -> Hasil<Akun> {
    serde_json::from_str(json).map_err(|e| Error::Json(e.to_string()))
}

impl Akun {
    /// Aplikasi hanya boleh membaca. True bila key TIDAK punya izin trade/withdraw.
    pub fn kunci_baca_saja(&self) -> bool {
        !self.can_trade && !self.can_withdraw && !self.can_withdraw_fiat
    }

    pub fn saldo_aset(&self, asset: &str) -> Option<&Saldo> {
        self.balances.iter().find(|s| s.asset.eq_ignore_ascii_case(asset))
    }

    /// Saldo IDR (tersedia + terkunci). Bila aset IDR tidak ada di daftar,
    /// kembalikan error; jangan dianggap nol tanpa konfirmasi.
    pub fn saldo_idr_total(&self) -> Hasil<Decimal> {
        self.saldo_aset("idr")
            .map(Saldo::total)
            .ok_or_else(|| Error::AsetTidakAda("idr".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    // Bentuk sama dengan keluaran fase0_akun.py; angka dikarang.
    const JSON: &str = r#"{"canTrade":false,"canWithdraw":false,"canWithdrawFiat":false,
      "accountType":"individual","uid":123456,
      "balances":[{"asset":"BTC","free":"0","locked":"0"},
                  {"asset":"IDR","free":"150000.5","locked":"65526"}]}"#;

    #[test]
    fn parse_dan_saldo_idr() {
        let a = parse_akun(JSON).unwrap();
        assert!(a.kunci_baca_saja());
        assert_eq!(a.saldo_idr_total().unwrap(), d("215526.5"));
        assert!(a.saldo_aset("btc").is_some()); // huruf besar/kecil tidak masalah
    }

    #[test]
    fn izin_trade_terdeteksi() {
        let a = parse_akun(&JSON.replace("\"canTrade\":false", "\"canTrade\":true")).unwrap();
        assert!(!a.kunci_baca_saja());
    }

    #[test]
    fn idr_tidak_ada_menjadi_error_bukan_nol() {
        let a = parse_akun(r#"{"balances":[{"asset":"BTC","free":"1","locked":"0"}]}"#).unwrap();
        assert!(matches!(a.saldo_idr_total(), Err(Error::AsetTidakAda(_))));
    }
}
