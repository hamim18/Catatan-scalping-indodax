//! Tanda tangan HMAC-SHA256 untuk TAPI v2 (terbukti di Fase 0).
use hmac::{Hmac, Mac};
use sha2::Sha256;

pub fn tanda_tangan(secret: &str, query: &str) -> String {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC menerima kunci apa pun");
    mac.update(query.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vektor_uji_baku_hmac_sha256() {
        assert_eq!(
            tanda_tangan("key", "The quick brown fox jumps over the lazy dog"),
            "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8"
        );
    }
}
