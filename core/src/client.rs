//! Klien REST Indodax TAPI v2. HANYA metode GET (baca saja).
use crate::arus_kas::{parse_fiat_orders, HasilParseFiat, JenisArusKas};
use crate::{account::{parse_akun, Akun}, model::parse_my_trades, sign::tanda_tangan, Error, Hasil, Trade};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const BASE_PRODUKSI: &str = "https://api.indodax.com";
const RECV_WINDOW_MS: u64 = 10_000;
/// Sama dengan skrip Fase 0 yang terbukti berhasil.
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36";

pub struct IndodaxClient {
    key: String,
    secret: String,
    base: String,
    agen: ureq::Agent,
}

// Jangan pernah mencetak key/secret ke log.
impl std::fmt::Debug for IndodaxClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndodaxClient").field("base", &self.base).finish_non_exhaustive()
    }
}

/// Susun query string dan tandatangani. Urutan: parameter, timestamp, recvWindow
/// (sama dengan skrip Fase 0). Yang ditandatangani = persis yang dikirim.
pub fn bangun_query(secret: &str, params: &[(&str, String)], now_ms: u64) -> (String, String) {
    let mut qs = String::new();
    for (k, v) in params {
        qs.push_str(&format!("{k}={v}&"));
    }
    qs.push_str(&format!("timestamp={now_ms}&recvWindow={RECV_WINDOW_MS}"));
    let sign = tanda_tangan(secret, &qs);
    (qs, sign)
}

fn sekarang_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn simbol_valid(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

impl IndodaxClient {
    pub fn new(key: impl Into<String>, secret: impl Into<String>) -> Self {
        Self::dengan_base(key, secret, BASE_PRODUKSI)
    }

    pub fn dengan_base(key: impl Into<String>, secret: impl Into<String>, base: &str) -> Self {
        Self {
            key: key.into(),
            secret: secret.into(),
            base: base.trim_end_matches('/').to_string(),
            agen: ureq::AgentBuilder::new().timeout(Duration::from_secs(20)).build(),
        }
    }

    fn get(&self, path: &str, params: &[(&str, String)]) -> Hasil<String> {
        let (qs, sign) = bangun_query(&self.secret, params, sekarang_ms());
        let url = format!("{}{}?{}", self.base, path, qs);
        let res = self
            .agen
            .get(&url)
            .set("User-Agent", USER_AGENT)
            .set("Accept", "application/json")
            .set("X-APIKEY", &self.key)
            .set("Sign", &sign)
            .call();
        match res {
            Ok(r) => r.into_string().map_err(|e| Error::Jaringan(e.to_string())),
            Err(ureq::Error::Status(kode, r)) => {
                let isi = r.into_string().unwrap_or_default();
                Err(Error::Http { kode, isi_awal: isi.chars().take(400).collect() })
            }
            Err(e) => Err(Error::Jaringan(e.to_string())),
        }
    }

    /// FR-02: validasi kunci. Berhasil bila GET /api/v2/account membalas HTTP 200.
    /// Mengembalikan JSON mentah.
    pub fn ambil_akun_mentah(&self) -> Hasil<String> {
        self.get("/api/v2/account", &[])
    }

    /// FR-02/FR-03: ambil akun dan saldo (sudah di-parse).
    pub fn ambil_akun(&self) -> Hasil<Akun> {
        parse_akun(&self.ambil_akun_mentah()?)
    }

    /// Satu permintaan myTrades. Rentang waktu maksimal 7 hari (aturan Indodax);
    /// pemecahan rentang dikerjakan di T1.2.
    pub fn ambil_trade(
        &self,
        symbol: &str,
        mulai_ms: Option<u64>,
        akhir_ms: Option<u64>,
        limit: u32,
    ) -> Hasil<Vec<Trade>> {
        if !simbol_valid(symbol) {
            return Err(Error::SimbolTidakValid(symbol.to_string()));
        }
        let mut p = vec![("symbol", symbol.to_string()), ("limit", limit.to_string())];
        if let Some(m) = mulai_ms {
            p.push(("startTime", m.to_string()));
        }
        if let Some(a) = akhir_ms {
            p.push(("endTime", a.to_string()));
        }
        parse_my_trades(&self.get("/api/v2/myTrades", &p)?)
    }

    /// T2.2: riwayat deposit/withdraw IDR (GET /api/v2/fiat/orders). Rentang maksimal 30 hari;
    /// pembelahan rentang dikerjakan `arus_kas::sinkron_arus_kas`. Nama parameter
    /// (transactionType, beginTime, endTime) sudah terbukti diterima Indodax (HTTP 200).
    pub fn ambil_arus_kas(&self, jenis: JenisArusKas, mulai_ms: u64, akhir_ms: u64) -> Hasil<HasilParseFiat> {
        let p = vec![
            ("transactionType", jenis.kode_api().to_string()),
            ("beginTime", mulai_ms.to_string()),
            ("endTime", akhir_ms.to_string()),
        ];
        parse_fiat_orders(&self.get("/api/v2/fiat/orders", &p)?, jenis)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_dan_tanda_tangan_konsisten() {
        let (qs, sign) = bangun_query("rahasia", &[("symbol", "icntidr".into()), ("limit", "100".into())], 1700000000000);
        assert_eq!(qs, "symbol=icntidr&limit=100&timestamp=1700000000000&recvWindow=10000");
        assert_eq!(sign, tanda_tangan("rahasia", &qs));
    }

    #[test]
    fn simbol_tidak_valid_ditolak_sebelum_kirim() {
        let c = IndodaxClient::new("k", "s");
        assert!(matches!(c.ambil_trade("ICNT&x=1", None, None, 10), Err(Error::SimbolTidakValid(_))));
    }

    #[test]
    fn debug_tidak_membocorkan_kunci() {
        let c = IndodaxClient::new("KUNCI-RAHASIA", "SECRET-RAHASIA");
        let s = format!("{:?}", c);
        assert!(!s.contains("RAHASIA"));
    }
}
