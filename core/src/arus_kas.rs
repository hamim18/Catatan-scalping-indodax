//! Arus kas IDR (deposit dan withdraw) dari GET /api/v2/fiat/orders (T2.2, FR-24).
//! HANYA membaca. Hanya order berstatus "success" dan mata uang IDR yang dicatat.
//!
//! Bentuk respons deposit sudah teramati di akun nyata (7 Nov 2026): data[] berisi
//! orderNo, amount (teks), totalFee (teks), method, fiatCurrency, status,
//! createTime dan updateTime (angka, milidetik). Respons withdraw baru teramati
//! KOSONG, jadi bentuknya diasumsikan sama dan BELUM tervalidasi.
use crate::{Error, Hasil, IndodaxClient, Penyimpanan};
use rust_decimal::Decimal;
use serde_json::Value;
use std::str::FromStr;

pub const MS_HARI: i64 = 86_400_000;
/// Batas Indodax 30 hari per permintaan; skrip Fase 2 berhasil dengan 29 hari.
pub const JENDELA_MAKS_MS: i64 = 29 * MS_HARI;
/// Tarikan berikutnya mundur sekian hari, supaya order yang tadinya pending dan baru
/// sukses tetap tertangkap.
pub const TUMPANG_TINDIH_MS: i64 = 14 * MS_HARI;
/// Ukuran halaman belum diketahui. Bila satu rentang mengembalikan sebanyak ini atau
/// lebih, rentang dibelah dua (supaya tidak ada baris yang diam-diam terpotong).
const DUGAAN_HALAMAN_PENUH: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JenisArusKas {
    Deposit,
    Withdraw,
}

impl JenisArusKas {
    /// Nilai parameter `transactionType` (0 = deposit, 1 = withdraw).
    pub fn kode_api(self) -> u8 {
        match self {
            JenisArusKas::Deposit => 0,
            JenisArusKas::Withdraw => 1,
        }
    }

    pub fn teks(self) -> &'static str {
        match self {
            JenisArusKas::Deposit => "deposit",
            JenisArusKas::Withdraw => "withdraw",
        }
    }

    pub fn dari_teks(s: &str) -> Option<Self> {
        match s {
            "deposit" => Some(JenisArusKas::Deposit),
            "withdraw" => Some(JenisArusKas::Withdraw),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArusKas {
    /// Unik: "<jenis>:<orderNo>" (anti-dobel, FR-09).
    pub id_sumber: String,
    pub jenis: JenisArusKas,
    /// `amount` dari API.
    pub jumlah: Decimal,
    /// `totalFee` dari API (0 bila tidak ada).
    pub biaya: Decimal,
    /// Waktu order selesai (updateTime; bila kosong createTime), milidetik.
    pub waktu_ms: i64,
}

impl ArusKas {
    /// Perubahan saldo IDR (positif = masuk, negatif = keluar).
    /// Belum diketahui apakah `amount` sudah memperhitungkan fee, jadi ada dua mode:
    /// - `amount_sudah_bersih` = true: saldo berubah sebesar amount saja.
    /// - false: deposit = amount - fee, withdraw = -(amount + fee).
    pub fn efek_saldo(&self, amount_sudah_bersih: bool) -> Decimal {
        match (self.jenis, amount_sudah_bersih) {
            (JenisArusKas::Deposit, true) => self.jumlah,
            (JenisArusKas::Deposit, false) => self.jumlah - self.biaya,
            (JenisArusKas::Withdraw, true) => -self.jumlah,
            (JenisArusKas::Withdraw, false) => -(self.jumlah + self.biaya),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HasilParseFiat {
    pub arus: Vec<ArusKas>,
    /// baris yang bukan sukses atau bukan IDR
    pub dilewati: usize,
}

fn teks_dari(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn desimal_dari(v: Option<&Value>) -> Option<Decimal> {
    let t = teks_dari(v?)?;
    Decimal::from_str(t.trim()).ok()
}

fn waktu_dari(v: Option<&Value>) -> Option<i64> {
    match v? {
        Value::Number(n) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// Parse respons fiat/orders. `jenis` berasal dari parameter permintaan (respons deposit
/// tidak memuat jenis). Angka dan waktu boleh berupa teks atau angka.
pub fn parse_fiat_orders(json: &str, jenis: JenisArusKas) -> Hasil<HasilParseFiat> {
    let v: Value = serde_json::from_str(json).map_err(|e| Error::Json(e.to_string()))?;
    let data: &[Value] = match v.get("data") {
        Some(Value::Array(a)) => a.as_slice(),
        Some(Value::Null) => &[],
        _ => return Err(Error::Json("respons fiat/orders tidak punya daftar 'data'".into())),
    };
    let mut arus = Vec::new();
    let mut dilewati = 0usize;
    for item in data {
        let status = item.get("status").and_then(teks_dari).unwrap_or_default().to_lowercase();
        if status != "success" {
            dilewati += 1;
            continue;
        }
        if let Some(mata_uang) = item.get("fiatCurrency").and_then(teks_dari) {
            if !mata_uang.eq_ignore_ascii_case("idr") {
                dilewati += 1;
                continue;
            }
        }
        let no = item
            .get("orderNo")
            .and_then(teks_dari)
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| Error::Json("baris fiat/orders tanpa orderNo".into()))?;
        let jumlah = desimal_dari(item.get("amount"))
            .ok_or_else(|| Error::Json(format!("amount tidak valid pada order {no}")))?;
        let biaya = match item.get("totalFee") {
            None | Some(Value::Null) => Decimal::ZERO,
            other => desimal_dari(other)
                .ok_or_else(|| Error::Json(format!("totalFee tidak valid pada order {no}")))?,
        };
        let waktu_ms = waktu_dari(item.get("updateTime"))
            .filter(|w| *w > 0)
            .or_else(|| waktu_dari(item.get("createTime")).filter(|w| *w > 0))
            .ok_or_else(|| Error::Json(format!("waktu tidak ada pada order {no}")))?;
        arus.push(ArusKas {
            id_sumber: format!("{}:{}", jenis.teks(), no.trim()),
            jenis,
            jumlah,
            biaya,
            waktu_ms,
        });
    }
    Ok(HasilParseFiat { arus, dilewati })
}

/// Jumlah perubahan saldo IDR dari arus kas pada rentang (setelah_ms, sampai_ms]:
/// batas awal TIDAK ikut, batas akhir ikut.
pub fn setoran_bersih(arus: &[ArusKas], setelah_ms: i64, sampai_ms: i64, amount_sudah_bersih: bool) -> Decimal {
    arus.iter()
        .filter(|a| a.waktu_ms > setelah_ms && a.waktu_ms <= sampai_ms)
        .map(|a| a.efek_saldo(amount_sudah_bersih))
        .sum::<Decimal>()
}

/// Bagi rentang waktu menjadi jendela maksimal 29 hari.
pub fn jendela_waktu(mulai_ms: i64, akhir_ms: i64) -> Vec<(i64, i64)> {
    let mut hasil = Vec::new();
    let mut awal = mulai_ms;
    while awal < akhir_ms {
        let ujung = (awal + JENDELA_MAKS_MS).min(akhir_ms);
        hasil.push((awal, ujung));
        awal = ujung;
    }
    hasil
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HasilArusKas {
    pub baru: usize,
    pub total: usize,
    pub dilewati: usize,
}

fn ambil_rentang(
    klien: &IndodaxClient,
    jenis: JenisArusKas,
    mulai_ms: i64,
    akhir_ms: i64,
    keluaran: &mut Vec<ArusKas>,
    dilewati: &mut usize,
) -> Hasil<()> {
    let h = klien.ambil_arus_kas(jenis, mulai_ms.max(0) as u64, akhir_ms.max(0) as u64)?;
    let jumlah = h.arus.len() + h.dilewati;
    if jumlah >= DUGAAN_HALAMAN_PENUH {
        if akhir_ms - mulai_ms > 3_600_000 {
            let tengah = mulai_ms + (akhir_ms - mulai_ms) / 2;
            ambil_rentang(klien, jenis, mulai_ms, tengah, keluaran, dilewati)?;
            return ambil_rentang(klien, jenis, tengah, akhir_ms, keluaran, dilewati);
        }
        return Err(Error::RentangTerlaluPadat { simbol: "fiat/orders".into(), jumlah });
    }
    keluaran.extend(h.arus);
    *dilewati += h.dilewati;
    Ok(())
}

/// Tarik deposit dan withdraw IDR sejak snapshot saldo awal (pertama kali) atau sejak
/// 14 hari sebelum sinkron terakhir (berikutnya), simpan tanpa dobel.
/// Butuh snapshot saldo awal (dibuat oleh sinkron trade pertama).
pub fn sinkron_arus_kas(klien: &IndodaxClient, simpan: &mut Penyimpanan, sekarang_ms: i64) -> Hasil<HasilArusKas> {
    let (awal_ms, _) = simpan.snapshot_awal()?.ok_or(Error::BelumAdaSnapshot)?;
    let mulai = match simpan.arus_kas_terakhir_ms()? {
        // pertama kali: mundur sehari, kalau-kalau ada order yang dibuat sebelum snapshot dan selesai sesudahnya
        None => awal_ms - MS_HARI,
        Some(t) => (t - TUMPANG_TINDIH_MS).max(awal_ms - MS_HARI),
    };
    let mut semua = Vec::new();
    let mut dilewati = 0usize;
    for jenis in [JenisArusKas::Deposit, JenisArusKas::Withdraw] {
        for (a, b) in jendela_waktu(mulai, sekarang_ms) {
            ambil_rentang(klien, jenis, a, b, &mut semua, &mut dilewati)?;
        }
    }
    let baru = simpan.simpan_arus_kas(&semua)?;
    simpan.atur_arus_kas_terakhir_ms(sekarang_ms)?;
    Ok(HasilArusKas { baru, total: simpan.jumlah_arus_kas()?, dilewati })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    const DEPOSIT: &str = r#"{"data":[{"orderNo":"123456","amount":"50000","totalFee":"2500",
        "method":"Bank X","fiatCurrency":"IDR","status":"success",
        "createTime":1793900000000,"updateTime":1793900600000}]}"#;

    #[test]
    fn parse_deposit_sukses() {
        let h = parse_fiat_orders(DEPOSIT, JenisArusKas::Deposit).unwrap();
        assert_eq!(h.dilewati, 0);
        assert_eq!(h.arus.len(), 1);
        let a = &h.arus[0];
        assert_eq!(a.id_sumber, "deposit:123456");
        assert_eq!(a.jumlah, d("50000"));
        assert_eq!(a.biaya, d("2500"));
        assert_eq!(a.waktu_ms, 1793900600000);
    }

    #[test]
    fn status_bukan_sukses_dan_mata_uang_lain_dilewati() {
        let j = r#"{"data":[
            {"orderNo":"1","amount":"1000","totalFee":"0","fiatCurrency":"IDR","status":"pending","createTime":1,"updateTime":1},
            {"orderNo":"2","amount":"1000","totalFee":"0","fiatCurrency":"USD","status":"success","createTime":1,"updateTime":1},
            {"orderNo":"3","amount":"1000","totalFee":"0","fiatCurrency":"IDR","status":"SUCCESS","createTime":5,"updateTime":0}]}"#;
        let h = parse_fiat_orders(j, JenisArusKas::Deposit).unwrap();
        assert_eq!(h.dilewati, 2);
        assert_eq!(h.arus.len(), 1);
        assert_eq!(h.arus[0].waktu_ms, 5); // updateTime 0 -> pakai createTime
    }

    #[test]
    fn data_kosong_dan_angka_berbentuk_angka() {
        assert!(parse_fiat_orders(r#"{"data":[]}"#, JenisArusKas::Withdraw).unwrap().arus.is_empty());
        assert!(parse_fiat_orders(r#"{"data":null}"#, JenisArusKas::Withdraw).unwrap().arus.is_empty());
        let j = r#"{"data":[{"orderNo":77,"amount":25000,"status":"success","createTime":"1793900000000"}]}"#;
        let h = parse_fiat_orders(j, JenisArusKas::Withdraw).unwrap();
        assert_eq!(h.arus[0].id_sumber, "withdraw:77");
        assert_eq!(h.arus[0].biaya, Decimal::ZERO);
        assert_eq!(h.arus[0].waktu_ms, 1793900000000);
    }

    #[test]
    fn respons_rusak_ditolak() {
        assert!(parse_fiat_orders("bukan json", JenisArusKas::Deposit).is_err());
        assert!(parse_fiat_orders(r#"{"x":1}"#, JenisArusKas::Deposit).is_err());
        let tanpa_no = r#"{"data":[{"amount":"1","status":"success","createTime":1}]}"#;
        assert!(parse_fiat_orders(tanpa_no, JenisArusKas::Deposit).is_err());
        let amount_rusak = r#"{"data":[{"orderNo":"1","amount":"abc","status":"success","createTime":1}]}"#;
        assert!(parse_fiat_orders(amount_rusak, JenisArusKas::Deposit).is_err());
    }

    #[test]
    fn efek_saldo_dua_mode() {
        let dep = ArusKas { id_sumber: "deposit:1".into(), jenis: JenisArusKas::Deposit, jumlah: d("50000"), biaya: d("2500"), waktu_ms: 1 };
        let wd = ArusKas { id_sumber: "withdraw:1".into(), jenis: JenisArusKas::Withdraw, jumlah: d("20000"), biaya: d("6500"), waktu_ms: 1 };
        assert_eq!(dep.efek_saldo(true), d("50000"));
        assert_eq!(dep.efek_saldo(false), d("47500"));
        assert_eq!(wd.efek_saldo(true), d("-20000"));
        assert_eq!(wd.efek_saldo(false), d("-26500"));
    }

    #[test]
    fn setoran_bersih_batas_awal_tidak_ikut_batas_akhir_ikut() {
        let dep = |id: &str, w: i64| ArusKas { id_sumber: id.into(), jenis: JenisArusKas::Deposit, jumlah: d("1000"), biaya: Decimal::ZERO, waktu_ms: w };
        let wd = ArusKas { id_sumber: "w".into(), jenis: JenisArusKas::Withdraw, jumlah: d("300"), biaya: Decimal::ZERO, waktu_ms: 25 };
        let arus = vec![dep("a", 10), dep("b", 20), wd, dep("c", 30)];
        assert_eq!(setoran_bersih(&arus, 10, 30, true), d("1700")); // b + (-300) + c
        assert_eq!(setoran_bersih(&arus, 30, 40, true), Decimal::ZERO);
        assert_eq!(setoran_bersih(&arus, 0, 9, true), Decimal::ZERO);
    }

    #[test]
    fn jendela_dibelah_29_hari() {
        let w = jendela_waktu(0, 60 * MS_HARI);
        assert_eq!(w, vec![(0, 29 * MS_HARI), (29 * MS_HARI, 58 * MS_HARI), (58 * MS_HARI, 60 * MS_HARI)]);
        assert!(jendela_waktu(5, 5).is_empty());
        assert_eq!(jendela_waktu(0, 1000), vec![(0, 1000)]);
    }

    #[test]
    fn jenis_bolak_balik_teks() {
        assert_eq!(JenisArusKas::dari_teks(JenisArusKas::Deposit.teks()), Some(JenisArusKas::Deposit));
        assert_eq!(JenisArusKas::dari_teks(JenisArusKas::Withdraw.teks()), Some(JenisArusKas::Withdraw));
        assert_eq!(JenisArusKas::dari_teks("x"), None);
        assert_eq!(JenisArusKas::Deposit.kode_api(), 0);
        assert_eq!(JenisArusKas::Withdraw.kode_api(), 1);
    }
}
