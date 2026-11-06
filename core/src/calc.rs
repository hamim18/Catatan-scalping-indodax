//! Perhitungan murni (tanpa jaringan). Semua uang memakai Decimal.
use crate::{Error, Hasil, Trade};
use rust_decimal::prelude::*;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sisi {
    Beli,
    Jual,
}

/// Gabungan semua pengisian (fill) milik satu order.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderRingkas {
    pub order_id: String,
    pub symbol: String,
    pub sisi: Sisi,
    pub jumlah_pengisian: usize,
    pub koin: Decimal,
    /// jumlah quoteQty (nilai trade, IDR)
    pub nilai: Decimal,
    /// jumlah commission (IDR)
    pub fee: Decimal,
    pub waktu_awal_ms: i64,
    pub waktu_akhir_ms: i64,
}

impl OrderRingkas {
    /// Beli: IDR yang benar-benar keluar = nilai + fee ("Order Terisi" di struk).
    pub fn idr_keluar(&self) -> Decimal {
        self.nilai + self.fee
    }
    /// Jual: IDR yang diterima = nilai - fee ("Total Diterima" di struk).
    pub fn idr_diterima(&self) -> Decimal {
        self.nilai - self.fee
    }
}

/// Kumpulkan per order. Anti-dobel: tradeId yang sama hanya dihitung sekali.
/// Fee selain IDR ditolak (belum pernah ditemui; jangan menebak).
pub fn gabung_per_order(trades: &[Trade]) -> Hasil<Vec<OrderRingkas>> {
    let mut terlihat: HashSet<&str> = HashSet::new();
    let mut hasil: Vec<OrderRingkas> = Vec::new();
    for t in trades {
        if !terlihat.insert(t.trade_id.as_str()) {
            continue;
        }
        if !t.commission_asset.eq_ignore_ascii_case("idr") {
            return Err(Error::FeeBukanIdr {
                trade_id: t.trade_id.clone(),
                aset: t.commission_asset.clone(),
            });
        }
        match hasil.iter_mut().find(|o| o.order_id == t.order_id) {
            Some(o) => {
                o.jumlah_pengisian += 1;
                o.koin += t.qty;
                o.nilai += t.quote_qty;
                o.fee += t.commission;
                o.waktu_awal_ms = o.waktu_awal_ms.min(t.time);
                o.waktu_akhir_ms = o.waktu_akhir_ms.max(t.time);
            }
            None => hasil.push(OrderRingkas {
                order_id: t.order_id.clone(),
                symbol: t.symbol.clone(),
                sisi: if t.is_buyer { Sisi::Beli } else { Sisi::Jual },
                jumlah_pengisian: 1,
                koin: t.qty,
                nilai: t.quote_qty,
                fee: t.commission,
                waktu_awal_ms: t.time,
                waktu_akhir_ms: t.time,
            }),
        }
    }
    hasil.sort_by_key(|o| o.waktu_awal_ms);
    Ok(hasil)
}

/// Posisi satu koin: total IDR keluar dan total koin (dirata-rata).
#[derive(Debug, Clone, PartialEq)]
pub struct Posisi {
    pub koin: Decimal,
    pub total_idr_keluar: Decimal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HasilJual {
    /// biaya modal untuk koin yang dijual (proporsional)
    pub modal_terjual: Decimal,
    pub idr_diterima: Decimal,
    pub profit: Decimal,
    /// persen terhadap modal_terjual
    pub persen: Decimal,
}

impl Posisi {
    pub fn dari_beli(o: &OrderRingkas) -> Hasil<Self> {
        if o.koin.is_zero() {
            return Err(Error::KoinNol);
        }
        Ok(Self { koin: o.koin, total_idr_keluar: o.idr_keluar() })
    }

    /// Beli tambahan pada koin yang sama: total digabung, modal efektif dirata-rata.
    pub fn tambah_beli(&mut self, o: &OrderRingkas) {
        self.koin += o.koin;
        self.total_idr_keluar += o.idr_keluar();
    }

    pub fn modal_efektif(&self) -> Hasil<Decimal> {
        if self.koin.is_zero() {
            return Err(Error::KoinNol);
        }
        Ok(self.total_idr_keluar / self.koin)
    }

    pub fn harga_tp(&self, persen: Decimal) -> Hasil<Decimal> {
        Ok(self.modal_efektif()? * (Decimal::ONE + persen / Decimal::ONE_HUNDRED))
    }

    pub fn harga_sl(&self, persen: Decimal) -> Hasil<Decimal> {
        Ok(self.modal_efektif()? * (Decimal::ONE - persen / Decimal::ONE_HUNDRED))
    }

    /// Jual sebagian atau penuh. Posisi dikurangi secara proporsional.
    pub fn jual(&mut self, koin_jual: Decimal, idr_diterima: Decimal) -> Hasil<HasilJual> {
        if self.koin.is_zero() {
            return Err(Error::KoinNol);
        }
        if koin_jual > self.koin {
            return Err(Error::JualMelebihiPosisi {
                jual: koin_jual.to_string(),
                posisi: self.koin.to_string(),
            });
        }
        // kali dulu baru bagi, supaya jual penuh tepat sama dengan total
        let modal_terjual = self.total_idr_keluar * koin_jual / self.koin;
        let profit = idr_diterima - modal_terjual;
        let persen = if modal_terjual.is_zero() {
            Decimal::ZERO
        } else {
            profit / modal_terjual * Decimal::ONE_HUNDRED
        };
        self.total_idr_keluar -= modal_terjual;
        self.koin -= koin_jual;
        Ok(HasilJual { modal_terjual, idr_diterima, profit, persen })
    }
}

/// Pembulatan untuk tampilan (rupiah utuh). Nilai asli tidak diubah.
pub fn bulatkan_idr(d: Decimal) -> Decimal {
    d.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_my_trades;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    // Angka beli dan jual ICNT 5 November dari PROGRESS.md (ID order dikarang).
    const JSON: &str = r#"{"data":[
      {"tradeId":"2","orderId":"icntidr-limit-JUAL","clientOrderId":"x","symbol":"icntidr","price":"4042","qty":"16.35667833","quoteQty":"66113","commission":"207","commissionAsset":"idr","isBuyer":false,"isMaker":true,"time":1793873222094},
      {"tradeId":"1","orderId":"icntidr-market-BELI","clientOrderId":"y","symbol":"icntidr","price":"3998","qty":"16.35667833","quoteQty":"65393","commission":"133","commissionAsset":"idr","isBuyer":true,"isMaker":false,"time":1793872445609}
    ]}"#;

    #[test]
    fn contoh_icnt_5_november_cocok_dengan_struk() {
        let trades = parse_my_trades(JSON).unwrap();
        let orders = gabung_per_order(&trades).unwrap();
        assert_eq!(orders.len(), 2);
        let (beli, jual) = (&orders[0], &orders[1]);
        assert_eq!(beli.sisi, Sisi::Beli);
        assert_eq!(beli.idr_keluar(), d("65526")); // "Order Terisi"
        assert_eq!(jual.idr_diterima(), d("65906")); // "Total Diterima"

        let mut pos = Posisi::dari_beli(beli).unwrap();
        assert_eq!(pos.modal_efektif().unwrap().round_dp(2), d("4006.07"));
        assert_eq!(bulatkan_idr(pos.harga_tp(d("1")).unwrap()), d("4046"));
        assert_eq!(bulatkan_idr(pos.harga_sl(d("1")).unwrap()), d("3966"));

        let h = pos.jual(jual.koin, jual.idr_diterima()).unwrap();
        assert_eq!(h.profit, d("380"));
        assert_eq!(h.persen.round_dp(2), d("0.58"));
        assert!(pos.koin.is_zero());
        assert!(pos.total_idr_keluar.is_zero());
    }

    #[test]
    fn order_terisi_beberapa_kali_dijumlahkan() {
        let mk = |id: &str, qty: &str, nilai: &str, fee: &str, t: i64| Trade {
            trade_id: id.into(),
            order_id: "o1".into(),
            symbol: "icntidr".into(),
            price: d("1"),
            qty: d(qty),
            quote_qty: d(nilai),
            commission: d(fee),
            commission_asset: "idr".into(),
            is_buyer: true,
            is_maker: true,
            time: t,
        };
        let trades = vec![mk("a", "19", "80000", "80", 10), mk("b", "1", "3500", "3", 20)];
        let o = gabung_per_order(&trades).unwrap();
        assert_eq!(o.len(), 1);
        assert_eq!(o[0].jumlah_pengisian, 2);
        assert_eq!(o[0].koin, d("20"));
        assert_eq!(o[0].idr_keluar(), d("83583"));
    }

    #[test]
    fn anti_dobel_berdasarkan_trade_id() {
        let mut trades = parse_my_trades(JSON).unwrap();
        let salinan = trades.clone();
        trades.extend(salinan);
        let o = gabung_per_order(&trades).unwrap();
        assert_eq!(o.len(), 2);
        assert_eq!(o[0].jumlah_pengisian, 1);
        assert_eq!(o[0].idr_keluar(), d("65526"));
    }

    #[test]
    fn fee_bukan_idr_ditolak() {
        let mut trades = parse_my_trades(JSON).unwrap();
        trades[0].commission_asset = "icnt".into();
        assert!(matches!(gabung_per_order(&trades), Err(Error::FeeBukanIdr { .. })));
    }

    #[test]
    fn beli_dua_kali_dirata_rata_lalu_jual_sebagian() {
        let mut pos = Posisi { koin: d("10"), total_idr_keluar: d("1000") };
        let o = OrderRingkas {
            order_id: "o".into(),
            symbol: "x".into(),
            sisi: Sisi::Beli,
            jumlah_pengisian: 1,
            koin: d("10"),
            nilai: d("1180"),
            fee: d("20"),
            waktu_awal_ms: 0,
            waktu_akhir_ms: 0,
        };
        pos.tambah_beli(&o);
        assert_eq!(pos.modal_efektif().unwrap(), d("110")); // (1000+1200)/20
        let h = pos.jual(d("5"), d("600")).unwrap();
        assert_eq!(h.modal_terjual, d("550"));
        assert_eq!(h.profit, d("50"));
        assert_eq!(pos.koin, d("15"));
        assert_eq!(pos.total_idr_keluar, d("1650"));
        assert!(pos.jual(d("99"), d("1")).is_err());
    }
}
