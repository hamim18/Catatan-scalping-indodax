//! T9.2: jembatan Screener. HANYA informasi: membaca data pasar publik (tanpa key), menyaring,
//! dan mengembalikan baris siap tampil. Tidak menyentuh posisi, profit, target, grafik,
//! statistik, atau database trade (database hanya dibaca untuk setelan ambang dan daftar koin
//! yang sedang dipegang, untuk penanda). Logika ada di `scalping_core::screener`.
//! Semua fungsi jaringan memblokir; panggil dari thread latar.
use super::{desimal, fmt_harga, gagal, hitung_aplikasi, nama_koin, sekarang_ms, ErrorApp};
use rust_decimal::Decimal;
use scalping_core::{
    cari_pemicu, muat_konfig, muat_konfig_lonjakan, nilai_lonjakan, nilai_tahap2, pair_path, parse_kedalaman,
    parse_ringkasan, parse_transaksi, saring_tahap1, simpan_konfig, simpan_konfig_lonjakan, urutkan,
    urutkan_lonjakan, BarisLonjakan, BarisScreener, IndodaxClient, KandidatScreener, KedalamanPasar,
    KonfigLonjakan, KonfigScreener, Penyimpanan, RiwayatHarga, TransaksiPublik, UrutanScreener,
};
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

/// Batas kandidat yang diperiksa di tahap 2 per pemindaian (urut volume 24 jam menurun).
/// Menjaga batas 300 permintaan per menit dan pemakaian data seluler: satu pemindaian
/// <= 40 permintaan `trades` (+ 40 `depth` bila filter kedalaman aktif).
const MAKS_KANDIDAT: usize = 40;
/// Permintaan tahap 2 yang berjalan bersamaan (sama dengan skrip Python pengguna).
const PEKERJA: usize = 4;

// ---------------------------------------------------------------------------
// Setelan ambang
// ---------------------------------------------------------------------------

/// Semua ambang sebagai teks, supaya Kotlin tidak menghitung apa pun. Teks kosong = filter mati
/// (untuk jendela, jarak kedalaman, dan biaya round-trip: wajib diisi). Angka berformat titik
/// desimal; masukan boleh memakai koma.
#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct SetelanScreenerUi {
    pub harga_min: String,
    pub harga_maks: String,
    pub vol_min: String,
    pub vol_maks: String,
    pub trx_mnt_min: String,
    pub jendela_menit: String,
    pub spread_maks: String,
    pub range24_min: String,
    pub usia_trx_maks: String,
    pub gerak_min: String,
    pub menit_aktif_min: String,
    pub arah_min: String,
    pub kedalaman_kelipatan: String,
    pub kedalaman_persen: String,
    pub dominasi_tanda: String,
    pub biaya_roundtrip: String,
    /// "skor" atau "trx"
    pub urutan: String,
}

fn teks_opsi(v: Option<Decimal>) -> String {
    v.map(|d| d.normalize().to_string()).unwrap_or_default()
}

fn ke_ui(k: &KonfigScreener) -> SetelanScreenerUi {
    SetelanScreenerUi {
        harga_min: teks_opsi(k.harga_min),
        harga_maks: teks_opsi(k.harga_maks),
        vol_min: teks_opsi(k.vol_min),
        vol_maks: teks_opsi(k.vol_maks),
        trx_mnt_min: teks_opsi(k.trx_mnt_min),
        jendela_menit: k.jendela_menit.to_string(),
        spread_maks: teks_opsi(k.spread_maks),
        range24_min: teks_opsi(k.range24_min),
        usia_trx_maks: k.usia_trx_maks_detik.map(|u| u.to_string()).unwrap_or_default(),
        gerak_min: teks_opsi(k.gerak_min_persen),
        menit_aktif_min: teks_opsi(k.menit_aktif_min_persen),
        arah_min: teks_opsi(k.arah_min_persen),
        kedalaman_kelipatan: teks_opsi(k.kedalaman_kelipatan),
        kedalaman_persen: k.kedalaman_persen.normalize().to_string(),
        dominasi_tanda: teks_opsi(k.dominasi_tanda_persen),
        biaya_roundtrip: k.biaya_roundtrip_persen.normalize().to_string(),
        urutan: match k.urutan {
            UrutanScreener::Skor => "skor".to_string(),
            UrutanScreener::TrxPerMenit => "trx".to_string(),
        },
    }
}

fn opsi_dec(t: &str) -> Result<Option<Decimal>, ErrorApp> {
    let t = t.trim();
    if t.is_empty() {
        Ok(None)
    } else {
        desimal(t).map(Some)
    }
}

fn wajib_dec(nama: &str, t: &str) -> Result<Decimal, ErrorApp> {
    opsi_dec(t)?.ok_or_else(|| ErrorApp::Gagal { pesan: format!("{nama} wajib diisi") })
}

fn dari_ui(s: &SetelanScreenerUi) -> Result<KonfigScreener, ErrorApp> {
    let jendela_menit = s
        .jendela_menit
        .trim()
        .parse::<u32>()
        .map_err(|_| ErrorApp::Gagal { pesan: "Jendela harus bilangan bulat (menit)".into() })?;
    let usia = s.usia_trx_maks.trim();
    let usia_trx_maks_detik = if usia.is_empty() {
        None
    } else {
        Some(
            usia.parse::<i64>()
                .map_err(|_| ErrorApp::Gagal { pesan: "Usia transaksi harus bilangan bulat (detik)".into() })?,
        )
    };
    let urutan = match s.urutan.trim() {
        "skor" => UrutanScreener::Skor,
        "trx" => UrutanScreener::TrxPerMenit,
        lain => return Err(ErrorApp::Gagal { pesan: format!("urutan tidak dikenal: {lain}") }),
    };
    Ok(KonfigScreener {
        harga_min: opsi_dec(&s.harga_min)?,
        harga_maks: opsi_dec(&s.harga_maks)?,
        vol_min: opsi_dec(&s.vol_min)?,
        vol_maks: opsi_dec(&s.vol_maks)?,
        trx_mnt_min: opsi_dec(&s.trx_mnt_min)?,
        jendela_menit,
        spread_maks: opsi_dec(&s.spread_maks)?,
        range24_min: opsi_dec(&s.range24_min)?,
        usia_trx_maks_detik,
        gerak_min_persen: opsi_dec(&s.gerak_min)?,
        menit_aktif_min_persen: opsi_dec(&s.menit_aktif_min)?,
        arah_min_persen: opsi_dec(&s.arah_min)?,
        kedalaman_kelipatan: opsi_dec(&s.kedalaman_kelipatan)?,
        kedalaman_persen: wajib_dec("Jarak kedalaman", &s.kedalaman_persen)?,
        dominasi_tanda_persen: opsi_dec(&s.dominasi_tanda)?,
        biaya_roundtrip_persen: wajib_dec("Biaya round-trip", &s.biaya_roundtrip)?,
        urutan,
    })
}

/// Setelan nilai awal (nilai skrip Python pengguna; filter tambahan mati). Tanpa database.
#[uniffi::export]
pub fn setelan_screener_bawaan() -> SetelanScreenerUi {
    ke_ui(&KonfigScreener::default())
}

/// Baca ambang tersimpan (atau bawaan bila belum pernah disimpan). Tanpa jaringan.
#[uniffi::export]
pub fn baca_setelan_screener(path_db: String) -> Result<SetelanScreenerUi, ErrorApp> {
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    Ok(ke_ui(&muat_konfig(&simpan).map_err(gagal)?))
}

/// Periksa dan simpan ambang. Galat berisi pesan Indonesia siap tampil. Tanpa jaringan.
#[uniffi::export]
pub fn simpan_setelan_screener(path_db: String, setelan: SetelanScreenerUi) -> Result<(), ErrorApp> {
    let konfig = dari_ui(&setelan)?;
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    simpan_konfig(&simpan, &konfig).map_err(gagal)
}

// ---------------------------------------------------------------------------
// Pemindaian
// ---------------------------------------------------------------------------

/// Satu baris daftar. Angka persen dan trx/mnt MENTAH (titik desimal, tanpa simbol) agar Kotlin
/// memakai `Format` yang sama dengan layar lain; hanya `harga` yang sudah berformat Indonesia.
#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct BarisScreenerUi {
    /// contoh "abc_idr"
    pub pair: String,
    /// contoh "ABC"
    pub nama: String,
    /// contoh "3.480" atau "12,5"
    pub harga: String,
    /// contoh "12.3"
    pub trx_mnt: String,
    /// persen 2 desimal, contoh "1.25"; kosong bila tidak ada
    pub spread: String,
    /// gerak harga di jendela, persen 2 desimal; kosong bila tidak ada
    pub gerak: String,
    /// persen transaksi beli, bulat, contoh "65"
    pub buy_persen: String,
    /// satu transaksi mendominasi nilai jendela (hanya tanda peringatan)
    pub dominasi_tinggi: bool,
    /// koin ini sedang menjadi posisi terbuka (hanya penanda; tidak memengaruhi posisi)
    pub dipegang: bool,
}

#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct HasilScreenerUi {
    pub baris: Vec<BarisScreenerUi>,
    /// waktu HP saat pemindaian selesai (hanya untuk teks status)
    pub waktu_ms: i64,
    /// jumlah pair yang lolos tahap 1
    pub jumlah_kandidat: i32,
    /// jumlah yang benar-benar diperiksa di tahap 2 (dibatasi MAKS_KANDIDAT)
    pub jumlah_diperiksa: i32,
    /// pair yang gagal diambil riwayat transaksinya (dilewati)
    pub jumlah_gagal: i32,
}

fn opsi_persen(v: Option<Decimal>) -> String {
    v.map(|d| d.round_dp(2).normalize().to_string()).unwrap_or_default()
}

fn ke_baris_ui(b: &BarisScreener, dipegang: &HashSet<String>) -> BarisScreenerUi {
    BarisScreenerUi {
        pair: b.pair.clone(),
        nama: nama_koin(&b.pair),
        harga: fmt_harga(b.harga),
        trx_mnt: b.metrik.trx_per_menit.round_dp(1).normalize().to_string(),
        spread: opsi_persen(b.spread),
        gerak: opsi_persen(b.metrik.gerak_persen),
        buy_persen: b.metrik.buy_persen.round_dp(0).normalize().to_string(),
        dominasi_tinggi: b.dominasi_tinggi,
        dipegang: dipegang.contains(&pair_path(&b.pair)),
    }
}

type DataPair = (Vec<TransaksiPublik>, Option<KedalamanPasar>);

/// Ambil riwayat transaksi (dan kedalaman bila diminta) untuk tiap kandidat, `PEKERJA` sekaligus.
/// Urutan hasil sama dengan `kand`. None = riwayat transaksi pair itu gagal diambil.
/// Kedalaman yang gagal diambil menjadi None (filter kedalaman dilewati untuk pair itu).
fn ambil_paralel(klien: &IndodaxClient, kand: &[KandidatScreener], dengan_kedalaman: bool) -> Vec<Option<DataPair>> {
    let hasil: Mutex<Vec<Option<DataPair>>> = Mutex::new((0..kand.len()).map(|_| None).collect());
    let berikut = AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..PEKERJA.min(kand.len()) {
            s.spawn(|| loop {
                let i = berikut.fetch_add(1, Ordering::SeqCst);
                if i >= kand.len() {
                    break;
                }
                let pair = &kand[i].ringkasan.pair;
                let trx = klien.ambil_transaksi_mentah(pair).and_then(|t| parse_transaksi(&t)).ok();
                let data = trx.map(|t| {
                    let kedalaman = if dengan_kedalaman {
                        klien.ambil_kedalaman_mentah(pair).and_then(|d| parse_kedalaman(&d)).ok()
                    } else {
                        None
                    };
                    (t, kedalaman)
                });
                if let Ok(mut g) = hasil.lock() {
                    g[i] = data;
                }
            });
        }
    });
    hasil.into_inner().unwrap_or_else(|e| e.into_inner())
}

/// Koin yang sedang menjadi posisi terbuka, dalam bentuk path pair ("icntidr"). Hanya baca;
/// gagal membaca = tanpa penanda (tidak menggagalkan pemindaian).
fn pair_dipegang(simpan: &Penyimpanan) -> HashSet<String> {
    match hitung_aplikasi(simpan) {
        Ok(h) => h.posisi.iter().map(|p| pair_path(&p.simbol.to_lowercase())).collect(),
        Err(_) => HashSet::new(),
    }
}

/// Satu pemindaian: ringkasan pasar -> tahap 1 -> riwayat transaksi kandidat -> tahap 2 -> urut.
/// `modal_trade`: isian opsional "Modal trade ini" (rupiah, tidak disimpan); kosong = filter
/// kedalaman dilewati. Galat bila ringkasan pasar gagal diambil; kegagalan per pair hanya dihitung.
#[uniffi::export]
pub fn jalankan_screener(path_db: String, modal_trade: String) -> Result<HasilScreenerUi, ErrorApp> {
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    let konfig = muat_konfig(&simpan).map_err(gagal)?;
    let dipegang = pair_dipegang(&simpan);
    let modal = opsi_dec(&modal_trade)?;

    let klien = IndodaxClient::new("", "");
    let ringkasan = parse_ringkasan(&klien.ambil_ringkasan_mentah().map_err(gagal)?).map_err(gagal)?;
    let mut kand = saring_tahap1(&konfig, &ringkasan);
    let jumlah_kandidat = kand.len();
    kand.sort_by(|a, b| b.ringkasan.vol_idr.cmp(&a.ringkasan.vol_idr));
    kand.truncate(MAKS_KANDIDAT);

    let pakai_kedalaman = konfig.kedalaman_kelipatan.is_some() && modal.map(|m| m > Decimal::ZERO).unwrap_or(false);
    let data = ambil_paralel(&klien, &kand, pakai_kedalaman);
    let sekarang_detik = sekarang_ms() / 1000;

    let mut jumlah_gagal = 0_i32;
    let mut baris: Vec<BarisScreener> = Vec::new();
    for (k, d) in kand.iter().zip(data) {
        match d {
            None => jumlah_gagal += 1,
            Some((trx, kedalaman)) => {
                if let Some(b) = nilai_tahap2(&konfig, k, &trx, sekarang_detik, kedalaman.as_ref(), modal) {
                    baris.push(b);
                }
            }
        }
    }
    urutkan(&konfig, &mut baris);

    Ok(HasilScreenerUi {
        baris: baris.iter().map(|b| ke_baris_ui(b, &dipegang)).collect(),
        waktu_ms: sekarang_ms(),
        jumlah_kandidat: jumlah_kandidat as i32,
        jumlah_diperiksa: kand.len() as i32,
        jumlah_gagal,
    })
}

// ---------------------------------------------------------------------------
// T9.4: filter Lonjakan (tanpa layanan, tanpa notifikasi)
// ---------------------------------------------------------------------------

/// Ambang Lonjakan sebagai teks (Kotlin tidak menghitung apa pun). Semua isian wajib diisi.
#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct SetelanLonjakanUi {
    pub naik: bool,
    pub turun: bool,
    pub ambang_persen: String,
    pub trx_mnt_min: String,
    pub jendela_menit: String,
    pub dasar_menit: String,
    pub pemanasan_menit: String,
    pub maks_periksa: String,
    pub interval_detik: String,
}

fn ke_ui_lonjakan(k: &KonfigLonjakan) -> SetelanLonjakanUi {
    SetelanLonjakanUi {
        naik: k.naik,
        turun: k.turun,
        ambang_persen: k.ambang_persen.normalize().to_string(),
        trx_mnt_min: k.trx_mnt_min.normalize().to_string(),
        jendela_menit: k.jendela_menit.to_string(),
        dasar_menit: k.dasar_menit.to_string(),
        pemanasan_menit: k.pemanasan_menit.to_string(),
        maks_periksa: k.maks_periksa.to_string(),
        interval_detik: k.interval_detik.to_string(),
    }
}

fn bulat_u32(nama: &str, t: &str) -> Result<u32, ErrorApp> {
    t.trim()
        .parse::<u32>()
        .map_err(|_| ErrorApp::Gagal { pesan: format!("{nama} harus bilangan bulat") })
}

fn dari_ui_lonjakan(s: &SetelanLonjakanUi) -> Result<KonfigLonjakan, ErrorApp> {
    Ok(KonfigLonjakan {
        naik: s.naik,
        turun: s.turun,
        ambang_persen: wajib_dec("Lonjakan minimum", &s.ambang_persen)?,
        trx_mnt_min: wajib_dec("Trx/mnt minimum", &s.trx_mnt_min)?,
        jendela_menit: bulat_u32("Jendela pendek", &s.jendela_menit)?,
        dasar_menit: bulat_u32("Rata-rata harga dasar", &s.dasar_menit)?,
        pemanasan_menit: bulat_u32("Pemanasan", &s.pemanasan_menit)?,
        maks_periksa: bulat_u32("Maksimal diperiksa", &s.maks_periksa)?,
        interval_detik: bulat_u32("Interval", &s.interval_detik)?,
    })
}

/// Nilai awal ambang Lonjakan (sementara; angka final dari uji lapangan). Tanpa database.
#[uniffi::export]
pub fn setelan_lonjakan_bawaan() -> SetelanLonjakanUi {
    ke_ui_lonjakan(&KonfigLonjakan::default())
}

/// Baca ambang Lonjakan tersimpan (atau bawaan). Tanpa jaringan.
#[uniffi::export]
pub fn baca_setelan_lonjakan(path_db: String) -> Result<SetelanLonjakanUi, ErrorApp> {
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    Ok(ke_ui_lonjakan(&muat_konfig_lonjakan(&simpan).map_err(gagal)?))
}

/// Periksa dan simpan ambang Lonjakan. Galat berisi pesan Indonesia siap tampil. Tanpa jaringan.
#[uniffi::export]
pub fn simpan_setelan_lonjakan(path_db: String, setelan: SetelanLonjakanUi) -> Result<(), ErrorApp> {
    let konfig = dari_ui_lonjakan(&setelan)?;
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    simpan_konfig_lonjakan(&simpan, &konfig).map_err(gagal)
}

#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct BarisLonjakanUi {
    pub pair: String,
    pub nama: String,
    /// sudah berformat Indonesia
    pub harga: String,
    /// harga dasar (rata-rata), berformat Indonesia
    pub dasar: String,
    /// penyimpangan dari harga dasar, nilai MUTLAK persen 2 desimal, contoh "3.21"
    pub perubahan: String,
    /// true = ▲ (di atas harga dasar), false = ▼
    pub naik: bool,
    pub trx_mnt: String,
    /// persen 2 desimal; kosong bila tidak ada
    pub spread: String,
    /// lama data harga dasar, menit bulat
    pub lama_dasar_menit: String,
    pub dominasi_tinggi: bool,
    pub dipegang: bool,
}

#[derive(uniffi::Record, Clone, Debug, PartialEq)]
pub struct HasilLonjakanUi {
    pub baris: Vec<BarisLonjakanUi>,
    pub waktu_ms: i64,
    /// rentang data riwayat tertua, menit (untuk status pemanasan)
    pub rentang_menit: i32,
    pub pemanasan_menit: i32,
    /// kandidat yang sudah punya harga dasar; 0 = masih pemanasan
    pub jumlah_punya_dasar: i32,
    /// pair yang melewati separuh ambang (sebelum dipotong)
    pub jumlah_pemicu: i32,
    /// yang benar-benar diperiksa lewat `trades`
    pub jumlah_diperiksa: i32,
    pub jumlah_gagal: i32,
}

fn ke_baris_lonjakan_ui(b: &BarisLonjakan, dipegang: &HashSet<String>) -> BarisLonjakanUi {
    BarisLonjakanUi {
        pair: b.baris.pair.clone(),
        nama: nama_koin(&b.baris.pair),
        harga: fmt_harga(b.baris.harga),
        dasar: fmt_harga(b.dasar),
        perubahan: b.perubahan.abs().round_dp(2).normalize().to_string(),
        naik: b.naik(),
        trx_mnt: b.baris.metrik.trx_per_menit.round_dp(1).normalize().to_string(),
        spread: opsi_persen(b.baris.spread),
        lama_dasar_menit: (b.lama_dasar_detik / 60).to_string(),
        dominasi_tinggi: b.baris.dominasi_tinggi,
        dipegang: dipegang.contains(&pair_path(&b.baris.pair)),
    }
}

/// Riwayat harga di memori selama proses aplikasi hidup (diputuskan: tetap ada saat layar
/// ditutup; titik > 60 menit dibuang; celah tidak dihitung). Tidak pernah masuk database.
static RIWAYAT_LONJAKAN: OnceLock<Mutex<RiwayatHarga>> = OnceLock::new();

fn riwayat_lonjakan() -> &'static Mutex<RiwayatHarga> {
    RIWAYAT_LONJAKAN.get_or_init(|| Mutex::new(RiwayatHarga::baru()))
}

/// Satu siklus Lonjakan: ringkasan pasar -> pemicu (dasar rata-rata 1 jam) -> `trades` pair
/// pemicu (maks `maks_periksa`) -> tahap 2 -> urut. Titik harga siklus ini disimpan SETELAH
/// pemicu dihitung. Galat bila ringkasan pasar gagal diambil (riwayat tidak berubah).
#[uniffi::export]
pub fn pindai_lonjakan(path_db: String) -> Result<HasilLonjakanUi, ErrorApp> {
    let simpan = Penyimpanan::buka(&path_db).map_err(gagal)?;
    let ks = muat_konfig(&simpan).map_err(gagal)?;
    let kl = muat_konfig_lonjakan(&simpan).map_err(gagal)?;
    let dipegang = pair_dipegang(&simpan);

    let klien = IndodaxClient::new("", "");
    let ringkasan = parse_ringkasan(&klien.ambil_ringkasan_mentah().map_err(gagal)?).map_err(gagal)?;
    let sekarang: i64 = ringkasan
        .iter()
        .filter_map(|r| r.server_time)
        .max()
        .unwrap_or_else(|| sekarang_ms() / 1000);

    let (pemicu, rentang) = {
        let mut rw = riwayat_lonjakan().lock().unwrap_or_else(|e| e.into_inner());
        let hasil = cari_pemicu(&ks, &kl, &rw, &ringkasan, sekarang);
        let rentang = rw.rentang_detik(sekarang);
        rw.tambah(sekarang, &ringkasan);
        (hasil, rentang)
    };

    let kand: Vec<KandidatScreener> = pemicu.pemicu.iter().map(|p| p.kandidat.clone()).collect();
    let data = ambil_paralel(&klien, &kand, false);
    let mut jumlah_gagal = 0_i32;
    let mut baris: Vec<BarisLonjakan> = Vec::new();
    for (p, d) in pemicu.pemicu.iter().zip(data) {
        match d {
            None => jumlah_gagal += 1,
            Some((trx, _)) => {
                if let Some(b) = nilai_lonjakan(&ks, &kl, p, &trx, sekarang) {
                    baris.push(b);
                }
            }
        }
    }
    urutkan_lonjakan(&mut baris);

    Ok(HasilLonjakanUi {
        baris: baris.iter().map(|b| ke_baris_lonjakan_ui(b, &dipegang)).collect(),
        waktu_ms: sekarang_ms(),
        rentang_menit: (rentang / 60) as i32,
        pemanasan_menit: kl.pemanasan_menit as i32,
        jumlah_punya_dasar: pemicu.jumlah_punya_dasar as i32,
        jumlah_pemicu: pemicu.jumlah_total as i32,
        jumlah_diperiksa: kand.len() as i32,
        jumlah_gagal,
    })
}

#[cfg(test)]
mod tes_screener_ffi {
    use super::*;
    use scalping_core::MetrikPair;

    fn contoh_baris(pair: &str) -> BarisScreener {
        let mut m = MetrikPair::kosong(600);
        m.trx_per_menit = Decimal::new(1234, 2); // 12,34
        m.gerak_persen = Some(Decimal::new(2456, 3)); // 2,456
        m.buy_persen = Decimal::new(654, 1); // 65,4
        BarisScreener {
            pair: pair.to_string(),
            harga: Decimal::from(3480_i64),
            vol_idr: Decimal::from(50_000_000_i64),
            spread: Some(Decimal::new(125, 2)), // 1,25
            range24: Decimal::from(5_i64),
            perubahan24: None,
            metrik: m,
            skor: None,
            kedalaman_bid_idr: None,
            kedalaman_ask_idr: None,
            dominasi_tinggi: true,
        }
    }

    #[test]
    fn baris_ui_berformat_dan_penanda_dipegang() {
        let mut dipegang = HashSet::new();
        let b = ke_baris_ui(&contoh_baris("abc_idr"), &dipegang);
        assert_eq!(b.nama, "ABC");
        assert_eq!(b.harga, "3.480");
        assert_eq!(b.trx_mnt, "12.3");
        assert_eq!(b.spread, "1.25");
        assert_eq!(b.gerak, "2.46");
        assert_eq!(b.buy_persen, "65");
        assert!(b.dominasi_tinggi);
        assert!(!b.dipegang);
        dipegang.insert("abcidr".to_string());
        assert!(ke_baris_ui(&contoh_baris("abc_idr"), &dipegang).dipegang);
        assert!(!ke_baris_ui(&contoh_baris("xyz_idr"), &dipegang).dipegang);
    }

    #[test]
    fn baris_ui_tanpa_spread_dan_gerak_kosong() {
        let mut b = contoh_baris("abc_idr");
        b.spread = None;
        b.metrik.gerak_persen = None;
        let u = ke_baris_ui(&b, &HashSet::new());
        assert_eq!(u.spread, "");
        assert_eq!(u.gerak, "");
    }

    #[test]
    fn setelan_ui_bolak_balik_sama_dengan_bawaan() {
        let ui = setelan_screener_bawaan();
        assert_eq!(ui.harga_min, "10");
        assert_eq!(ui.vol_maks, "10000000000");
        assert_eq!(ui.jendela_menit, "10");
        assert_eq!(ui.kedalaman_persen, "0.5");
        assert_eq!(ui.gerak_min, "");
        assert_eq!(ui.urutan, "skor");
        assert_eq!(dari_ui(&ui).unwrap(), KonfigScreener::default());
    }

    #[test]
    fn dari_ui_kosong_berarti_mati_dan_koma_diterima() {
        let mut ui = setelan_screener_bawaan();
        ui.harga_min = String::new();
        ui.trx_mnt_min = "3,5".to_string();
        ui.usia_trx_maks = String::new();
        let k = dari_ui(&ui).unwrap();
        assert_eq!(k.harga_min, None);
        assert_eq!(k.trx_mnt_min, Some(Decimal::new(35, 1)));
        assert_eq!(k.usia_trx_maks_detik, None);
    }

    #[test]
    fn dari_ui_menolak_masukan_rusak() {
        let mut ui = setelan_screener_bawaan();
        ui.harga_min = "abc".to_string();
        assert!(dari_ui(&ui).is_err());

        let mut ui = setelan_screener_bawaan();
        ui.jendela_menit = String::new();
        assert!(dari_ui(&ui).is_err());

        let mut ui = setelan_screener_bawaan();
        ui.kedalaman_persen = String::new();
        assert!(dari_ui(&ui).is_err());

        let mut ui = setelan_screener_bawaan();
        ui.urutan = "aneh".to_string();
        assert!(dari_ui(&ui).is_err());
    }

    #[test]
    fn setelan_lonjakan_bolak_balik_sama_dengan_bawaan() {
        let ui = setelan_lonjakan_bawaan();
        assert!(ui.naik && ui.turun);
        assert_eq!(ui.ambang_persen, "2");
        assert_eq!(ui.dasar_menit, "60");
        assert_eq!(ui.interval_detik, "20");
        assert_eq!(dari_ui_lonjakan(&ui).unwrap(), KonfigLonjakan::default());
    }

    #[test]
    fn dari_ui_lonjakan_menerima_koma_dan_menolak_rusak() {
        let mut ui = setelan_lonjakan_bawaan();
        ui.ambang_persen = "2,5".to_string();
        assert_eq!(dari_ui_lonjakan(&ui).unwrap().ambang_persen, Decimal::new(25, 1));

        let mut ui = setelan_lonjakan_bawaan();
        ui.interval_detik = "abc".to_string();
        assert!(dari_ui_lonjakan(&ui).is_err());

        let mut ui = setelan_lonjakan_bawaan();
        ui.ambang_persen = String::new();
        assert!(dari_ui_lonjakan(&ui).is_err());
    }

    #[test]
    fn baris_lonjakan_ui_berformat_dan_arah() {
        let mut dipegang = HashSet::new();
        dipegang.insert("abcidr".to_string());
        let b = BarisLonjakan {
            baris: contoh_baris("abc_idr"),
            dasar: Decimal::from(3400_i64),
            perubahan: Decimal::new(-3214, 3), // -3,214
            lama_dasar_detik: 1805,
        };
        let u = ke_baris_lonjakan_ui(&b, &dipegang);
        assert_eq!(u.nama, "ABC");
        assert_eq!(u.harga, "3.480");
        assert_eq!(u.dasar, "3.400");
        assert_eq!(u.perubahan, "3.21"); // nilai mutlak
        assert!(!u.naik);
        assert_eq!(u.lama_dasar_menit, "30");
        assert_eq!(u.spread, "1.25");
        assert!(u.dipegang);
        let naik = BarisLonjakan { perubahan: Decimal::from(2_i64), ..b };
        assert!(ke_baris_lonjakan_ui(&naik, &HashSet::new()).naik);
    }
}
