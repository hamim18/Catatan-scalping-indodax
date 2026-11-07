//! Rekonsiliasi REST (FR-09, FR-10) dan snapshot saldo awal (FR-03).
//! Alur: tarik myTrades per koin per rentang <= 6 hari, simpan dengan
//! anti-dobel, ingat batas waktu sinkron per koin. Hanya membaca.
use crate::{Error, Hasil, IndodaxClient, Penyimpanan, Trade};
use rust_decimal::prelude::*;
use std::collections::BTreeSet;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const HARI_MS: u64 = 24 * 60 * 60 * 1000;
/// Indodax membatasi rentang 7 hari per permintaan; pakai 6 hari agar aman.
const JENDELA_MS: u64 = 6 * HARI_MS;
/// Rentang terkecil yang masih boleh dibelah saat hasil penuh.
const RENTANG_MIN_MS: u64 = 60_000;
/// Tumpang tindih antar sinkron; aman karena anti-dobel.
const TUMPANG_TINDIH_MS: u64 = 60_000;

/// Sumber trade. Dipisah dari klien supaya logika bisa dites tanpa jaringan.
pub trait SumberTrade {
    fn ambil_trade(&self, simbol: &str, mulai_ms: u64, akhir_ms: u64, batas: u32) -> Hasil<Vec<Trade>>;
}

impl SumberTrade for IndodaxClient {
    fn ambil_trade(&self, simbol: &str, mulai_ms: u64, akhir_ms: u64, batas: u32) -> Hasil<Vec<Trade>> {
        IndodaxClient::ambil_trade(self, simbol, Some(mulai_ms), Some(akhir_ms), batas)
    }
}

#[derive(Debug, Clone)]
pub struct Opsi {
    /// Sinkron pertama sebuah koin menarik sekian hari ke belakang.
    pub hari_awal: u32,
    /// Jeda antar permintaan (batas laju 300/menit).
    pub jeda_ms: u64,
    /// Jumlah maksimal trade per permintaan. Bila hasil penuh, rentang dibelah.
    pub batas: u32,
    /// Koin tambahan di luar saldo dan riwayat, contoh "icntidr".
    pub koin_tambahan: Vec<String>,
}

impl Default for Opsi {
    fn default() -> Self {
        Self { hari_awal: 7, jeda_ms: 250, batas: 100, koin_tambahan: vec![] }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HasilSinkron {
    pub koin_dipindai: Vec<String>,
    pub trade_baru: usize,
    /// Simbol yang ditolak Indodax (pasangan IDR tidak ada), dilewati.
    pub koin_dilewati: Vec<String>,
    pub snapshot_awal_dibuat: bool,
}

fn sekarang_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn kunci_meta(simbol: &str) -> String {
    format!("sinkron:{simbol}")
}

/// Tarik satu rentang. Bila hasil sebanyak `batas` (kemungkinan terpotong),
/// belah rentang jadi dua sampai tidak penuh. Tidak bergantung urutan hasil.
fn ambil_rentang<S: SumberTrade>(
    s: &S,
    simbol: &str,
    mulai: u64,
    akhir: u64,
    opsi: &Opsi,
    keluar: &mut Vec<Trade>,
) -> Hasil<()> {
    let batch = s.ambil_trade(simbol, mulai, akhir, opsi.batas)?;
    if opsi.jeda_ms > 0 {
        std::thread::sleep(Duration::from_millis(opsi.jeda_ms));
    }
    if batch.len() as u32 >= opsi.batas {
        if akhir - mulai <= RENTANG_MIN_MS {
            // Jangan diam-diam kehilangan trade.
            return Err(Error::RentangTerlaluPadat { simbol: simbol.to_string(), jumlah: batch.len() });
        }
        let tengah = mulai + (akhir - mulai) / 2;
        ambil_rentang(s, simbol, mulai, tengah, opsi, keluar)?;
        ambil_rentang(s, simbol, tengah + 1, akhir, opsi, keluar)?;
    } else {
        keluar.extend(batch);
    }
    Ok(())
}

/// Indodax menolak simbol yang tidak ada: HTTP 400, kode 1109 "invalid symbol".
fn simbol_ditolak(e: &Error) -> bool {
    matches!(e, Error::Http { kode: 400, isi_awal }
        if isi_awal.contains("1109") || isi_awal.contains("invalid symbol"))
}

/// Sinkron daftar koin. Batas waktu per koin baru dimajukan setelah SEMUA
/// rentang koin itu berhasil disimpan, jadi sinkron yang gagal bisa diulang.
/// Simbol yang ditolak Indodax dilewati (dikembalikan di daftar kedua), bukan
/// menghentikan sinkron koin lain. Mengembalikan (jumlah trade baru, simbol dilewati).
pub fn sinkron_koin<S: SumberTrade>(
    s: &S,
    simpan: &mut Penyimpanan,
    koin: &[String],
    sekarang: u64,
    opsi: &Opsi,
) -> Hasil<(usize, Vec<String>)> {
    let mut baru = 0usize;
    let mut dilewati: Vec<String> = Vec::new();
    for simbol in koin {
        let kunci = kunci_meta(simbol);
        let mut mulai = match simpan.meta(&kunci)?.and_then(|v| v.parse::<u64>().ok()) {
            Some(terakhir) => terakhir.saturating_sub(TUMPANG_TINDIH_MS),
            None => sekarang.saturating_sub(opsi.hari_awal as u64 * HARI_MS),
        };
        let mut semua: Vec<Trade> = Vec::new();
        let mut ditolak = false;
        while mulai < sekarang {
            let akhir = (mulai + JENDELA_MS).min(sekarang);
            match ambil_rentang(s, simbol, mulai, akhir, opsi, &mut semua) {
                Ok(()) => {}
                Err(e) if simbol_ditolak(&e) => {
                    ditolak = true;
                    break;
                }
                Err(e) => return Err(e),
            }
            mulai = akhir + 1;
        }
        if ditolak {
            dilewati.push(simbol.clone());
            continue;
        }
        baru += simpan.simpan_banyak(&semua)?;
        simpan.atur_meta(&kunci, &sekarang.to_string())?;
    }
    Ok((baru, dilewati))
}

/// Sinkron lengkap: validasi key baca saja, snapshot saldo awal bila belum ada,
/// tentukan koin, tarik trade, simpan.
pub fn sinkron(klien: &IndodaxClient, simpan: &mut Penyimpanan, opsi: &Opsi) -> Hasil<HasilSinkron> {
    let akun = klien.ambil_akun()?;
    if !akun.kunci_baca_saja() {
        return Err(Error::KunciBukanBacaSaja);
    }
    let sekarang = sekarang_ms();

    // FR-03: saldo awal = saldo IDR (tersedia + terkunci) saat pertama sinkron.
    let mut snapshot_dibuat = false;
    if simpan.snapshot_awal()?.is_none() {
        simpan.tambah_snapshot("awal", sekarang as i64, akun.saldo_idr_total()?)?;
        snapshot_dibuat = true;
    }

    // Koin yang dipindai: saldo koin tidak nol + koin yang sudah ada di
    // riwayat tersimpan + koin tambahan dari pengguna.
    let mut daftar: BTreeSet<String> = BTreeSet::new();
    for s in &akun.balances {
        if !s.asset.eq_ignore_ascii_case("idr") && !s.total().is_zero() {
            daftar.insert(format!("{}idr", s.asset.to_lowercase()));
        }
    }
    for s in simpan.simbol_tersimpan()? {
        daftar.insert(s);
    }
    for s in &opsi.koin_tambahan {
        let s = s.trim().to_lowercase();
        if !s.is_empty() {
            daftar.insert(s);
        }
    }
    let daftar: Vec<String> = daftar.into_iter().collect();

    let (baru, dilewati) = sinkron_koin(klien, simpan, &daftar, sekarang, opsi)?;
    simpan.atur_meta("sinkron_terakhir", &sekarang.to_string())?;
    Ok(HasilSinkron { koin_dipindai: daftar, trade_baru: baru, koin_dilewati: dilewati, snapshot_awal_dibuat: snapshot_dibuat })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn trade(id: &str, simbol: &str, t: i64) -> Trade {
        Trade {
            trade_id: id.into(),
            order_id: format!("o{id}"),
            symbol: simbol.into(),
            price: d("100"),
            qty: d("1"),
            quote_qty: d("100"),
            commission: d("1"),
            commission_asset: "idr".into(),
            is_buyer: true,
            is_maker: false,
            time: t,
        }
    }

    /// Sumber palsu: menyaring berdasarkan simbol dan rentang waktu (inklusif),
    /// memotong sebanyak `batas`, dan mencatat setiap permintaan.
    struct Palsu {
        data: Vec<Trade>,
        panggilan: RefCell<Vec<(u64, u64)>>,
    }

    impl SumberTrade for Palsu {
        fn ambil_trade(&self, simbol: &str, mulai: u64, akhir: u64, batas: u32) -> Hasil<Vec<Trade>> {
            self.panggilan.borrow_mut().push((mulai, akhir));
            Ok(self
                .data
                .iter()
                .filter(|t| t.symbol == simbol && (t.time as u64) >= mulai && (t.time as u64) <= akhir)
                .take(batas as usize)
                .cloned()
                .collect())
        }
    }

    fn sinkron_baru<S: SumberTrade>(s: &S, p: &mut Penyimpanan, k: &[String], t: u64, o: &Opsi) -> Hasil<usize> {
        sinkron_koin(s, p, k, t, o).map(|(n, _)| n)
    }

    /// Sumber yang selalu menolak simbol seperti Indodax (HTTP 400, kode 1109).
    struct TolakSemua;
    impl SumberTrade for TolakSemua {
        fn ambil_trade(&self, _: &str, _: u64, _: u64, _: u32) -> Hasil<Vec<Trade>> {
            Err(Error::Http {
                kode: 400,
                isi_awal: r#"{"code":1109,"error":"invalid parameter value"}"#.into(),
            })
        }
    }

    #[test]
    fn simbol_tidak_valid_dilewati_bukan_menggagalkan_sinkron() {
        let mut s = Penyimpanan::dalam_memori().unwrap();
        let (n, lewat) = sinkron_koin(&TolakSemua, &mut s, &["usdtidr".to_string()], SEKARANG, &opsi()).unwrap();
        assert_eq!(n, 0);
        assert_eq!(lewat, vec!["usdtidr"]);
        assert_eq!(s.meta("sinkron:usdtidr").unwrap(), None);
    }

    fn opsi() -> Opsi {
        Opsi { hari_awal: 7, jeda_ms: 0, batas: 100, koin_tambahan: vec![] }
    }

    const SEKARANG: u64 = 100 * HARI_MS;

    #[test]
    fn sinkron_pertama_menyimpan_dan_mencatat_batas_waktu() {
        let palsu = Palsu {
            data: vec![trade("1", "icntidr", (SEKARANG - HARI_MS) as i64), trade("2", "icntidr", (SEKARANG - 2 * HARI_MS) as i64)],
            panggilan: RefCell::new(vec![]),
        };
        let mut s = Penyimpanan::dalam_memori().unwrap();
        let n = sinkron_baru(&palsu, &mut s, &["icntidr".to_string()], SEKARANG, &opsi()).unwrap();
        assert_eq!(n, 2);
        assert_eq!(s.jumlah_trade().unwrap(), 2);
        assert_eq!(s.meta("sinkron:icntidr").unwrap(), Some(SEKARANG.to_string()));
    }

    #[test]
    fn sinkron_ulang_tidak_membuat_catatan_ganda() {
        let palsu = Palsu {
            data: vec![trade("1", "icntidr", (SEKARANG - HARI_MS) as i64)],
            panggilan: RefCell::new(vec![]),
        };
        let mut s = Penyimpanan::dalam_memori().unwrap();
        let koin = ["icntidr".to_string()];
        assert_eq!(sinkron_baru(&palsu, &mut s, &koin, SEKARANG, &opsi()).unwrap(), 1);
        // sinkron kedua: tumpang tindih 1 menit, tradenya jauh sebelum itu
        assert_eq!(sinkron_baru(&palsu, &mut s, &koin, SEKARANG + 5 * 60_000, &opsi()).unwrap(), 0);
        assert_eq!(s.jumlah_trade().unwrap(), 1);
    }

    #[test]
    fn trade_yang_terlewat_dalam_tumpang_tindih_tetap_masuk_tanpa_dobel() {
        let t_akhir = (SEKARANG - 30_000) as i64; // 30 detik sebelum sinkron pertama
        let palsu = Palsu { data: vec![trade("1", "icntidr", t_akhir)], panggilan: RefCell::new(vec![]) };
        let mut s = Penyimpanan::dalam_memori().unwrap();
        let koin = ["icntidr".to_string()];
        sinkron_baru(&palsu, &mut s, &koin, SEKARANG, &opsi()).unwrap();
        let n = sinkron_baru(&palsu, &mut s, &koin, SEKARANG + 60_000, &opsi()).unwrap();
        assert_eq!(n, 0); // terambil lagi lewat tumpang tindih, tapi tidak ganda
        assert_eq!(s.jumlah_trade().unwrap(), 1);
    }

    #[test]
    fn rentang_lebih_dari_6_hari_dipecah() {
        let palsu = Palsu { data: vec![], panggilan: RefCell::new(vec![]) };
        let mut s = Penyimpanan::dalam_memori().unwrap();
        let o = Opsi { hari_awal: 14, ..opsi() };
        sinkron_baru(&palsu, &mut s, &["icntidr".to_string()], SEKARANG, &o).unwrap();
        let p = palsu.panggilan.borrow();
        assert_eq!(p.len(), 3); // 14 hari = 6 + 6 + 2
        assert!(p.iter().all(|(m, a)| a - m <= JENDELA_MS));
    }

    #[test]
    fn hasil_penuh_membelah_rentang_dan_tidak_kehilangan_trade() {
        let data: Vec<Trade> = (0..5)
            .map(|i| trade(&i.to_string(), "icntidr", (SEKARANG - (i + 1) * 3_600_000) as i64))
            .collect();
        let palsu = Palsu { data, panggilan: RefCell::new(vec![]) };
        let mut s = Penyimpanan::dalam_memori().unwrap();
        let o = Opsi { batas: 2, ..opsi() }; // paksa sering penuh
        let n = sinkron_baru(&palsu, &mut s, &["icntidr".to_string()], SEKARANG, &o).unwrap();
        assert_eq!(n, 5);
    }

    #[test]
    fn rentang_minimal_tetap_penuh_menjadi_error_bukan_data_hilang() {
        // 3 trade pada milidetik yang sama, batas 2: tidak mungkin dibelah.
        let data: Vec<Trade> = (0..3).map(|i| trade(&i.to_string(), "icntidr", (SEKARANG - 1000) as i64)).collect();
        let palsu = Palsu { data, panggilan: RefCell::new(vec![]) };
        let mut s = Penyimpanan::dalam_memori().unwrap();
        let o = Opsi { batas: 2, hari_awal: 1, ..opsi() };
        let r = sinkron_baru(&palsu, &mut s, &["icntidr".to_string()], SEKARANG, &o);
        assert!(matches!(r, Err(Error::RentangTerlaluPadat { .. })));
        assert_eq!(s.meta("sinkron:icntidr").unwrap(), None); // batas waktu tidak dimajukan
    }

    #[test]
    fn koin_lain_tidak_tercampur() {
        let palsu = Palsu {
            data: vec![trade("1", "icntidr", (SEKARANG - 1000) as i64), trade("2", "dogeidr", (SEKARANG - 1000) as i64)],
            panggilan: RefCell::new(vec![]),
        };
        let mut s = Penyimpanan::dalam_memori().unwrap();
        let n = sinkron_baru(&palsu, &mut s, &["dogeidr".to_string()], SEKARANG, &opsi()).unwrap();
        assert_eq!(n, 1);
        assert_eq!(s.simbol_tersimpan().unwrap(), vec!["dogeidr"]);
    }
}
