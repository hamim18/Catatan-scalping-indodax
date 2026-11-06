//! Tes koneksi langsung (FR-02): memanggil GET /api/v2/account dengan key baca saja.
//! Membaca INDODAX_API_KEY / INDODAX_API_SECRET dari lingkungan atau file .env.
//! Hanya mencetak status, bukan isi saldo.
//!   cargo run -p scalping-core --example cek_koneksi
use scalping_core::IndodaxClient;

fn muat_env() {
    if let Ok(t) = std::fs::read_to_string(".env") {
        for b in t.lines() {
            let b = b.trim();
            if b.is_empty() || b.starts_with('#') { continue; }
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
    match IndodaxClient::new(key, secret).ambil_akun_mentah() {
        Ok(isi) => println!("Tersambung (HTTP 200). Panjang respons: {} karakter.", isi.len()),
        Err(e) => { eprintln!("Gagal: {e}"); std::process::exit(2); }
    }
}
