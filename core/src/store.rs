//! Penyimpanan lokal (SQLite) untuk T1.2. Data disimpan sebagai CATATAN
//! (trade per pengisian), bukan angka saldo mentah. Angka uang disimpan
//! sebagai teks desimal supaya tidak ada floating point.
//! Anti-dobel (FR-09): `trade_id` adalah PRIMARY KEY, INSERT OR IGNORE.
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
";

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
    fn snapshot_awal_hanya_yang_tertua() {
        let s = Penyimpanan::dalam_memori().unwrap();
        assert_eq!(s.snapshot_awal().unwrap(), None);
        s.tambah_snapshot("awal", 2000, d("1450000")).unwrap();
        s.tambah_snapshot("awal", 3000, d("9")).unwrap();
        assert_eq!(s.snapshot_awal().unwrap(), Some((2000, d("1450000"))));
    }
}
