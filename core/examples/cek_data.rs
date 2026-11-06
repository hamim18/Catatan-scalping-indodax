//! Baca file mentah hasil fase0_validasi.py (offline, tanpa key) dan cetak
//! ringkasan per order memakai kode Rust. Bandingkan dengan keluaran Python.
//!   cargo run -p scalping-core --example cek_data -- data/raw_icntidr_XXXX.json
use scalping_core::{calc::bulatkan_idr, gabung_per_order, parse_my_trades, Posisi, Sisi};
use std::str::FromStr;

fn main() {
    let path = std::env::args().nth(1).expect("pemakaian: cek_data <file.json>");
    let teks = std::fs::read_to_string(&path).expect("file tidak terbaca");
    let trades = parse_my_trades(&teks).expect("JSON tidak dikenali");
    let orders = gabung_per_order(&trades).expect("gabung gagal");
    let tp = rust_decimal::Decimal::from_str("1").unwrap();
    println!("{} trade -> {} order", trades.len(), orders.len());
    for o in &orders {
        println!(
            "\n{}  [{}]  {} pengisian  koin {}  nilai {}  fee {}",
            o.order_id,
            if o.sisi == Sisi::Beli { "BELI" } else { "JUAL" },
            o.jumlah_pengisian, o.koin, o.nilai, o.fee
        );
        if o.sisi == Sisi::Beli {
            let p = Posisi::dari_beli(o).unwrap();
            println!(
                "  IDR keluar {}  modal efektif {}  TP1% {}  SL1% {}",
                o.idr_keluar(),
                p.modal_efektif().unwrap().round_dp(2),
                bulatkan_idr(p.harga_tp(tp).unwrap()),
                bulatkan_idr(p.harga_sl(tp).unwrap())
            );
        } else {
            println!("  IDR diterima {}", o.idr_diterima());
        }
    }
}
