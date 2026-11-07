#!/usr/bin/env python3
"""
Lihat BENTUK respons GET /api/v2/fiat/orders (riwayat deposit/withdraw IDR). Hanya baca.
Nama kunci dan bentuk nilai dipertahankan; angka diganti 9 dan teks diganti x,
jadi aman ditempel ke chat. Respons mentah TIDAK disimpan.

  python tools/fase2_fiat.py                # deposit (0), 29 hari terakhir
  python tools/fase2_fiat.py --jenis 1      # withdraw (1)
  python tools/fase2_fiat.py --hari 20

Memakai INDODAX_API_KEY / INDODAX_API_SECRET dari .env (sama seperti fase0_akun.py).
Nama parameter (transactionType, beginTime, endTime) BELUM terkonfirmasi dari dokumen resmi
Indodax. Skrip mencoba camelCase dulu, lalu snake_case bila HTTP 400.
"""
import argparse, hashlib, hmac, json, os, re, sys, time
import urllib.error, urllib.parse, urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
UA = ("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) "
      "Chrome/124.0 Safari/537.36")
# Nilai teks yang boleh tampil apa adanya bila kuncinya mengandung salah satu kata ini
KUNCI_AMAN = ("status", "type", "currency", "asset", "method", "state", "code", "success", "message")


def muat_env():
    f = ROOT / ".env"
    if f.exists():
        for b in f.read_text(encoding="utf-8").splitlines():
            b = b.strip()
            if b and not b.startswith("#") and "=" in b:
                k, v = b.split("=", 1)
                os.environ.setdefault(k.strip(), v.strip().strip('"').strip("'"))


def bentuk(s):
    """Angka -> 9, huruf -> x/X, tanda baca tetap. Nama dan nomor rekening tidak bocor."""
    s = re.sub(r"\d", "9", str(s))
    s = re.sub(r"[a-z]", "x", s)
    return re.sub(r"[A-Z]", "X", s)


def topeng(x, kunci=""):
    if isinstance(x, dict):
        item = list(x.items())
        out = {k: topeng(v, k) for k, v in item[:14]}
        if len(item) > 14:
            out[f"...(+{len(item) - 14} kunci lain)"] = "..."
        return out
    if isinstance(x, list):
        return [topeng(v, kunci) for v in x[:2]] + ([f"...(+{len(x) - 2} item)"] if len(x) > 2 else [])
    if isinstance(x, bool) or x is None:
        return x
    if isinstance(x, (int, float)):
        return "<angka> " + bentuk(x)
    if any(w in kunci.lower() for w in KUNCI_AMAN) and len(str(x)) <= 30:
        return "<teks aman> " + str(x)
    return "<teks> " + bentuk(x)


def panggil(key, secret, parameter):
    qs = urllib.parse.urlencode({**parameter, "timestamp": int(time.time() * 1000), "recvWindow": 10000})
    sign = hmac.new(secret.encode(), qs.encode(), hashlib.sha256).hexdigest()
    req = urllib.request.Request(
        f"https://api.indodax.com/api/v2/fiat/orders?{qs}",
        headers={"User-Agent": UA, "Accept": "application/json", "X-APIKEY": key, "Sign": sign},
    )
    try:
        with urllib.request.urlopen(req, timeout=20) as r:
            return r.status, r.read().decode(errors="replace")
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode(errors="replace")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--jenis", type=int, choices=(0, 1), default=0, help="0 = deposit, 1 = withdraw")
    ap.add_argument("--hari", type=int, default=29, help="maks 30")
    a = ap.parse_args()
    muat_env()
    key, secret = os.environ.get("INDODAX_API_KEY", ""), os.environ.get("INDODAX_API_SECRET", "")
    if not key or not secret or key.startswith("isi_"):
        sys.exit("Isi INDODAX_API_KEY dan INDODAX_API_SECRET di file .env dulu.")
    akhir = int(time.time() * 1000)
    awal = akhir - min(a.hari, 30) * 24 * 3600 * 1000
    varian = [
        ("camelCase", {"transactionType": a.jenis, "beginTime": awal, "endTime": akhir}),
        ("snake_case", {"transaction_type": a.jenis, "begin_time": awal, "end_time": akhir}),
    ]
    for nama, parameter in varian:
        kode, isi = panggil(key, secret, parameter)
        print(f"[{nama}] HTTP {kode}")
        if kode != 400:
            break
        print("  (ditolak, mencoba varian berikutnya)")
    try:
        print(json.dumps(topeng(json.loads(isi)), indent=2, ensure_ascii=False))
    except Exception:
        print("Bukan JSON. Awal isi:", bentuk(isi[:200]))
    print("\nPeriksa dulu: tidak ada nama/nomor rekening yang tampak, baru tempel ke chat.")


if __name__ == "__main__":
    main()
