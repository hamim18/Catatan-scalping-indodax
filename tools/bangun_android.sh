#!/usr/bin/env bash
# Membangun inti Rust untuk Android, membuat binding Kotlin (UniFFI), lalu APK debug.
#   bash tools/bangun_android.sh                      build saja
#   bash tools/bangun_android.sh --pasang [SERIAL]    build lalu pasang ke HP
#   bash tools/bangun_android.sh --hanya-pasang [SERIAL]   pasang APK yang sudah ada (tanpa build)
# SERIAL dari `adb devices` (atau atur ANDROID_SERIAL). Wajib bila lebih dari satu perangkat.
# Butuh: rustup (target aarch64-linux-android), cargo-ndk, ANDROID_HOME, ANDROID_NDK_HOME, gradle.
set -euo pipefail
cd "$(dirname "$0")/.."

MODE="${1:-}"
SERIAL="${2:-${ANDROID_SERIAL:-}}"
APK_DIR="app/aplikasi/build/outputs/apk/debug"

pasang() {
  local apk
  apk="$(find "$APK_DIR" -name '*.apk' 2>/dev/null | head -n1)"
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
  echo "Memasang ke $SERIAL ..."
  adb -s "$SERIAL" install -r "$apk"
  echo "Terpasang. Buka aplikasi 'Catatan Scalping' di HP."
}

if [ "$MODE" = "--hanya-pasang" ]; then pasang; exit 0; fi

: "${ANDROID_NDK_HOME:?ANDROID_NDK_HOME belum diatur}"
: "${ANDROID_HOME:?ANDROID_HOME belum diatur}"

echo "== 1/4 Build Rust untuk host (sumber metadata binding) =="
cargo build -p scalping-ffi

echo "== 2/4 Hasilkan binding Kotlin =="
OUT="app/aplikasi/src/main/java"
mkdir -p "$OUT"
cargo run -q -p uniffi-bindgen -- generate target/debug/libscalping_ffi.so \
  --language kotlin --out-dir "$OUT" --no-format

echo "== 3/4 Build Rust untuk Android (arm64-v8a) =="
cargo ndk -t arm64-v8a -o app/aplikasi/src/main/jniLibs build -p scalping-ffi --release

echo "== 4/4 Build APK debug =="
( cd app && if [ -x ./gradlew ]; then ./gradlew :aplikasi:assembleDebug; else gradle :aplikasi:assembleDebug; fi )
echo "APK: $(find "$APK_DIR" -name '*.apk' | head -n1)"

if [ "$MODE" = "--pasang" ]; then pasang; fi
