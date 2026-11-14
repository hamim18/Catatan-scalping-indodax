//! T9.4: setelan ambang Lonjakan. Disimpan di tabel `meta` dengan awalan `lonjakan_` (tanpa tabel
//! baru). Kunci belum ada = nilai bawaan; nilai rusak = bawaan. Hanya informasi: tidak menyentuh
//! posisi, profit, target, grafik, atau statistik.
use crate::lonjakan::{KonfigLonjakan, MAKS_RIWAYAT_DETIK};
use crate::{Error, Hasil, Penyimpanan};
use rust_decimal::Decimal;
use std::str::FromStr;

fn kunci(nama: &str) -> String {
    format!("lonjakan_{nama}")
}

fn baca_dec(simpan: &Penyimpanan, nama: &str, bawaan: Decimal) -> Hasil<Decimal> {
    Ok(match simpan.meta(&kunci(nama))? {
        None => bawaan,
        Some(t) => Decimal::from_str(t.trim()).unwrap_or(bawaan),
    })
}

fn baca_u32(simpan: &Penyimpanan, nama: &str, bawaan: u32) -> Hasil<u32> {
    Ok(match simpan.meta(&kunci(nama))? {
        None => bawaan,
        Some(t) => t.trim().parse::<u32>().unwrap_or(bawaan),
    })
}

fn baca_bool(simpan: &Penyimpanan, nama: &str, bawaan: bool) -> Hasil<bool> {
    Ok(match simpan.meta(&kunci(nama))?.as_deref().map(str::trim) {
        Some("1") => true,
        Some("0") => false,
        _ => bawaan,
    })
}

fn tidak_valid(pesan: &str) -> Error {
    Error::SetelanTidakValid(pesan.to_string())
}

/// Periksa ambang sebelum disimpan. Pesan galat berbahasa Indonesia untuk ditampilkan langsung.
pub fn periksa_konfig_lonjakan(c: &KonfigLonjakan) -> Hasil<()> {
    let maks_dasar = (MAKS_RIWAYAT_DETIK / 60) as u32;
    if !c.naik && !c.turun {
        return Err(tidak_valid("Aktifkan minimal satu arah (naik atau turun)"));
    }
    if c.ambang_persen <= Decimal::ZERO || c.ambang_persen > Decimal::from(100_i64) {
        return Err(tidak_valid("Lonjakan minimum harus di atas 0 sampai 100%"));
    }
    if c.trx_mnt_min < Decimal::ZERO {
        return Err(tidak_valid("Trx/mnt minimum tidak boleh negatif"));
    }
    if c.jendela_menit < 1 || c.jendela_menit > 60 {
        return Err(tidak_valid("Jendela pendek harus 1 sampai 60 menit"));
    }
    if c.dasar_menit < 5 || c.dasar_menit > maks_dasar {
        return Err(tidak_valid(&format!("Rata-rata harga dasar harus 5 sampai {maks_dasar} menit")));
    }
    if c.pemanasan_menit < 1 || c.pemanasan_menit > c.dasar_menit {
        return Err(tidak_valid("Pemanasan harus 1 menit sampai selebar rata-rata harga dasar"));
    }
    if c.maks_periksa < 1 || c.maks_periksa > 40 {
        return Err(tidak_valid("Maksimal pair diperiksa harus 1 sampai 40"));
    }
    if c.interval_detik < 10 || c.interval_detik > 120 {
        return Err(tidak_valid("Interval harus 10 sampai 120 detik"));
    }
    Ok(())
}

/// Baca ambang. Bila kombinasi tersimpan tidak valid (meta rusak), kembali ke bawaan.
pub fn muat_konfig_lonjakan(simpan: &Penyimpanan) -> Hasil<KonfigLonjakan> {
    let b = KonfigLonjakan::default();
    let c = KonfigLonjakan {
        naik: baca_bool(simpan, "naik", b.naik)?,
        turun: baca_bool(simpan, "turun", b.turun)?,
        ambang_persen: baca_dec(simpan, "ambang_persen", b.ambang_persen)?,
        trx_mnt_min: baca_dec(simpan, "trx_mnt_min", b.trx_mnt_min)?,
        jendela_menit: baca_u32(simpan, "jendela_menit", b.jendela_menit)?,
        dasar_menit: baca_u32(simpan, "dasar_menit", b.dasar_menit)?,
        pemanasan_menit: baca_u32(simpan, "pemanasan_menit", b.pemanasan_menit)?,
        maks_periksa: baca_u32(simpan, "maks_periksa", b.maks_periksa)?,
        interval_detik: baca_u32(simpan, "interval_detik", b.interval_detik)?,
    };
    if periksa_konfig_lonjakan(&c).is_ok() {
        Ok(c)
    } else {
        Ok(b)
    }
}

/// Periksa lalu simpan semua ambang ke `meta`. Tanpa jaringan.
pub fn simpan_konfig_lonjakan(simpan: &Penyimpanan, c: &KonfigLonjakan) -> Hasil<()> {
    periksa_konfig_lonjakan(c)?;
    simpan.atur_meta(&kunci("naik"), if c.naik { "1" } else { "0" })?;
    simpan.atur_meta(&kunci("turun"), if c.turun { "1" } else { "0" })?;
    simpan.atur_meta(&kunci("ambang_persen"), &c.ambang_persen.normalize().to_string())?;
    simpan.atur_meta(&kunci("trx_mnt_min"), &c.trx_mnt_min.normalize().to_string())?;
    simpan.atur_meta(&kunci("jendela_menit"), &c.jendela_menit.to_string())?;
    simpan.atur_meta(&kunci("dasar_menit"), &c.dasar_menit.to_string())?;
    simpan.atur_meta(&kunci("pemanasan_menit"), &c.pemanasan_menit.to_string())?;
    simpan.atur_meta(&kunci("maks_periksa"), &c.maks_periksa.to_string())?;
    simpan.atur_meta(&kunci("interval_detik"), &c.interval_detik.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kosong_berarti_bawaan() {
        let s = Penyimpanan::dalam_memori().unwrap();
        assert_eq!(muat_konfig_lonjakan(&s).unwrap(), KonfigLonjakan::default());
    }

    #[test]
    fn simpan_dan_muat_kembali() {
        let s = Penyimpanan::dalam_memori().unwrap();
        let mut c = KonfigLonjakan::default();
        c.turun = false;
        c.ambang_persen = Decimal::new(25, 1); // 2,5
        c.trx_mnt_min = Decimal::from(4_i64);
        c.jendela_menit = 5;
        c.dasar_menit = 30;
        c.pemanasan_menit = 8;
        c.maks_periksa = 10;
        c.interval_detik = 30;
        simpan_konfig_lonjakan(&s, &c).unwrap();
        assert_eq!(muat_konfig_lonjakan(&s).unwrap(), c);
    }

    #[test]
    fn periksa_menolak_nilai_tidak_masuk_akal() {
        let b = KonfigLonjakan::default();
        assert!(periksa_konfig_lonjakan(&b).is_ok());

        let mut c = b.clone();
        c.naik = false;
        c.turun = false;
        assert!(periksa_konfig_lonjakan(&c).is_err());

        let mut c = b.clone();
        c.ambang_persen = Decimal::ZERO;
        assert!(periksa_konfig_lonjakan(&c).is_err());

        let mut c = b.clone();
        c.dasar_menit = 90; // lebih dari 60
        assert!(periksa_konfig_lonjakan(&c).is_err());

        let mut c = b.clone();
        c.pemanasan_menit = 61;
        assert!(periksa_konfig_lonjakan(&c).is_err());

        let mut c = b.clone();
        c.interval_detik = 5;
        assert!(periksa_konfig_lonjakan(&c).is_err());

        let mut c = b.clone();
        c.maks_periksa = 0;
        assert!(periksa_konfig_lonjakan(&c).is_err());

        // simpan menolak dan tidak menulis apa pun
        let s = Penyimpanan::dalam_memori().unwrap();
        let mut c = b;
        c.interval_detik = 500;
        assert!(simpan_konfig_lonjakan(&s, &c).is_err());
        assert_eq!(s.meta("lonjakan_interval_detik").unwrap(), None);
    }

    #[test]
    fn nilai_meta_rusak_kembali_ke_bawaan() {
        let s = Penyimpanan::dalam_memori().unwrap();
        s.atur_meta("lonjakan_ambang_persen", "abc").unwrap();
        s.atur_meta("lonjakan_naik", "mungkin").unwrap();
        s.atur_meta("lonjakan_interval_detik", "-3").unwrap();
        assert_eq!(muat_konfig_lonjakan(&s).unwrap(), KonfigLonjakan::default());
    }

    #[test]
    fn kombinasi_tersimpan_tidak_valid_kembali_ke_bawaan() {
        let s = Penyimpanan::dalam_memori().unwrap();
        s.atur_meta("lonjakan_naik", "0").unwrap();
        s.atur_meta("lonjakan_turun", "0").unwrap();
        assert_eq!(muat_konfig_lonjakan(&s).unwrap(), KonfigLonjakan::default());
    }
}
