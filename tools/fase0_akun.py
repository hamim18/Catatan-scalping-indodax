#!/usr/bin/env python3
"""
Lihat BENTUK respons GET /api/v2/account tanpa membocorkan saldo (hanya baca).
Semua angka diganti 9, jadi aman ditempel ke chat. Respons mentah tidak disimpan.

  python tools/fase0_akun.py
Memakai INDODAX_API_KEY / INDODAX_API_SECRET dari .env (sama seperti fase0_validasi.py).
"""
import hashlib, hmac, json, os, re, sys, time
import urllib.error, urllib.parse, urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
UA = ("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) "
      "Chrome/124.0 Safari/537.36")


def muat_env():
    f = ROOT / ".env"
    if f.exists():
        for b in f.read_text(encoding="utf-8").splitlines():
            b = b.strip()
            if b and not b.startswith("#") and "=" in b:
                k, v = b.split("=", 1)
                os.environ.setdefault(k.strip(), v.strip().strip('"').strip("'"))


def angka9(s):
    return re.sub(r"\d", "9", str(s))


def topeng(x, kedalaman=0):
    """Pertahankan nama kunci dan bentuk nilai; angka -> 9."""
    if isinstance(x, dict):
        item = list(x.items())
        out = {k: topeng(v, kedalaman + 1) for k, v in item[:6]}
        if len(item) > 6:
            out[f"...(+{len(item) - 6} kunci lain)"] = "..."
        return out
    if isinstance(x, list):
        return [topeng(v, kedalaman + 1) for v in x[:2]] + ([f"...(+{len(x) - 2} item)"] if len(x) > 2 else [])
    if isinstance(x, bool) or x is None:
        return x
    if isinstance(x, (int, float)):
        return "<angka> " + angka9(x)
    return "<teks> " + angka9(x)


def main():
    muat_env()
    key, secret = os.environ.get("INDODAX_API_KEY", ""), os.environ.get("INDODAX_API_SECRET", "")
    if not key or not secret or key.startswith("isi_"):
        sys.exit("Isi INDODAX_API_KEY dan INDODAX_API_SECRET di file .env dulu.")
    qs = urllib.parse.urlencode({"timestamp": int(time.time() * 1000), "recvWindow": 10000})
    sign = hmac.new(secret.encode(), qs.encode(), hashlib.sha256).hexdigest()
    req = urllib.request.Request(
        f"https://api.indodax.com/api/v2/account?{qs}",
        headers={"User-Agent": UA, "Accept": "application/json", "X-APIKEY": key, "Sign": sign},
    )
    try:
        with urllib.request.urlopen(req, timeout=20) as r:
            kode, isi = r.status, r.read().decode(errors="replace")
    except urllib.error.HTTPError as e:
        kode, isi = e.code, e.read().decode(errors="replace")
    print(f"HTTP {kode}")
    try:
        print(json.dumps(topeng(json.loads(isi)), indent=2, ensure_ascii=False))
    except Exception:
        print("Bukan JSON. Awal isi:", angka9(isi[:200]))


if __name__ == "__main__":
    main()
