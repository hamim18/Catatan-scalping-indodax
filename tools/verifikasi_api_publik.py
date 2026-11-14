#!/usr/bin/env python3
"""
T9.0: verifikasi API publik Indodax untuk fitur Screener (Fase 9).

HANYA MEMBACA, tanpa API key, tanpa tanda tangan, tanpa library tambahan.
Jalankan dari folder utama proyek (catatan-scalping):

    python tools/verifikasi_api_publik.py
    python tools/verifikasi_api_publik.py --durasi 120 --interval 5
    python tools/verifikasi_api_publik.py --tanpa-polling      # lebih cepat

Yang diperiksa (semuanya diukur, tidak ditebak):
  A. Bentuk /api/summaries (kunci, isi satu pair, server_time).
  B. /api/trades/<pair>: jumlah transaksi yang dikembalikan, urutan,
     rentang waktu yang terjangkau (apakah sampai 10 menit untuk koin ramai).
  C. /api/depth/<pair>: bentuk data, jumlah level, kedalaman dalam 0,5%.
  D. Polling /api/summaries: seberapa sering last, vol_idr, dan server_time
     berubah; apakah vol_idr bergulir (bisa turun) atau hanya naik; dan
     perbandingan kenaikan vol_idr dengan nilai transaksi baru dari /trades.

Hasil lengkap disimpan di data/verif_publik_<waktu>.json (tidak di-commit).
Kirim bagian "RINGKASAN" di akhir layar (angka pasar publik, aman dibagikan).
"""
import argparse
import json
import os
import time
import urllib.request
from datetime import datetime

BASE = "https://indodax.com"
UA = "Mozilla/5.0 (verifikasi T9.0)"
JEDA_REQUEST = 0.4          # jeda antar request agar aman dari batas laju
MIN_AKTIF_IDR = 10_000_000  # pair dianggap aktif bila vol_idr >= ini


# ---------- util ----------

def ambil(path, timeout=10):
    """Return (data, detik, ukuran_byte, galat). Tidak pernah melempar error."""
    t0 = time.time()
    try:
        req = urllib.request.Request(BASE + path, headers={"User-Agent": UA})
        with urllib.request.urlopen(req, timeout=timeout) as r:
            raw = r.read()
        return json.loads(raw.decode()), time.time() - t0, len(raw), None
    except Exception as e:  # noqa: BLE001 (alat uji, tampilkan apa adanya)
        return None, time.time() - t0, 0, f"{type(e).__name__}: {e}"


def num(x):
    try:
        return float(x)
    except (TypeError, ValueError):
        return None


def judul(teks):
    print("\n" + "=" * 70)
    print(teks)
    print("=" * 70)


def potong(obj, maks=600):
    s = json.dumps(obj, ensure_ascii=False)
    return s if len(s) <= maks else s[:maks] + f"... (+{len(s) - maks} karakter)"


def path_pair(pair):
    return pair.replace("_", "")


# ---------- A. summaries ----------

def bagian_a(hasil):
    judul("A. /api/summaries")
    d, dt, ukuran, err = ambil("/api/summaries")
    if err:
        print("GAGAL:", err)
        hasil["A"] = {"galat": err}
        return None
    print(f"Respons {ukuran:,} byte dalam {dt:.2f} detik")
    print("Kunci tingkat atas:", list(d.keys()))
    tickers = d.get("tickers", {})
    idr = {p: t for p, t in tickers.items() if p.endswith("_idr")}
    print(f"Jumlah ticker: {len(tickers)}; pair IDR: {len(idr)}")
    if not idr:
        hasil["A"] = {"galat": "tidak ada pair IDR"}
        return None

    muat = {}
    for t in idr.values():
        for k in t.keys():
            muat[k] = muat.get(k, 0) + 1
    print("Kunci di tiap ticker IDR (jumlah pair yang memuatnya):")
    for k, n in sorted(muat.items()):
        print(f"   {k}: {n}")

    contoh_pair = max(idr, key=lambda p: num(idr[p].get("vol_idr")) or 0)
    print(f"\nContoh mentah ({contoh_pair}, vol_idr terbesar):")
    print(" ", potong(idr[contoh_pair], 800))

    st = {t.get("server_time") for t in idr.values()}
    print(f"\nserver_time berbeda-beda antar pair? {len(st)} nilai unik dari {len(idr)} pair")
    p24 = d.get("prices_24h", {})
    print(f"prices_24h: {len(p24)} entri; contoh kunci: {list(p24.keys())[:3]}")

    hasil["A"] = {
        "ukuran_byte": ukuran, "detik": round(dt, 3),
        "kunci_atas": list(d.keys()), "jumlah_ticker": len(tickers),
        "jumlah_idr": len(idr), "kunci_ticker": muat,
        "contoh_pair": contoh_pair, "contoh_mentah": idr[contoh_pair],
        "server_time_unik": len(st),
    }
    return idr


def pilih_contoh(idr, n):
    aktif = sorted(
        ((p, num(t.get("vol_idr")) or 0) for p, t in idr.items()),
        key=lambda x: x[1], reverse=True)
    aktif = [x for x in aktif if x[1] >= MIN_AKTIF_IDR] or [x for x in aktif if x[1] > 0]
    if not aktif:
        return []
    if len(aktif) <= n:
        return [p for p, _ in aktif]
    idx = sorted({round(i * (len(aktif) - 1) / (n - 1)) for i in range(n)})
    return [aktif[i][0] for i in idx]


# ---------- B. trades ----------

def analisis_trades(pair, idr, server_now):
    d, dt, ukuran, err = ambil(f"/api/trades/{path_pair(pair)}")
    r = {"pair": pair, "detik": round(dt, 3)}
    if err:
        r["galat"] = err
        return r, None
    if not isinstance(d, list):
        r["galat"] = f"bukan list: {potong(d, 200)}"
        return r, None
    r["jumlah"] = len(d)
    if not d:
        return r, d
    r["kunci_item"] = list(d[0].keys()) if isinstance(d[0], dict) else str(type(d[0]))
    r["contoh_mentah"] = d[:2]
    tgl = [int(num(t.get("date")) or 0) for t in d]
    r["urutan"] = "terbaru dulu" if tgl[0] >= tgl[-1] else "terlama dulu"
    baru, lama = max(tgl), min(tgl)
    r["rentang_menit"] = round((baru - lama) / 60, 2)
    now = server_now or time.time()
    r["usia_transaksi_terakhir_detik"] = round(now - baru)
    r["usia_transaksi_tertua_menit"] = round((now - lama) / 60, 2)
    r["dalam_10_menit"] = sum(1 for x in tgl if x >= now - 600)
    r["mencapai_10_menit"] = lama <= now - 600
    # harga transaksi terbaru vs last di summaries
    terbaru = max(d, key=lambda t: int(num(t.get("date")) or 0))
    r["harga_trade_terbaru"] = num(terbaru.get("price"))
    r["last_summaries"] = num(idr.get(pair, {}).get("last"))
    return r, d


def bagian_b(hasil, idr, contoh):
    judul("B. /api/trades/<pair>")
    server_now = None
    for p in contoh:
        st = num(idr[p].get("server_time"))
        if st:
            server_now = st
            break
    hasil["B"] = []
    trades_cache = {}
    for p in contoh:
        r, d = analisis_trades(p, idr, server_now)
        trades_cache[p] = d
        hasil["B"].append(r)
        print(f"\n{p}  (vol_idr {num(idr[p].get('vol_idr')) or 0:,.0f})")
        if "galat" in r:
            print("  GAGAL:", r["galat"])
        else:
            print(f"  jumlah transaksi dikembalikan : {r['jumlah']}")
            print(f"  kunci item                    : {r.get('kunci_item')}")
            print(f"  urutan                        : {r.get('urutan')}")
            print(f"  rentang waktu terjangkau      : {r.get('rentang_menit')} menit")
            print(f"  usia transaksi terakhir       : {r.get('usia_transaksi_terakhir_detik')} detik")
            print(f"  usia transaksi tertua         : {r.get('usia_transaksi_tertua_menit')} menit")
            print(f"  transaksi dalam 10 menit      : {r.get('dalam_10_menit')}")
            print(f"  jendela 10 menit terjangkau?  : {'YA' if r.get('mencapai_10_menit') else 'TIDAK'}")
            print(f"  harga trade terbaru vs last   : {r.get('harga_trade_terbaru')} vs {r.get('last_summaries')}")
            print(f"  contoh mentah                 : {potong(r.get('contoh_mentah'), 300)}")
        time.sleep(JEDA_REQUEST)
    return trades_cache


# ---------- C. depth ----------

def level(x):
    if isinstance(x, (list, tuple)) and len(x) >= 2:
        return num(x[0]), num(x[1])
    if isinstance(x, dict):
        p = num(x.get("price", x.get("harga")))
        a = num(x.get("amount", x.get("qty", x.get("volume"))))
        return p, a
    return None, None


def analisis_depth(pair, idr):
    d, dt, ukuran, err = ambil(f"/api/depth/{path_pair(pair)}")
    r = {"pair": pair, "detik": round(dt, 3), "ukuran_byte": ukuran}
    if err:
        r["galat"] = err
        return r
    if not isinstance(d, dict):
        r["galat"] = f"bukan objek: {potong(d, 200)}"
        return r
    r["kunci"] = list(d.keys())
    buy, sell = d.get("buy"), d.get("sell")
    if not isinstance(buy, list) or not isinstance(sell, list):
        r["galat"] = "kunci 'buy'/'sell' tidak ada atau bukan list"
        r["mentah"] = potong(d, 500)
        return r
    r["jumlah_level_buy"], r["jumlah_level_sell"] = len(buy), len(sell)
    r["contoh_buy"], r["contoh_sell"] = buy[:3], sell[:3]
    bids = [lv for lv in map(level, buy) if None not in lv]
    asks = [lv for lv in map(level, sell) if None not in lv]
    if not bids or not asks:
        r["galat"] = "level tidak bisa dibaca sebagai (harga, jumlah)"
        return r
    bb = max(p for p, _ in bids)
    ba = min(p for p, _ in asks)
    r["best_bid"], r["best_ask"] = bb, ba
    r["bid_summaries"] = num(idr.get(pair, {}).get("buy"))
    r["ask_summaries"] = num(idr.get(pair, {}).get("sell"))
    r["urutan_buy"] = "menurun" if buy and level(buy[0])[0] >= level(buy[-1])[0] else "menaik"
    r["jangkauan_bid_persen"] = round((bb - min(p for p, _ in bids)) / bb * 100, 2)
    r["jangkauan_ask_persen"] = round((max(p for p, _ in asks) - ba) / ba * 100, 2)
    for persen in (0.5, 1.0):
        f = persen / 100
        r[f"nilai_bid_{persen}%"] = round(sum(p * a for p, a in bids if p >= bb * (1 - f)))
        r[f"nilai_ask_{persen}%"] = round(sum(p * a for p, a in asks if p <= ba * (1 + f)))
    return r


def bagian_c(hasil, idr, contoh):
    judul("C. /api/depth/<pair>  (nilai = harga x jumlah, ASUMSI jumlah dalam koin)")
    hasil["C"] = []
    for p in contoh[:3]:
        r = analisis_depth(p, idr)
        hasil["C"].append(r)
        print(f"\n{p}")
        for k, v in r.items():
            if k in ("pair",):
                continue
            print(f"  {k}: {potong(v, 300)}")
        time.sleep(JEDA_REQUEST)
    print("\nCatatan: bandingkan nilai di atas dengan tampilan order book di aplikasi"
          " Indodax untuk memastikan arti 'jumlah' (koin atau IDR).")


# ---------- D. polling summaries ----------

def bagian_d(hasil, idr, contoh, trades_awal, durasi, interval):
    judul(f"D. Polling /api/summaries ({durasi} detik, tiap {interval} detik)")
    seri = {p: [] for p in contoh}
    server_times = []
    latensi = []
    n = max(2, int(durasi / interval))
    for i in range(n):
        t0 = time.time()
        d, dt, _, err = ambil("/api/summaries")
        latensi.append(dt)
        if err:
            print(f"  poll {i + 1}/{n}: GAGAL {err}")
        else:
            tk = d.get("tickers", {})
            for p in contoh:
                t = tk.get(p)
                if t:
                    seri[p].append((time.time(), num(t.get("last")),
                                    num(t.get("vol_idr")), t.get("server_time")))
            st = [num(t.get("server_time")) for t in tk.values() if num(t.get("server_time"))]
            if st:
                server_times.append(max(st))
            print(f"  poll {i + 1}/{n}: {dt:.2f} dtk", end="\r")
        sisa = interval - (time.time() - t0)
        if sisa > 0 and i < n - 1:
            time.sleep(sisa)
    print(" " * 40)

    hasil["D"] = {"latensi_rata2": round(sum(latensi) / len(latensi), 3), "pair": {}}
    print(f"Latensi rata-rata /summaries: {sum(latensi) / len(latensi):.2f} detik")
    if server_times:
        beda = sum(1 for a, b in zip(server_times, server_times[1:]) if b != a)
        print(f"server_time global berubah di {beda} dari {len(server_times) - 1} selisih poll")

    print(f"\n{'PAIR':<12}{'poll':>5}{'last berubah':>14}{'vol berubah':>13}"
          f"{'vol naik':>10}{'vol turun':>11}{'st berubah':>12}")
    for p in contoh:
        s = seri[p]
        if len(s) < 2:
            continue
        pasang = list(zip(s, s[1:]))
        last_b = sum(1 for a, b in pasang if a[1] != b[1])
        vol_b = sum(1 for a, b in pasang if a[2] != b[2])
        vol_n = sum(1 for a, b in pasang if (b[2] or 0) > (a[2] or 0))
        vol_t = sum(1 for a, b in pasang if (b[2] or 0) < (a[2] or 0))
        st_b = sum(1 for a, b in pasang if a[3] != b[3])
        print(f"{p:<12}{len(s):>5}{last_b:>11}/{len(pasang):<2}{vol_b:>10}/{len(pasang):<2}"
              f"{vol_n:>10}{vol_t:>11}{st_b:>10}/{len(pasang):<2}")
        hasil["D"]["pair"][p] = {
            "poll": len(s), "selisih": len(pasang), "last_berubah": last_b,
            "vol_berubah": vol_b, "vol_naik": vol_n, "vol_turun": vol_t,
            "server_time_berubah": st_b,
        }

    # perbandingan kenaikan vol_idr dengan nilai transaksi baru dari /trades
    print("\nPerbandingan kenaikan vol_idr dengan nilai transaksi baru dari /trades:")
    hasil["D"]["bandingkan"] = []
    for p in contoh[:3]:
        awal = trades_awal.get(p)
        s = seri[p]
        if not awal or len(s) < 2:
            continue
        akhir, _, _, err = ambil(f"/api/trades/{path_pair(p)}")
        time.sleep(JEDA_REQUEST)
        if err or not isinstance(akhir, list):
            print(f"  {p}: gagal ambil trades akhir ({err})")
            continue
        tid_awal = {str(t.get("tid")) for t in awal}
        baru = [t for t in akhir if str(t.get("tid")) not in tid_awal]
        nilai = sum((num(t.get("price")) or 0) * (num(t.get("amount")) or 0) for t in baru)
        delta = (s[-1][2] or 0) - (s[0][2] or 0)
        selang = s[-1][0] - s[0][0]
        baris = {"pair": p, "selang_detik": round(selang), "transaksi_baru": len(baru),
                 "nilai_transaksi_baru_idr": round(nilai), "delta_vol_idr": round(delta)}
        hasil["D"]["bandingkan"].append(baris)
        print(f"  {p}: selang {selang:.0f} dtk | transaksi baru {len(baru)} bernilai "
              f"Rp {nilai:,.0f} | delta vol_idr Rp {delta:,.0f}")
    print("\nCara membaca: bila delta vol_idr ~ nilai transaksi baru, vol_idr bertambah"
          " tiap transaksi. Bila sering turun atau jauh lebih kecil, vol_idr bergulir"
          " (volume 24 jam yang transaksi lamanya ikut keluar). Jangan menyimpulkan dari"
          " satu kali jalan; ulangi di jam ramai dan sepi.")


# ---------- ringkasan ----------

def ringkasan(hasil):
    judul("RINGKASAN (kirim bagian ini)")
    a = hasil.get("A", {})
    print(f"summaries: {a.get('jumlah_idr')} pair IDR, {a.get('ukuran_byte')} byte, "
          f"{a.get('detik')} dtk, server_time unik={a.get('server_time_unik')}")
    b = [r for r in hasil.get("B", []) if "galat" not in r]
    if b:
        print("trades jumlah per pair :", {r["pair"]: r["jumlah"] for r in b})
        print("trades rentang (menit) :", {r["pair"]: r["rentang_menit"] for r in b})
        print("trades capai 10 menit  :", {r["pair"]: r["mencapai_10_menit"] for r in b})
        print("trades urutan          :", sorted({r["urutan"] for r in b}))
    c = [r for r in hasil.get("C", []) if "galat" not in r]
    if c:
        r = c[0]
        print(f"depth kunci={r['kunci']} level buy/sell={r['jumlah_level_buy']}/"
              f"{r['jumlah_level_sell']} jangkauan bid/ask %="
              f"{r['jangkauan_bid_persen']}/{r['jangkauan_ask_persen']} "
              f"urutan_buy={r['urutan_buy']}")
    elif hasil.get("C"):
        print("depth: gagal ->", [r.get("galat") for r in hasil["C"]])
    d = hasil.get("D")
    if d:
        print("polling latensi rata2 :", d["latensi_rata2"], "dtk")
        for p, v in d["pair"].items():
            print(f"  {p}: last berubah {v['last_berubah']}/{v['selisih']}, vol naik/turun "
                  f"{v['vol_naik']}/{v['vol_turun']}, server_time berubah "
                  f"{v['server_time_berubah']}/{v['selisih']}")
        for x in d.get("bandingkan", []):
            print(f"  {x['pair']}: transaksi baru Rp {x['nilai_transaksi_baru_idr']:,} "
                  f"vs delta vol_idr Rp {x['delta_vol_idr']:,} ({x['selang_detik']} dtk)")


def main():
    ap = argparse.ArgumentParser(description="T9.0 verifikasi API publik Indodax (baca saja)")
    ap.add_argument("--durasi", type=int, default=90, help="lama polling summaries (detik)")
    ap.add_argument("--interval", type=int, default=5, help="jeda antar poll (detik)")
    ap.add_argument("--contoh", type=int, default=5, help="jumlah pair contoh (2-8)")
    ap.add_argument("--tanpa-polling", action="store_true", help="lewati bagian D")
    args = ap.parse_args()
    args.contoh = max(2, min(8, args.contoh))
    args.interval = max(3, args.interval)

    hasil = {"versi": "T9.0", "waktu": datetime.now().isoformat(timespec="seconds"),
             "durasi": args.durasi, "interval": args.interval}
    print("T9.0: verifikasi API publik Indodax (hanya baca, tanpa key)")
    print("Waktu mulai:", hasil["waktu"])

    idr = bagian_a(hasil)
    if not idr:
        print("\nBagian A gagal; tidak bisa lanjut. Periksa koneksi internet.")
        return
    contoh = pilih_contoh(idr, args.contoh)
    hasil["contoh_pair"] = contoh
    print("\nPair contoh (dari aktif ke sepi):", contoh)

    trades_cache = bagian_b(hasil, idr, contoh)
    bagian_c(hasil, idr, contoh)
    if not args.tanpa_polling:
        bagian_d(hasil, idr, contoh, trades_cache, args.durasi, args.interval)
    ringkasan(hasil)

    os.makedirs("data", exist_ok=True)
    nama = f"data/verif_publik_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
    with open(nama, "w", encoding="utf-8") as fh:
        json.dump(hasil, fh, ensure_ascii=False, indent=2, default=str)
    print(f"\nHasil lengkap tersimpan: {nama}")


if __name__ == "__main__":
    main()
