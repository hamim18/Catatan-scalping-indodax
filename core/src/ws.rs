//! Protokol WebSocket privat Indodax (T1.4a). LOGIKA MURNI: tanpa jaringan,
//! tanpa thread. Sambungan sebenarnya dikerjakan di T1.4b.
//!
//! Peran WebSocket di aplikasi ini: HANYA PEMICU. Begitu order selesai, aplikasi
//! menarik trade koin itu lewat REST (`sinkron_koin`), karena angka resmi
//! (quoteQty, commission) datang dari REST dan sudah terbukti cocok dengan struk.
//! Hasil tes T0.6 (7 Nov 2026, UAI): event FILL tidak membawa quoteQty dan
//! harganya pembulatan, sedangkan `fee + tax + clearing` dari event sama dengan
//! `commission` REST (beli 43 = 43; jual 43 + 22 + 0 = 65).
//!
//! Hanya membaca: modul ini tidak membuat order.
use crate::calc::Sisi;
use crate::{Error, Hasil, Trade};
use hmac::{Hmac, Mac};
use rust_decimal::Decimal;
use serde_json::Value;
use sha2::Sha512;
use std::collections::{HashMap, VecDeque};
use std::str::FromStr;
use std::time::Duration;

/// Alamat WebSocket privat produksi (dokumen resmi Indodax).
pub const URL_WS_PRIVAT: &str = "wss://pws.indodax.com/ws/?cf_ws_frame_ping_pong=true";
/// Alamat permintaan token (POST). Tanda tangan HMAC-SHA512, bukan SHA256.
pub const URL_MINTA_TOKEN: &str = "https://indodax.com/api/private_ws/v1/generate_token";
pub const ID_CONNECT: i64 = 1;
pub const ID_SUBSCRIBE: i64 = 2;
/// Server mengirim `{}` sebagai ping aplikasi; balas dengan `{}`.
pub const BALASAN_PING: &str = "{}";

// ---------- Token ----------

/// Badan permintaan token: `client=tapi&tapi_key=<KEY>`.
pub fn badan_minta_token(api_key: &str) -> String {
    format!("client=tapi&tapi_key={api_key}")
}

/// Isi header `Sign` untuk permintaan token: HMAC-SHA512(secret, badan), hex.
pub fn tanda_tangan_token(secret: &str, badan: &str) -> String {
    let mut mac =
        Hmac::<Sha512>::new_from_slice(secret.as_bytes()).expect("HMAC menerima kunci apa pun");
    mac.update(badan.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Token sambungan dan nama channel pribadi.
#[derive(Clone, PartialEq)]
pub struct TokenWs {
    pub token: String,
    pub channel: String,
}

// Jangan pernah mencetak token ke log.
impl std::fmt::Debug for TokenWs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenWs").finish_non_exhaustive()
    }
}

/// Baca respons generate_token. Sukses bila `success` = 1 dan `connToken`
/// serta `channel` terisi.
pub fn parse_token(json: &str) -> Hasil<TokenWs> {
    let v: Value = serde_json::from_str(json).map_err(|e| Error::Json(e.to_string()))?;
    if v.get("success").and_then(Value::as_i64) != Some(1) {
        let kode = v.get("error_code").and_then(Value::as_str).unwrap_or("?");
        let pesan = v.get("error").and_then(Value::as_str).unwrap_or("?");
        return Err(Error::Json(format!("token WebSocket ditolak: {kode} ({pesan})")));
    }
    let r = v
        .get("return")
        .ok_or_else(|| Error::Json("respons token tanpa 'return'".into()))?;
    let ambil = |k: &str| {
        r.get(k)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    match (ambil("connToken"), ambil("channel")) {
        (Some(token), Some(channel)) => Ok(TokenWs { token, channel }),
        _ => Err(Error::Json("connToken atau channel kosong".into())),
    }
}

/// Pesan autentikasi setelah handshake.
pub fn pesan_connect(token: &str) -> String {
    serde_json::json!({ "connect": { "token": token }, "id": ID_CONNECT }).to_string()
}

/// Pesan subscribe ke channel pribadi.
pub fn pesan_subscribe(channel: &str) -> String {
    serde_json::json!({ "subscribe": { "channel": channel }, "id": ID_SUBSCRIBE }).to_string()
}

// ---------- Pesan masuk ----------

#[derive(Debug, Clone, PartialEq)]
pub enum StatusOrder {
    Baru,
    Fill,
    Selesai,
    Batal,
    Ditolak,
    /// Status lain yang belum dikenal; diabaikan, bukan error.
    Lain(String),
}

/// Rincian satu pengisian (hanya ada pada status FILL).
#[derive(Debug, Clone, PartialEq)]
pub struct Pengisian {
    pub trade_id: Option<String>,
    pub qty: Decimal,
    /// fee + pajak + clearing dari event. Hanya informasi; angka resmi dari REST.
    pub biaya_total: Decimal,
    /// "MAKER" atau "TAKER"
    pub peran: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventOrder {
    pub order_id: String,
    /// huruf kecil, contoh "uaiidr"
    pub simbol: String,
    pub sisi: Sisi,
    pub status: StatusOrder,
    pub jumlah_asal: Decimal,
    pub jumlah_terisi: Decimal,
    /// Harga di event: pembulatan, jangan dipakai untuk menghitung modal.
    pub harga: Decimal,
    pub waktu_ms: i64,
    pub pengisian: Option<Pengisian>,
    pub alasan_batal: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PesanMasuk {
    /// `{}` dari server; balas dengan `BALASAN_PING`.
    Ping,
    /// Balasan atas connect (id 1) atau subscribe (id 2).
    Balasan { id: i64, galat: Option<(i64, String)>, ttl_detik: Option<i64> },
    Event(EventOrder),
    /// Tidak dikenali; diabaikan (isi dipotong 120 karakter).
    Lain(String),
}

fn desimal(v: Option<&Value>) -> Option<Decimal> {
    match v? {
        Value::String(s) => Decimal::from_str(s.trim()).ok(),
        Value::Number(n) => Decimal::from_str(&n.to_string()).ok(),
        _ => None,
    }
}

fn potong(teks: &str) -> String {
    teks.chars().take(120).collect()
}

fn parse_event(e: &Value) -> Option<EventOrder> {
    if e.get("eventType").and_then(Value::as_str) != Some("order_update") {
        return None;
    }
    let o = e.get("order")?;
    let teks = |k: &str| o.get(k).and_then(Value::as_str).map(str::to_string);
    let order_id = teks("orderId")?;
    let simbol = teks("symbol")?.to_lowercase();
    let sisi = match teks("side")?.to_uppercase().as_str() {
        "BUY" => Sisi::Beli,
        "SELL" => Sisi::Jual,
        _ => return None,
    };
    // Dokumen resmi menulis FILLED di tabel tetapi FILL di contoh; data nyata FILL.
    let status = match teks("status")?.to_uppercase().as_str() {
        "NEW" => StatusOrder::Baru,
        "FILL" | "FILLED" => StatusOrder::Fill,
        "DONE" => StatusOrder::Selesai,
        "CANCELLED" | "CANCELED" => StatusOrder::Batal,
        "REJECTED" => StatusOrder::Ditolak,
        lain => StatusOrder::Lain(lain.to_string()),
    };
    let pengisian = o.get("fillInformation").map(|f| Pengisian {
        trade_id: teks("tradeId"),
        qty: desimal(f.get("qty")).unwrap_or(Decimal::ZERO),
        biaya_total: ["fee", "tax", "clearing"]
            .iter()
            .filter_map(|k| desimal(f.get(*k)))
            .sum(),
        peran: f
            .get("participant")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
    });
    Some(EventOrder {
        order_id,
        simbol,
        sisi,
        status,
        jumlah_asal: desimal(o.get("origQty")).unwrap_or(Decimal::ZERO),
        jumlah_terisi: desimal(o.get("executedQty")).unwrap_or(Decimal::ZERO),
        harga: desimal(o.get("price")).unwrap_or(Decimal::ZERO),
        waktu_ms: o.get("transactionTime").and_then(Value::as_i64).unwrap_or(0),
        pengisian,
        alasan_batal: teks("cancelReason"),
    })
}

/// Urai satu frame teks dari server. Satu frame bisa berisi beberapa pesan
/// (dipisah baris baru) dan satu pesan bisa membawa beberapa event. Tidak
/// pernah panik; yang tidak dikenali menjadi `Lain`.
pub fn parse_pesan(teks: &str) -> Vec<PesanMasuk> {
    let mut hasil = Vec::new();
    for baris in teks.lines().map(str::trim).filter(|b| !b.is_empty()) {
        let v: Value = match serde_json::from_str(baris) {
            Ok(v) => v,
            Err(_) => {
                hasil.push(PesanMasuk::Lain(potong(baris)));
                continue;
            }
        };
        if v.as_object().map_or(false, |m| m.is_empty()) {
            hasil.push(PesanMasuk::Ping);
            continue;
        }
        if let Some(data) = v.pointer("/push/pub/data").and_then(Value::as_array) {
            for e in data {
                if let Some(ev) = parse_event(e) {
                    hasil.push(PesanMasuk::Event(ev));
                }
            }
            continue;
        }
        if let Some(id) = v.get("id").and_then(Value::as_i64) {
            let galat = v.get("error").map(|e| {
                (
                    e.get("code").and_then(Value::as_i64).unwrap_or(0),
                    e.get("message").and_then(Value::as_str).unwrap_or("").to_string(),
                )
            });
            let ttl_detik = v.pointer("/connect/ttl").and_then(Value::as_i64);
            hasil.push(PesanMasuk::Balasan { id, galat, ttl_detik });
            continue;
        }
        hasil.push(PesanMasuk::Lain(potong(baris)));
    }
    hasil
}

// ---------- Pemicu sinkron ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlasanPemicu {
    /// Status DONE (FR-07).
    OrderSelesai,
    /// Dibatalkan setelah sebagian terisi; yang terisi tetap dicatat (FR-08).
    BatalSebagian,
}

/// Perintah: tarik trade `simbol` lewat REST sekarang.
#[derive(Debug, Clone, PartialEq)]
pub struct Pemicu {
    pub order_id: String,
    pub simbol: String,
    pub sisi: Sisi,
    pub alasan: AlasanPemicu,
}

const BATAS_INGATAN: usize = 64;

/// Memutuskan kapan sebuah event harus memicu sinkron REST. NEW, REJECTED,
/// dan CANCELLED tanpa eksekusi tidak memicu apa pun (FR-08).
#[derive(Debug, Default)]
pub struct PemicuSinkron {
    terisi: HashMap<String, Decimal>,
    sudah: VecDeque<String>,
}

impl PemicuSinkron {
    pub fn baru() -> Self {
        Self::default()
    }

    pub fn proses(&mut self, e: &EventOrder) -> Option<Pemicu> {
        let ingat = self.terisi.get(&e.order_id).copied().unwrap_or(Decimal::ZERO);
        let terisi = ingat.max(e.jumlah_terisi);
        match &e.status {
            StatusOrder::Baru | StatusOrder::Lain(_) => None,
            StatusOrder::Fill => {
                self.terisi.insert(e.order_id.clone(), terisi);
                None
            }
            StatusOrder::Ditolak => {
                self.terisi.remove(&e.order_id);
                None
            }
            StatusOrder::Selesai | StatusOrder::Batal => {
                self.terisi.remove(&e.order_id);
                if terisi <= Decimal::ZERO || self.sudah.contains(&e.order_id) {
                    return None;
                }
                self.sudah.push_back(e.order_id.clone());
                if self.sudah.len() > BATAS_INGATAN {
                    self.sudah.pop_front();
                }
                let alasan = if e.status == StatusOrder::Selesai {
                    AlasanPemicu::OrderSelesai
                } else {
                    AlasanPemicu::BatalSebagian
                };
                Some(Pemicu {
                    order_id: e.order_id.clone(),
                    simbol: e.simbol.clone(),
                    sisi: e.sisi,
                    alasan,
                })
            }
        }
    }
}

// ---------- Bantu: ulang sinkron dan umur sambungan ----------

/// Jeda sebelum percobaan ke-n (mulai 0) menarik trade setelah event DONE.
/// Dipakai bila REST belum memuat order itu. None = berhenti mencoba; sisanya
/// ditangani rekonsiliasi berikutnya.
pub fn jeda_ulang(percobaan: usize) -> Option<Duration> {
    const JEDA_MS: [u64; 5] = [0, 1_000, 2_000, 4_000, 8_000];
    JEDA_MS.get(percobaan).map(|ms| Duration::from_millis(*ms))
}

/// Apakah sudah ada trade untuk order ini di daftar trade tersimpan.
pub fn order_sudah_tersimpan(trades: &[Trade], order_id: &str) -> bool {
    trades.iter().any(|t| t.order_id == order_id)
}

/// Sambungan sebaiknya diperbarui (token baru, sambung ulang) sebelum token
/// kedaluwarsa: ttl dikurangi 10 menit, minimal 60 detik.
pub fn umur_sambungan_maks(ttl_detik: i64) -> Duration {
    Duration::from_secs((ttl_detik - 600).max(60) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_my_trades;

    // Event nyata dari tes T0.6 (UAI, 7 Nov 2026), dipadatkan.
    const BELI_NEW: &str = r#"{"eventType":"order_update","order":{"orderId":"uaiidr-market-16844229","symbol":"uaiidr","side":"BUY","origQty":"3.69352663","unfilledQty":"3.69352663","executedQty":"0","price":"0","description":"UAI/IDR","status":"NEW","transactionTime":1794014723267,"clientOrderId":"uaiidr-market-16844229"}}"#;
    const BELI_FILL: &str = r#"{"eventType":"order_update","order":{"orderId":"uaiidr-market-16844229","tradeId":"144115188260590414","symbol":"uaiidr","side":"BUY","origQty":"4.24612403","unfilledQty":"0","executedQty":"4.24612403","price":"2580","description":"UAI/IDR","status":"FILL","transactionTime":1794014723267,"fillInformation":{"participant":"TAKER","filledQty":"4.24612403","qty":"4.24612403","feeAsset":"idr","feeRate":0.004,"fee":"43","taxAsset":"idr","taxRate":0.0,"tax":"0","clearingAsset":"idr","clearingRate":5.55e-05,"clearing":"0"},"clientOrderId":"uaiidr-market-16844229"}}"#;
    const BELI_DONE: &str = r#"{"eventType":"order_update","order":{"orderId":"uaiidr-market-16844229","symbol":"uaiidr","side":"BUY","origQty":"4.24612403","unfilledQty":"0","executedQty":"4.24612403","price":"0","description":"UAI/IDR","status":"DONE","transactionTime":1794014723267,"clientOrderId":"uaiidr-market-16844229"}}"#;
    const JUAL_FILL: &str = r#"{"eventType":"order_update","order":{"orderId":"uaiidr-market-16844238","tradeId":"144115188260590424","symbol":"uaiidr","side":"SELL","origQty":"4.24612403","unfilledQty":"0","executedQty":"4.24612403","price":"2571","description":"UAI/IDR","status":"FILL","transactionTime":1794014736705,"fillInformation":{"participant":"TAKER","filledQty":"4.24612403","qty":"4.24612403","feeAsset":"idr","feeRate":0.004,"fee":"43","taxAsset":"idr","taxRate":0.0021,"tax":"22","clearingAsset":"idr","clearingRate":5.55e-05,"clearing":"0"},"clientOrderId":"uaiidr-market-16844238"}}"#;
    const JUAL_DONE: &str = r#"{"eventType":"order_update","order":{"orderId":"uaiidr-market-16844238","symbol":"uaiidr","side":"SELL","origQty":"4.24612403","unfilledQty":"0","executedQty":"4.24612403","price":"0","description":"UAI/IDR","status":"DONE","transactionTime":1794014736705,"clientOrderId":"uaiidr-market-16844238"}}"#;
    const BATAL_NOL: &str = r#"{"eventType":"order_update","order":{"orderId":"uaiidr-limit-16841367","symbol":"uaiidr","side":"BUY","origQty":"14.13277511","unfilledQty":"14.13277511","executedQty":"0","price":"2508","status":"CANCELLED","transactionTime":1794013236000}}"#;

    fn push(data: &[&str]) -> String {
        format!(
            r#"{{"push":{{"channel":"pws:#abc","pub":{{"data":[{}]}}}}}}"#,
            data.join(",")
        )
    }

    fn events(teks: &str) -> Vec<EventOrder> {
        parse_pesan(teks)
            .into_iter()
            .filter_map(|p| match p {
                PesanMasuk::Event(e) => Some(e),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn tanda_tangan_sha512_vektor_rfc4231() {
        assert_eq!(
            tanda_tangan_token("Jefe", "what do ya want for nothing?"),
            "164b7a7bfcf819e2e395fbe73b56e0a387bd64222e831fd610270cd7ea2505549758bf75c05a994a6d034f65f8f0e6fdcaeab1a34d4a6b4b636e070a38bce737"
        );
    }

    #[test]
    fn badan_dan_tanda_tangan_token() {
        let badan = badan_minta_token("ABC123");
        assert_eq!(badan, "client=tapi&tapi_key=ABC123");
        assert_eq!(
            tanda_tangan_token("rahasia", &badan),
            "a7c991f5a6d393a4357c3a23244ab6a55a0ac8c33cf510ae77f643c3bc6297152c22963c634403dc78311ef1fefdd7d149d3e54e4d7b602872beb96969cdd001"
        );
    }

    #[test]
    fn parse_token_sukses_dan_gagal() {
        let ok = r#"{"success":1,"return":{"connToken":"TOKEN-RAHASIA","channel":"pws:#c12d"}}"#;
        let t = parse_token(ok).unwrap();
        assert_eq!(t.token, "TOKEN-RAHASIA");
        assert_eq!(t.channel, "pws:#c12d");
        assert!(!format!("{:?}", t).contains("RAHASIA"), "Debug tidak boleh membocorkan token");

        let gagal = r#"{"success":0,"error":"Invalid credentials. Bad sign.","error_code":"bad_sign"}"#;
        let e = parse_token(gagal).unwrap_err().to_string();
        assert!(e.contains("bad_sign"));

        assert!(parse_token(r#"{"success":1,"return":{"connToken":"","channel":"x"}}"#).is_err());
        assert!(parse_token("bukan json").is_err());
    }

    #[test]
    fn pesan_connect_dan_subscribe() {
        let c: Value = serde_json::from_str(&pesan_connect("T")).unwrap();
        assert_eq!(c["connect"]["token"], "T");
        assert_eq!(c["id"], 1);
        let s: Value = serde_json::from_str(&pesan_subscribe("pws:#abc")).unwrap();
        assert_eq!(s["subscribe"]["channel"], "pws:#abc");
        assert_eq!(s["id"], 2);
    }

    #[test]
    fn ping_balasan_dan_galat() {
        assert_eq!(parse_pesan("{}"), vec![PesanMasuk::Ping]);
        let p = parse_pesan(r#"{"id":1,"connect":{"client":"x","version":"5.2.0","expires":true,"ttl":86133}}"#);
        assert_eq!(p, vec![PesanMasuk::Balasan { id: 1, galat: None, ttl_detik: Some(86133) }]);
        let p = parse_pesan(r#"{"id":1,"error":{"code":109,"message":"token expired"}}"#);
        assert_eq!(
            p,
            vec![PesanMasuk::Balasan { id: 1, galat: Some((109, "token expired".into())), ttl_detik: None }]
        );
        let p = parse_pesan(r#"{"id":2,"subscribe":{}}"#);
        assert_eq!(p, vec![PesanMasuk::Balasan { id: 2, galat: None, ttl_detik: None }]);
    }

    #[test]
    fn satu_frame_banyak_pesan_dan_sampah_tidak_panik() {
        let frame = format!("{{}}\n{}\nbukan json\n", push(&[BELI_DONE]));
        let p = parse_pesan(&frame);
        assert_eq!(p.len(), 3);
        assert_eq!(p[0], PesanMasuk::Ping);
        assert!(matches!(p[1], PesanMasuk::Event(_)));
        assert!(matches!(p[2], PesanMasuk::Lain(_)));
        assert!(parse_pesan("").is_empty());
        // event tanpa field wajib diabaikan, bukan panik
        assert!(events(&push(&[r#"{"eventType":"order_update","order":{"symbol":"x"}}"#])).is_empty());
        assert!(events(&push(&[r#"{"eventType":"lain","order":{}}"#])).is_empty());
    }

    #[test]
    fn event_beli_market_nyata() {
        let ev = events(&push(&[BELI_NEW, BELI_FILL, BELI_DONE]));
        assert_eq!(ev.len(), 3);
        assert_eq!(ev[0].status, StatusOrder::Baru);
        assert_eq!(ev[1].status, StatusOrder::Fill);
        assert_eq!(ev[2].status, StatusOrder::Selesai);
        let f = &ev[1];
        assert_eq!(f.order_id, "uaiidr-market-16844229");
        assert_eq!(f.simbol, "uaiidr");
        assert_eq!(f.sisi, Sisi::Beli);
        assert_eq!(f.jumlah_terisi, Decimal::from_str("4.24612403").unwrap());
        let p = f.pengisian.as_ref().unwrap();
        assert_eq!(p.trade_id.as_deref(), Some("144115188260590414"));
        assert_eq!(p.peran, "TAKER");
        // sama dengan commission REST untuk trade ini (43)
        assert_eq!(p.biaya_total, Decimal::from(43));
        assert!(ev[0].pengisian.is_none());
    }

    #[test]
    fn biaya_jual_nyata_fee_pajak_clearing() {
        let ev = events(&push(&[JUAL_FILL]));
        assert_eq!(ev[0].sisi, Sisi::Jual);
        // 43 + 22 + 0 = 65 = commission REST untuk trade ini
        assert_eq!(ev[0].pengisian.as_ref().unwrap().biaya_total, Decimal::from(65));
    }

    #[test]
    fn pemicu_beli_market_sekali_saat_done() {
        let mut pemicu = PemicuSinkron::baru();
        let ev = events(&push(&[BELI_NEW, BELI_FILL, BELI_DONE]));
        let hasil: Vec<Option<Pemicu>> = ev.iter().map(|e| pemicu.proses(e)).collect();
        assert_eq!(hasil[0], None);
        assert_eq!(hasil[1], None);
        let p = hasil[2].clone().unwrap();
        assert_eq!(p.simbol, "uaiidr");
        assert_eq!(p.order_id, "uaiidr-market-16844229");
        assert_eq!(p.sisi, Sisi::Beli);
        assert_eq!(p.alasan, AlasanPemicu::OrderSelesai);
        // DONE yang terkirim dua kali tidak memicu lagi
        assert_eq!(pemicu.proses(&ev[2]), None);
    }

    #[test]
    fn pemicu_jual_dan_batal() {
        let mut pemicu = PemicuSinkron::baru();
        let jual = events(&push(&[JUAL_FILL, JUAL_DONE]));
        assert_eq!(pemicu.proses(&jual[0]), None);
        assert_eq!(pemicu.proses(&jual[1]).unwrap().sisi, Sisi::Jual);

        // batal tanpa eksekusi (event nyata T0.6): tidak memicu (FR-08)
        let nol = events(&push(&[BATAL_NOL]));
        assert_eq!(pemicu.proses(&nol[0]), None);

        // batal setelah sebagian terisi: memicu, yang terisi dicatat
        let sebagian = r#"{"eventType":"order_update","order":{"orderId":"o-1","tradeId":"7","symbol":"icntidr","side":"SELL","origQty":"10","unfilledQty":"6","executedQty":"4","price":"4100","status":"FILL","transactionTime":1,"fillInformation":{"participant":"MAKER","filledQty":"4","qty":"4","fee":"10","tax":"0","clearing":"0"}}}"#;
        let batal = r#"{"eventType":"order_update","order":{"orderId":"o-1","symbol":"icntidr","side":"SELL","origQty":"10","unfilledQty":"6","executedQty":"0","price":"4100","status":"CANCELLED","transactionTime":2}}"#;
        let ev = events(&push(&[sebagian, batal]));
        assert_eq!(pemicu.proses(&ev[0]), None);
        let p = pemicu.proses(&ev[1]).unwrap();
        assert_eq!(p.alasan, AlasanPemicu::BatalSebagian);
        assert_eq!(p.simbol, "icntidr");
    }

    #[test]
    fn pemicu_ditolak_dan_status_asing_tidak_memicu() {
        let mut pemicu = PemicuSinkron::baru();
        // contoh dokumen: REJECTED membawa executedQty = origQty; tetap tidak memicu
        let tolak = r#"{"eventType":"order_update","order":{"orderId":"o-2","symbol":"x","side":"BUY","origQty":"1","executedQty":"1","price":"1","status":"REJECTED","transactionTime":1}}"#;
        let asing = r#"{"eventType":"order_update","order":{"orderId":"o-3","symbol":"x","side":"BUY","origQty":"1","executedQty":"1","price":"1","status":"PARTIAL","transactionTime":1}}"#;
        let ev = events(&push(&[tolak, asing]));
        assert_eq!(ev[1].status, StatusOrder::Lain("PARTIAL".into()));
        assert_eq!(pemicu.proses(&ev[0]), None);
        assert_eq!(pemicu.proses(&ev[1]), None);
    }

    #[test]
    fn ingatan_pemicu_terbatas() {
        let mut pemicu = PemicuSinkron::baru();
        for i in 0..(BATAS_INGATAN + 10) {
            let d = format!(
                r#"{{"eventType":"order_update","order":{{"orderId":"o-{i}","symbol":"x","side":"BUY","origQty":"1","executedQty":"1","price":"1","status":"DONE","transactionTime":1}}}}"#
            );
            let ev = events(&push(&[d.as_str()]));
            assert!(pemicu.proses(&ev[0]).is_some());
        }
        assert!(pemicu.sudah.len() <= BATAS_INGATAN);
    }

    #[test]
    fn jeda_ulang_dan_cek_tersimpan() {
        assert_eq!(jeda_ulang(0), Some(Duration::from_millis(0)));
        assert_eq!(jeda_ulang(4), Some(Duration::from_millis(8_000)));
        assert_eq!(jeda_ulang(5), None);

        let trades = parse_my_trades(
            r#"{"data":[{"tradeId":"1","orderId":"o-1","symbol":"uaiidr","price":"2580","qty":"4.2","quoteQty":"10954","commission":"43","commissionAsset":"idr","isBuyer":true,"time":1}]}"#,
        )
        .unwrap();
        assert!(order_sudah_tersimpan(&trades, "o-1"));
        assert!(!order_sudah_tersimpan(&trades, "o-2"));
    }

    #[test]
    fn umur_sambungan() {
        assert_eq!(umur_sambungan_maks(86_400), Duration::from_secs(85_800));
        assert_eq!(umur_sambungan_maks(300), Duration::from_secs(60));
        assert_eq!(umur_sambungan_maks(-5), Duration::from_secs(60));
    }
}
