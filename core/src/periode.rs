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
        setoran_bersih(&self.arus_kas, setelah_ms, sampai_ms, self.amount_sudah_bersih)
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
            snapshot_awal_ms: 1000,
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
            snapshot_awal_ms: 1000,
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
            snapshot_awal_ms: 0,
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
            snapshot_awal_ms: 0,
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
        let data = DataSaldo { snapshot_awal_ms: 0, saldo_awal: Decimal::ZERO, profit_tertutup: vec![(10, d("100"))], arus_kas: vec![], amount_sudah_bersih: true };
        let p = data.pencapaian(0, 100, Some(d("1")));
        assert_eq!(p.persen, Decimal::ZERO);
    }

    #[test]
    fn ringkasan_harian_dan_mingguan_wib() {
        let awal = SABTU - 20 * MS_JAM; // Jumat 04:00 WIB
        let data = DataSaldo {
            snapshot_awal_ms: awal,
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

    #[test]
    fn fee_deposit_mengubah_saldo_sesuai_mode() {
        let mut a = deposit("d1", "50000", 5);
        a.biaya = d("2500");
        let data = |bersih| DataSaldo { snapshot_awal_ms: 0, saldo_awal: d("1000"), profit_tertutup: vec![], arus_kas: vec![a.clone()], amount_sudah_bersih: bersih };
        assert_eq!(data(true).saldo_pada(10), d("51000"));
        assert_eq!(data(false).saldo_pada(10), d("48500"));
    }
}
