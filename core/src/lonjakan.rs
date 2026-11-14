//! T9.4: filter Lonjakan di layar Screener. HANYA informasi: tidak menyentuh posisi, profit,
//! target, grafik, statistik, maupun database trade. Modul ini MURNI (tanpa jaringan dan tanpa
//! database): snapshot harga dan data pasar diberikan pemanggil; modul ini yang menghitung harga
//! dasar, pemicu, dan arah (Decimal). Tanpa notifikasi dan tanpa layanan latar (dibatalkan 14 Nov 2026).
//!
//! Alur: (1) tiap polling ringkasan pasar: `cari_pemicu` membandingkan `last` sekarang dengan harga
//! dasar (rata-rata berbobot waktu `last` dalam `dasar_menit` terakhir) dan memilih pair yang sudah
//! melewati separuh ambang X; (2) pemanggil mengambil `trades` untuk pair itu; (3) `nilai_lonjakan`
//! menerapkan syarat trx/mnt jendela pendek (M), spread, usia transaksi, dan ambang X penuh.
//! Setelah itu pemanggil memanggil `RiwayatHarga::tambah` (titik sekarang TIDAK ikut dasar).
//!
//! Riwayat hanya di memori. Titik lebih tua dari 60 menit dibuang. Selisih waktu antar dua titik
//! lebih dari `CELAH_DETIK` dianggap celah (misalnya layar sempat ditutup) dan tidak dihitung sama
//! sekali, jadi harga lama tidak "dipaksa" menutupi celah itu.
use crate::screener::{
    nilai_tahap2, saring_tahap1, BarisScreener, KandidatScreener, KonfigScreener, RingkasanPasar, TransaksiPublik,
};
use rust_decimal::Decimal;
use std::cmp::Ordering;
use std::collections::{BTreeMap, VecDeque};

/// Titik yang lebih tua dari ini (detik) dibuang.
pub const MAKS_RIWAYAT_DETIK: i64 = 3600;
/// Selisih dua titik berurutan di atas ini (detik) dianggap celah.
pub const CELAH_DETIK: i64 = 180;

/// Ambang Lonjakan. Nilai awal sementara; angka final setelah uji lapangan (PRD pertanyaan 19).
#[derive(Debug, Clone, PartialEq)]
pub struct KonfigLonjakan {
    /// tampilkan pair yang harganya di atas harga dasar (▲)
    pub naik: bool,
    /// tampilkan pair yang harganya di bawah harga dasar (▼)
    pub turun: bool,
    /// X: penyimpangan minimum dari harga dasar, dalam persen
    pub ambang_persen: Decimal,
    /// M: trx/mnt minimum di jendela pendek
    pub trx_mnt_min: Decimal,
    /// jendela pendek untuk trx/mnt, menit
    pub jendela_menit: u32,
    /// lebar rata-rata harga dasar, menit (5 sampai 60)
    pub dasar_menit: u32,
    /// data dasar minimal (menit tercakup) sebelum sebuah pair dinilai
    pub pemanasan_menit: u32,
    /// maksimal pair yang diperiksa lewat `trades` per siklus
    pub maks_periksa: u32,
    /// jarak polling ringkasan pasar, detik
    pub interval_detik: u32,
}

impl Default for KonfigLonjakan {
    fn default() -> Self {
        Self {
            naik: true,
            turun: true,
            ambang_persen: Decimal::from(2_i64),
            trx_mnt_min: Decimal::from(3_i64),
            jendela_menit: 3,
            dasar_menit: 60,
            pemanasan_menit: 10,
            maks_periksa: 15,
            interval_detik: 20,
        }
    }
}

// ---------------------------------------------------------------------------
// Riwayat harga (hanya memori)
// ---------------------------------------------------------------------------

/// Harga dasar sebuah pair beserta lama data yang benar-benar tercakup.
#[derive(Debug, Clone, PartialEq)]
pub struct DasarHarga {
    pub harga: Decimal,
    /// jumlah detik yang tercakup (celah tidak dihitung)
    pub lama_detik: i64,
}

/// Titik `last` per pair beserta waktunya (epoch detik).
#[derive(Debug, Default, Clone)]
pub struct RiwayatHarga {
    titik: BTreeMap<String, VecDeque<(i64, Decimal)>>,
}

impl RiwayatHarga {
    pub fn baru() -> Self {
        Self::default()
    }

    pub fn kosongkan(&mut self) {
        self.titik.clear();
    }

    /// Tambah satu titik per pair dari ringkasan pasar pada `waktu`. Titik dengan waktu yang tidak
    /// lebih baru dari titik terakhir pair itu dilewati (server_time bisa sama di dua polling).
    /// Titik lebih tua dari `MAKS_RIWAYAT_DETIK` dibuang.
    pub fn tambah(&mut self, waktu: i64, data: &[RingkasanPasar]) {
        for r in data {
            let antrean = self.titik.entry(r.pair.clone()).or_default();
            if let Some(&(w, _)) = antrean.back() {
                if waktu <= w {
                    continue;
                }
            }
            antrean.push_back((waktu, r.last));
            while let Some(&(w, _)) = antrean.front() {
                if w < waktu - MAKS_RIWAYAT_DETIK {
                    antrean.pop_front();
                } else {
                    break;
                }
            }
        }
    }

    /// Jarak dari titik tertua seluruh riwayat sampai `sekarang`, detik (0 bila kosong).
    pub fn rentang_detik(&self, sekarang: i64) -> i64 {
        self.titik
            .values()
            .filter_map(|d| d.front().map(|&(w, _)| w))
            .min()
            .map(|w| (sekarang - w).max(0))
            .unwrap_or(0)
    }

    /// Harga dasar = rata-rata harga berbobot waktu (trapesium) dari segmen antar dua titik berurutan
    /// yang seluruhnya berada dalam `dasar_menit` terakhir. Segmen lebih panjang dari `CELAH_DETIK`
    /// dilewati. None bila pair tidak dikenal atau data tercakup kurang dari `pemanasan_menit`.
    pub fn dasar(&self, pair: &str, sekarang: i64, dasar_menit: u32, pemanasan_menit: u32) -> Option<DasarHarga> {
        let antrean = self.titik.get(pair)?;
        let awal = sekarang - i64::from(dasar_menit.max(1)) * 60;
        let dua = Decimal::from(2_i64);
        let mut total_detik: i64 = 0;
        let mut jumlah = Decimal::ZERO;
        let mut sebelumnya: Option<(i64, Decimal)> = None;
        for &(w, h) in antrean.iter() {
            if let Some((w0, h0)) = sebelumnya {
                let dt = w - w0;
                if w0 >= awal && w <= sekarang && dt > 0 && dt <= CELAH_DETIK {
                    let rata = h0.checked_add(h)?.checked_div(dua)?;
                    jumlah = jumlah.checked_add(rata.checked_mul(Decimal::from(dt))?)?;
                    total_detik += dt;
                }
            }
            sebelumnya = Some((w, h));
        }
        if total_detik <= 0 || total_detik < i64::from(pemanasan_menit) * 60 {
            return None;
        }
        let harga = jumlah.checked_div(Decimal::from(total_detik))?;
        if harga <= Decimal::ZERO {
            return None;
        }
        Some(DasarHarga { harga, lama_detik: total_detik })
    }
}

// ---------------------------------------------------------------------------
// Pemicu (tahap 1) dan penilaian (tahap 2)
// ---------------------------------------------------------------------------

/// Konfigurasi Screener yang dipakai untuk Lonjakan: syarat dasar (harga, volume, spread, usia
/// transaksi) tetap berlaku; range 24 jam dan filter tambahan dimatikan; trx/mnt dan jendela
/// diganti milik Lonjakan.
pub fn konfig_saring(ks: &KonfigScreener, kl: &KonfigLonjakan) -> KonfigScreener {
    let mut k = ks.clone();
    k.range24_min = None;
    k.trx_mnt_min = Some(kl.trx_mnt_min);
    k.jendela_menit = kl.jendela_menit.max(1);
    k.gerak_min_persen = None;
    k.menit_aktif_min_persen = None;
    k.arah_min_persen = None;
    k.kedalaman_kelipatan = None;
    k
}

/// (harga - dasar) / dasar x 100. None bila dasar tidak positif atau terjadi luapan.
fn perubahan_persen(harga: Decimal, dasar: Decimal) -> Option<Decimal> {
    if dasar <= Decimal::ZERO {
        return None;
    }
    (harga - dasar).checked_mul(Decimal::from(100_i64))?.checked_div(dasar)
}

fn arah_diizinkan(kl: &KonfigLonjakan, perubahan: Decimal) -> bool {
    (perubahan > Decimal::ZERO && kl.naik) || (perubahan < Decimal::ZERO && kl.turun)
}

#[derive(Debug, Clone, PartialEq)]
pub struct PemicuLonjakan {
    pub kandidat: KandidatScreener,
    pub dasar: Decimal,
    /// persen, boleh negatif
    pub perubahan: Decimal,
    pub lama_dasar_detik: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HasilPemicu {
    /// pair yang lolos pemicu, penyimpangan terbesar lebih dulu, dipotong `maks_periksa`
    pub pemicu: Vec<PemicuLonjakan>,
    /// jumlah pair yang lolos pemicu sebelum dipotong
    pub jumlah_total: usize,
    /// kandidat tahap 1 yang sudah punya harga dasar (lewat masa pemanasan)
    pub jumlah_punya_dasar: usize,
}

/// Tahap 1 Lonjakan dari satu ringkasan pasar. Panggil SEBELUM `RiwayatHarga::tambah` untuk
/// ringkasan yang sama. Pair yang sudah melewati separuh ambang X diteruskan ke tahap 2.
pub fn cari_pemicu(
    ks: &KonfigScreener,
    kl: &KonfigLonjakan,
    riwayat: &RiwayatHarga,
    data: &[RingkasanPasar],
    sekarang: i64,
) -> HasilPemicu {
    let k = konfig_saring(ks, kl);
    let separuh = kl.ambang_persen.checked_div(Decimal::from(2_i64)).unwrap_or(kl.ambang_persen);
    let mut jumlah_punya_dasar = 0_usize;
    let mut pemicu: Vec<PemicuLonjakan> = Vec::new();
    for kand in saring_tahap1(&k, data) {
        let Some(dasar) = riwayat.dasar(&kand.ringkasan.pair, sekarang, kl.dasar_menit, kl.pemanasan_menit) else {
            continue;
        };
        jumlah_punya_dasar += 1;
        let Some(perubahan) = perubahan_persen(kand.ringkasan.last, dasar.harga) else {
            continue;
        };
        if perubahan.abs() < separuh || !arah_diizinkan(kl, perubahan) {
            continue;
        }
        pemicu.push(PemicuLonjakan {
            kandidat: kand,
            dasar: dasar.harga,
            perubahan,
            lama_dasar_detik: dasar.lama_detik,
        });
    }
    pemicu.sort_by(|a, b| {
        b.perubahan
            .abs()
            .cmp(&a.perubahan.abs())
            .then_with(|| a.kandidat.ringkasan.pair.cmp(&b.kandidat.ringkasan.pair))
    });
    let jumlah_total = pemicu.len();
    pemicu.truncate(kl.maks_periksa as usize);
    HasilPemicu { pemicu, jumlah_total, jumlah_punya_dasar }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BarisLonjakan {
    pub baris: BarisScreener,
    pub dasar: Decimal,
    /// persen, boleh negatif (positif = naik)
    pub perubahan: Decimal,
    pub lama_dasar_detik: i64,
}

impl BarisLonjakan {
    pub fn naik(&self) -> bool {
        self.perubahan > Decimal::ZERO
    }
}

/// Tahap 2 Lonjakan untuk satu pemicu: ambang X penuh, arah, lalu syarat trx/mnt jendela pendek,
/// spread, dan usia transaksi dari `nilai_tahap2`. None bila tidak lolos.
pub fn nilai_lonjakan(
    ks: &KonfigScreener,
    kl: &KonfigLonjakan,
    p: &PemicuLonjakan,
    trx: &[TransaksiPublik],
    sekarang_cadangan: i64,
) -> Option<BarisLonjakan> {
    if p.perubahan.abs() < kl.ambang_persen || !arah_diizinkan(kl, p.perubahan) {
        return None;
    }
    let k = konfig_saring(ks, kl);
    let baris = nilai_tahap2(&k, &p.kandidat, trx, sekarang_cadangan, None, None)?;
    Some(BarisLonjakan { baris, dasar: p.dasar, perubahan: p.perubahan, lama_dasar_detik: p.lama_dasar_detik })
}

/// Penyimpangan terbesar lebih dulu; seri diputus trx/mnt menurun, lalu nama pair.
pub fn urutkan_lonjakan(baris: &mut [BarisLonjakan]) {
    baris.sort_by(|a, b| {
        let utama: Ordering = b.perubahan.abs().cmp(&a.perubahan.abs());
        utama
            .then_with(|| b.baris.metrik.trx_per_menit.cmp(&a.baris.metrik.trx_per_menit))
            .then_with(|| a.baris.pair.cmp(&b.baris.pair))
    });
}

// ---------------------------------------------------------------------------
// Tes
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screener::JenisTransaksi;
    use std::str::FromStr;

    const NOW: i64 = 1_794_623_510;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn ringkas(pair: &str, last: &str) -> RingkasanPasar {
        RingkasanPasar {
            pair: pair.to_string(),
            last: d(last),
            bid: d("98"),
            ask: d("100"),
            high: d("104"),
            low: d("96"),
            vol_idr: d("50000000"),
            harga_24j: None,
            server_time: Some(NOW),
        }
    }

    /// Isi titik tiap `langkah` detik dari `dari` sampai `sampai` (inklusif) pada harga tetap.
    fn isi(rw: &mut RiwayatHarga, pair: &str, dari: i64, sampai: i64, langkah: i64, harga: &str) {
        let mut t = dari;
        while t <= sampai {
            rw.tambah(t, &[ringkas(pair, harga)]);
            t += langkah;
        }
    }

    fn riwayat_tiga_pair() -> RiwayatHarga {
        let mut rw = RiwayatHarga::baru();
        for p in ["abc_idr", "def_idr", "ghi_idr"] {
            isi(&mut rw, p, NOW - 900, NOW - 20, 20, "100");
        }
        rw
    }

    fn data_sekarang() -> Vec<RingkasanPasar> {
        vec![ringkas("abc_idr", "103"), ringkas("def_idr", "100.5"), ringkas("ghi_idr", "97")]
    }

    fn trx(usia: i64) -> TransaksiPublik {
        TransaksiPublik {
            tid: usia.to_string(),
            jenis: JenisTransaksi::Beli,
            harga: d("103"),
            jumlah: d("1"),
            waktu: NOW - usia,
        }
    }

    /// 12 transaksi, usia 5 sampai 170 detik.
    fn trx_ramai() -> Vec<TransaksiPublik> {
        (0..12).map(|i| trx(5 + 15 * i)).collect()
    }

    #[test]
    fn dasar_harga_tetap_dan_lama_data() {
        let rw = riwayat_tiga_pair();
        let b = rw.dasar("abc_idr", NOW, 60, 10).unwrap();
        assert_eq!(b.harga, d("100"));
        assert_eq!(b.lama_detik, 880); // 45 titik, 44 segmen x 20 detik
        assert!(rw.dasar("tidak_ada", NOW, 60, 10).is_none());
    }

    #[test]
    fn dasar_belum_ada_selama_pemanasan() {
        let mut rw = RiwayatHarga::baru();
        isi(&mut rw, "abc_idr", 0, 300, 20, "100"); // hanya 300 detik
        assert!(rw.dasar("abc_idr", 300, 60, 10).is_none());
        assert!(rw.dasar("abc_idr", 300, 60, 5).is_some());
    }

    #[test]
    fn celah_tidak_dihitung() {
        let mut rw = RiwayatHarga::baru();
        isi(&mut rw, "abc_idr", 0, 600, 20, "100");
        isi(&mut rw, "abc_idr", 2000, 2600, 20, "200"); // celah 1400 detik di antaranya
        let b = rw.dasar("abc_idr", 2600, 60, 10).unwrap();
        assert_eq!(b.lama_detik, 1200);
        assert_eq!(b.harga, d("150")); // 600 detik di 100 dan 600 detik di 200
    }

    #[test]
    fn segmen_di_luar_jendela_dasar_tidak_dihitung() {
        let mut rw = RiwayatHarga::baru();
        isi(&mut rw, "abc_idr", 0, 600, 20, "100");
        isi(&mut rw, "abc_idr", 620, 1200, 20, "200");
        // dasar 10 menit dari t=1200: hanya segmen yang mulai di t >= 600 (dua segmen di 100 dan 200 bercampur di 600-620)
        let b = rw.dasar("abc_idr", 1200, 10, 5).unwrap();
        assert!(b.harga > d("190") && b.harga <= d("200"));
    }

    #[test]
    fn titik_tua_dibuang_dan_waktu_mundur_dilewati() {
        let mut rw = RiwayatHarga::baru();
        rw.tambah(0, &[ringkas("abc_idr", "100")]);
        rw.tambah(4000, &[ringkas("abc_idr", "100")]);
        assert_eq!(rw.titik["abc_idr"].len(), 1);
        rw.tambah(4000, &[ringkas("abc_idr", "999")]); // waktu sama
        rw.tambah(3000, &[ringkas("abc_idr", "999")]); // waktu mundur
        assert_eq!(rw.titik["abc_idr"].len(), 1);
        assert_eq!(rw.rentang_detik(4100), 100);
        rw.kosongkan();
        assert_eq!(rw.rentang_detik(4100), 0);
    }

    #[test]
    fn pemicu_naik_dan_turun_serta_urutan() {
        let rw = riwayat_tiga_pair();
        let kl = KonfigLonjakan::default(); // X = 2, separuh = 1
        let h = cari_pemicu(&KonfigScreener::default(), &kl, &rw, &data_sekarang(), NOW);
        assert_eq!(h.jumlah_punya_dasar, 3);
        assert_eq!(h.jumlah_total, 2);
        let nama: Vec<&str> = h.pemicu.iter().map(|p| p.kandidat.ringkasan.pair.as_str()).collect();
        assert_eq!(nama, vec!["abc_idr", "ghi_idr"]); // def hanya 0,5%; seri 3% diurut nama
        assert_eq!(h.pemicu[0].perubahan, d("3"));
        assert_eq!(h.pemicu[1].perubahan, d("-3"));
        assert_eq!(h.pemicu[0].dasar, d("100"));
    }

    #[test]
    fn pemicu_saklar_arah_dan_batas_periksa() {
        let rw = riwayat_tiga_pair();
        let mut kl = KonfigLonjakan::default();
        kl.turun = false;
        let h = cari_pemicu(&KonfigScreener::default(), &kl, &rw, &data_sekarang(), NOW);
        assert_eq!(h.pemicu.len(), 1);
        assert_eq!(h.pemicu[0].kandidat.ringkasan.pair, "abc_idr");

        kl.turun = true;
        kl.maks_periksa = 1;
        let h = cari_pemicu(&KonfigScreener::default(), &kl, &rw, &data_sekarang(), NOW);
        assert_eq!(h.pemicu.len(), 1);
        assert_eq!(h.jumlah_total, 2);
    }

    #[test]
    fn pemicu_kosong_selama_pemanasan() {
        let mut rw = RiwayatHarga::baru();
        isi(&mut rw, "abc_idr", NOW - 200, NOW - 20, 20, "100");
        let h = cari_pemicu(&KonfigScreener::default(), &KonfigLonjakan::default(), &rw, &data_sekarang(), NOW);
        assert_eq!(h.jumlah_punya_dasar, 0);
        assert!(h.pemicu.is_empty());
    }

    #[test]
    fn pemicu_menghormati_syarat_dasar_spread() {
        let rw = riwayat_tiga_pair();
        let mut data = data_sekarang();
        data[0].bid = Decimal::ZERO; // abc tanpa bid: spread tidak ada, gugur di tahap 1
        let h = cari_pemicu(&KonfigScreener::default(), &KonfigLonjakan::default(), &rw, &data, NOW);
        let nama: Vec<&str> = h.pemicu.iter().map(|p| p.kandidat.ringkasan.pair.as_str()).collect();
        assert_eq!(nama, vec!["ghi_idr"]);
    }

    #[test]
    fn tahap2_trx_mnt_dan_ambang_penuh() {
        let rw = riwayat_tiga_pair();
        let ks = KonfigScreener::default();
        let kl = KonfigLonjakan::default();
        let h = cari_pemicu(&ks, &kl, &rw, &data_sekarang(), NOW);
        let abc = &h.pemicu[0];

        let b = nilai_lonjakan(&ks, &kl, abc, &trx_ramai(), 0).unwrap();
        assert!(b.naik());
        assert_eq!(b.perubahan, d("3"));
        assert_eq!(b.dasar, d("100"));
        assert!(b.baris.metrik.trx_per_menit > d("4") && b.baris.metrik.trx_per_menit < d("4.3"));

        let mut ketat = kl.clone();
        ketat.trx_mnt_min = d("10"); // M terlalu tinggi
        assert!(nilai_lonjakan(&ks, &ketat, abc, &trx_ramai(), 0).is_none());

        let mut ambang = kl.clone();
        ambang.ambang_persen = d("5"); // 3% belum cukup
        assert!(nilai_lonjakan(&ks, &ambang, abc, &trx_ramai(), 0).is_none());

        let mut tanpa_naik = kl.clone();
        tanpa_naik.naik = false;
        assert!(nilai_lonjakan(&ks, &tanpa_naik, abc, &trx_ramai(), 0).is_none());

        assert!(nilai_lonjakan(&ks, &kl, abc, &[], 0).is_none()); // tanpa transaksi
    }

    #[test]
    fn tahap2_usia_transaksi_terakhir_berlaku() {
        let rw = riwayat_tiga_pair();
        let ks = KonfigScreener::default(); // usia maksimum 120 detik
        let kl = KonfigLonjakan::default();
        let h = cari_pemicu(&ks, &kl, &rw, &data_sekarang(), NOW);
        let basi: Vec<TransaksiPublik> = (0..12).map(|i| trx(150 + 3 * i)).collect(); // terbaru 150 detik
        assert!(nilai_lonjakan(&ks, &kl, &h.pemicu[0], &basi, 0).is_none());
    }

    #[test]
    fn urutan_penyimpangan_terbesar_dulu() {
        let rw = riwayat_tiga_pair();
        let ks = KonfigScreener::default();
        let kl = KonfigLonjakan::default();
        let mut data = data_sekarang();
        data[2] = ringkas("ghi_idr", "95"); // -5%
        let h = cari_pemicu(&ks, &kl, &rw, &data, NOW);
        let mut baris: Vec<BarisLonjakan> =
            h.pemicu.iter().filter_map(|p| nilai_lonjakan(&ks, &kl, p, &trx_ramai(), 0)).collect();
        // trx_ramai memakai harga 103 untuk semua; yang diuji hanya urutan
        baris.reverse();
        urutkan_lonjakan(&mut baris);
        let nama: Vec<&str> = baris.iter().map(|b| b.baris.pair.as_str()).collect();
        assert_eq!(nama, vec!["ghi_idr", "abc_idr"]);
        assert!(!baris[0].naik());
    }

    #[test]
    fn konfig_saring_mematikan_range_dan_filter_tambahan() {
        let mut ks = KonfigScreener::default();
        ks.gerak_min_persen = Some(d("1"));
        ks.arah_min_persen = Some(d("1"));
        ks.kedalaman_kelipatan = Some(d("2"));
        let k = konfig_saring(&ks, &KonfigLonjakan::default());
        assert_eq!(k.range24_min, None);
        assert_eq!(k.gerak_min_persen, None);
        assert_eq!(k.arah_min_persen, None);
        assert_eq!(k.kedalaman_kelipatan, None);
        assert_eq!(k.trx_mnt_min, Some(d("3")));
        assert_eq!(k.jendela_menit, 3);
        assert_eq!(k.spread_maks, ks.spread_maks); // syarat dasar tetap
        assert_eq!(k.usia_trx_maks_detik, ks.usia_trx_maks_detik);
    }
}
