//! Periode WIB, target harian/mingguan dan kurva dari PROFIT TRADE TERTUTUP (T6.2, T6.4).
//!
//! Sejak T6.4 tidak ada lagi saldo versi aplikasi, snapshot harian/mingguan, modal aktif,
//! setoran bersih, atau koreksi saldo. Deposit dan withdraw tidak memengaruhi angka apa pun
//! di modul ini. Semua fungsi murni (tanpa jaringan dan tanpa database).
use rust_decimal::Decimal;

pub const MS_JAM: i64 = 3_600_000;
pub const MS_HARI: i64 = 86_400_000;
/// WIB = UTC+7 (tanpa DST).
pub const OFFSET_WIB_MS: i64 = 7 * MS_JAM;

/// Awal hari (00:00 WIB) yang memuat `ms`.
pub fn awal_hari_wib(ms: i64) -> i64 {
    (ms + OFFSET_WIB_MS).div_euclid(MS_HARI) * MS_HARI - OFFSET_WIB_MS
}

/// Awal minggu (Senin 00:00 WIB) yang memuat `ms`.
pub fn awal_minggu_wib(ms: i64) -> i64 {
    let hari = (ms + OFFSET_WIB_MS).div_euclid(MS_HARI);
    // 1 Jan 1970 hari Kamis; dengan Senin = 0, Kamis = 3
    let sejak_senin = (hari + 3).rem_euclid(7);
    (hari - sejak_senin) * MS_HARI - OFFSET_WIB_MS
}

// ---------------------------------------------------------------------------
// T6.2: target dan grafik dari PROFIT TRADE TERTUTUP (bukan dari selisih saldo).
// Tanpa jaringan dan tanpa snapshot: hanya daftar (waktu tutup, profit) dan total aset sekarang.
//   persen periode = profit periode / (total aset sekarang - profit periode) x 100
// Deposit dan withdraw tidak ikut dihitung. Periode WIB: hari mulai 00:00, minggu mulai Senin.
// Trade yang tutup tepat pada `mulai_ms` dihitung milik periode itu (>= mulai, <= sekarang).
// ---------------------------------------------------------------------------

/// Pencapaian satu periode dari profit trade tertutup.
#[derive(Debug, Clone, PartialEq)]
pub struct PencapaianProfit {
    pub mulai_ms: i64,
    /// jumlah profit trade yang tertutup dalam periode
    pub profit: Decimal,
    /// None bila total aset belum diketahui atau dasar hitung tidak positif
    pub persen: Option<Decimal>,
    /// None bila target belum diatur (target 0 dianggap belum diatur)
    pub target_persen: Option<Decimal>,
    /// persen / target x 100 (bisa negatif atau lebih dari 100)
    pub progress_persen: Option<Decimal>,
    pub tercapai: bool,
}

pub fn pencapaian_profit(
    profit_tertutup: &[(i64, Decimal)],
    mulai_ms: i64,
    sekarang_ms: i64,
    total_aset: Option<Decimal>,
    target: Option<Decimal>,
) -> PencapaianProfit {
    let profit: Decimal = profit_tertutup
        .iter()
        .filter(|(w, _)| *w >= mulai_ms && *w <= sekarang_ms)
        .map(|(_, p)| *p)
        .sum();
    let seratus = Decimal::from(100);
    let persen = total_aset.and_then(|total| {
        let dasar = total - profit;
        if dasar > Decimal::ZERO {
            Some(profit / dasar * seratus)
        } else {
            None
        }
    });
    let target_valid = target.filter(|t| *t > Decimal::ZERO);
    let progress_persen = match (persen, target_valid) {
        (Some(p), Some(t)) => Some(p / t * seratus),
        _ => None,
    };
    let tercapai = match (persen, target_valid) {
        (Some(p), Some(t)) => p >= t,
        _ => false,
    };
    PencapaianProfit { mulai_ms, profit, persen, target_persen: target_valid, progress_persen, tercapai }
}

/// Satu titik kurva akumulasi profit trade tertutup.
#[derive(Debug, Clone, PartialEq)]
pub struct TitikProfit {
    pub waktu_ms: i64,
    pub profit_kumulatif: Decimal,
}

/// Kurva akumulasi profit: satu titik di `awal_ms`, satu di tiap waktu tutup trade, satu di
/// `sekarang_ms`. Bila titik lebih dari `maks_titik`, dijarangkan (titik pertama dan terakhir
/// selalu ada); nilai tiap titik tetap tepat.
pub fn deret_profit(
    awal_ms: i64,
    sekarang_ms: i64,
    profit_tertutup: &[(i64, Decimal)],
    maks_titik: usize,
) -> Vec<TitikProfit> {
    let sekarang_ms = sekarang_ms.max(awal_ms);
    let mut waktu: Vec<i64> = vec![awal_ms, sekarang_ms];
    waktu.extend(profit_tertutup.iter().map(|(w, _)| *w));
    waktu.retain(|w| *w >= awal_ms && *w <= sekarang_ms);
    waktu.sort_unstable();
    waktu.dedup();
    let maks = maks_titik.max(2);
    if waktu.len() > maks {
        let langkah = (waktu.len() + maks - 1) / maks;
        let terakhir = waktu.len() - 1;
        waktu = waktu
            .iter()
            .enumerate()
            .filter(|(i, _)| i % langkah == 0 || *i == terakhir)
            .map(|(_, w)| *w)
            .collect();
    }
    waktu
        .into_iter()
        .map(|w| TitikProfit {
            waktu_ms: w,
            profit_kumulatif: profit_tertutup
                .iter()
                .filter(|(x, _)| *x >= awal_ms && *x <= w)
                .map(|(_, p)| *p)
                .sum(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Sabtu 7 Nov 2026 00:00 WIB dan Senin 2 Nov 2026 00:00 WIB
    const SABTU: i64 = 1_793_984_400_000;
    const SENIN: i64 = 1_793_552_400_000;

    #[test]
    fn awal_hari_wib_benar() {
        assert_eq!(awal_hari_wib(SABTU), SABTU);
        assert_eq!(awal_hari_wib(SABTU + 5 * MS_JAM), SABTU);
        assert_eq!(awal_hari_wib(SABTU + MS_HARI - 1), SABTU); // 23:59:59.999 WIB
        assert_eq!(awal_hari_wib(SABTU - 1), SABTU - MS_HARI); // 23:59:59.999 Jumat
    }

    #[test]
    fn hari_wib_berbeda_dari_utc() {
        // 17:30 UTC Jumat = 00:30 WIB Sabtu: di WIB sudah hari Sabtu
        assert_eq!(awal_hari_wib(SABTU + 30 * 60_000), SABTU);
    }

    #[test]
    fn awal_minggu_senin_wib() {
        assert_eq!(awal_minggu_wib(SENIN), SENIN);
        assert_eq!(awal_minggu_wib(SABTU + 5 * MS_JAM), SENIN);
        assert_eq!(awal_minggu_wib(SABTU + MS_HARI + 3 * MS_JAM), SENIN); // Minggu masih minggu itu
        assert_eq!(awal_minggu_wib(SENIN + 7 * MS_HARI), SENIN + 7 * MS_HARI);
        assert_eq!(awal_minggu_wib(SENIN - 1), SENIN - 7 * MS_HARI); // Minggu 23:59 minggu lalu
    }

}

#[cfg(test)]
mod tes_profit {
    use super::*;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    // Sabtu 7 Nov 2026 00:00 WIB
    const SABTU: i64 = 1_793_984_400_000;

    #[test]
    fn persen_adalah_profit_dibagi_total_aset_dikurangi_profit() {
        let p = vec![(10, d("15000"))];
        let r = pencapaian_profit(&p, 0, 100, Some(d("1015000")), Some(d("1.5")));
        assert_eq!(r.profit, d("15000"));
        assert_eq!(r.persen, Some(d("1.5")));
        assert_eq!(r.progress_persen, Some(d("100")));
        assert!(r.tercapai);
    }

    #[test]
    fn rugi_memberi_persen_dan_progress_negatif() {
        let p = vec![(10, d("-10000"))];
        let r = pencapaian_profit(&p, 0, 100, Some(d("990000")), Some(d("2")));
        assert_eq!(r.persen, Some(d("-1")));
        assert_eq!(r.progress_persen, Some(d("-50")));
        assert!(!r.tercapai);
    }

    #[test]
    fn batas_periode_trade_tepat_pada_awal_ikut_periode_baru() {
        let p = vec![(SABTU - 1, d("500")), (SABTU, d("380"))];
        let r = pencapaian_profit(&p, SABTU, SABTU + 12 * MS_JAM, None, None);
        assert_eq!(r.profit, d("380"));
        // trade setelah `sekarang` tidak ikut
        let r2 = pencapaian_profit(&p, SABTU - 10, SABTU - 1, None, None);
        assert_eq!(r2.profit, d("500"));
    }

    #[test]
    fn tanpa_total_aset_persen_kosong_tetapi_profit_ada() {
        let p = vec![(10, d("380"))];
        let r = pencapaian_profit(&p, 0, 100, None, Some(d("1.5")));
        assert_eq!(r.profit, d("380"));
        assert_eq!(r.persen, None);
        assert_eq!(r.progress_persen, None);
        assert!(!r.tercapai);
    }

    #[test]
    fn dasar_hitung_nol_atau_negatif_tidak_membagi() {
        let p = vec![(10, d("100"))];
        assert_eq!(pencapaian_profit(&p, 0, 100, Some(d("100")), None).persen, None);
        assert_eq!(pencapaian_profit(&p, 0, 100, Some(d("50")), None).persen, None);
    }

    #[test]
    fn target_nol_atau_kosong_dianggap_belum_diatur() {
        let p = vec![(10, d("100"))];
        let r = pencapaian_profit(&p, 0, 100, Some(d("10100")), Some(Decimal::ZERO));
        assert_eq!(r.target_persen, None);
        assert_eq!(r.progress_persen, None);
        assert!(!r.tercapai);
        assert_eq!(pencapaian_profit(&p, 0, 100, Some(d("10100")), None).target_persen, None);
    }

    #[test]
    fn deret_profit_akumulasi() {
        let p = vec![(10, d("5")), (20, d("-2"))];
        let t = deret_profit(0, 30, &p, 100);
        let waktu: Vec<i64> = t.iter().map(|x| x.waktu_ms).collect();
        assert_eq!(waktu, vec![0, 10, 20, 30]);
        let nilai: Vec<Decimal> = t.iter().map(|x| x.profit_kumulatif).collect();
        assert_eq!(nilai, vec![d("0"), d("5"), d("3"), d("3")]);
    }

    #[test]
    fn deret_profit_tanpa_trade_dua_titik_nol() {
        let t = deret_profit(5, 50, &[], 100);
        assert_eq!(t.len(), 2);
        assert!(t.iter().all(|x| x.profit_kumulatif == Decimal::ZERO));
        assert_eq!(deret_profit(5, 5, &[], 100).len(), 1);
    }

    #[test]
    fn deret_profit_dijarangkan_tetapi_ujung_dan_total_tepat() {
        let p: Vec<(i64, Decimal)> = (1..=50).map(|i| (i * 10, d("1"))).collect();
        let t = deret_profit(0, 1000, &p, 10);
        assert!(t.len() <= 12);
        assert_eq!(t.first().unwrap().waktu_ms, 0);
        assert_eq!(t.last().unwrap().waktu_ms, 1000);
        assert_eq!(t.last().unwrap().profit_kumulatif, d("50"));
    }
}
