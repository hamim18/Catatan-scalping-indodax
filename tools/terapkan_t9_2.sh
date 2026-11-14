#!/usr/bin/env bash
# T9.2: menambah 2 baris kecil di berkas yang sudah ada (aman dijalankan berulang).
set -euo pipefail
cd "$(dirname "$0")/.."

# core/src/lib.rs: modul dan ekspor setelan Screener
if ! grep -q '^pub mod screener_setelan;' core/src/lib.rs; then
  sed -i 's/^pub mod screener;$/pub mod screener;\npub mod screener_setelan;/' core/src/lib.rs
fi
if ! grep -q 'screener_setelan::' core/src/lib.rs; then
  printf '\npub use screener_setelan::{muat_konfig, periksa_konfig, simpan_konfig};\n' >> core/src/lib.rs
fi

# ffi/src/lib.rs: modul jembatan Screener
if ! grep -q '^mod screener_ffi;' ffi/src/lib.rs; then
  sed -i 's/^uniffi::setup_scaffolding!();$/uniffi::setup_scaffolding!();\n\nmod screener_ffi;/' ffi/src/lib.rs
fi

grep -n 'screener_setelan' core/src/lib.rs
grep -n 'screener_ffi' ffi/src/lib.rs

# Aturan PROGRESS: Cargo tidak mengompilasi ulang berkas yang waktunya lebih tua dari build terakhir.
touch core/src/*.rs ffi/src/lib.rs ffi/src/screener_ffi.rs app/aplikasi/src/main/kotlin/id/catatanscalping/app/*.kt
echo "Selesai. Lanjut: cargo test -p scalping-core && cargo test -p scalping-ffi"
