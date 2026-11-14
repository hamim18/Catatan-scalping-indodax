//! T9.5: setelan Harga global. Disimpan di tabel `meta` dengan awalan `global_` (tanpa tabel baru).
//! Aturan sama dengan `screener_setelan`: kunci belum ada = bawaan; nilai rusak = bawaan (layar
//! Screener tidak boleh macet karena meta rusak). Semua isian wajib (tidak ada filter yang bisa
//! dimatikan; untuk mematikan fitur pakai `aktif`).
//! Hanya informasi: tidak menyentuh posisi, profit, target, grafik, atau statistik.
use crate::harga_global::KonfigGlobal;
use crate::{Error, Hasil, Penyimpanan};
use rust_decimal::Decimal;
use std::str::FromStr;

fn kunci(nama: &str) -> String {
    format!("global_{nama}")
}

fn baca_dec(simpan: &Penyimpanan, nama: &str, bawaan: Decimal) -> Hasil<Decimal> {
    Ok(match simpan.meta(&kunci(nama))? {
        None => bawaan,
        Some(t) => Decimal::from_str(t.trim()).unwrap_or(bawaan),
    })
}

/// Baca semua ambang. Tanpa jaringan.
pub fn muat_konfig_global(simpan: &Penyimpanan) -> Hasil<KonfigGlobal> {
    let b = KonfigGlobal::default();
    let aktif = match simpan.meta(&kunci("aktif"))?.as_deref().map(str::trim) {
        Some("1") => true,
        Some("0") => false,
        _ => b.aktif,
    };
    let interval_detik = match simpan.meta(&kunci("interval_detik"))? {
        Some(t) => t.trim().parse::<u32>().ok().filter(|x| (15..=600).contains(x)).unwrap_or(b.interval_detik),
        None => b.interval_detik,
    };
    // T9.6: lebar jendela arah; nilai di luar 5..=30 dianggap rusak (bawaan)
    let arah_menit = match simpan.meta(&kunci("arah_menit"))? {
        Some(t) => t.trim().parse::<u32>().ok().filter(|x| (5..=30).contains(x)).unwrap_or(b.arah_menit),
        None => b.arah_menit,
    };
    Ok(KonfigGlobal {
        aktif,
        sebaran_maks_persen: baca_dec(simpan, "sebaran_maks_persen", b.sebaran_maks_persen)?,
        curiga_persen: baca_dec(simpan, "curiga_persen", b.curiga_persen)?,
        kurs_tanda_persen: baca_dec(simpan, "kurs_tanda_persen", b.kurs_tanda_persen)?,
        interval_detik,
        arah_menit,
        arah_ambang_persen: baca_dec(simpan, "arah_ambang_persen", b.arah_ambang_persen)?,
    })
}

fn tidak_valid(pesan: &str) -> Error {
    Error::SetelanTidakValid(pesan.to_string())
}

fn di_atas_nol_sampai_seratus(nama: &str, v: Decimal) -> Hasil<()> {
    if v <= Decimal::ZERO || v > Decimal::from(100_i64) {
        return Err(tidak_valid(&format!("{nama} harus di atas 0 sampai 100%")));
    }
    Ok(())
}

/// Periksa ambang sebelum disimpan. Pesan galat berbahasa Indonesia untuk ditampilkan langsung.
pub fn periksa_konfig_global(c: &KonfigGlobal) -> Hasil<()> {
    di_atas_nol_sampai_seratus("Sebaran maksimum", c.sebaran_maks_persen)?;
    di_atas_nol_sampai_seratus("Batas curiga selisih", c.curiga_persen)?;
    di_atas_nol_sampai_seratus("Batas tanda kurs", c.kurs_tanda_persen)?;
    if c.interval_detik < 15 || c.interval_detik > 600 {
        return Err(tidak_valid("Interval harus 15 sampai 600 detik"));
    }
    di_atas_nol_sampai_seratus("Batas tanda arah", c.arah_ambang_persen)?;
    if c.arah_menit < 5 || c.arah_menit > 30 {
        return Err(tidak_valid("Jendela arah harus 5 sampai 30 menit"));
    }
    Ok(())
}

/// Periksa lalu simpan semua ambang ke `meta`. Tanpa jaringan.
pub fn simpan_konfig_global(simpan: &Penyimpanan, c: &KonfigGlobal) -> Hasil<()> {
    periksa_konfig_global(c)?;
    simpan.atur_meta(&kunci("aktif"), if c.aktif { "1" } else { "0" })?;
    simpan.atur_meta(&kunci("sebaran_maks_persen"), &c.sebaran_maks_persen.normalize().to_string())?;
    simpan.atur_meta(&kunci("curiga_persen"), &c.curiga_persen.normalize().to_string())?;
    simpan.atur_meta(&kunci("kurs_tanda_persen"), &c.kurs_tanda_persen.normalize().to_string())?;
    simpan.atur_meta(&kunci("interval_detik"), &c.interval_detik.to_string())?;
    simpan.atur_meta(&kunci("arah_menit"), &c.arah_menit.to_string())?;
    simpan.atur_meta(&kunci("arah_ambang_persen"), &c.arah_ambang_persen.normalize().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kosong_berarti_bawaan() {
        let s = Penyimpanan::dalam_memori().unwrap();
        assert_eq!(muat_konfig_global(&s).unwrap(), KonfigGlobal::default());
    }

    #[test]
    fn simpan_dan_muat_kembali() {
        let s = Penyimpanan::dalam_memori().unwrap();
        let c = KonfigGlobal {
            aktif: false,
            sebaran_maks_persen: Decimal::new(35, 1), // 3,5
            curiga_persen: Decimal::from(15_i64),
            kurs_tanda_persen: Decimal::new(5, 1), // 0,5
            interval_detik: 90,
            arah_menit: 15,
            arah_ambang_persen: Decimal::new(8, 1), // 0,8
        };
        simpan_konfig_global(&s, &c).unwrap();
        assert_eq!(muat_konfig_global(&s).unwrap(), c);
    }

    #[test]
    fn periksa_menolak_nilai_tidak_masuk_akal_dan_tidak_menulis() {
        assert!(periksa_konfig_global(&KonfigGlobal::default()).is_ok());
        let mut c = KonfigGlobal::default();
        c.interval_detik = 5;
        assert!(periksa_konfig_global(&c).is_err());
        let mut c = KonfigGlobal::default();
        c.sebaran_maks_persen = Decimal::ZERO;
        assert!(periksa_konfig_global(&c).is_err());
        let mut c = KonfigGlobal::default();
        c.curiga_persen = Decimal::from(101_i64);
        assert!(periksa_konfig_global(&c).is_err());
        let mut c = KonfigGlobal::default();
        c.kurs_tanda_persen = Decimal::from(-1_i64);
        let s = Penyimpanan::dalam_memori().unwrap();
        assert!(simpan_konfig_global(&s, &c).is_err());
        assert_eq!(s.meta("global_aktif").unwrap(), None);
    }

    #[test]
    fn nilai_meta_rusak_kembali_ke_bawaan() {
        let s = Penyimpanan::dalam_memori().unwrap();
        s.atur_meta("global_aktif", "mungkin").unwrap();
        s.atur_meta("global_curiga_persen", "abc").unwrap();
        s.atur_meta("global_interval_detik", "3").unwrap();
        let m = muat_konfig_global(&s).unwrap();
        let b = KonfigGlobal::default();
        assert_eq!(m.aktif, b.aktif);
        assert_eq!(m.curiga_persen, b.curiga_persen);
        assert_eq!(m.interval_detik, b.interval_detik);
    }

    #[test]
    fn arah_diperiksa_dan_nilai_rusak_kembali_ke_bawaan() {
        let b = KonfigGlobal::default();
        assert_eq!((b.arah_menit, b.arah_ambang_persen), (10, Decimal::new(5, 1)));
        for menit in [4_u32, 31] {
            let mut c = KonfigGlobal::default();
            c.arah_menit = menit;
            assert!(periksa_konfig_global(&c).is_err());
        }
        let mut c = KonfigGlobal::default();
        c.arah_ambang_persen = Decimal::ZERO;
        assert!(periksa_konfig_global(&c).is_err());

        let s = Penyimpanan::dalam_memori().unwrap();
        s.atur_meta("global_arah_menit", "99").unwrap();
        s.atur_meta("global_arah_ambang_persen", "abc").unwrap();
        let m = muat_konfig_global(&s).unwrap();
        assert_eq!(m.arah_menit, b.arah_menit);
        assert_eq!(m.arah_ambang_persen, b.arah_ambang_persen);
    }
}
