#!/usr/bin/env python3
"""
Cek lingkungan pengembangan Android + Rust (hanya BACA, tidak mengubah apa pun,
tidak memakai key API). Hasilnya aman ditempel ke chat.

  python tools/cek_lingkungan.py

Python 3.8+, tanpa pip install.
"""
import json, os, platform, shutil, subprocess, sys
from pathlib import Path

HOME = str(Path.home())


def aman(teks):
    """Sembunyikan nama pengguna di jalur."""
    return str(teks).replace(HOME, "~")


def jalankan(perintah, timeout=25):
    try:
        r = subprocess.run(perintah, capture_output=True, text=True, timeout=timeout)
        return (r.stdout + r.stderr).strip()
    except Exception as e:
        return f"(gagal: {e})"


def versi(nama, args=("--version",)):
    p = shutil.which(nama)
    if not p:
        return None
    return jalankan([p, *args])


def baris(label, nilai):
    print(f"{label:<22}: {nilai}")


def cari_sdk():
    for k in ("ANDROID_HOME", "ANDROID_SDK_ROOT"):
        v = os.environ.get(k)
        if v and Path(v).is_dir():
            return Path(v), k
    kandidat = [
        Path(HOME) / "AppData/Local/Android/Sdk",          # Windows
        Path(HOME) / "Library/Android/sdk",                # macOS
        Path(HOME) / "Android/Sdk",                        # Linux
    ]
    for c in kandidat:
        if c.is_dir():
            return c, "lokasi bawaan (variabel lingkungan belum diatur)"
    return None, None


def daftar(folder):
    return sorted(x.name for x in folder.iterdir() if x.is_dir()) if folder.is_dir() else []


def android_studio():
    kandidat = [
        r"C:\Program Files\Android\Android Studio\product-info.json",
        "/Applications/Android Studio.app/Contents/Resources/product-info.json",
        str(Path(HOME) / "android-studio/product-info.json"),
        "/opt/android-studio/product-info.json",
        "/usr/local/android-studio/product-info.json",
    ]
    for c in kandidat:
        if Path(c).is_file():
            try:
                d = json.loads(Path(c).read_text(encoding="utf-8"))
                return f"{d.get('name', 'Android Studio')} {d.get('version', '?')} (build {d.get('buildNumber', '?')})"
            except Exception:
                return "ditemukan, tapi info versi tidak terbaca"
    return None


def main():
    print("== Cek lingkungan Catatan Scalping ==\n")
    baris("Sistem operasi", f"{platform.system()} {platform.release()} ({platform.machine()})")
    baris("Python", sys.version.split()[0])
    g = versi("git")
    baris("git", g or "TIDAK ADA")

    print("\n-- Rust --")
    baris("rustc", versi("rustc") or "TIDAK ADA")
    baris("cargo", versi("cargo") or "TIDAK ADA")
    if shutil.which("rustup"):
        baris("rustup", (versi("rustup") or "").splitlines()[0])
        tg = jalankan(["rustup", "target", "list", "--installed"]).split()
        android = [t for t in tg if "android" in t]
        baris("target Android", ", ".join(android) if android else "BELUM ADA (dibutuhkan T1.1b)")
    else:
        baris("rustup", "TIDAK ADA")
    cn = jalankan(["cargo", "ndk", "--version"]) if shutil.which("cargo") else ""
    baris("cargo-ndk", cn.splitlines()[0] if cn and "no such command" not in cn.lower() and "gagal" not in cn else "belum ada (dibutuhkan T1.1b)")

    print("\n-- Java / Gradle --")
    j = versi("java", ("-version",))
    baris("java", (j.splitlines()[0] if j else "TIDAK ADA di PATH"))
    jh = os.environ.get("JAVA_HOME")
    baris("JAVA_HOME", aman(jh) if jh else "belum diatur (tidak masalah jika Android Studio dipakai)")
    gr = versi("gradle", ("--version",))
    if gr:
        for l in gr.splitlines():
            if l.startswith("Gradle "):
                baris("gradle (PATH)", l)
                break
    else:
        baris("gradle (PATH)", "tidak ada di PATH (normal; proyek memakai gradle wrapper)")

    print("\n-- Android --")
    baris("Android Studio", android_studio() or "tidak ditemukan otomatis; lihat Help > About lalu tulis manual")
    sdk, asal = cari_sdk()
    if not sdk:
        baris("Android SDK", "TIDAK DITEMUKAN")
    else:
        baris("Android SDK", f"{aman(sdk)}  [{asal}]")
        baris("platforms", ", ".join(daftar(sdk / "platforms")) or "kosong")
        baris("build-tools", ", ".join(daftar(sdk / "build-tools")) or "kosong")
        baris("NDK", ", ".join(daftar(sdk / "ndk")) or "BELUM TERPASANG (dibutuhkan T1.1b)")
        baris("cmdline-tools", ", ".join(daftar(sdk / "cmdline-tools")) or "tidak ada")
        baris("platform-tools", "ada" if (sdk / "platform-tools").is_dir() else "tidak ada")
    baris("ANDROID_NDK_HOME", aman(os.environ.get("ANDROID_NDK_HOME", "belum diatur")))

    print("\n-- Perangkat (adb) --")
    adb = shutil.which("adb")
    if not adb and sdk:
        for nama in ("adb", "adb.exe"):
            cand = sdk / "platform-tools" / nama
            if cand.is_file():
                adb = str(cand)
                break
    if not adb:
        baris("adb", "tidak ditemukan")
    else:
        keluaran = jalankan([adb, "devices"]).splitlines()[1:]
        perangkat = [l.split()[0] for l in keluaran if l.strip().endswith("device")]
        if not perangkat:
            baris("HP tes", "tidak ada yang tersambung (untuk adb nirkabel jalankan dulu: adb connect <ip>:<port>)")
        for i, s in enumerate(perangkat, 1):
            rilis = jalankan([adb, "-s", s, "shell", "getprop", "ro.build.version.release"])
            api = jalankan([adb, "-s", s, "shell", "getprop", "ro.build.version.sdk"])
            abi = jalankan([adb, "-s", s, "shell", "getprop", "ro.product.cpu.abi"])
            baris(f"perangkat {i}", f"Android {rilis} (API {api}), ABI {abi}")

    print("\nSelesai. Tempel seluruh keluaran di atas ke chat.")


if __name__ == "__main__":
    main()
