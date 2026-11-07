//! T1.3: posisi, modal efektif, TP/SL (FR-11 sampai FR-13).
//! Semua dihitung ULANG dari catatan trade (bukan angka mentah), jadi
//! mengoreksi satu catatan dapat menghitung ulang semuanya. Tanpa jaringan.
//!
//! Aturan:
//! - Satu posisi per koin. Beli berikutnya merata-ratakan modal efektif.
//! - Jual sebagian: posisi tetap terbuka, profit proporsional. Jual penuh: posisi tutup.
//! - TP%/SL% diambil dari setelan yang berlaku saat posisi DIBUKA (FR-05).
//! - Jual melebihi posisi (koin dibeli sebelum dihitung): bagian tanpa modal
//!   dicatat sebagai penyesuaian dan tidak masuk profit.
use crate::calc::{bulatkan_idr, gabung_per_order, Posisi, Sisi};
use crate::{Error, Hasil, Trade};
use rust_decimal::prelude::*;
use std::collections::HashMap;

/// Riwayat setelan TP/SL: (berlaku_sejak_ms, TP%, SL%).
#[derive(Debug, Clone, PartialEq)]
pub struct RiwayatSetelan {
    baris: Vec<(i64, Decimal, Decimal)>,
}

impl RiwayatSetelan {
    pub fn baru(mut baris: Vec<(i64, Decimal, Decimal)>) -> Self {
        baris.sort_by_key(|b| b.0);
        Self { baris }
    }

    /// Setelan yang berlaku pada `waktu_ms`. Bila belum ada yang berlaku
    /// (posisi dibuka sebelum setelan pertama dicatat), pakai setelan tertua.
    pub fn pada(&self, waktu_ms: i64) -> Hasil<(Decimal, Decimal)> {
        let mut pilih = None;
        for b in &self.baris {
            if b.0 <= waktu_ms {
                pilih = Some(b);
            }
        }
        pilih
            .or_else(|| self.baris.first())
            .map(|b| (b.1, b.2))
            .ok_or(Error::SetelanBelumDiisi)
    }
}

/// TP harus > 0; SL harus > 0 dan < 100 (SL 100% atau lebih tidak masuk akal).
pub fn periksa_persen(tp: Decimal, sl: Decimal) -> Hasil<()> {
    if tp <= Decimal::ZERO {
        return Err(Error::SetelanTidakValid("TP harus lebih dari 0".into()));
    }
    if sl <= Decimal::ZERO || sl >= Decimal::ONE_HUNDRED {
        return Err(Error::SetelanTidakValid("SL harus lebih dari 0 dan kurang dari 100".into()));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct PosisiTerbuka {
    pub simbol: String,
    pub koin: Decimal,
    pub total_idr_keluar: Decimal,
    pub modal_efektif: Decimal,
    pub tp_persen: Decimal,
    pub sl_persen: Decimal,
    pub harga_tp: Decimal,
    pub harga_sl: Decimal,
    pub waktu_buka_ms: i64,
}

/// Satu order jual yang cocok dengan posisi (penuh atau sebagian).
#[derive(Debug, Clone, PartialEq)]
pub struct TradeTertutup {
    pub simbol: String,
    pub koin: Decimal,
    pub modal: Decimal,
    pub idr_diterima: Decimal,
    pub profit: Decimal,
    pub persen: Decimal,
    pub waktu_buka_ms: i64,
    pub waktu_tutup_ms: i64,
}

/// Penjualan koin yang tidak punya modal tercatat. Dikeluarkan dari profit.
#[derive(Debug, Clone, PartialEq)]
pub struct Penyesuaian {
    pub simbol: String,
    pub order_id: String,
    pub koin: Decimal,
    pub idr_diterima: Decimal,
    pub waktu_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct HasilHitung {
    pub posisi: Vec<PosisiTerbuka>,
    pub tertutup: Vec<TradeTertutup>,
    pub penyesuaian: Vec<Penyesuaian>,
}

struct Aktif {
    pos: Posisi,
    tp: Decimal,
    sl: Decimal,
    waktu_buka: i64,
}

/// Hitung ulang semua posisi dari trade. Trade dengan waktu < `sejak_ms` diabaikan.
pub fn hitung(trades: &[Trade], sejak_ms: i64, setelan: &RiwayatSetelan) -> Hasil<HasilHitung> {
    let relevan: Vec<Trade> = trades.iter().filter(|t| t.time >= sejak_ms).cloned().collect();
    let mut orders = gabung_per_order(&relevan)?;
    // Urut waktu; bila sama, beli lebih dulu dari jual.
    orders.sort_by_key(|o| (o.waktu_awal_ms, o.sisi == Sisi::Jual));

    let mut aktif: HashMap<String, Aktif> = HashMap::new();
    let mut hasil = HasilHitung::default();

    for o in &orders {
        match o.sisi {
            Sisi::Beli => {
                if let Some(a) = aktif.get_mut(&o.symbol) {
                    a.pos.tambah_beli(o);
                } else {
                    let (tp, sl) = setelan.pada(o.waktu_awal_ms)?;
                    aktif.insert(
                        o.symbol.clone(),
                        Aktif { pos: Posisi::dari_beli(o)?, tp, sl, waktu_buka: o.waktu_awal_ms },
                    );
                }
            }
            Sisi::Jual => {
                let Some(a) = aktif.get_mut(&o.symbol) else {
                    hasil.penyesuaian.push(Penyesuaian {
                        simbol: o.symbol.clone(),
                        order_id: o.order_id.clone(),
                        koin: o.koin,
                        idr_diterima: o.idr_diterima(),
                        waktu_ms: o.waktu_akhir_ms,
                    });
                    continue;
                };
                let (koin_cocok, idr_cocok) = if o.koin <= a.pos.koin {
                    (o.koin, o.idr_diterima())
                } else {
                    // kali dulu baru bagi
                    (a.pos.koin, o.idr_diterima() * a.pos.koin / o.koin)
                };
                let h = a.pos.jual(koin_cocok, idr_cocok)?;
                hasil.tertutup.push(TradeTertutup {
                    simbol: o.symbol.clone(),
                    koin: koin_cocok,
                    modal: h.modal_terjual,
                    idr_diterima: h.idr_diterima,
                    profit: h.profit,
                    persen: h.persen,
                    waktu_buka_ms: a.waktu_buka,
                    waktu_tutup_ms: o.waktu_akhir_ms,
                });
                if koin_cocok < o.koin {
                    hasil.penyesuaian.push(Penyesuaian {
                        simbol: o.symbol.clone(),
                        order_id: o.order_id.clone(),
                        koin: o.koin - koin_cocok,
                        idr_diterima: o.idr_diterima() - idr_cocok,
                        waktu_ms: o.waktu_akhir_ms,
                    });
                }
                if a.pos.koin.is_zero() {
                    aktif.remove(&o.symbol);
                }
            }
        }
    }

    for (simbol, a) in &aktif {
        hasil.posisi.push(PosisiTerbuka {
            simbol: simbol.clone(),
            koin: a.pos.koin,
            total_idr_keluar: a.pos.total_idr_keluar,
            modal_efektif: a.pos.modal_efektif()?,
            tp_persen: a.tp,
            sl_persen: a.sl,
            harga_tp: a.pos.harga_tp(a.tp)?,
            harga_sl: a.pos.harga_sl(a.sl)?,
            waktu_buka_ms: a.waktu_buka,
        });
    }
    hasil.posisi.sort_by(|x, y| x.simbol.cmp(&y.simbol));
    Ok(hasil)
}

/// Bulatkan ke rupiah utuh (hanya tampilan).
pub fn rupiah(d: Decimal) -> Decimal {
    bulatkan_idr(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    /// Trade palsu: beli: IDR keluar = nilai + fee; jual: diterima = nilai - fee.
    fn t(id: &str, order: &str, simbol: &str, beli: bool, qty: &str, nilai: &str, fee: &str, waktu: i64) -> Trade {
        Trade {
            trade_id: id.into(),
            order_id: order.into(),
            symbol: simbol.into(),
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

    fn setelan_1_1() -> RiwayatSetelan {
        RiwayatSetelan::baru(vec![(0, d("1"), d("1"))])
    }

    #[test]
    fn beli_icnt_menghasilkan_posisi_tp_sl_sesuai_struk() {
        let trades = vec![t("1", "b1", "icntidr", true, "16.35667833", "65393", "133", 1000)];
        let h = hitung(&trades, 0, &setelan_1_1()).unwrap();
        assert_eq!(h.posisi.len(), 1);
        let p = &h.posisi[0];
        assert_eq!(p.modal_efektif.round_dp(2), d("4006.07"));
        assert_eq!(rupiah(p.harga_tp), d("4046"));
        assert_eq!(rupiah(p.harga_sl), d("3966"));
        assert!(h.tertutup.is_empty());
    }

    #[test]
    fn beli_lalu_jual_icnt_profit_380_dan_posisi_tutup() {
        let trades = vec![
            t("1", "b1", "icntidr", true, "16.35667833", "65393", "133", 1000),
            t("2", "j1", "icntidr", false, "16.35667833", "66113", "207", 2000),
        ];
        let h = hitung(&trades, 0, &setelan_1_1()).unwrap();
        assert!(h.posisi.is_empty());
        assert_eq!(h.tertutup.len(), 1);
        assert_eq!(h.tertutup[0].profit, d("380"));
        assert_eq!(h.tertutup[0].persen.round_dp(2), d("0.58"));
        assert_eq!(h.tertutup[0].waktu_buka_ms, 1000);
        assert_eq!(h.tertutup[0].waktu_tutup_ms, 2000);
        assert!(h.penyesuaian.is_empty());
    }

    #[test]
    fn beli_dua_kali_dirata_rata_dan_tp_sl_dihitung_ulang() {
        let trades = vec![
            t("1", "b1", "xidr", true, "10", "990", "10", 1000), // keluar 1000
            t("2", "b2", "xidr", true, "10", "1180", "20", 2000), // keluar 1200
        ];
        let h = hitung(&trades, 0, &setelan_1_1()).unwrap();
        let p = &h.posisi[0];
        assert_eq!(p.koin, d("20"));
        assert_eq!(p.modal_efektif, d("110"));
        assert_eq!(p.harga_tp, d("111.1"));
        assert_eq!(p.harga_sl, d("108.9"));
        assert_eq!(p.waktu_buka_ms, 1000);
    }

    #[test]
    fn jual_sebagian_posisi_tetap_terbuka_profit_proporsional() {
        let trades = vec![
            t("1", "b1", "xidr", true, "10", "990", "10", 1000), // modal 100/koin
            t("2", "j1", "xidr", false, "4", "450", "10", 2000), // diterima 440
        ];
        let h = hitung(&trades, 0, &setelan_1_1()).unwrap();
        assert_eq!(h.tertutup[0].modal, d("400"));
        assert_eq!(h.tertutup[0].profit, d("40"));
        assert_eq!(h.tertutup[0].persen, d("10"));
        assert_eq!(h.posisi[0].koin, d("6"));
        assert_eq!(h.posisi[0].total_idr_keluar, d("600"));
        assert_eq!(h.posisi[0].modal_efektif, d("100"));
    }

    #[test]
    fn sisa_koin_kecil_ikut_dirata_rata_pada_beli_berikutnya() {
        // seperti data nyata: beli 14,29, jual 14, lalu beli lagi
        let trades = vec![
            t("1", "b1", "uaiidr", true, "14.29", "1429", "0", 1000),
            t("2", "j1", "uaiidr", false, "14", "1500", "0", 2000),
            t("3", "b2", "uaiidr", true, "10", "1000", "0", 3000),
        ];
        let h = hitung(&trades, 0, &setelan_1_1()).unwrap();
        assert_eq!(h.posisi[0].koin, d("10.29"));
        assert_eq!(h.posisi[0].total_idr_keluar, d("1029")); // sisa 0,29 koin bermodal 29 + beli 1000
    }

    #[test]
    fn jual_tanpa_posisi_menjadi_penyesuaian_bukan_profit() {
        let trades = vec![t("1", "j1", "xidr", false, "5", "500", "5", 1000)];
        let h = hitung(&trades, 0, &setelan_1_1()).unwrap();
        assert!(h.tertutup.is_empty());
        assert!(h.posisi.is_empty());
        assert_eq!(h.penyesuaian.len(), 1);
        assert_eq!(h.penyesuaian[0].koin, d("5"));
        assert_eq!(h.penyesuaian[0].idr_diterima, d("495"));
    }

    #[test]
    fn jual_melebihi_posisi_dipecah_cocok_dan_penyesuaian() {
        let trades = vec![
            t("1", "b1", "xidr", true, "10", "1000", "0", 1000), // modal 100
            t("2", "j1", "xidr", false, "15", "1650", "0", 2000),
        ];
        let h = hitung(&trades, 0, &setelan_1_1()).unwrap();
        assert_eq!(h.tertutup[0].koin, d("10"));
        assert_eq!(h.tertutup[0].idr_diterima, d("1100"));
        assert_eq!(h.tertutup[0].profit, d("100"));
        assert_eq!(h.penyesuaian[0].koin, d("5"));
        assert_eq!(h.penyesuaian[0].idr_diterima, d("550"));
        assert!(h.posisi.is_empty());
    }

    #[test]
    fn setelan_baru_hanya_untuk_posisi_baru() {
        let setelan = RiwayatSetelan::baru(vec![(0, d("1"), d("1")), (1500, d("2"), d("2"))]);
        let trades = vec![
            t("1", "b1", "aidr", true, "10", "1000", "0", 1000), // dibuka dengan 1%
            t("2", "b2", "aidr", true, "10", "1000", "0", 1600), // posisi sama, tetap 1%
            t("3", "b3", "bidr", true, "10", "1000", "0", 1700), // posisi baru, 2%
        ];
        let h = hitung(&trades, 0, &setelan).unwrap();
        let a = h.posisi.iter().find(|p| p.simbol == "aidr").unwrap();
        let b = h.posisi.iter().find(|p| p.simbol == "bidr").unwrap();
        assert_eq!(a.tp_persen, d("1"));
        assert_eq!(b.tp_persen, d("2"));
        assert_eq!(b.sl_persen, d("2"));
    }

    #[test]
    fn trade_sebelum_batas_waktu_diabaikan() {
        let trades = vec![
            t("1", "b1", "xidr", true, "10", "1000", "0", 500),
            t("2", "j1", "xidr", false, "10", "1100", "0", 2000),
        ];
        let h = hitung(&trades, 1000, &setelan_1_1()).unwrap();
        // beli sebelum batas tidak dihitung, jadi jual menjadi penyesuaian
        assert!(h.tertutup.is_empty());
        assert_eq!(h.penyesuaian.len(), 1);
    }

    #[test]
    fn koin_berbeda_tidak_tercampur() {
        let trades = vec![
            t("1", "b1", "aidr", true, "10", "1000", "0", 1000),
            t("2", "b2", "bidr", true, "5", "500", "0", 1100),
            t("3", "j1", "aidr", false, "10", "1100", "0", 1200),
        ];
        let h = hitung(&trades, 0, &setelan_1_1()).unwrap();
        assert_eq!(h.posisi.len(), 1);
        assert_eq!(h.posisi[0].simbol, "bidr");
        assert_eq!(h.tertutup[0].simbol, "aidr");
    }

    #[test]
    fn tanpa_setelan_menjadi_error() {
        let trades = vec![t("1", "b1", "xidr", true, "10", "1000", "0", 1000)];
        let kosong = RiwayatSetelan::baru(vec![]);
        assert!(matches!(hitung(&trades, 0, &kosong), Err(Error::SetelanBelumDiisi)));
    }

    #[test]
    fn periksa_persen_menolak_nilai_aneh() {
        assert!(periksa_persen(d("1"), d("1")).is_ok());
        assert!(periksa_persen(d("0"), d("1")).is_err());
        assert!(periksa_persen(d("1"), d("100")).is_err());
        assert!(periksa_persen(d("1"), d("-1")).is_err());
    }
}
