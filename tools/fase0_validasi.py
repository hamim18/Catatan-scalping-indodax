#!/usr/bin/env python3
"""
Fase 0 - Validasi data Indodax (hanya BACA, tidak membuat order).

Mengambil trade lewat GET /api/v2/myTrades, menyimpan respons mentah, lalu
mencetak ringkasan per order (modal efektif untuk beli, IDR diterima untuk jual)
supaya bisa dibandingkan dengan struk Indodax.

Hanya pustaka bawaan Python 3.8+ (tanpa pip install).

Contoh:
  python tools/fase0_validasi.py icntidr --hari 3
  python tools/fase0_validasi.py --diagnosa        (cek koneksi saja, tanpa key)
"""
import argparse, hashlib, hmac, json, os, sys, time
import urllib.error, urllib.parse, urllib.request
from decimal import Decimal
from pathlib import Path

BASE = "https://api.indodax.com"
UA = ("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) "
      "Chrome/124.0 Safari/537.36")
ROOT = Path(__file__).resolve().parent.parent


def muat_env():
    f = ROOT / ".env"
    if f.exists():
        for baris in f.read_text(encoding="utf-8").splitlines():
            baris = baris.strip()
            if baris and not baris.startswith("#") and "=" in baris:
                k, v = baris.split("=", 1)
                os.environ.setdefault(k.strip(), v.strip().strip('"').strip("'"))


def http_get(url, headers):
    """Kembalikan (kode, header_penting, isi_teks)."""
    req = urllib.request.Request(url, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=20) as r:
            return r.status, dict(r.headers), r.read().decode(errors="replace")
    except urllib.error.HTTPError as e:
        return e.code, dict(e.headers), e.read().decode(errors="replace")
    except Exception as e:  # DNS, timeout, SSL, dll.
        return 0, {}, f"GAGAL SAMBUNG: {e}"


def panggil(path, params, key, secret, algo, base):
    params = dict(params)
    params["timestamp"] = int(time.time() * 1000)
    params["recvWindow"] = 10000
    qs = urllib.parse.urlencode(params)
    digest = hashlib.sha256 if algo == "sha256" else hashlib.sha512
    sign = hmac.new(secret.encode(), qs.encode(), digest).hexdigest()
    kode, hdr, isi = http_get(
        f"{base}{path}?{qs}",
        {"User-Agent": UA, "Accept": "application/json", "X-APIKEY": key, "Sign": sign},
    )
    try:
        return kode, hdr, json.loads(isi)
    except Exception:
        return kode, hdr, {"bukan_json": True, "isi_awal": isi[:400]}


def diagnosa(base):
    print(f"== Diagnosa koneksi ke {base} (tanpa key) ==")
    for url in (f"{base}/", "https://indodax.com/api/ticker/btcidr"):
        kode, hdr, isi = http_get(url, {"User-Agent": UA, "Accept": "application/json"})
        print(f"\nGET {url}\n  HTTP {kode}")
        for h in ("Server", "CF-RAY", "Content-Type"):
            for k, v in hdr.items():
                if k.lower() == h.lower():
                    print(f"  {k}: {v}")
        print("  isi awal:", isi[:200].replace("\n", " "))


def ringkas(data):
    """Ringkasan per order dari field myTrades resmi."""
    per = {}
    for t in data:
        per.setdefault(t["orderId"], []).append(t)
    print("\n== Ringkasan per order ==")
    for oid, ts in sorted(per.items(), key=lambda x: min(t["time"] for t in x[1])):
        qty = sum(Decimal(t["qty"]) for t in ts)
        nilai = sum(Decimal(t["quoteQty"]) for t in ts)
        fee = sum(Decimal(t["commission"]) for t in ts)
        aset = {t["commissionAsset"] for t in ts}
        beli = ts[0]["isBuyer"]
        waktu = time.strftime("%d-%m %H:%M:%S", time.localtime(min(t["time"] for t in ts) / 1000))
        print(f"\n{oid}  [{'BELI' if beli else 'JUAL'}]  {waktu}  ({len(ts)} pengisian)")
        print(f"  koin (qty)         : {qty}")
        print(f"  nilai (quoteQty)   : {nilai}")
        print(f"  fee (commission)   : {fee}  aset: {', '.join(aset)}")
        if beli:
            keluar = nilai + fee
            print(f"  IDR keluar = nilai + fee : {keluar}   (bandingkan dgn 'jumlah order' di struk)")
            print(f"  modal efektif = keluar / koin : {keluar / qty:.2f}")
        else:
            print(f"  IDR diterima = nilai - fee : {nilai - fee}   (bandingkan dgn 'total diterima' di struk)")


def main():
    ap = argparse.ArgumentParser(description="Fase 0: validasi data Indodax (baca saja)")
    ap.add_argument("symbol", nargs="?", help="contoh: icntidr")
    ap.add_argument("--hari", type=int, default=0, help="N hari ke belakang (maks 7)")
    ap.add_argument("--limit", type=int, default=100)
    ap.add_argument("--base", default=BASE)
    ap.add_argument("--diagnosa", action="store_true", help="tes koneksi tanpa key")
    a = ap.parse_args()

    if a.diagnosa:
        return diagnosa(a.base)
    if not a.symbol:
        sys.exit("Isi simbol, contoh: python tools/fase0_validasi.py icntidr --hari 3")

    muat_env()
    key, secret = os.environ.get("INDODAX_API_KEY", ""), os.environ.get("INDODAX_API_SECRET", "")
    if not key or not secret or key.startswith("isi_"):
        sys.exit("Isi INDODAX_API_KEY dan INDODAX_API_SECRET di file .env dulu.")

    # Dokumen resmi terbaru: TAPI v2 memakai HMAC-SHA256.
    kode, hdr, res = panggil("/api/v2/account", {}, key, secret, "sha256", a.base)
    print(f"[cek akun] HTTP {kode}")
    if kode != 200:
        print(json.dumps(res, ensure_ascii=False, indent=2))
        for k, v in hdr.items():
            if k.lower() in ("server", "cf-ray", "content-type"):
                print(f"  header {k}: {v}")
        sys.exit("\nGagal. Jalankan: python tools/fase0_validasi.py --diagnosa  lalu kirim semua cetakan ke Claude.")
    print("Tersambung.\n")

    params = {"symbol": a.symbol.lower(), "limit": a.limit}
    if a.hari:
        now = int(time.time() * 1000)
        params["startTime"], params["endTime"] = now - min(a.hari, 7) * 86400000, now
    kode, hdr, res = panggil("/api/v2/myTrades", params, key, secret, "sha256", a.base)
    print(f"[myTrades] HTTP {kode}")
    teks = json.dumps(res, indent=2, ensure_ascii=False)
    out = ROOT / "data" / f"raw_{a.symbol}_{time.strftime('%Y%m%d_%H%M%S')}.json"
    out.write_text(teks, encoding="utf-8")
    print(f"Respons mentah disimpan: {out}")
    if kode == 200 and isinstance(res, dict) and res.get("data"):
        ringkas(res["data"])
    else:
        print(teks)


if __name__ == "__main__":
    main()

