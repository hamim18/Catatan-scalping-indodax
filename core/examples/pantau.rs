//! Uji T1.4b di komputer (tanpa Android): pantau order realtime lewat WebSocket
//! privat, tarik trade lewat REST saat order selesai. HANYA MEMBACA.
//! Membaca INDODAX_API_KEY / INDODAX_API_SECRET dari lingkungan atau file .env.
//! Database uji: data/pantau_uji.db (folder data/ tidak di-commit).
//!   cargo run -p scalping-core --example pantau
//! Tekan Enter untuk berhenti. Saat berjalan, lakukan transaksi kecil di Indodax.
use scalping_core::ws_klien::{pantau, KejadianPantau};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn muat_env() {
    if let Ok(t) = std::fs::read_to_string(".env") {
        for b in t.lines() {
            let b = b.trim();
            if b.is_empty() || b.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = b.split_once('=') {
                if std::env::var(k.trim()).is_err() {
                    std::env::set_var(k.trim(), v.trim().trim_matches('"').trim_matches('\''));
                }
            }
        }
    }
}

fn main() {
    muat_env();
    let key = std::env::var("INDODAX_API_KEY").unwrap_or_default();
    let secret = std::env::var("INDODAX_API_SECRET").unwrap_or_default();
    if key.is_empty() || secret.is_empty() || key.starts_with("isi_") {
        eprintln!("Isi INDODAX_API_KEY dan INDODAX_API_SECRET di .env dulu.");
        std::process::exit(1);
    }
    let _ = std::fs::create_dir_all("data");
    let berhenti = Arc::new(AtomicBool::new(false));
    {
        let b = berhenti.clone();
        std::thread::spawn(move || {
            let mut s = String::new();
            let _ = std::io::stdin().read_line(&mut s);
            b.store(true, Ordering::Relaxed);
        });
    }
    println!("Memantau. Tekan Enter untuk berhenti.\n");
    let t0 = std::time::Instant::now();
    let hasil = pantau(&key, &secret, "data/pantau_uji.db", &berhenti, |k| {
        let d = t0.elapsed().as_secs();
        match k {
            KejadianPantau::Tersambung => println!("[{d:>4}s] tersambung dan subscribe berhasil"),
            KejadianPantau::Terputus { alasan, coba_lagi_detik } => {
                println!("[{d:>4}s] terputus: {alasan} (coba lagi {coba_lagi_detik} dtk)")
            }
            KejadianPantau::OrderSelesai { simbol, order_id, sisi, trade_baru } => println!(
                "[{d:>4}s] ORDER SELESAI {simbol} {sisi:?} {order_id} (trade baru: {trade_baru})"
            ),
            KejadianPantau::Rekonsiliasi { trade_baru } => {
                println!("[{d:>4}s] rekonsiliasi REST: {trade_baru} trade baru")
            }
            KejadianPantau::Galat(p) => println!("[{d:>4}s] galat: {p}"),
        }
    });
    match hasil {
        Ok(()) => println!("Berhenti."),
        Err(e) => {
            eprintln!("Berhenti karena galat: {e}");
            std::process::exit(2);
        }
    }
}
