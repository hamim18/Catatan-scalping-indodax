#!/usr/bin/env python3
"""
T0.6 - Tes token WebSocket privat Indodax dengan key TAPI v2 (HANYA BACA).

Menjawab satu pertanyaan terakhir di PRD: apakah key v2 (izin baca saja) bisa
dipakai untuk membuat token WebSocket privat dan menerima event order_update?

Langkah:
  1. POST /api/private_ws/v1/generate_token  (body: client=tapi&tapi_key=KEY,
     header Sign = HMAC-SHA512 dari body memakai secret).
  2. Sambung ke wss://pws.indodax.com, kirim {"connect":{"token":...}}.
  3. Subscribe ke channel pribadi.
  4. Tunggu event (--tunggu detik). Saat menunggu, lakukan satu pembelian kecil
     atau pasang/batalkan satu limit order di Indodax agar event muncul.

Tidak ada order yang dibuat skrip ini. Key/secret/token tidak pernah dicetak
utuh. Event mentah disimpan di data/ws_events_<waktu>.json (tidak di-commit).
Hanya pustaka bawaan Python 3.8+.

Contoh:
  python tools/tes_websocket_token.py --hanya-token
  python tools/tes_websocket_token.py --tunggu 180
"""
import argparse, base64, hashlib, hmac, json, os, socket, ssl, struct, sys, time
import urllib.error, urllib.parse, urllib.request
from pathlib import Path

BASE = "https://indodax.com"
WS_URL = "wss://pws.indodax.com/ws/?cf_ws_frame_ping_pong=true"
UA = ("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) "
      "Chrome/124.0 Safari/537.36")
ROOT = Path(__file__).resolve().parent.parent
GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"


def muat_env():
    f = ROOT / ".env"
    if f.exists():
        for baris in f.read_text(encoding="utf-8").splitlines():
            baris = baris.strip()
            if baris and not baris.startswith("#") and "=" in baris:
                k, v = baris.split("=", 1)
                os.environ.setdefault(k.strip(), v.strip().strip('"').strip("'"))


def samarkan(s, awal=6, akhir=4):
    s = str(s)
    return s if len(s) <= awal + akhir + 3 else f"{s[:awal]}...{s[-akhir:]} (panjang {len(s)})"


# ---------- Langkah 1: token ----------
def tanda_tangan_sha512(secret, body):
    return hmac.new(secret.encode(), body.encode(), hashlib.sha512).hexdigest()


def minta_token(key, secret, base):
    body = f"client=tapi&tapi_key={key}"
    req = urllib.request.Request(
        f"{base}/api/private_ws/v1/generate_token",
        data=body.encode(),
        method="POST",
        headers={
            "User-Agent": UA,
            "Accept": "application/json",
            "Content-Type": "application/x-www-form-urlencoded",
            "Sign": tanda_tangan_sha512(secret, body),
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=20) as r:
            kode, isi = r.status, r.read().decode(errors="replace")
    except urllib.error.HTTPError as e:
        kode, isi = e.code, e.read().decode(errors="replace")
    except Exception as e:
        return 0, {"gagal_sambung": str(e)}
    try:
        return kode, json.loads(isi)
    except Exception:
        return kode, {"bukan_json": True, "isi_awal": isi[:300]}


# ---------- Langkah 2-4: WebSocket minimal (stdlib) ----------
class MiniWS:
    def __init__(self, url):
        u = urllib.parse.urlparse(url)
        self.tls = u.scheme == "wss"
        self.host = u.hostname
        self.port = u.port or (443 if self.tls else 80)
        self.path = (u.path or "/") + (f"?{u.query}" if u.query else "")
        self.sock = None
        self.buf = b""

    def sambung(self, timeout=15):
        raw = socket.create_connection((self.host, self.port), timeout=timeout)
        self.sock = ssl.create_default_context().wrap_socket(raw, server_hostname=self.host) if self.tls else raw
        kunci = base64.b64encode(os.urandom(16)).decode()
        # Sengaja TANPA header Origin (saran dokumen resmi agar tidak 403).
        req = (f"GET {self.path} HTTP/1.1\r\nHost: {self.host}\r\nUpgrade: websocket\r\n"
               f"Connection: Upgrade\r\nSec-WebSocket-Key: {kunci}\r\n"
               f"Sec-WebSocket-Version: 13\r\nUser-Agent: {UA}\r\n\r\n")
        self.sock.sendall(req.encode())
        while b"\r\n\r\n" not in self.buf:
            d = self.sock.recv(4096)
            if not d:
                raise ConnectionError("server menutup koneksi saat handshake")
            self.buf += d
        kepala, self.buf = self.buf.split(b"\r\n\r\n", 1)
        baris = kepala.decode(errors="replace").split("\r\n")
        if " 101 " not in baris[0]:
            raise ConnectionError(f"handshake ditolak: {baris[0]}")
        akan = base64.b64encode(hashlib.sha1((kunci + GUID).encode()).digest()).decode()
        for b in baris[1:]:
            if b.lower().startswith("sec-websocket-accept:") and b.split(":", 1)[1].strip() != akan:
                raise ConnectionError("Sec-WebSocket-Accept tidak cocok")

    def _baca(self, n):
        while len(self.buf) < n:
            d = self.sock.recv(65536)
            if not d:
                raise ConnectionError("koneksi ditutup server")
            self.buf += d
        out, self.buf = self.buf[:n], self.buf[n:]
        return out

    def kirim(self, opcode, data=b""):
        kepala = bytes([0x80 | opcode])
        n = len(data)
        if n < 126:
            kepala += bytes([0x80 | n])
        elif n < 65536:
            kepala += bytes([0x80 | 126]) + struct.pack(">H", n)
        else:
            kepala += bytes([0x80 | 127]) + struct.pack(">Q", n)
        mask = os.urandom(4)
        isi = bytes(b ^ mask[i % 4] for i, b in enumerate(data))
        self.sock.sendall(kepala + mask + isi)

    def kirim_teks(self, teks):
        self.kirim(0x1, teks.encode())

    def terima(self, timeout):
        """Kembalikan teks pesan berikutnya, atau None bila timeout.
        Ping frame dibalas pong otomatis."""
        self.sock.settimeout(timeout)
        potongan, op_awal = b"", None
        while True:
            try:
                b1, b2 = self._baca(2)
                n = b2 & 0x7F
                if n == 126:
                    n = struct.unpack(">H", self._baca(2))[0]
                elif n == 127:
                    n = struct.unpack(">Q", self._baca(8))[0]
                if b2 & 0x80:
                    mask = self._baca(4)
                    data = bytes(b ^ mask[i % 4] for i, b in enumerate(self._baca(n)))
                else:
                    data = self._baca(n)
            except (socket.timeout, ssl.SSLError) as e:
                if isinstance(e, ssl.SSLError) and "timed out" not in str(e):
                    raise
                return None
            op, fin = b1 & 0x0F, bool(b1 & 0x80)
            if op == 0x9:
                self.kirim(0xA, data)
                continue
            if op == 0xA:
                continue
            if op == 0x8:
                kode = struct.unpack(">H", data[:2])[0] if len(data) >= 2 else None
                raise ConnectionError(f"server menutup koneksi (kode {kode}) {data[2:].decode(errors='replace')}")
            if op in (0x1, 0x2):
                op_awal, potongan = op, data
            else:
                potongan += data
            if fin:
                return potongan.decode(errors="replace")

    def tutup(self):
        try:
            self.kirim(0x8, struct.pack(">H", 1000))
            self.sock.close()
        except Exception:
            pass


def ringkas_event(e):
    """Satu baris ringkas dari satu event order_update."""
    o = e.get("order", {})
    baris = (f"{o.get('status','?'):9} {o.get('side','?'):4} {o.get('symbol','?')} "
             f"order={o.get('orderId','?')} harga={o.get('price','?')} "
             f"terisi={o.get('executedQty','?')}/{o.get('origQty','?')}")
    fi = o.get("fillInformation")
    if fi:
        baris += (f" | trade={o.get('tradeId')} qty={fi.get('qty')} {fi.get('participant')}"
                  f" fee={fi.get('fee')} pajak={fi.get('tax')} clearing={fi.get('clearing')}")
    return baris


def jalankan_ws(token, channel, tunggu, simpan):
    ws = MiniWS(WS_URL)
    print(f"\n[ws] menyambung ke {WS_URL}")
    ws.sambung()
    print("[ws] handshake berhasil (101)")

    def kirim_dan_tunggu(pesan, id_, label):
        ws.kirim_teks(json.dumps(pesan))
        batas = time.time() + 15
        while time.time() < batas:
            m = ws.terima(max(0.5, batas - time.time()))
            if m is None:
                break
            for baris in m.split("\n"):
                baris = baris.strip()
                if not baris:
                    continue
                if baris == "{}":
                    ws.kirim_teks("{}")  # balas ping aplikasi (protokol Centrifugo)
                    continue
                try:
                    j = json.loads(baris)
                except Exception:
                    print(f"[ws] pesan tak dikenal: {baris[:200]}")
                    continue
                if j.get("id") == id_:
                    return j
        print(f"[ws] tidak ada balasan untuk {label} dalam 15 detik")
        return None

    r1 = kirim_dan_tunggu({"connect": {"token": token}, "id": 1}, 1, "connect")
    if not r1 or "error" in r1:
        print(f"[ws] AUTENTIKASI GAGAL: {json.dumps(r1, ensure_ascii=False)[:300]}")
        return False, []
    c = r1.get("connect", {})
    print(f"[ws] autentikasi berhasil. ttl={c.get('ttl')} detik, versi server={c.get('version')}")

    r2 = kirim_dan_tunggu({"subscribe": {"channel": channel}, "id": 2}, 2, "subscribe")
    if not r2 or "error" in r2:
        print(f"[ws] SUBSCRIBE GAGAL: {json.dumps(r2, ensure_ascii=False)[:300]}")
        return False, []
    print("[ws] subscribe ke channel pribadi berhasil")

    if tunggu <= 0:
        ws.tutup()
        return True, []
    print(f"\n[ws] menunggu event {tunggu} detik. Sekarang lakukan satu transaksi kecil di Indodax")
    print("     (mis. beli kecil lalu jual, atau pasang limit order lalu batalkan). Ctrl+C untuk selesai.\n")
    events, akhir = [], time.time() + tunggu
    try:
        while time.time() < akhir:
            m = ws.terima(min(5, max(0.5, akhir - time.time())))
            if m is None:
                continue
            for baris in m.split("\n"):
                baris = baris.strip()
                if not baris:
                    continue
                if baris == "{}":
                    ws.kirim_teks("{}")
                    continue
                try:
                    j = json.loads(baris)
                except Exception:
                    continue
                data = j.get("push", {}).get("pub", {}).get("data", [])
                for e in data:
                    events.append(e)
                    print(f"  {time.strftime('%H:%M:%S')}  {ringkas_event(e)}")
    except KeyboardInterrupt:
        print("\n[ws] dihentikan pengguna")
    except ConnectionError as e:
        print(f"\n[ws] koneksi terputus: {e}")
    ws.tutup()
    return True, events


def main():
    ap = argparse.ArgumentParser(description="T0.6: tes token WebSocket privat Indodax (baca saja)")
    ap.add_argument("--base", default=BASE, help="host untuk generate_token (default indodax.com)")
    ap.add_argument("--hanya-token", action="store_true", help="hanya langkah 1 (token), tanpa WebSocket")
    ap.add_argument("--tunggu", type=int, default=120, help="detik menunggu event setelah subscribe (default 120)")
    a = ap.parse_args()

    muat_env()
    key, secret = os.environ.get("INDODAX_API_KEY", ""), os.environ.get("INDODAX_API_SECRET", "")
    if not key or not secret or key.startswith("isi_"):
        sys.exit("Isi INDODAX_API_KEY dan INDODAX_API_SECRET di file .env dulu.")

    print("== Langkah 1: minta token WebSocket privat ==")
    kode, res = minta_token(key, secret, a.base)
    print(f"HTTP {kode}")
    if kode != 200 or res.get("success") != 1:
        print("TOKEN DITOLAK atau gagal. Respons:")
        print(json.dumps(res, ensure_ascii=False, indent=2)[:600])
        print("\nHASIL T0.6: GAGAL di langkah token.")
        print("  invalid_credentials / invalid_tapi_key -> kemungkinan key v2 tidak dikenali endpoint ini")
        print("  bad_sign -> tanda tangan HMAC-SHA512 ditolak (secret salah atau key v2 butuh cara lain)")
        print("Kirim cetakan ini ke Claude (tanpa key/secret).")
        sys.exit(1)

    ret = res.get("return", {})
    token, channel = ret.get("connToken", ""), ret.get("channel", "")
    print(f"Token diterima : {samarkan(token)}")
    print(f"Channel pribadi: {samarkan(channel, 8, 4)}")
    if not token or not channel:
        sys.exit("Respons sukses tetapi connToken/channel kosong. Kirim ke Claude.")
    if a.hanya_token:
        print("\nHASIL T0.6 (sebagian): token BERHASIL dibuat dengan key v2. Jalankan lagi tanpa --hanya-token untuk tes WebSocket.")
        return

    try:
        ok, events = jalankan_ws(token, channel, a.tunggu, True)
    except Exception as e:
        print(f"\n[ws] GAGAL: {e}")
        print("HASIL T0.6: token berhasil, tetapi sambungan WebSocket gagal. Kirim cetakan ini ke Claude.")
        sys.exit(2)

    if events:
        out = ROOT / "data" / f"ws_events_{time.strftime('%Y%m%d_%H%M%S')}.json"
        out.parent.mkdir(exist_ok=True)
        out.write_text(json.dumps(events, indent=2, ensure_ascii=False), encoding="utf-8")
        print(f"\nEvent mentah disimpan: {out}  (kirim isinya ke Claude, tanpa menyertakan key)")
    if ok:
        print("\nHASIL T0.6: BERHASIL. Key v2 dapat membuat token, autentikasi, dan subscribe channel pribadi.")
        if not events:
            print("  Catatan: belum ada event order yang diterima. Ulangi dengan transaksi kecil agar format event terkonfirmasi.")


if __name__ == "__main__":
    main()

