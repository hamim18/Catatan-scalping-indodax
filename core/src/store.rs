//! Penyimpanan lokal (SQLite) untuk T1.2. Data disimpan sebagai CATATAN
//! (trade per pengisian), bukan angka saldo mentah. Angka uang disimpan
//! sebagai teks desimal supaya tidak ada floating point.
//! Anti-dobel (FR-09): `trade_id` adalah PRIMARY KEY, INSERT OR IGNORE.
use crate::arus_kas::{ArusKas, JenisArusKas};
use crate::{Error, Hasil, Trade};
use rusqlite::{params, Connection, OptionalExtension, Row};
use rust_decimal::Decimal;
use std::str::FromStr;

const SKEMA: &str = "
CREATE TABLE IF NOT EXISTS trade (
    trade_id         TEXT PRIMARY KEY,
    order_id         TEXT NOT NULL,
    symbol           TEXT NOT NULL,
    price            TEXT NOT NULL,
    qty              TEXT NOT NULL,
    quote_qty        TEXT NOT NULL,
    commission       TEXT NOT NULL,
    commission_asset TEXT NOT NULL,
    is_buyer         INTEGER NOT NULL,
    is_maker         INTEGER NOT NULL,
    waktu_ms         INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_trade_waktu ON trade(waktu_ms);
CREATE TABLE IF NOT EXISTS meta (
    kunci TEXT PRIMARY KEY,
    nilai TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS setelan_tp_sl (
    berlaku_sejak_ms INTEGER PRIMARY KEY,
    tp_persen        TEXT NOT NULL,
    sl_persen        TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS snapshot (
    jenis    TEXT NOT NULL,
    waktu_ms INTEGER NOT NULL,
    saldo    TEXT NOT NULL,
    PRIMARY KEY (jenis, waktu_ms)
);
CREATE TABLE IF NOT EXISTS arus_kas (
    id_sumber TEXT PRIMARY KEY,
    jenis     TEXT NOT NULL,
    jumlah    TEXT NOT NULL,
    biaya     TEXT NOT NULL,
    waktu_ms  INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_arus_kas_waktu ON arus_kas(waktu_ms);
";

const META_TARGET_HARIAN: &str = "target_harian_persen";
const META_TARGET_MINGGUAN: &str = "target_mingguan_persen";
const META_AMOUNT_BERSIH: &str = "arus_kas_amount_sudah_bersih";
const META_ARUS_KAS_TERAKHIR: &str = "arus_kas_sinkron_terakhir_ms";

fn gagal(e: impl std::fmt::Display) -> Error {
    Error::Penyimpanan(e.to_string())
}

fn kolom_desimal(r: &Row, i: usize) -> rusqlite::Result<Decimal> {
    let s: String = r.get(i)?;
    Decimal::from_str(&s).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(i, rusqlite::types::Type::Text, Box::new(e))
    })
}

pub struct Penyimpanan {
    conn: Connection,
}

impl Penyimpanan {
    /// Buka (atau buat) file database di `path`.
    pub fn buka(path: &str) -> Hasil<Self> {
        Self::dari(Connection::open(path).map_err(gagal)?)
    }

    /// Database sementara di memori (untuk tes).
    pub fn dalam_memori() -> Hasil<Self> {
        Self::dari(Connection::open_in_memory().map_err(gagal)?)
    }

    fn dari(conn: Connection) -> Hasil<Self> {
        // Pantauan (menulis) dan tampilan (membaca) bisa membuka database bersamaan:
        // tunggu sampai 5 detik alih-alih langsung gagal "database is locked".
        conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(gagal)?;
        conn.execute_batch(SKEMA).map_err(gagal)?;
        Ok(Self { conn })
    }

    /// Simpan banyak trade dalam satu transaksi. Mengembalikan jumlah trade
    /// BARU (yang `trade_id`-nya belum ada). Mencatat ulang data yang sama
    /// tidak membuat catatan ganda.
    pub fn simpan_banyak(&mut self, trades: &[Trade]) -> Hasil<usize> {
        let tx = self.conn.transaction().map_err(gagal)?;
        let mut baru = 0usize;
        {
            let mut st = tx
                .prepare(
                    "INSERT OR IGNORE INTO trade
                     (trade_id, order_id, symbol, price, qty, quote_qty, commission,
                      commission_asset, is_buyer, is_maker, waktu_ms)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                )
                .map_err(gagal)?;
            for t in trades {
                baru += st
                    .execute(params![
                        t.trade_id,
                        t.order_id,
                        t.symbol,
                        t.price.to_string(),
                        t.qty.to_string(),
                        t.quote_qty.to_string(),
                        t.commission.to_string(),
                        t.commission_asset,
                        t.is_buyer,
                        t.is_maker,
                        t.time
                    ])
                    .map_err(gagal)?;
            }
        }
        tx.commit().map_err(gagal)?;
        Ok(baru)
    }

    /// Semua trade, urut waktu naik.
    pub fn semua_trade(&self) -> Hasil<Vec<Trade>> {
        let mut st = self
            .conn
            .prepare(
                "SELECT trade_id, order_id, symbol, price, qty, quote_qty, commission,
                        commission_asset, is_buyer, is_maker, waktu_ms
                 FROM trade ORDER BY waktu_ms ASC, trade_id ASC",
            )
            .map_err(gagal)?;
        let baris = st
            .query_map([], |r| {
                Ok(Trade {
                    trade_id: r.get(0)?,
                    order_id: r.get(1)?,
                    symbol: r.get(2)?,
                    price: kolom_desimal(r, 3)?,
                    qty: kolom_desimal(r, 4)?,
                    quote_qty: kolom_desimal(r, 5)?,
                    commission: kolom_desimal(r, 6)?,
                    commission_asset: r.get(7)?,
                    is_buyer: r.get(8)?,
                    is_maker: r.get(9)?,
                    time: r.get(10)?,
                })
            })
            .map_err(gagal)?;
        baris.collect::<Result<Vec<_>, _>>().map_err(gagal)
    }

    pub fn jumlah_trade(&self) -> Hasil<usize> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM trade", [], |r| r.get(0))
            .map_err(gagal)?;
        Ok(n as usize)
    }

    /// Simbol yang pernah muncul di trade tersimpan.
    pub fn simbol_tersimpan(&self) -> Hasil<Vec<String>> {
        let mut st = self
            .conn
            .prepare("SELECT DISTINCT symbol FROM trade ORDER BY symbol")
            .map_err(gagal)?;
        let baris = st.query_map([], |r| r.get::<_, String>(0)).map_err(gagal)?;
        baris.collect::<Result<Vec<_>, _>>().map_err(gagal)
    }

    pub fn meta(&self, kunci: &str) -> Hasil<Option<String>> {
        self.conn
            .query_row("SELECT nilai FROM meta WHERE kunci = ?1", [kunci], |r| r.get(0))
            .optional()
            .map_err(gagal)
    }

    pub fn atur_meta(&self, kunci: &str, nilai: &str) -> Hasil<()> {
        self.conn
            .execute(
                "INSERT INTO meta (kunci, nilai) VALUES (?1, ?2)
                 ON CONFLICT(kunci) DO UPDATE SET nilai = excluded.nilai",
                params![kunci, nilai],
            )
            .map_err(gagal)?;
        Ok(())
    }

    /// Catat setelan TP/SL (persen) yang berlaku sejak `waktu_ms`. Riwayat disimpan,
    /// supaya posisi lama tetap memakai persen saat ia dibuka (FR-05).
    pub fn tambah_setelan(&self, waktu_ms: i64, tp: Decimal, sl: Decimal) -> Hasil<()> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO setelan_tp_sl (berlaku_sejak_ms, tp_persen, sl_persen)
                 VALUES (?1, ?2, ?3)",
                params![waktu_ms, tp.to_string(), sl.to_string()],
            )
            .map_err(gagal)?;
        Ok(())
    }

    /// Riwayat setelan: (berlaku_sejak_ms, TP%, SL%), urut waktu naik.
    pub fn riwayat_setelan(&self) -> Hasil<Vec<(i64, Decimal, Decimal)>> {
        let mut st = self
            .conn
            .prepare("SELECT berlaku_sejak_ms, tp_persen, sl_persen FROM setelan_tp_sl ORDER BY berlaku_sejak_ms ASC")
            .map_err(gagal)?;
        let baris = st
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, kolom_desimal(r, 1)?, kolom_desimal(r, 2)?)))
            .map_err(gagal)?;
        baris.collect::<Result<Vec<_>, _>>().map_err(gagal)
    }

    /// Simpan target harian dan mingguan (persen). Disimpan di meta; hanya nilai terbaru
    /// yang dipakai (target tidak punya riwayat).
    pub fn atur_target(&self, harian: Decimal, mingguan: Decimal) -> Hasil<()> {
        self.atur_meta(META_TARGET_HARIAN, &harian.to_string())?;
        self.atur_meta(META_TARGET_MINGGUAN, &mingguan.to_string())
    }

    /// Target (harian %, mingguan %). None bila belum pernah diatur.
    pub fn target(&self) -> Hasil<(Option<Decimal>, Option<Decimal>)> {
        let baca = |kunci: &str| -> Hasil<Option<Decimal>> {
            match self.meta(kunci)? {
                None => Ok(None),
                Some(t) => Decimal::from_str(&t).map(Some).map_err(gagal),
            }
        };
        Ok((baca(META_TARGET_HARIAN)?, baca(META_TARGET_MINGGUAN)?))
    }

    /// T2.2: simpan arus kas (deposit/withdraw IDR) tanpa dobel (`id_sumber` unik).
    /// Mengembalikan jumlah baris BARU.
    pub fn simpan_arus_kas(&mut self, arus: &[ArusKas]) -> Hasil<usize> {
        let tx = self.conn.transaction().map_err(gagal)?;
        let mut baru = 0usize;
        {
            let mut st = tx
                .prepare(
                    "INSERT OR IGNORE INTO arus_kas (id_sumber, jenis, jumlah, biaya, waktu_ms)
                     VALUES (?1,?2,?3,?4,?5)",
                )
                .map_err(gagal)?;
            for a in arus {
                baru += st
                    .execute(params![a.id_sumber, a.jenis.teks(), a.jumlah.to_string(), a.biaya.to_string(), a.waktu_ms])
                    .map_err(gagal)?;
            }
        }
        tx.commit().map_err(gagal)?;
        Ok(baru)
    }

    /// Semua arus kas, urut waktu naik.
    pub fn semua_arus_kas(&self) -> Hasil<Vec<ArusKas>> {
        let mut st = self
            .conn
            .prepare("SELECT id_sumber, jenis, jumlah, biaya, waktu_ms FROM arus_kas ORDER BY waktu_ms ASC, id_sumber ASC")
            .map_err(gagal)?;
        let baris = st
            .query_map([], |r| {
                let jenis_teks: String = r.get(1)?;
                let jenis = JenisArusKas::dari_teks(&jenis_teks).ok_or_else(|| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        "jenis arus kas tidak dikenal".into(),
                    )
                })?;
                Ok(ArusKas {
                    id_sumber: r.get(0)?,
                    jenis,
                    jumlah: kolom_desimal(r, 2)?,
                    biaya: kolom_desimal(r, 3)?,
                    waktu_ms: r.get(4)?,
                })
            })
            .map_err(gagal)?;
        baris.collect::<Result<Vec<_>, _>>().map_err(gagal)
    }

    pub fn jumlah_arus_kas(&self) -> Hasil<usize> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM arus_kas", [], |r| r.get(0))
            .map_err(gagal)?;
        Ok(n as usize)
    }

    /// Apakah `amount` deposit/withdraw sudah bersih (fee tidak dikurangkan lagi). Bawaan: false.
    pub fn amount_sudah_bersih(&self) -> Hasil<bool> {
        Ok(self.meta(META_AMOUNT_BERSIH)?.as_deref() == Some("1"))
    }

    pub fn atur_amount_sudah_bersih(&self, ya: bool) -> Hasil<()> {
        self.atur_meta(META_AMOUNT_BERSIH, if ya { "1" } else { "0" })
    }

    /// Waktu (ms) sinkron arus kas terakhir yang berhasil. None bila belum pernah.
    pub fn arus_kas_terakhir_ms(&self) -> Hasil<Option<i64>> {
        match self.meta(META_ARUS_KAS_TERAKHIR)? {
            None => Ok(None),
            Some(t) => t.parse::<i64>().map(Some).map_err(gagal),
        }
    }

    pub fn atur_arus_kas_terakhir_ms(&self, ms: i64) -> Hasil<()> {
        self.atur_meta(META_ARUS_KAS_TERAKHIR, &ms.to_string())
    }

    /// Catat snapshot saldo (jenis: "awal", nanti "harian"/"mingguan" di Fase 2).
    pub fn tambah_snapshot(&self, jenis: &str, waktu_ms: i64, saldo: Decimal) -> Hasil<()> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO snapshot (jenis, waktu_ms, saldo) VALUES (?1, ?2, ?3)",
                params![jenis, waktu_ms, saldo.to_string()],
            )
            .map_err(gagal)?;
        Ok(())
    }

    /// Snapshot "awal" tertua: (waktu_ms, saldo IDR). None bila belum ada (FR-03).
    pub fn snapshot_awal(&self) -> Hasil<Option<(i64, Decimal)>> {
        self.conn
            .query_row(
                "SELECT waktu_ms, saldo FROM snapshot WHERE jenis = 'awal'
                 ORDER BY waktu_ms ASC LIMIT 1",
                [],
                |r| Ok((r.get::<_, i64>(0)?, kolom_desimal(r, 1)?)),
            )
            .optional()
            .map_err(gagal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    pub fn trade(id: &str, order: &str, simbol: &str, beli: bool, t: i64) -> Trade {
        Trade {
            trade_id: id.into(),
            order_id: order.into(),
            symbol: simbol.into(),
            price: d("3998"),
            qty: d("16.35667833"),
            quote_qty: d("65393"),
            commission: d("133"),
            commission_asset: "idr".into(),
            is_buyer: beli,
            is_maker: false,
            time: t,
        }
    }

    #[test]
    fn simpan_dan_baca_kembali_tanpa_kehilangan_presisi() {
        let mut s = Penyimpanan::dalam_memori().unwrap();
        let t = trade("1", "o1", "icntidr", true, 1000);
        assert_eq!(s.simpan_banyak(&[t.clone()]).unwrap(), 1);
        let baca = s.semua_trade().unwrap();
        assert_eq!(baca, vec![t]);
        assert_eq!(baca[0].qty, d("16.35667833"));
    }

    #[test]
    fn anti_dobel_trade_id_sama_tidak_masuk_dua_kali() {
        let mut s = Penyimpanan::dalam_memori().unwrap();
        let t = trade("1", "o1", "icntidr", true, 1000);
        assert_eq!(s.simpan_banyak(&[t.clone(), t.clone()]).unwrap(), 1);
        assert_eq!(s.simpan_banyak(&[t]).unwrap(), 0);
        assert_eq!(s.jumlah_trade().unwrap(), 1);
    }

    #[test]
    fn meta_dan_simbol() {
        let mut s = Penyimpanan::dalam_memori().unwrap();
        assert_eq!(s.meta("x").unwrap(), None);
        s.atur_meta("x", "1").unwrap();
        s.atur_meta("x", "2").unwrap();
        assert_eq!(s.meta("x").unwrap().as_deref(), Some("2"));
        s.simpan_banyak(&[
            trade("1", "o1", "icntidr", true, 1),
            trade("2", "o2", "dogeidr", true, 2),
        ])
        .unwrap();
        assert_eq!(s.simbol_tersimpan().unwrap(), vec!["dogeidr", "icntidr"]);
    }

    #[test]
    fn riwayat_setelan_tersimpan_berurutan() {
        let s = Penyimpanan::dalam_memori().unwrap();
        s.tambah_setelan(2000, d("2"), d("1.5")).unwrap();
        s.tambah_setelan(1000, d("1"), d("1")).unwrap();
        assert_eq!(
            s.riwayat_setelan().unwrap(),
            vec![(1000, d("1"), d("1")), (2000, d("2"), d("1.5"))]
        );
    }

    #[test]
    fn target_belum_diatur_lalu_disimpan_dan_diganti() {
        let s = Penyimpanan::dalam_memori().unwrap();
        assert_eq!(s.target().unwrap(), (None, None));
        s.atur_target(d("1.5"), d("5")).unwrap();
        assert_eq!(s.target().unwrap(), (Some(d("1.5")), Some(d("5"))));
        s.atur_target(d("2"), d("6.5")).unwrap();
        assert_eq!(s.target().unwrap(), (Some(d("2")), Some(d("6.5"))));
    }

    fn arus(id: &str, jenis: JenisArusKas, jumlah: &str, w: i64) -> ArusKas {
        ArusKas { id_sumber: id.into(), jenis, jumlah: d(jumlah), biaya: d("2500"), waktu_ms: w }
    }

    #[test]
    fn arus_kas_anti_dobel_dan_presisi() {
        let mut s = Penyimpanan::dalam_memori().unwrap();
        let a = arus("deposit:1", JenisArusKas::Deposit, "50000.5", 20);
        let b = arus("withdraw:1", JenisArusKas::Withdraw, "20000", 10);
        assert_eq!(s.simpan_arus_kas(&[a.clone(), a.clone(), b.clone()]).unwrap(), 2);
        assert_eq!(s.simpan_arus_kas(&[a.clone()]).unwrap(), 0);
        assert_eq!(s.jumlah_arus_kas().unwrap(), 2);
        assert_eq!(s.semua_arus_kas().unwrap(), vec![b, a]); // urut waktu
    }

    #[test]
    fn mode_fee_dan_sinkron_terakhir() {
        let s = Penyimpanan::dalam_memori().unwrap();
        assert!(!s.amount_sudah_bersih().unwrap());
        s.atur_amount_sudah_bersih(true).unwrap();
        assert!(s.amount_sudah_bersih().unwrap());
        assert_eq!(s.arus_kas_terakhir_ms().unwrap(), None);
        s.atur_arus_kas_terakhir_ms(1793984400000).unwrap();
        assert_eq!(s.arus_kas_terakhir_ms().unwrap(), Some(1793984400000));
    }

    #[test]
    fn snapshot_awal_hanya_yang_tertua() {
        let s = Penyimpanan::dalam_memori().unwrap();
        assert_eq!(s.snapshot_awal().unwrap(), None);
        s.tambah_snapshot("awal", 2000, d("1450000")).unwrap();
        s.tambah_snapshot("awal", 3000, d("9")).unwrap();
        assert_eq!(s.snapshot_awal().unwrap(), Some((2000, d("1450000"))));
    }
}
