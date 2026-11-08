#!/usr/bin/env bash
# Membangun inti Rust untuk Android, membuat binding Kotlin (UniFFI), lalu APK debug.
#   bash tools/bangun_android.sh                      build saja
#   bash tools/bangun_android.sh --pasang [SERIAL]    build lalu pasang ke HP
#   bash tools/bangun_android.sh --hanya-pasang [SERIAL]   pasang APK yang sudah ada (tanpa build)
# SERIAL dari `adb devices` (atau atur ANDROID_SERIAL). Wajib bila lebih dari satu perangkat.
# Butuh: rustup (target aarch64-linux-android), cargo-ndk, ANDROID_HOME, ANDROID_NDK_HOME, gradle.
#
# Perubahan (T5.2): setelah build, skrip memeriksa bahwa kode Rust (.text) di dalam APK
# SAMA dengan hasil cargo ndk yang baru. Bila beda, build dihentikan (mencegah APK dengan
# Kotlin baru tetapi Rust lama). Pemasangan juga memakai APK terbaru, bukan sembarang APK.
set -euo pipefail
cd "$(dirname "$0")/.."

MODE="${1:-}"
SERIAL="${2:-${ANDROID_SERIAL:-}}"
APK_DIR="app/aplikasi/build/outputs/apk/debug"
JNI_DIR="app/aplikasi/src/main/jniLibs"
SO_NAMA="libscalping_ffi.so"

apk_terbaru() {
  find "$APK_DIR" -name '*.apk' -printf '%T@ %p\n' 2>/dev/null | sort -nr | head -n1 | cut -d' ' -f2-
}

sha() { sha256sum "$1" | cut -d' ' -f1; }

pasang() {
  local apk
  apk="$(apk_terbaru)"
  [ -n "$apk" ] || { echo "APK belum ada. Jalankan build dulu."; exit 1; }
  if [ -z "$SERIAL" ]; then
    mapfile -t perangkat < <(adb devices | awk 'NR>1 && $2=="device" {print $1}')
    if [ "${#perangkat[@]}" -eq 0 ]; then echo "Tidak ada perangkat adb yang tersambung."; exit 1; fi
    if [ "${#perangkat[@]}" -gt 1 ]; then
      echo "Ada ${#perangkat[@]} perangkat. Pilih salah satu serial:"
      printf '  %s\n' "${perangkat[@]}"
      echo "Contoh: bash tools/bangun_android.sh --hanya-pasang ${perangkat[0]}"
      exit 1
    fi
    SERIAL="${perangkat[0]}"
  fi
  echo "APK: $apk (dibuat $(date -r "$apk" '+%d %b %H:%M:%S'))"
  echo "Memasang ke $SERIAL ..."
  adb -s "$SERIAL" install -r "$apk"
  echo "Terpasang. Buka aplikasi 'Catatan Scalping' di HP."
}

if [ "$MODE" = "--hanya-pasang" ]; then
  echo "PERHATIAN: --hanya-pasang TIDAK membangun ulang. Ini memasang APK lama bila kode berubah."
  pasang
  exit 0
fi

: "${ANDROID_NDK_HOME:?ANDROID_NDK_HOME belum diatur}"
: "${ANDROID_HOME:?ANDROID_HOME belum diatur}"

PENANDA="$(mktemp)"
trap 'rm -f "$PENANDA"' EXIT

echo "== 1/4 Build Rust untuk host (sumber metadata binding) =="
cargo build -p scalping-ffi

echo "== 2/4 Hasilkan binding Kotlin =="
OUT="app/aplikasi/src/main/java"
mkdir -p "$OUT"
cargo run -q -p uniffi-bindgen -- generate target/debug/libscalping_ffi.so \
  --language kotlin --out-dir "$OUT" --no-format

echo "== 3/4 Build Rust untuk Android (arm64-v8a) =="
cargo ndk -t arm64-v8a -o "$JNI_DIR" build -p scalping-ffi --release
SO_BARU="$JNI_DIR/arm64-v8a/$SO_NAMA"
[ -f "$SO_BARU" ] || { echo "GAGAL: $SO_BARU tidak ada setelah cargo ndk."; exit 1; }
echo "   $SO_BARU  ($(date -r "$SO_BARU" '+%d %b %H:%M:%S'))  sha256 $(sha "$SO_BARU" | cut -c1-12)"

echo "== 4/4 Build APK debug =="
( cd app && if [ -x ./gradlew ]; then ./gradlew :aplikasi:assembleDebug; else gradle :aplikasi:assembleDebug; fi )
APK="$(apk_terbaru)"
[ -n "$APK" ] || { echo "GAGAL: APK tidak ditemukan di $APK_DIR."; exit 1; }
echo "APK: $APK"

# Pastikan kode Rust di dalam APK sama dengan hasil cargo ndk yang baru.
# Hash seluruh berkas TIDAK bisa dipakai: AGP menjalankan strip pada .so di APK sehingga
# hash selalu beda walau isinya baru. Yang dibandingkan adalah isi section .text (kode mesin),
# yang tidak berubah oleh strip.
OBJCOPY="$(ls "$ANDROID_NDK_HOME"/toolchains/llvm/prebuilt/*/bin/llvm-objcopy 2>/dev/null | head -n1 || true)"
[ -n "$OBJCOPY" ] || OBJCOPY="$(command -v llvm-objcopy || command -v objcopy || true)"
if command -v unzip >/dev/null 2>&1 && [ -n "$OBJCOPY" ]; then
  TMP="$(mktemp -d)"
  unzip -p "$APK" "lib/arm64-v8a/$SO_NAMA" > "$TMP/apk.so"
  "$OBJCOPY" -O binary --only-section=.text "$TMP/apk.so" "$TMP/apk.text"
  "$OBJCOPY" -O binary --only-section=.text "$SO_BARU" "$TMP/baru.text"
  H_APK="$(sha "$TMP/apk.text")"; H_BARU="$(sha "$TMP/baru.text")"
  rm -rf "$TMP"
  if [ "$H_APK" != "$H_BARU" ]; then
    echo "GAGAL: kode Rust di dalam APK BERBEDA dari hasil build terbaru (.text)."
    echo "  APK : ${H_APK:0:12}"
    echo "  Baru: ${H_BARU:0:12}"
    echo "Coba: (cd app && gradle clean) lalu jalankan skrip ini lagi."
    exit 1
  fi
  echo "OK: kode Rust di APK sama dengan hasil build terbaru (.text ${H_APK:0:12})."
else
  echo "PERINGATAN: 'unzip' atau 'objcopy' tidak ada, pemeriksaan isi APK dilewati."
fi

if [ "$MODE" = "--pasang" ]; then pasang; fi

