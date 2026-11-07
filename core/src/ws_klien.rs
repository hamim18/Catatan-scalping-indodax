//! Klien WebSocket privat Indodax (T1.4b). Hanya MEMBACA event order; tidak
//! pernah membuat atau membatalkan order.
//!
//! Dibuat sendiri di atas `rustls` (penyedia `ring`) dan `webpki-roots` yang
//! SUDAH ada di proyek lewat `ureq`, supaya tidak ada penyedia kripto kedua
//! (aws-lc) yang ikut terkompilasi untuk Android. Protokol yang dipakai sama
//! dengan skrip T0.6 yang terbukti jalan di server Indodax.
//!
//! Bagian murni (pembaca/penulis frame, handshake) dites tanpa jaringan.
//! Alur `pantau` memblokir; jalankan dari thread latar (T1.4c).
use crate::calc::Sisi;
use crate::sync::{sinkron, sinkron_koin, Opsi};
use crate::ws::{
    badan_minta_token, jeda_ulang, order_sudah_tersimpan, parse_pesan, parse_token, pesan_connect,
    pesan_subscribe, tanda_tangan_token, umur_sambungan_maks, Pemicu, PemicuSinkron, PesanMasuk,
    TokenWs, BALASAN_PING, ID_CONNECT, ID_SUBSCRIBE, URL_MINTA_TOKEN,
};
use crate::{Error, Hasil, IndodaxClient, Penyimpanan};
use base64::Engine;
use rustls::crypto::{CryptoProvider, SecureRandom};
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const HOST_WS: &str = "pws.indodax.com";
const PATH_WS: &str = "/ws/?cf_ws_frame_ping_pong=true";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36";
/// Batas ukuran satu pesan WebSocket (jaga memori).
const MAKS_PESAN: usize = 1 << 20;
const JEDA_AWAL_DETIK: u64 = 1;
const JEDA_MAKS_DETIK: u64 = 60;
/// Rekonsiliasi penuh sebelum sambung tidak diulang bila baru dilakukan.
const JARAK_REKONSILIASI: Duration = Duration::from_secs(30);

fn galat_ws(pesan: impl std::fmt::Display) -> Error {
    Error::Jaringan(format!("WebSocket: {pesan}"))
}

fn sekarang_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

// ---------- Frame WebSocket (murni) ----------

#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    Teks(String),
    Biner(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Tutup(Option<u16>),
}

/// Kode satu frame dari klien (wajib bertopeng). `topeng` = 4 byte acak.
pub fn kode_frame(opcode: u8, isi: &[u8], topeng: [u8; 4]) -> Vec<u8> {
    let mut out = Vec::with_capacity(isi.len() + 14);
    out.push(0x80 | (opcode & 0x0F));
    let n = isi.len();
    if n < 126 {
        out.push(0x80 | (n as u8));
    } else if n <= 0xFFFF {
        out.push(0x80 | 126);
        out.extend_from_slice(&(n as u16).to_be_bytes());
    } else {
        out.push(0x80 | 127);
        out.extend_from_slice(&(n as u64).to_be_bytes());
    }
    out.extend_from_slice(&topeng);
    out.extend(isi.iter().enumerate().map(|(i, b)| b ^ topeng[i % 4]));
    out
}

fn buat_frame(opcode: u8, data: Vec<u8>) -> Frame {
    if opcode == 0x1 {
        Frame::Teks(String::from_utf8_lossy(&data).into_owned())
    } else {
        Frame::Biner(data)
    }
}

/// Mengumpulkan byte dari soket dan mengurainya menjadi frame utuh. Aman
/// terhadap data yang datang sepotong-sepotong dan terhadap fragmentasi.
#[derive(Debug, Default)]
pub struct PembacaFrame {
    buf: Vec<u8>,
    potongan: Vec<u8>,
    op_awal: u8,
}

impl PembacaFrame {
    pub fn isi(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    /// Frame lengkap berikutnya, atau Ok(None) bila belum lengkap.
    pub fn berikut(&mut self) -> Hasil<Option<Frame>> {
        loop {
            let b = &self.buf;
            if b.len() < 2 {
                return Ok(None);
            }
            let fin = b[0] & 0x80 != 0;
            let op = b[0] & 0x0F;
            let bertopeng = b[1] & 0x80 != 0;
            let n7 = (b[1] & 0x7F) as usize;
            let (panjang, mut kepala) = match n7 {
                126 => {
                    if b.len() < 4 {
                        return Ok(None);
                    }
                    (u16::from_be_bytes([b[2], b[3]]) as u64, 4usize)
                }
                127 => {
                    if b.len() < 10 {
                        return Ok(None);
                    }
                    let mut x = [0u8; 8];
                    x.copy_from_slice(&b[2..10]);
                    (u64::from_be_bytes(x), 10usize)
                }
                n => (n as u64, 2usize),
            };
            if panjang > MAKS_PESAN as u64 {
                return Err(galat_ws(format!("frame terlalu besar ({panjang} byte)")));
            }
            let panjang = panjang as usize;
            let topeng = if bertopeng {
                if b.len() < kepala + 4 {
                    return Ok(None);
                }
                let t = [b[kepala], b[kepala + 1], b[kepala + 2], b[kepala + 3]];
                kepala += 4;
                Some(t)
            } else {
                None
            };
            if b.len() < kepala + panjang {
                return Ok(None);
            }
            let mut isi = b[kepala..kepala + panjang].to_vec();
            if let Some(t) = topeng {
                for (i, x) in isi.iter_mut().enumerate() {
                    *x ^= t[i % 4];
                }
            }
            self.buf.drain(..kepala + panjang);

            match op {
                0x9 => return Ok(Some(Frame::Ping(isi))),
                0xA => return Ok(Some(Frame::Pong(isi))),
                0x8 => {
                    let kode = if isi.len() >= 2 { Some(u16::from_be_bytes([isi[0], isi[1]])) } else { None };
                    return Ok(Some(Frame::Tutup(kode)));
                }
                0x1 | 0x2 => {
                    if fin {
                        return Ok(Some(buat_frame(op, isi)));
                    }
                    self.op_awal = op;
                    self.potongan = isi;
                }
                0x0 => {
                    if self.op_awal == 0 {
                        return Err(galat_ws("frame lanjutan tanpa awal"));
                    }
                    if self.potongan.len() + isi.len() > MAKS_PESAN {
                        return Err(galat_ws("pesan terfragmentasi terlalu besar"));
                    }
                    self.potongan.extend_from_slice(&isi);
                    if fin {
                        let op_awal = self.op_awal;
                        self.op_awal = 0;
                        let data = std::mem::take(&mut self.potongan);
                        return Ok(Some(buat_frame(op_awal, data)));
                    }
                }
                lain => return Err(galat_ws(format!("opcode tidak dikenal: {lain}"))),
            }
        }
    }
}

// ---------- Handshake (murni) ----------

/// Permintaan upgrade. Sengaja TANPA header `Origin` (dokumen resmi Indodax:
/// header Origin yang tidak cocok menyebabkan 403).
pub fn permintaan_handshake(host: &str, path: &str, kunci: &str) -> String {
    format!(
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Key: {kunci}\r\nSec-WebSocket-Version: 13\r\nUser-Agent: {USER_AGENT}\r\n\r\n"
    )
}

/// Periksa baris status balasan: harus 101. (Sec-WebSocket-Accept tidak
/// diperiksa; keaslian server sudah dijamin TLS.)
pub fn periksa_balasan_handshake(kepala: &str) -> Hasil<()> {
    let baris = kepala.lines().next().unwrap_or("");
    if baris.split_whitespace().nth(1) == Some("101") {
        Ok(())
    } else {
        Err(galat_ws(format!("handshake ditolak: {}", baris.chars().take(120).collect::<String>())))
    }
}

// ---------- Sambungan (jaringan) ----------

pub struct SambunganWs {
    tls: StreamOwned<ClientConnection, TcpStream>,
    baca: PembacaFrame,
    provider: Arc<CryptoProvider>,
}

fn acak(provider: &CryptoProvider, buf: &mut [u8]) -> Hasil<()> {
    provider
        .secure_random
        .fill(buf)
        .map_err(|_| galat_ws("gagal membuat angka acak"))
}

fn kena_batas_waktu(e: &std::io::Error) -> bool {
    matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut)
}

impl SambunganWs {
    /// Buka TCP + TLS + handshake. `batas_baca` = lama menunggu data per
    /// panggilan `terima` (agar loop bisa memeriksa tanda berhenti).
    pub fn buka(batas_baca: Duration) -> Hasil<Self> {
        let mut tcp: Option<TcpStream> = None;
        let mut galat = String::from("alamat tidak ditemukan");
        let alamat = (HOST_WS, 443u16).to_socket_addrs().map_err(|e| galat_ws(e))?;
        for a in alamat {
            match TcpStream::connect_timeout(&a, Duration::from_secs(15)) {
                Ok(s) => {
                    tcp = Some(s);
                    break;
                }
                Err(e) => galat = e.to_string(),
            }
        }
        let tcp = tcp.ok_or_else(|| galat_ws(format!("gagal tersambung ke {HOST_WS}: {galat}")))?;
        tcp.set_read_timeout(Some(batas_baca)).map_err(|e| galat_ws(e))?;
        tcp.set_write_timeout(Some(Duration::from_secs(10))).map_err(|e| galat_ws(e))?;
        let _ = tcp.set_nodelay(true);

        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let akar = RootCertStore { roots: webpki_roots::TLS_SERVER_ROOTS.to_vec() };
        let config = ClientConfig::builder_with_provider(provider.clone())
            .with_safe_default_protocol_versions()
            .map_err(|e| galat_ws(format!("TLS: {e}")))?
            .with_root_certificates(akar)
            .with_no_client_auth();
        let nama = rustls::pki_types::ServerName::try_from(HOST_WS)
            .map_err(|e| galat_ws(format!("nama server: {e}")))?;
        let conn = ClientConnection::new(Arc::new(config), nama).map_err(|e| galat_ws(format!("TLS: {e}")))?;
        let mut tls = StreamOwned::new(conn, tcp);

        let mut kunci_mentah = [0u8; 16];
        acak(&provider, &mut kunci_mentah)?;
        let kunci = base64::engine::general_purpose::STANDARD.encode(kunci_mentah);
        tls.write_all(permintaan_handshake(HOST_WS, PATH_WS, &kunci).as_bytes())
            .map_err(|e| galat_ws(e))?;
        tls.flush().map_err(|e| galat_ws(e))?;

        // Baca balasan sampai akhir header (\r\n\r\n); sisanya = awal frame.
        let batas = Instant::now() + Duration::from_secs(15);
        let mut masuk: Vec<u8> = Vec::new();
        let akhir_header = loop {
            if let Some(p) = masuk.windows(4).position(|w| w == b"\r\n\r\n") {
                break p;
            }
            if Instant::now() >= batas {
                return Err(galat_ws("handshake tidak dibalas dalam 15 detik"));
            }
            let mut tmp = [0u8; 4096];
            match tls.read(&mut tmp) {
                Ok(0) => return Err(galat_ws("server menutup koneksi saat handshake")),
                Ok(n) => masuk.extend_from_slice(&tmp[..n]),
                Err(e) if kena_batas_waktu(&e) => continue,
                Err(e) => return Err(galat_ws(e)),
            }
        };
        let kepala = String::from_utf8_lossy(&masuk[..akhir_header]).into_owned();
        periksa_balasan_handshake(&kepala)?;
        let mut baca = PembacaFrame::default();
        baca.isi(&masuk[akhir_header + 4..]);
        Ok(Self { tls, baca, provider })
    }

    fn kirim_frame(&mut self, opcode: u8, isi: &[u8]) -> Hasil<()> {
        let mut topeng = [0u8; 4];
        acak(&self.provider, &mut topeng)?;
        self.tls.write_all(&kode_frame(opcode, isi, topeng)).map_err(|e| galat_ws(e))?;
        self.tls.flush().map_err(|e| galat_ws(e))
    }

    pub fn kirim_teks(&mut self, teks: &str) -> Hasil<()> {
        self.kirim_frame(0x1, teks.as_bytes())
    }

    pub fn kirim_pong(&mut self, data: &[u8]) -> Hasil<()> {
        self.kirim_frame(0xA, data)
    }

    /// Tutup sebaik mungkin; galat diabaikan.
    pub fn tutup(&mut self) {
        let _ = self.kirim_frame(0x8, &1000u16.to_be_bytes());
    }

    /// Frame berikutnya, atau Ok(None) bila belum ada data dalam `batas_baca`.
    pub fn terima(&mut self) -> Hasil<Option<Frame>> {
        loop {
            if let Some(f) = self.baca.berikut()? {
                return Ok(Some(f));
            }
            let mut tmp = [0u8; 8192];
            match self.tls.read(&mut tmp) {
                Ok(0) => return Err(galat_ws("koneksi ditutup server")),
                Ok(n) => self.baca.isi(&tmp[..n]),
                Err(e) if kena_batas_waktu(&e) => return Ok(None),
                Err(e) => return Err(galat_ws(e)),
            }
        }
    }
}

// ---------- Token (jaringan) ----------

fn minta_token(key: &str, secret: &str) -> Hasil<TokenWs> {
    let badan = badan_minta_token(key);
    let tanda = tanda_tangan_token(secret, &badan);
    let agen = ureq::AgentBuilder::new().timeout(Duration::from_secs(20)).build();
    let res = agen
        .post(URL_MINTA_TOKEN)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/json")
        .set("Content-Type", "application/x-www-form-urlencoded")
        .set("Sign", &tanda)
        .send_string(&badan);
    match res {
        Ok(r) => parse_token(&r.into_string().map_err(|e| Error::Jaringan(e.to_string()))?),
        Err(ureq::Error::Status(kode, r)) => {
            let isi = r.into_string().unwrap_or_default();
            Err(Error::Http { kode, isi_awal: isi.chars().take(400).collect() })
        }
        Err(e) => Err(Error::Jaringan(e.to_string())),
    }
}

// ---------- Alur pemantauan ----------

/// Kejadian untuk antarmuka (T1.4c meneruskannya ke Kotlin).
#[derive(Debug, Clone, PartialEq)]
pub enum KejadianPantau {
    /// Terautentikasi dan subscribe berhasil.
    Tersambung,
    /// Sambungan putus; akan dicoba lagi.
    Terputus { alasan: String, coba_lagi_detik: u64 },
    /// Order selesai dan trade-nya sudah tersimpan dari REST.
    OrderSelesai { simbol: String, order_id: String, sisi: Sisi, trade_baru: usize },
    /// Rekonsiliasi REST selesai.
    Rekonsiliasi { trade_baru: usize },
    /// Masalah yang tidak menghentikan pemantauan.
    Galat(String),
}

enum Sesi {
    Berhenti,
    Kedaluwarsa,
}

/// Kesalahan yang tidak ada gunanya diulang (kunci salah, key bukan baca saja).
pub fn galat_fatal(e: &Error) -> bool {
    if matches!(e, Error::KunciBukanBacaSaja) {
        return true;
    }
    let teks = e.to_string();
    ["invalid_tapi_key", "invalid_credentials", "bad_sign"].iter().any(|k| teks.contains(k))
}

fn tidur_bisa_dihentikan(detik: u64, berhenti: &AtomicBool) {
    for _ in 0..detik {
        if berhenti.load(Ordering::Relaxed) {
            return;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}

fn tunggu_balasan(
    ws: &mut SambunganWs,
    id: i64,
    label: &str,
    berhenti: &AtomicBool,
) -> Hasil<Option<i64>> {
    let batas = Instant::now() + Duration::from_secs(15);
    while Instant::now() < batas {
        if berhenti.load(Ordering::Relaxed) {
            return Err(galat_ws("dihentikan"));
        }
        match ws.terima()? {
            Some(Frame::Teks(t)) => {
                for p in parse_pesan(&t) {
                    match p {
                        PesanMasuk::Ping => ws.kirim_teks(BALASAN_PING)?,
                        PesanMasuk::Balasan { id: i, galat, ttl_detik } if i == id => {
                            if let Some((kode, pesan)) = galat {
                                return Err(galat_ws(format!("{label} ditolak: {kode} {pesan}")));
                            }
                            return Ok(ttl_detik);
                        }
                        _ => {}
                    }
                }
            }
            Some(Frame::Ping(d)) => ws.kirim_pong(&d)?,
            Some(Frame::Tutup(k)) => return Err(galat_ws(format!("server menutup koneksi (kode {k:?})"))),
            _ => {}
        }
    }
    Err(galat_ws(format!("tidak ada balasan {label} dalam 15 detik")))
}

/// Tarik trade koin dari event, ulangi bila REST belum memuat order itu.
fn proses_pemicu<F: FnMut(KejadianPantau)>(
    klien: &IndodaxClient,
    simpan: &mut Penyimpanan,
    pemicu: &Pemicu,
    lapor: &mut F,
) {
    let mut baru_total = 0usize;
    let mut percobaan = 0usize;
    while let Some(jeda) = jeda_ulang(percobaan) {
        percobaan += 1;
        if !jeda.is_zero() {
            std::thread::sleep(jeda);
        }
        match sinkron_koin(klien, simpan, &[pemicu.simbol.clone()], sekarang_ms(), &Opsi::default()) {
            Ok((baru, _)) => baru_total += baru,
            Err(e) => {
                lapor(KejadianPantau::Galat(format!("sinkron {} gagal: {e}", pemicu.simbol)));
                continue;
            }
        }
        if let Ok(trades) = simpan.semua_trade() {
            if order_sudah_tersimpan(&trades, &pemicu.order_id) {
                lapor(KejadianPantau::OrderSelesai {
                    simbol: pemicu.simbol.clone(),
                    order_id: pemicu.order_id.clone(),
                    sisi: pemicu.sisi,
                    trade_baru: baru_total,
                });
                return;
            }
        }
    }
    lapor(KejadianPantau::Galat(format!(
        "order {} belum muncul di REST; ditunda ke rekonsiliasi",
        pemicu.order_id
    )));
}

#[allow(clippy::too_many_arguments)]
fn satu_sesi<F: FnMut(KejadianPantau)>(
    klien: &IndodaxClient,
    key: &str,
    secret: &str,
    simpan: &mut Penyimpanan,
    berhenti: &AtomicBool,
    lapor: &mut F,
    sinkron_penuh_terakhir: &mut Option<Instant>,
    jeda: &mut u64,
) -> Hasil<Sesi> {
    // 1. Rekonsiliasi penuh sebelum tersambung (koin baru, saldo), kecuali baru saja.
    let perlu = sinkron_penuh_terakhir.map_or(true, |t| t.elapsed() >= JARAK_REKONSILIASI);
    if perlu {
        match sinkron(klien, simpan, &Opsi::default()) {
            Ok(h) => lapor(KejadianPantau::Rekonsiliasi { trade_baru: h.trade_baru }),
            Err(e) if galat_fatal(&e) => return Err(e),
            Err(e) => lapor(KejadianPantau::Galat(format!("rekonsiliasi gagal: {e}"))),
        }
        *sinkron_penuh_terakhir = Some(Instant::now());
    }
    if berhenti.load(Ordering::Relaxed) {
        return Ok(Sesi::Berhenti);
    }

    // 2. Token, sambung, autentikasi, subscribe.
    let token = minta_token(key, secret)?;
    let mut ws = SambunganWs::buka(Duration::from_secs(1))?;
    ws.kirim_teks(&pesan_connect(&token.token))?;
    let ttl = tunggu_balasan(&mut ws, ID_CONNECT, "autentikasi", berhenti)?;
    ws.kirim_teks(&pesan_subscribe(&token.channel))?;
    tunggu_balasan(&mut ws, ID_SUBSCRIBE, "subscribe", berhenti)?;
    *jeda = JEDA_AWAL_DETIK;
    lapor(KejadianPantau::Tersambung);

    // 3. Tutup celah antara sinkron terakhir dan subscribe: tarik ulang koin
    //    yang sudah dikenal (batas waktu per koin + tumpang tindih 60 dtk).
    if let Ok(koin) = simpan.simbol_tersimpan() {
        let opsi = Opsi { jeda_ms: 0, ..Opsi::default() };
        match sinkron_koin(klien, simpan, &koin, sekarang_ms(), &opsi) {
            Ok((baru, _)) if baru > 0 => lapor(KejadianPantau::Rekonsiliasi { trade_baru: baru }),
            Ok(_) => {}
            Err(e) => lapor(KejadianPantau::Galat(format!("sinkron celah gagal: {e}"))),
        }
    }

    // 4. Dengarkan event.
    let batas_umur = umur_sambungan_maks(ttl.unwrap_or(86_400));
    let mulai = Instant::now();
    let mut pemicu = PemicuSinkron::baru();
    loop {
        if berhenti.load(Ordering::Relaxed) {
            ws.tutup();
            return Ok(Sesi::Berhenti);
        }
        if mulai.elapsed() >= batas_umur {
            ws.tutup();
            return Ok(Sesi::Kedaluwarsa);
        }
        match ws.terima()? {
            None => {}
            Some(Frame::Ping(d)) => ws.kirim_pong(&d)?,
            Some(Frame::Tutup(k)) => return Err(galat_ws(format!("server menutup koneksi (kode {k:?})"))),
            Some(Frame::Teks(t)) => {
                for p in parse_pesan(&t) {
                    match p {
                        PesanMasuk::Ping => ws.kirim_teks(BALASAN_PING)?,
                        PesanMasuk::Balasan { galat: Some((kode, pesan)), .. } => {
                            return Err(galat_ws(format!("galat dari server: {kode} {pesan}")));
                        }
                        PesanMasuk::Event(e) => {
                            if let Some(pm) = pemicu.proses(&e) {
                                proses_pemicu(klien, simpan, &pm, lapor);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Some(_) => {}
        }
    }
}

/// Pantau order realtime sampai `berhenti` bernilai true. Sambung ulang
/// otomatis (jeda 1, 2, 4 ... maks 60 detik) dengan token baru. Mengembalikan
/// Err hanya untuk kesalahan yang tidak berguna diulang (kunci salah dsb.).
/// Memblokir: panggil dari thread latar.
pub fn pantau<F: FnMut(KejadianPantau)>(
    key: &str,
    secret: &str,
    path_db: &str,
    berhenti: &AtomicBool,
    mut lapor: F,
) -> Hasil<()> {
    let key = key.trim();
    let secret = secret.trim();
    if key.is_empty() || secret.is_empty() {
        return Err(Error::Jaringan("Key dan secret wajib diisi".into()));
    }
    let klien = IndodaxClient::new(key, secret);
    let mut simpan = Penyimpanan::buka(path_db)?;
    let mut jeda = JEDA_AWAL_DETIK;
    let mut sinkron_penuh_terakhir: Option<Instant> = None;
    while !berhenti.load(Ordering::Relaxed) {
        let hasil = satu_sesi(
            &klien,
            key,
            secret,
            &mut simpan,
            berhenti,
            &mut lapor,
            &mut sinkron_penuh_terakhir,
            &mut jeda,
        );
        match hasil {
            Ok(Sesi::Berhenti) => return Ok(()),
            Ok(Sesi::Kedaluwarsa) => {}
            Err(e) => {
                if berhenti.load(Ordering::Relaxed) {
                    return Ok(());
                }
                if galat_fatal(&e) {
                    return Err(e);
                }
                lapor(KejadianPantau::Terputus { alasan: e.to_string(), coba_lagi_detik: jeda });
                tidur_bisa_dihentikan(jeda, berhenti);
                jeda = (jeda * 2).min(JEDA_MAKS_DETIK);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ws::URL_WS_PRIVAT;

    /// Frame dari server (tanpa topeng), untuk uji.
    fn frame_server(fin: bool, op: u8, isi: &[u8]) -> Vec<u8> {
        let mut o = vec![(if fin { 0x80 } else { 0 }) | op];
        let n = isi.len();
        if n < 126 {
            o.push(n as u8);
        } else if n <= 0xFFFF {
            o.push(126);
            o.extend_from_slice(&(n as u16).to_be_bytes());
        } else {
            o.push(127);
            o.extend_from_slice(&(n as u64).to_be_bytes());
        }
        o.extend_from_slice(isi);
        o
    }

    #[test]
    fn url_konsisten_dengan_modul_ws() {
        assert_eq!(URL_WS_PRIVAT, format!("wss://{HOST_WS}{PATH_WS}"));
    }

    #[test]
    fn frame_klien_bertopeng_bisa_dibaca_kembali() {
        let kode = kode_frame(0x1, b"hello", [1, 2, 3, 4]);
        assert_eq!(kode[0], 0x81);
        assert_eq!(kode[1] & 0x80, 0x80, "frame klien wajib bertopeng");
        assert_ne!(&kode[6..], b"hello", "isi harus tersamar");
        let mut p = PembacaFrame::default();
        p.isi(&kode);
        assert_eq!(p.berikut().unwrap(), Some(Frame::Teks("hello".into())));
        assert_eq!(p.berikut().unwrap(), None);
    }

    #[test]
    fn panjang_menengah_dan_besar() {
        for n in [125usize, 126, 200, 70_000] {
            let isi = vec![b'a'; n];
            let mut p = PembacaFrame::default();
            p.isi(&kode_frame(0x2, &isi, [9, 8, 7, 6]));
            assert_eq!(p.berikut().unwrap(), Some(Frame::Biner(isi)), "n={n}");
            let isi = vec![b'b'; n];
            p.isi(&frame_server(true, 0x2, &isi));
            assert_eq!(p.berikut().unwrap(), Some(Frame::Biner(isi)), "server n={n}");
        }
    }

    #[test]
    fn data_datang_sepotong_sepotong() {
        let semua = frame_server(true, 0x1, "{}".as_bytes());
        let mut p = PembacaFrame::default();
        for (i, b) in semua.iter().enumerate() {
            p.isi(&[*b]);
            let r = p.berikut().unwrap();
            if i + 1 < semua.len() {
                assert_eq!(r, None, "byte ke-{i}");
            } else {
                assert_eq!(r, Some(Frame::Teks("{}".into())));
            }
        }
    }

    #[test]
    fn dua_frame_dalam_satu_bacaan_dan_kontrol() {
        let mut data = frame_server(true, 0x9, b"x");
        data.extend(frame_server(true, 0x1, b"abc"));
        data.extend(frame_server(true, 0x8, &[0x03, 0xE8, b'o', b'k']));
        let mut p = PembacaFrame::default();
        p.isi(&data);
        assert_eq!(p.berikut().unwrap(), Some(Frame::Ping(b"x".to_vec())));
        assert_eq!(p.berikut().unwrap(), Some(Frame::Teks("abc".into())));
        assert_eq!(p.berikut().unwrap(), Some(Frame::Tutup(Some(1000))));
        assert_eq!(p.berikut().unwrap(), None);
    }

    #[test]
    fn fragmentasi_dengan_ping_di_tengah() {
        let mut data = frame_server(false, 0x1, b"ab");
        data.extend(frame_server(true, 0x9, b"p"));
        data.extend(frame_server(false, 0x0, b"cd"));
        data.extend(frame_server(true, 0x0, b"ef"));
        let mut p = PembacaFrame::default();
        p.isi(&data);
        assert_eq!(p.berikut().unwrap(), Some(Frame::Ping(b"p".to_vec())));
        assert_eq!(p.berikut().unwrap(), Some(Frame::Teks("abcdef".into())));
    }

    #[test]
    fn frame_rusak_ditolak_bukan_panik() {
        let mut p = PembacaFrame::default();
        p.isi(&frame_server(true, 0x0, b"x")); // lanjutan tanpa awal
        assert!(p.berikut().is_err());

        let mut p = PembacaFrame::default();
        p.isi(&frame_server(true, 0x3, b"x")); // opcode asing
        assert!(p.berikut().is_err());

        let mut p = PembacaFrame::default();
        let mut besar = vec![0x82, 127];
        besar.extend_from_slice(&((MAKS_PESAN as u64) + 1).to_be_bytes());
        p.isi(&besar);
        assert!(p.berikut().is_err());
    }

    #[test]
    fn handshake_tanpa_origin_dan_periksa_balasan() {
        let r = permintaan_handshake(HOST_WS, PATH_WS, "KUNCI");
        assert!(r.starts_with("GET /ws/?cf_ws_frame_ping_pong=true HTTP/1.1\r\n"));
        assert!(r.contains("Host: pws.indodax.com\r\n"));
        assert!(r.contains("Sec-WebSocket-Key: KUNCI\r\n"));
        assert!(r.ends_with("\r\n\r\n"));
        assert!(!r.to_lowercase().contains("origin"), "jangan kirim header Origin");

        assert!(periksa_balasan_handshake("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket").is_ok());
        assert!(periksa_balasan_handshake("HTTP/1.1 403 Forbidden").is_err());
        assert!(periksa_balasan_handshake("").is_err());
    }

    #[test]
    fn galat_fatal_hanya_untuk_kunci_bermasalah() {
        assert!(galat_fatal(&Error::KunciBukanBacaSaja));
        assert!(galat_fatal(&Error::Json("token WebSocket ditolak: bad_sign (x)".into())));
        assert!(galat_fatal(&Error::Http { kode: 401, isi_awal: r#"{"error_code":"invalid_credentials"}"#.into() }));
        assert!(!galat_fatal(&Error::Jaringan("timeout".into())));
        assert!(!galat_fatal(&Error::Http { kode: 500, isi_awal: "oops".into() }));
    }
}
