#!/usr/bin/env python3
"""
Fase 0 - Validasi data Indodax (hanya BACA, tidak membuat order).

Tujuan: mengambil trade nyata lewat GET /api/v2/myTrades lalu menyimpan
respons mentahnya, supaya kita bisa memastikan:
  (a) fee dipotong dari IDR atau dari koin yang diterima
  (b) total biaya di struk = nilai commission dari API
  (c) rumus modal efektif di PRD benar

Hanya memakai pustaka bawaan Python 3.8+ (tanpa pip install).

Contoh:
  python tools/fase0_validasi.py icntidr
  python tools/fase0_validasi.py icntidr --hari 3
  python tools/fase0_validasi.py icntidr --algo sha512
"""
import argparse, hashlib, hmac, json, os, sys, time
import urllib.error, urllib.parse, urllib.request
from pathlib import Path

BASE = "https://api.indodax.com"
ROOT = Path(__file__).resolve().parent.parent


def muat_env():
    """Baca .env sederhana (KEY=VALUE) tanpa pustaka tambahan."""
    f = ROOT / ".env"
    if f.exists():
        for baris in f.read_text(encoding="utf-8").splitlines():
            baris = baris.strip()
            if baris and not baris.startswith("#") and "=" in baris:
                k, v = baris.split("=", 1)
                os.environ.setdefault(k.strip(), v.strip().strip('"').strip("'"))


def panggil(path, params, key, secret, algo):
    """GET bertanda tangan. Sign = HMAC(secret, query string) heksadesimal."""
    params = dict(params)
    params["timestamp"] = int(time.time() * 1000)
    params["recvWindow"] = 10000
    qs = urllib.parse.urlencode(params)
    digest = hashlib.sha256 if algo == "sha256" else hashlib.sha512
    sign = hmac.new(secret.encode(), qs.encode(), digest).hexdigest()
    req = urllib.request.Request(
        f"{BASE}{path}?{qs}",
        headers={"Accept": "application/json", "Content-Type": "application/json",
                 "X-APIKEY": key, "Sign": sign},
    )
    try:
        with urllib.request.urlopen(req, timeout=20) as r:
            return r.status, json.loads(r.read().decode())
    except urllib.error.HTTPError as e:
        try:
            return e.code, json.loads(e.read().decode())
        except Exception:
            return e.code, {"error": "respons bukan JSON"}


def main():
    ap = argparse.ArgumentParser(description="Fase 0: validasi data Indodax (baca saja)")
    ap.add_argument("symbol", help="pasangan, contoh: icntidr (coba juga icnt_idr bila gagal)")
    ap.add_argument("--hari", type=int, default=0,
                    help="ambil N hari ke belakang (maks 7). Default: bawaan API (24 jam)")
    ap.add_argument("--limit", type=int, default=100)
    ap.add_argument("--algo", choices=["auto", "sha256", "sha512"], default="auto",
                    help="algoritma tanda tangan. PRD menulis SHA256, dokumen lain SHA512, 'auto' mencoba keduanya")
    a = ap.parse_args()

    muat_env()
    key, secret = os.environ.get("INDODAX_API_KEY", ""), os.environ.get("INDODAX_API_SECRET", "")
    if not key or not secret or key.startswith("isi_"):
        sys.exit("Isi INDODAX_API_KEY dan INDODAX_API_SECRET di file .env dulu (lihat .env.example).")

    algos = ["sha256", "sha512"] if a.algo == "auto" else [a.algo]

    # Langkah 1: cek koneksi & algoritma lewat /account
    algo_ok = None
    for algo in algos:
        kode, res = panggil("/api/v2/account", {}, key, secret, algo)
        print(f"[cek akun] algoritma={algo} -> HTTP {kode}")
        if kode == 200:
            algo_ok = algo
            break
        print("   respons:", json.dumps(res, ensure_ascii=False)[:300])
    if not algo_ok:
        sys.exit("Gagal tersambung. Periksa: key harus khusus TAPI v2, izin baca aktif, "
                 "tanpa pembatasan IP. Kirim pesan error di atas ke Claude.")
    print(f"Tersambung. Algoritma yang berhasil: {algo_ok}\n")

    # Langkah 2: ambil trade
    params = {"symbol": a.symbol, "limit": a.limit}
    if a.hari:
        sekarang = int(time.time() * 1000)
        params["startTime"] = sekarang - min(a.hari, 7) * 86400 * 1000
        params["endTime"] = sekarang
    kode, res = panggil("/api/v2/myTrades", params, key, secret, algo_ok)
    print(f"[myTrades] HTTP {kode}")
    teks = json.dumps(res, indent=2, ensure_ascii=False)
    print(teks)

    out = ROOT / "data" / f"raw_{a.symbol}_{time.strftime('%Y%m%d_%H%M%S')}.json"
    out.write_text(teks, encoding="utf-8")
    print(f"\nRespons mentah disimpan: {out}")
    print("Bandingkan dengan struk Indodax, lalu kirim isi di atas ke Claude (key/secret tidak tercetak).")


if __name__ == "__main__":
    main()
