//! T4.1: statistik trade tertutup (FR-25). Tanpa jaringan, semua uang memakai Decimal.
//!
//! - Win rate = trade untung / semua trade x 100 (impas dan rugi tidak dihitung menang).
//! - Rata-rata untung / rugi: hanya dari trade yang untung / rugi.
//! - Total fee: fee beli proporsional + fee jual (sudah terkandung di profit bersih).
//! - Profit kotor = hasil bersih + total fee (hasil andai tanpa fee).
//! - Persen profit termakan fee = total fee / profit kotor x 100; hanya bila profit kotor > 0.
use crate::posisi::TradeTertutup;
use rust_decimal::prelude::*;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Statistik {
    pub jumlah: usize,
    pub untung: usize,
    pub rugi: usize,
    pub impas: usize,
    pub win_rate_persen: Option<Decimal>,
    pub rata_untung: Option<Decimal>,
    /// negatif
    pub rata_rugi: Option<Decimal>,
    pub hasil_bersih: Decimal,
    pub total_fee: Decimal,
    pub profit_kotor: Decimal,
    pub persen_fee_dari_profit: Option<Decimal>,
}

/// Statistik trade yang ditutup pada atau setelah `dari_ms` (gunakan `i64::MIN` untuk semua).
pub fn hitung_statistik(tertutup: &[TradeTertutup], dari_ms: i64) -> Statistik {
    let mut s = Statistik::default();
    let mut jumlah_untung = Decimal::ZERO;
    let mut jumlah_rugi = Decimal::ZERO;
    for t in tertutup.iter().filter(|t| t.waktu_tutup_ms >= dari_ms) {
        s.jumlah += 1;
        s.hasil_bersih += t.profit;
        s.total_fee += t.fee;
        if t.profit > Decimal::ZERO {
            s.untung += 1;
            jumlah_untung += t.profit;
        } else if t.profit < Decimal::ZERO {
            s.rugi += 1;
            jumlah_rugi += t.profit;
        } else {
            s.impas += 1;
        }
    }
    s.profit_kotor = s.hasil_bersih + s.total_fee;
    if s.jumlah > 0 {
        s.win_rate_persen = Some(Decimal::from(s.untung as u64) / Decimal::from(s.jumlah as u64) * Decimal::ONE_HUNDRED);
    }
    if s.untung > 0 {
        s.rata_untung = Some(jumlah_untung / Decimal::from(s.untung as u64));
    }
    if s.rugi > 0 {
        s.rata_rugi = Some(jumlah_rugi / Decimal::from(s.rugi as u64));
    }
    if s.profit_kotor > Decimal::ZERO {
        s.persen_fee_dari_profit = Some(s.total_fee / s.profit_kotor * Decimal::ONE_HUNDRED);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::posisi::{hitung, RiwayatSetelan};
    use crate::Trade;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn tt(profit: &str, fee: &str, tutup: i64) -> TradeTertutup {
        TradeTertutup {
            simbol: "x".into(),
            koin: d("1"),
            modal: d("1000"),
            idr_diterima: d("1000") + d(profit),
            profit: d(profit),
            persen: Decimal::ZERO,
            fee: d(fee),
            waktu_buka_ms: 0,
            waktu_tutup_ms: tutup,
        }
    }

    fn trade(id: &str, order: &str, beli: bool, qty: &str, nilai: &str, fee: &str, waktu: i64) -> Trade {
        Trade {
            trade_id: id.into(),
            order_id: order.into(),
            symbol: "icntidr".into(),
            price: d("1"),
            qty: d(qty),
            quote_qty: d(nilai),
            commission: d(fee),
            commission_asset: "idr".into(),
            is_buyer: beli,
            is_maker: false,
            time: waktu,
        }
    }

    fn setelan() -> RiwayatSetelan {
        RiwayatSetelan::baru(vec![(0, d("1"), d("1"))])
    }

    #[test]
    fn kosong_tidak_menghasilkan_persen() {
        let s = hitung_statistik(&[], i64::MIN);
        assert_eq!(s.jumlah, 0);
        assert!(s.win_rate_persen.is_none() && s.rata_untung.is_none() && s.rata_rugi.is_none());
        assert!(s.persen_fee_dari_profit.is_none());
    }

    #[test]
    fn campuran_untung_rugi_impas() {
        let t = vec![tt("380", "340", 10), tt("100", "50", 20), tt("-200", "60", 30), tt("0", "10", 40)];
        let s = hitung_statistik(&t, i64::MIN);
        assert_eq!((s.jumlah, s.untung, s.rugi, s.impas), (4, 2, 1, 1));
        assert_eq!(s.win_rate_persen, Some(d("50")));
        assert_eq!(s.rata_untung, Some(d("240")));
        assert_eq!(s.rata_rugi, Some(d("-200")));
        assert_eq!(s.hasil_bersih, d("280"));
        assert_eq!(s.total_fee, d("460"));
        assert_eq!(s.profit_kotor, d("740"));
        assert_eq!(s.persen_fee_dari_profit.unwrap().round_dp(2), d("62.16"));
    }

    #[test]
    fn rentang_waktu_memfilter_berdasarkan_waktu_tutup() {
        let t = vec![tt("100", "10", 10), tt("200", "20", 50)];
        let s = hitung_statistik(&t, 50);
        assert_eq!(s.jumlah, 1);
        assert_eq!(s.hasil_bersih, d("200"));
    }

    #[test]
    fn profit_kotor_tidak_positif_tanpa_persen_fee() {
        let s = hitung_statistik(&[tt("-100", "30", 1)], i64::MIN);
        assert_eq!(s.profit_kotor, d("-70"));
        assert!(s.persen_fee_dari_profit.is_none());
    }

    #[test]
    fn fee_icnt_dari_hitung_340_dan_profit_kotor_720() {
        let trades = vec![
            trade("1", "b1", true, "16.35667833", "65393", "133", 1000),
            trade("2", "j1", false, "16.35667833", "66113", "207", 2000),
        ];
        let h = hitung(&trades, 0, &setelan()).unwrap();
        assert_eq!(h.tertutup[0].fee, d("340"));
        let s = hitung_statistik(&h.tertutup, i64::MIN);
        assert_eq!(s.hasil_bersih, d("380"));
        assert_eq!(s.profit_kotor, d("720"));
        assert_eq!(s.persen_fee_dari_profit.unwrap().round_dp(2), d("47.22"));
    }

    #[test]
    fn jual_sebagian_fee_beli_proporsional_dan_sisa_dilanjutkan() {
        // beli 10 koin: nilai 1000 + fee 10. Jual 4 koin: nilai 500 - fee 5.
        // fee = 10*4/10 + 5 = 9; profit = 495 - 404 = 91.
        let trades = vec![
            trade("1", "b1", true, "10", "1000", "10", 1000),
            trade("2", "j1", false, "4", "500", "5", 2000),
            trade("3", "j2", false, "6", "700", "7", 3000),
        ];
        let h = hitung(&trades, 0, &setelan()).unwrap();
        assert_eq!(h.tertutup.len(), 2);
        assert_eq!(h.tertutup[0].fee, d("9"));
        assert_eq!(h.tertutup[0].profit, d("91"));
        // sisa 6 koin: fee beli 6 + fee jual 7 = 13; profit = 693 - 606 = 87
        assert_eq!(h.tertutup[1].fee, d("13"));
        assert_eq!(h.tertutup[1].profit, d("87"));
        let s = hitung_statistik(&h.tertutup, i64::MIN);
        assert_eq!(s.total_fee, d("22")); // 10 + 5 + 7, tidak ada yang hilang
    }
}
