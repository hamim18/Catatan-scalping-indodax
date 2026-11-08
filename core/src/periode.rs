//! Periode WIB, saldo versi aplikasi, pencapaian dan progress target (T2.3, FR-19 sampai FR-21).
//!
//! Saldo versi aplikasi = saldo IDR + nilai open posisi pada modal efektif (PRD bagian 5).
//! Karena membeli koin hanya memindahkan IDR ke posisi (nilai tetap), saldo itu hanya berubah
//! oleh (1) profit/loss trade yang ditutup dan (2) deposit/withdraw. Maka saldo pada waktu t
//! dihitung dari CATATAN, tanpa memanggil jaringan:
//!   saldo(t) = saldo awal + arus kas bersih (awal, t] + profit trade tertutup (awal, t]
//! Ini sejalan dengan prinsip PRD "data disimpan sebagai catatan": mengoreksi satu catatan
//! menghitung ulang semuanya, dan snapshot awal hari/minggu tetap tepat walau layanan mati.
use crate::arus_kas::{setoran_bersih, ArusKas};
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

#[derive(Debug, Clone, PartialEq)]
pub struct Pencapaian {
    /// awal periode yang dipakai (bila periode dimulai sebelum snapshot awal, pakai waktu snapshot)
    pub mulai_ms: i64,
    pub saldo_awal_periode: Decimal,
    pub saldo_sekarang: Decimal,
    pub setoran_bersih: Decimal,
    /// saldo sekarang - saldo awal periode - setoran bersih
    pub profit: Decimal,
    /// profit / saldo awal periode x 100
    pub persen: Decimal,
    pub target_persen: Option<Decimal>,
    /// persen / target x 100 (bisa negatif atau lebih dari 100)
    pub progress_persen: Option<Decimal>,
    pub tercapai: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RingkasanTarget {
    pub snapshot_awal_ms: i64,
    pub saldo_awal: Decimal,
    pub saldo_sekarang: Decimal,
    /// saldo awal + setoran bersih sejak aplikasi dipasang
    pub modal_aktif: Decimal,
    pub harian: Pencapaian,
    pub mingguan: Pencapaian,
}

/// Bahan hitung. Semua milik sendiri supaya mudah diduplikasi (mis. "tanpa trade ini").
#[derive(Debug, Clone)]
pub struct DataSaldo {
    pub snapshot_awal_ms: i64,
    pub saldo_awal: Decimal,
    /// (waktu tutup ms, profit bersih) tiap trade tertutup
    pub profit_tertutup: Vec<(i64, Decimal)>,
    pub arus_kas: Vec<ArusKas>,
    pub amount_sudah_bersih: bool,
    /// (waktu ms, jumlah) koreksi saldo yang dicatat pengguna (T3.3). Diperlakukan seperti
    /// setoran: mengubah saldo dan modal aktif tetapi BUKAN profit, jadi tidak masuk pencapaian.
    pub penyesuaian: Vec<(i64, Decimal)>,
}

/// Satu titik grafik (FR-22): saldo versi aplikasi dan modal aktif pada `waktu_ms`.
#[derive(Debug, Clone, PartialEq)]
pub struct TitikGrafik {
    pub waktu_ms: i64,
    pub saldo: Decimal,
    pub modal_aktif: Decimal,
}

/// Perbandingan saldo aplikasi dengan saldo asli Indodax (T3.3).
#[derive(Debug, Clone, PartialEq)]
pub struct SelisihSaldo {
    /// saldo IDR asli dari /account (tersedia + terkunci)
    pub idr_asli: Decimal,
    /// saldo IDR yang diharapkan menurut catatan aplikasi
    pub idr_menurut_aplikasi: Decimal,
    /// modal posisi terbuka (pada modal efektif)
    pub nilai_posisi: Decimal,
    pub saldo_aplikasi: Decimal,
    /// idr_asli - idr_menurut_aplikasi. Positif: IDR asli lebih besar dari catatan.
    pub selisih: Decimal,
}

impl DataSaldo {
    fn profit_antara(&self, setelah_ms: i64, sampai_ms: i64) -> Decimal {
        self.profit_tertutup
            .iter()
            .filter(|(w, _)| *w > setelah_ms && *w <= sampai_ms)
            .map(|(_, p)| *p)
            .sum::<Decimal>()
    }

    fn arus_antara(&self, setelah_ms: i64, sampai_ms: i64) -> Decimal {
        let koreksi: Decimal = self
            .penyesuaian
            .iter()
            .filter(|(w, _)| *w > setelah_ms && *w <= sampai_ms)
            .map(|(_, j)| *j)
            .sum::<Decimal>();
        setoran_bersih(&self.arus_kas, setelah_ms, sampai_ms, self.amount_sudah_bersih) + koreksi
    }

    /// Saldo versi aplikasi pada waktu `t`. Sebelum snapshot awal = saldo awal.
    pub fn saldo_pada(&self, t: i64) -> Decimal {
        let t = t.max(self.snapshot_awal_ms);
        self.saldo_awal + self.arus_antara(self.snapshot_awal_ms, t) + self.profit_antara(self.snapshot_awal_ms, t)
    }

    /// Modal aktif pada `t` = saldo awal + setoran bersih sejak snapshot awal.
    pub fn modal_aktif_pada(&self, t: i64) -> Decimal {
        let t = t.max(self.snapshot_awal_ms);
        self.saldo_awal + self.arus_antara(self.snapshot_awal_ms, t)
    }

    /// Pencapaian periode yang dimulai `mulai_ms` sampai `sekarang_ms` (FR-20), dengan rumus PRD:
    /// (saldo sekarang - saldo awal periode - setoran bersih periode) / saldo awal periode x 100.
    pub fn pencapaian(&self, mulai_ms: i64, sekarang_ms: i64, target: Option<Decimal>) -> Pencapaian {
        let mulai = mulai_ms.max(self.snapshot_awal_ms);
        let sekarang_ms = sekarang_ms.max(mulai);
        let saldo_awal_periode = self.saldo_pada(mulai);
        let saldo_sekarang = self.saldo_pada(sekarang_ms);
        let setoran = self.arus_antara(mulai, sekarang_ms);
        let profit = saldo_sekarang - saldo_awal_periode - setoran;
        let seratus = Decimal::from(100);
        let persen = if saldo_awal_periode > Decimal::ZERO { profit / saldo_awal_periode * seratus } else { Decimal::ZERO };
        let target_valid = target.filter(|t| *t > Decimal::ZERO);
        Pencapaian {
            mulai_ms: mulai,
            saldo_awal_periode,
            saldo_sekarang,
            setoran_bersih: setoran,
            profit,
            persen,
            target_persen: target_valid,
            progress_persen: target_valid.map(|t| persen / t * seratus),
            tercapai: target_valid.map_or(false, |t| persen >= t),
        }
    }

    /// Deret saldo dan modal aktif untuk grafik (FR-22): satu titik di snapshot awal, satu di tiap
    /// kejadian (trade tertutup, arus kas, koreksi) dan satu di `sekarang_ms`. Jarak antara kedua
    /// garis = profit. Bila titik lebih dari `maks_titik`, titik dijarangkan (titik pertama dan
    /// terakhir selalu ada); nilai tiap titik tetap tepat.
    pub fn deret(&self, sekarang_ms: i64, maks_titik: usize) -> Vec<TitikGrafik> {
        let sekarang_ms = sekarang_ms.max(self.snapshot_awal_ms);
        let mut waktu: Vec<i64> = vec![self.snapshot_awal_ms, sekarang_ms];
        waktu.extend(self.profit_tertutup.iter().map(|(w, _)| *w));
        waktu.extend(self.arus_kas.iter().map(|a| a.waktu_ms));
        waktu.extend(self.penyesuaian.iter().map(|(w, _)| *w));
        waktu.retain(|w| *w >= self.snapshot_awal_ms && *w <= sekarang_ms);
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
            .map(|w| TitikGrafik { waktu_ms: w, saldo: self.saldo_pada(w), modal_aktif: self.modal_aktif_pada(w) })
            .collect()
    }

    /// Bandingkan dengan saldo IDR asli. `nilai_posisi` = jumlah modal posisi terbuka.
    /// IDR yang diharapkan = saldo aplikasi - nilai posisi (posisi dinilai pada modal efektif).
    pub fn selisih_saldo(&self, idr_asli: Decimal, nilai_posisi: Decimal, sekarang_ms: i64) -> SelisihSaldo {
        let saldo_aplikasi = self.saldo_pada(sekarang_ms);
        let idr_menurut_aplikasi = saldo_aplikasi - nilai_posisi;
        SelisihSaldo {
            idr_asli,
            idr_menurut_aplikasi,
            nilai_posisi,
            saldo_aplikasi,
            selisih: idr_asli - idr_menurut_aplikasi,
        }
    }

    /// Ringkasan harian dan mingguan (WIB, minggu mulai Senin). `target` = (harian %, mingguan %).
    pub fn ringkasan(&self, sekarang_ms: i64, target: (Option<Decimal>, Option<Decimal>)) -> RingkasanTarget {
        let sekarang_ms = sekarang_ms.max(self.snapshot_awal_ms);
        RingkasanTarget {
            snapshot_awal_ms: self.snapshot_awal_ms,
            saldo_awal: self.saldo_awal,
            saldo_sekarang: self.saldo_pada(sekarang_ms),
            modal_aktif: self.modal_aktif_pada(sekarang_ms),
            harian: self.pencapaian(awal_hari_wib(sekarang_ms), sekarang_ms, target.0),
            mingguan: self.pencapaian(awal_minggu_wib(sekarang_ms), sekarang_ms, target.1),
        }
    }
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
    use crate::arus_kas::JenisArusKas;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    // Sabtu 7 Nov 2026 00:00 WIB dan Senin 2 Nov 2026 00:00 WIB
    const SABTU: i64 = 1_793_984_400_000;
    const SENIN: i64 = 1_793_552_400_000;

    fn deposit(id: &str, jumlah: &str, w: i64) -> ArusKas {
        ArusKas { id_sumber: id.into(), jenis: JenisArusKas::Deposit, jumlah: d(jumlah), biaya: Decimal::ZERO, waktu_ms: w }
    }

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

    #[test]
    fn saldo_pada_menjumlah_profit_dan_arus_kas() {
        let data = DataSaldo {
            snapshot_awal_ms: 1000, penyesuaian: vec![],
            saldo_awal: d("1000000"),
            profit_tertutup: vec![(2000, d("380")), (3000, d("-100"))],
            arus_kas: vec![deposit("d1", "100000", 2500)],
            amount_sudah_bersih: true,
        };
        assert_eq!(data.saldo_pada(1000), d("1000000"));
        assert_eq!(data.saldo_pada(500), d("1000000")); // sebelum snapshot
        assert_eq!(data.saldo_pada(2000), d("1000380"));
        assert_eq!(data.saldo_pada(3000), d("1100280"));
        assert_eq!(data.modal_aktif_pada(3000), d("1100000"));
    }

    #[test]
    fn pencapaian_mengeluarkan_setoran() {
        let data = DataSaldo {
            snapshot_awal_ms: 1000, penyesuaian: vec![],
            saldo_awal: d("1000000"),
            profit_tertutup: vec![(2000, d("380")), (3000, d("-100"))],
            arus_kas: vec![deposit("d1", "100000", 2500)],
            amount_sudah_bersih: true,
        };
        let p = data.pencapaian(1500, 3000, Some(d("1.5")));
        assert_eq!(p.saldo_awal_periode, d("1000000"));
        assert_eq!(p.saldo_sekarang, d("1100280"));
        assert_eq!(p.setoran_bersih, d("100000"));
        assert_eq!(p.profit, d("280"));
        assert_eq!(p.persen, d("0.028"));
        assert!(!p.tercapai);
    }

    #[test]
    fn progress_dan_tercapai() {
        let data = DataSaldo {
            snapshot_awal_ms: 0, penyesuaian: vec![],
            saldo_awal: d("1000000"),
            profit_tertutup: vec![(10, d("15000"))],
            arus_kas: vec![],
            amount_sudah_bersih: true,
        };
        let p = data.pencapaian(0, 100, Some(d("1.5")));
        assert_eq!(p.persen, d("1.5"));
        assert_eq!(p.progress_persen, Some(d("100")));
        assert!(p.tercapai);
        let setengah = data.pencapaian(0, 100, Some(d("3")));
        assert_eq!(setengah.progress_persen, Some(d("50")));
        assert!(!setengah.tercapai);
        let tanpa = data.pencapaian(0, 100, None);
        assert_eq!(tanpa.progress_persen, None);
        assert!(!tanpa.tercapai);
        let nol = data.pencapaian(0, 100, Some(Decimal::ZERO));
        assert_eq!(nol.target_persen, None); // target 0 dianggap belum diatur
    }

    #[test]
    fn rugi_memberi_persen_dan_progress_negatif() {
        let data = DataSaldo {
            snapshot_awal_ms: 0, penyesuaian: vec![],
            saldo_awal: d("1000000"),
            profit_tertutup: vec![(10, d("-10000"))],
            arus_kas: vec![],
            amount_sudah_bersih: true,
        };
        let p = data.pencapaian(0, 100, Some(d("2")));
        assert_eq!(p.persen, d("-1"));
        assert_eq!(p.progress_persen, Some(d("-50")));
        assert!(!p.tercapai);
    }

    #[test]
    fn saldo_awal_nol_tidak_membagi_nol() {
        let data = DataSaldo { snapshot_awal_ms: 0, penyesuaian: vec![], saldo_awal: Decimal::ZERO, profit_tertutup: vec![(10, d("100"))], arus_kas: vec![], amount_sudah_bersih: true };
        let p = data.pencapaian(0, 100, Some(d("1")));
        assert_eq!(p.persen, Decimal::ZERO);
    }

    #[test]
    fn ringkasan_harian_dan_mingguan_wib() {
        let awal = SABTU - 20 * MS_JAM; // Jumat 04:00 WIB
        let data = DataSaldo {
            snapshot_awal_ms: awal, penyesuaian: vec![],
            saldo_awal: d("1000000"),
            profit_tertutup: vec![(awal + MS_JAM, d("500")), (SABTU + 10 * MS_JAM, d("380"))],
            arus_kas: vec![deposit("d1", "100000", SABTU + 11 * MS_JAM)],
            amount_sudah_bersih: true,
        };
        let r = data.ringkasan(SABTU + 12 * MS_JAM, (Some(d("1.5")), Some(d("5"))));
        // harian: hanya trade Sabtu; setoran dikeluarkan
        assert_eq!(r.harian.mulai_ms, SABTU);
        assert_eq!(r.harian.saldo_awal_periode, d("1000500"));
        assert_eq!(r.harian.profit, d("380"));
        // mingguan dimulai Senin, sebelum snapshot, jadi dipakai waktu snapshot
        assert_eq!(r.mingguan.mulai_ms, awal);
        assert_eq!(r.mingguan.profit, d("880"));
        assert_eq!(r.saldo_sekarang, d("1100880"));
        assert_eq!(r.modal_aktif, d("1100000"));
    }

    fn data_koreksi() -> DataSaldo {
        DataSaldo {
            snapshot_awal_ms: 1000,
            penyesuaian: vec![(2500, d("-300"))],
            saldo_awal: d("1000000"),
            profit_tertutup: vec![(2000, d("380"))],
            arus_kas: vec![deposit("d1", "100000", 3000)],
            amount_sudah_bersih: true,
        }
    }

    #[test]
    fn koreksi_mengubah_saldo_dan_modal_tetapi_bukan_profit() {
        let data = data_koreksi();
        assert_eq!(data.saldo_pada(2400), d("1000380"));
        assert_eq!(data.saldo_pada(2500), d("1000080"));
        assert_eq!(data.modal_aktif_pada(2500), d("999700"));
        // selisih saldo - modal tetap = profit (380)
        assert_eq!(data.saldo_pada(4000) - data.modal_aktif_pada(4000), d("380"));
        let p = data.pencapaian(1000, 4000, None);
        assert_eq!(p.profit, d("380"));
        assert_eq!(p.setoran_bersih, d("99700")); // deposit 100000 + koreksi -300
    }

    #[test]
    fn deret_grafik_dua_garis() {
        let data = data_koreksi();
        let t = data.deret(4000, 100);
        let waktu: Vec<i64> = t.iter().map(|x| x.waktu_ms).collect();
        assert_eq!(waktu, vec![1000, 2000, 2500, 3000, 4000]);
        assert_eq!(t[0].saldo, d("1000000"));
        assert_eq!(t[0].modal_aktif, d("1000000"));
        assert_eq!(t[4].saldo, d("1100080"));
        assert_eq!(t[4].modal_aktif, d("1099700"));
        for x in &t {
            assert_eq!(x.saldo - x.modal_aktif, if x.waktu_ms >= 2000 { d("380") } else { Decimal::ZERO });
        }
    }

    #[test]
    fn deret_dijarangkan_tetapi_ujung_tetap_ada() {
        let profit: Vec<(i64, Decimal)> = (1..=50).map(|i| (i * 10, d("1"))).collect();
        let data = DataSaldo { snapshot_awal_ms: 0, penyesuaian: vec![], saldo_awal: d("100"), profit_tertutup: profit, arus_kas: vec![], amount_sudah_bersih: true };
        let t = data.deret(1000, 10);
        assert!(t.len() <= 12);
        assert_eq!(t.first().unwrap().waktu_ms, 0);
        assert_eq!(t.last().unwrap().waktu_ms, 1000);
        assert_eq!(t.last().unwrap().saldo, d("150"));
    }

    #[test]
    fn deret_tanpa_kejadian_dua_titik() {
        let data = DataSaldo { snapshot_awal_ms: 5, penyesuaian: vec![], saldo_awal: d("10"), profit_tertutup: vec![], arus_kas: vec![], amount_sudah_bersih: true };
        let t = data.deret(5, 100);
        assert_eq!(t.len(), 1); // awal dan sekarang sama
        assert_eq!(data.deret(50, 100).len(), 2);
    }

    #[test]
    fn selisih_saldo_dengan_posisi_terbuka() {
        // saldo aplikasi 1.000.380 = IDR 940.380 + posisi 60.000
        let data = DataSaldo { snapshot_awal_ms: 0, penyesuaian: vec![], saldo_awal: d("1000000"), profit_tertutup: vec![(10, d("380"))], arus_kas: vec![], amount_sudah_bersih: true };
        let s = data.selisih_saldo(d("940400"), d("60000"), 100);
        assert_eq!(s.saldo_aplikasi, d("1000380"));
        assert_eq!(s.idr_menurut_aplikasi, d("940380"));
        assert_eq!(s.selisih, d("20"));
        let cocok = data.selisih_saldo(d("940380"), d("60000"), 100);
        assert_eq!(cocok.selisih, Decimal::ZERO);
    }

    #[test]
    fn fee_deposit_tidak_mengubah_saldo_fee_withdraw_sesuai_mode() {
        // deposit: amount sudah bersih (fee hanya informasi), modenya tidak berpengaruh
        let mut a = deposit("d1", "50000", 5);
        a.biaya = d("2500");
        let data = |bersih| DataSaldo { snapshot_awal_ms: 0, penyesuaian: vec![], saldo_awal: d("1000"), profit_tertutup: vec![], arus_kas: vec![a.clone()], amount_sudah_bersih: bersih };
        assert_eq!(data(true).saldo_pada(10), d("51000"));
        assert_eq!(data(false).saldo_pada(10), d("51000"));
        // withdraw: masih dua mode (belum terverifikasi)
        let w = ArusKas { id_sumber: "withdraw:1".into(), jenis: JenisArusKas::Withdraw, jumlah: d("20000"), biaya: d("6500"), waktu_ms: 5 };
        let dw = |bersih| DataSaldo { snapshot_awal_ms: 0, penyesuaian: vec![], saldo_awal: d("100000"), profit_tertutup: vec![], arus_kas: vec![w.clone()], amount_sudah_bersih: bersih };
        assert_eq!(dw(true).saldo_pada(10), d("80000"));
        assert_eq!(dw(false).saldo_pada(10), d("73500"));
    }

    #[test]
    fn deposit_nyata_8_nov_saldo_cocok_dengan_saldo_asli() {
        // saldo aplikasi 69.652 + deposit (amount 7.670, fee 3.330) = saldo asli 77.322
        let mut a = deposit("d-nyata", "7670", 5);
        a.biaya = d("3330");
        let data = DataSaldo { snapshot_awal_ms: 0, penyesuaian: vec![], saldo_awal: d("69652"), profit_tertutup: vec![], arus_kas: vec![a], amount_sudah_bersih: false };
        assert_eq!(data.saldo_pada(10), d("77322"));
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
