#!/data/data/com.termux/files/usr/bin/bash
# T9.5: uji endpoint exchange luar dari jaringan Anda (Termux atau Linux). Hanya membaca.
# Mencetak kode HTTP, ukuran respons (byte), dan waktu tiap exchange, lalu kurs USDTIDR Binance.
# Jalankan dengan Private DNS aktif (dns.google atau one.one.one.one) bila DNS Anda memblokir.
for u in \
  "Binance|https://api.binance.com/api/v3/ticker/price" \
  "Bybit|https://api.bybit.com/v5/market/tickers?category=spot" \
  "OKX|https://www.okx.com/api/v5/market/tickers?instType=SPOT"; do
  n="${u%%|*}"; url="${u#*|}"
  printf '%-8s ' "$n"
  curl -s -m 20 -o /tmp/g_$n.json -w 'HTTP %{http_code}  %{size_download} byte  %{time_total} dtk\n' "$url" || echo "GAGAL (curl keluar $?)"
done
echo "--- kurs USDTIDR Binance (dari daftar yang sama):"
grep -o '{"symbol":"USDTIDR","price":"[^"]*"}' /tmp/g_Binance.json || echo "tidak ditemukan"
echo "--- gzip (ukuran bila dikompresi):"
for n in Binance Bybit OKX; do printf '%-8s %s byte\n' "$n" "$(gzip -c /tmp/g_$n.json 2>/dev/null | wc -c)"; done
