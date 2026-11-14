//! T9.2: setelan ambang Screener (FR-33). Disimpan di tabel `meta` dengan awalan `screener_`
//! (tanpa tabel baru). Aturan simpan per kunci:
//! - kunci belum ada      -> nilai bawaan (`KonfigScreener::default()`)
//! - nilai kosong ("")    -> filter DIMATIKAN (None)
//! - nilai angka          -> dipakai
//! - nilai rusak          -> kembali ke bawaan (layar Screener tidak boleh macet karena meta rusak)
//! Hanya informasi: tidak menyentuh posisi, profit, target, grafik, atau statistik.
use crate::screener::{KonfigScreener, UrutanScreener};
use crate::{Error, Hasil, Penyimpanan};
use rust_decimal::Decimal;
use std::str::FromStr;

fn kunci(nama: &str) -> String {
    format!("screener_{nama}")
}

fn baca_opsi_dec(simpan: &Penyimpanan, nama: &str, bawaan: Option<Decimal>) -> Hasil<Option<Decimal>> {
    Ok(match simpan.meta(&kunci(nama))? {
        None => bawaan,
        Some(t) if t.trim().is_empty() => None,
        Some(t) => Decimal::from_str(t.trim()).ok().or(bawaan),
    })
}

fn baca_dec(simpan: &Penyimpanan, nama: &str, bawaan: Decimal) -> Hasil<Decimal> {
    Ok(match simpan.meta(&kunci(nama))? {
        None => bawaan,
        Some(t) => Decimal::from_str(t.trim()).unwrap_or(bawaan),
    })
}

fn baca_opsi_int(simpan: &Penyimpanan, nama: &str, bawaan: Option<i64>) -> Hasil<Option<i64>> {
    Ok(match simpan.meta(&kunci(nama))? {
        None => bawaan,
        Some(t) if t.trim().is_empty() => None,
        Some(t) => t.trim().parse::<i64>().ok().or(bawaan),
    })
}

fn atur_opsi_dec(simpan: &Penyimpanan, nama: &str, v: Option<Decimal>) -> Hasil<()> {
    let teks = v.map(|d| d.normalize().to_string()).unwrap_or_default();
    simpan.atur_meta(&kunci(nama), &teks)
}

/// Baca semua ambang. Tanpa jaringan.
pub fn muat_konfig(simpan: &Penyimpanan) -> Hasil<KonfigScreener> {
    let b = KonfigScreener::default();
    let jendela_menit = match simpan.meta(&kunci("jendela_menit"))? {
        Some(t) => t.trim().parse::<u32>().ok().filter(|x| *x >= 1).unwrap_or(b.jendela_menit),
        None => b.jendela_menit,
    };
    let urutan = match simpan.meta(&kunci("urutan"))?.as_deref() {
        Some("trx") => UrutanScreener::TrxPerMenit,
        Some("skor") => UrutanScreener::Skor,
        _ => b.urutan,
    };
    Ok(KonfigScreener {
        harga_min: baca_opsi_dec(simpan, "harga_min", b.harga_min)?,
        harga_maks: baca_opsi_dec(simpan, "harga_maks", b.harga_maks)?,
        vol_min: baca_opsi_dec(simpan, "vol_min", b.vol_min)?,
        vol_maks: baca_opsi_dec(simpan, "vol_maks", b.vol_maks)?,
        trx_mnt_min: baca_opsi_dec(simpan, "trx_mnt_min", b.trx_mnt_min)?,
        jendela_menit,
        spread_maks: baca_opsi_dec(simpan, "spread_maks", b.spread_maks)?,
        range24_min: baca_opsi_dec(simpan, "range24_min", b.range24_min)?,
        usia_trx_maks_detik: baca_opsi_int(simpan, "usia_trx_maks_detik", b.usia_trx_maks_detik)?,
        gerak_min_persen: baca_opsi_dec(simpan, "gerak_min_persen", b.gerak_min_persen)?,
        menit_aktif_min_persen: baca_opsi_dec(simpan, "menit_aktif_min_persen", b.menit_aktif_min_persen)?,
        arah_min_persen: baca_opsi_dec(simpan, "arah_min_persen", b.arah_min_persen)?,
        kedalaman_kelipatan: baca_opsi_dec(simpan, "kedalaman_kelipatan", b.kedalaman_kelipatan)?,
        kedalaman_persen: baca_dec(simpan, "kedalaman_persen", b.kedalaman_persen)?,
        dominasi_tanda_persen: baca_opsi_dec(simpan, "dominasi_tanda_persen", b.dominasi_tanda_persen)?,
        biaya_roundtrip_persen: baca_dec(simpan, "biaya_roundtrip_persen", b.biaya_roundtrip_persen)?,
        urutan,
    })
}

fn tidak_valid(pesan: &str) -> Error {
    Error::SetelanTidakValid(pesan.to_string())
}

fn tidak_negatif(nama: &str, v: Option<Decimal>) -> Hasil<()> {
    if let Some(x) = v {
        if x < Decimal::ZERO {
            return Err(tidak_valid(&format!("{nama} tidak boleh negatif")));
        }
    }
    Ok(())
}

/// Periksa ambang sebelum disimpan. Pesan galat berbahasa Indonesia untuk ditampilkan langsung.
pub fn periksa_konfig(c: &KonfigScreener) -> Hasil<()> {
    let seratus = Decimal::from(100_i64);
    tidak_negatif("Harga minimum", c.harga_min)?;
    tidak_negatif("Harga maksimum", c.harga_maks)?;
    tidak_negatif("Volume minimum", c.vol_min)?;
    tidak_negatif("Volume maksimum", c.vol_maks)?;
    tidak_negatif("Trx/mnt minimum", c.trx_mnt_min)?;
    tidak_negatif("Spread maksimum", c.spread_maks)?;
    tidak_negatif("Range 24 jam minimum", c.range24_min)?;
    tidak_negatif("Gerak minimum", c.gerak_min_persen)?;
    tidak_negatif("Menit aktif minimum", c.menit_aktif_min_persen)?;
    tidak_negatif("Kelipatan kedalaman", c.kedalaman_kelipatan)?;
    if let (Some(a), Some(b)) = (c.harga_min, c.harga_maks) {
        if a > b {
            return Err(tidak_valid("Harga minimum melebihi maksimum"));
        }
    }
    if let (Some(a), Some(b)) = (c.vol_min, c.vol_maks) {
        if a > b {
            return Err(tidak_valid("Volume minimum melebihi maksimum"));
        }
    }
    if c.jendela_menit < 1 || c.jendela_menit > 60 {
        return Err(tidak_valid("Jendela harus 1 sampai 60 menit"));
    }
    if let Some(u) = c.usia_trx_maks_detik {
        if u < 1 {
            return Err(tidak_valid("Usia transaksi maksimum minimal 1 detik"));
        }
    }
    if let Some(m) = c.menit_aktif_min_persen {
        if m > seratus {
            return Err(tidak_valid("Menit aktif maksimal 100%"));
        }
    }
    if c.kedalaman_persen <= Decimal::ZERO || c.kedalaman_persen > seratus {
        return Err(tidak_valid("Jarak kedalaman harus di atas 0 sampai 100%"));
    }
    if let Some(d) = c.dominasi_tanda_persen {
        if d <= Decimal::ZERO || d > seratus {
            return Err(tidak_valid("Tanda dominasi harus di atas 0 sampai 100%"));
        }
    }
    if c.biaya_roundtrip_persen < Decimal::ZERO || c.biaya_roundtrip_persen > seratus {
        return Err(tidak_valid("Biaya round-trip harus 0 sampai 100%"));
    }
    Ok(())
}

/// Periksa lalu simpan semua ambang ke `meta`. Tanpa jaringan.
pub fn simpan_konfig(simpan: &Penyimpanan, c: &KonfigScreener) -> Hasil<()> {
    periksa_konfig(c)?;
    atur_opsi_dec(simpan, "harga_min", c.harga_min)?;
    atur_opsi_dec(simpan, "harga_maks", c.harga_maks)?;
    atur_opsi_dec(simpan, "vol_min", c.vol_min)?;
    atur_opsi_dec(simpan, "vol_maks", c.vol_maks)?;
    atur_opsi_dec(simpan, "trx_mnt_min", c.trx_mnt_min)?;
    simpan.atur_meta(&kunci("jendela_menit"), &c.jendela_menit.to_string())?;
    atur_opsi_dec(simpan, "spread_maks", c.spread_maks)?;
    atur_opsi_dec(simpan, "range24_min", c.range24_min)?;
    simpan.atur_meta(
        &kunci("usia_trx_maks_detik"),
        &c.usia_trx_maks_detik.map(|u| u.to_string()).unwrap_or_default(),
    )?;
    atur_opsi_dec(simpan, "gerak_min_persen", c.gerak_min_persen)?;
    atur_opsi_dec(simpan, "menit_aktif_min_persen", c.menit_aktif_min_persen)?;
    atur_opsi_dec(simpan, "arah_min_persen", c.arah_min_persen)?;
    atur_opsi_dec(simpan, "kedalaman_kelipatan", c.kedalaman_kelipatan)?;
    simpan.atur_meta(&kunci("kedalaman_persen"), &c.kedalaman_persen.normalize().to_string())?;
    atur_opsi_dec(simpan, "dominasi_tanda_persen", c.dominasi_tanda_persen)?;
    simpan.atur_meta(&kunci("biaya_roundtrip_persen"), &c.biaya_roundtrip_persen.normalize().to_string())?;
    simpan.atur_meta(
        &kunci("urutan"),
        match c.urutan {
            UrutanScreener::Skor => "skor",
            UrutanScreener::TrxPerMenit => "trx",
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kosong_berarti_bawaan() {
        let s = Penyimpanan::dalam_memori().unwrap();
        assert_eq!(muat_konfig(&s).unwrap(), KonfigScreener::default());
    }

    #[test]
    fn simpan_dan_muat_kembali() {
        let s = Penyimpanan::dalam_memori().unwrap();
        let mut c = KonfigScreener::default();
        c.harga_min = Some(Decimal::from(50_i64));
        c.trx_mnt_min = Some(Decimal::new(35, 1)); // 3,5
        c.jendela_menit = 5;
        c.gerak_min_persen = Some(Decimal::new(82, 2)); // 0,82
        c.arah_min_persen = Some(Decimal::from(-1_i64));
        c.kedalaman_kelipatan = Some(Decimal::from(3_i64));
        c.usia_trx_maks_detik = Some(45);
        c.urutan = UrutanScreener::TrxPerMenit;
        simpan_konfig(&s, &c).unwrap();
        assert_eq!(muat_konfig(&s).unwrap(), c);
    }

    #[test]
    fn filter_dimatikan_tetap_mati_bukan_kembali_ke_bawaan() {
        let s = Penyimpanan::dalam_memori().unwrap();
        let mut c = KonfigScreener::default();
        c.harga_min = None;
        c.range24_min = None;
        c.usia_trx_maks_detik = None;
        simpan_konfig(&s, &c).unwrap();
        let m = muat_konfig(&s).unwrap();
        assert_eq!(m.harga_min, None);
        assert_eq!(m.range24_min, None);
        assert_eq!(m.usia_trx_maks_detik, None);
        assert_eq!(m.vol_min, KonfigScreener::default().vol_min);
    }

    #[test]
    fn periksa_menolak_nilai_tidak_masuk_akal() {
        let bawaan = KonfigScreener::default();
        assert!(periksa_konfig(&bawaan).is_ok());

        let mut c = bawaan.clone();
        c.harga_min = Some(Decimal::from(100_i64));
        c.harga_maks = Some(Decimal::from(10_i64));
        assert!(periksa_konfig(&c).is_err());

        let mut c = bawaan.clone();
        c.jendela_menit = 0;
        assert!(periksa_konfig(&c).is_err());

        let mut c = bawaan.clone();
        c.kedalaman_persen = Decimal::ZERO;
        assert!(periksa_konfig(&c).is_err());

        let mut c = bawaan.clone();
        c.spread_maks = Some(Decimal::from(-1_i64));
        assert!(periksa_konfig(&c).is_err());

        // simpan_konfig juga menolak dan tidak menulis apa pun
        let s = Penyimpanan::dalam_memori().unwrap();
        let mut c = bawaan;
        c.jendela_menit = 99;
        assert!(simpan_konfig(&s, &c).is_err());
        assert_eq!(s.meta("screener_jendela_menit").unwrap(), None);
    }

    #[test]
    fn nilai_meta_rusak_kembali_ke_bawaan() {
        let s = Penyimpanan::dalam_memori().unwrap();
        s.atur_meta("screener_harga_min", "abc").unwrap();
        s.atur_meta("screener_jendela_menit", "0").unwrap();
        s.atur_meta("screener_urutan", "aneh").unwrap();
        let m = muat_konfig(&s).unwrap();
        let b = KonfigScreener::default();
        assert_eq!(m.harga_min, b.harga_min);
        assert_eq!(m.jendela_menit, b.jendela_menit);
        assert_eq!(m.urutan, b.urutan);
    }
}
