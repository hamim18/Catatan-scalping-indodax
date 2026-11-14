package id.catatanscalping.app

import android.view.View
import android.widget.LinearLayout

/**
 * T10.1: Panduan dan FAQ (dibuka dari menu ☰). Teks statis; ketuk pertanyaan untuk membuka jawabannya.
 * Tidak ada angka atau perhitungan di sini, jadi tidak menyentuh Rust.
 */
class LayarPanduan(a: MainActivity) : Layar(a) {

    private val isiPanduan: List<Pair<String, List<Pair<String, String>>>> = listOf(
        "Dasar" to listOf(
            "Apa yang dilakukan aplikasi ini?" to
                "Mencatat trading di Indodax (pair IDR) otomatis lewat API baca saja. Aplikasi tidak pernah " +
                "membuat atau membatalkan order dan tidak memindahkan dana.",
            "Bagaimana trade tercatat?" to
                "Saat pantauan menyala, order yang terisi terdeteksi realtime lalu angka resminya diambil dari " +
                "REST Indodax. Saat aplikasi dibuka atau tombol Sinkron sekarang ditekan, trade yang terlewat " +
                "dilengkapi. Hanya trade sejak aplikasi pertama kali tersinkron yang dihitung.",
            "Mengapa posisi atau profit belum berubah setelah saya jual?" to
                "Bila pantauan mati atau terputus, penjualan baru tercatat setelah sinkron. Buka Setelan lalu " +
                "tekan Sinkron sekarang, dan biarkan pantauan menyala saat trading.",
        ),
        "Angka di layar" to listOf(
            "Apa itu modal efektif, TP, dan SL?" to
                "Modal efektif = IDR yang keluar untuk beli (termasuk fee) dibagi koin yang diterima. TP dan SL " +
                "adalah modal efektif ditambah atau dikurangi persen dari Setelan. Hanya acuan: aplikasi tidak " +
                "memasang order.",
            "Mengapa hasil bersih di TP lebih kecil dari persen TP?" to
                "Fee jual belum termasuk di harga TP. Estimasi bersih memakai fee jual limit sekitar 0,31%. " +
                "Jual market bisa sekitar 0,61%, jadi hasil nyata bisa lebih kecil.",
            "Apa bedanya profit trading dan floating?" to
                "Profit trading hanya dari trade yang sudah ditutup (setelah fee). Floating adalah naik-turun " +
                "nilai koin yang sedang dipegang, dihitung sebelum fee jual, dan tidak masuk target atau grafik.",
            "Bagaimana persen target dihitung?" to
                "Profit periode dibagi (total aset sekarang dikurangi profit periode). Hari mulai 00:00 WIB, " +
                "minggu mulai Senin 00:00 WIB. Deposit dan withdraw tidak memengaruhi target.",
            "Harga beli dan jual di Riwayat?" to
                "Harga per koin yang sudah termasuk fee (modal dan IDR diterima dibagi koin), jadi bisa sedikit " +
                "berbeda dari harga order di Indodax.",
        ),
        "Screener" to listOf(
            "Apa itu Screener?" to
                "Daftar pair IDR yang lolos kriteria buatan Anda sendiri. Hanya informasi, bukan saran beli atau " +
                "jual, dan tidak memengaruhi profit, target, atau statistik. Semua ambang bisa diubah di bagian " +
                "Ambang.",
            "Arti trx/mnt, spread, gerak, dan buy?" to
                "Trx/mnt: transaksi per menit. Spread: selisih jual terendah dan beli tertinggi, dalam persen. " +
                "Gerak: selisih tertinggi dan terendah harga transaksi dalam jendela waktu, dalam persen. " +
                "Buy: porsi transaksi bertipe beli.",
            "Apa itu Lonjakan?" to
                "Pair yang harganya sekarang menyimpang dari rata-rata harga beberapa menit terakhir sebesar " +
                "ambang atau lebih (▲ naik, ▼ turun) dan ramai transaksinya. Butuh masa pemanasan setelah layar " +
                "dibuka; data harga hanya disimpan di memori.",
            "Apa itu harga wajar dan selisih?" to
                "Harga wajar adalah median harga koin yang sama di Binance, Bybit, dan OKX dikonversi ke IDR. " +
                "Selisih positif berarti Indodax lebih mahal. Fee dan spread membuat selisih kecil itu normal. " +
                "Tanda ⚠ berarti selisih terlalu besar: bisa jadi koin lain bertiker sama.",
            "Mengapa baris Global kosong atau ada tanda ✗?" to
                "Exchange luar bisa tidak terjangkau dari jaringan Anda (sering terblokir di DNS). Coba atur " +
                "Private DNS HP ke dns.google. Exchange yang gagal dilewati dan Screener tetap jalan.",
        ),
        "Data dan pemeliharaan" to listOf(
            "Bagaimana mencadangkan data?" to
                "Setelan → Cadangan data → Ekspor data. Kunci API tidak ikut. Instal ulang atau hapus data " +
                "mengosongkan riwayat, jadi ekspor berkala.",
            "Mengapa pantauan perlu optimasi baterai dimatikan?" to
                "Android bisa mematikan layanan latar. Matikan optimasi baterai untuk aplikasi ini dan izinkan " +
                "notifikasi supaya notifikasi TP/SL tepat waktu.",
            "Berapa pemakaian data Screener?" to
                "Hanya berjalan saat layar Screener terbuka. Lonjakan memuat ringkasan pasar (±95 KB) tiap " +
                "interval, ditambah beberapa pair; Harga global menambah data dari exchange luar. Naikkan " +
                "interval di Ambang bila ingin lebih hemat.",
        ),
    )

    init {
        header("Panduan", kembali = true)
        for ((bagian, daftar) in isiPanduan) {
            isi.tambah(ui.judulBagian(bagian), atas = 14, bawah = 8)
            val kartu = ui.kartu()
            for ((i, qa) in daftar.withIndex()) {
                if (i > 0) kartu.tambah(View(a).apply { setBackgroundColor(t.line) }, tinggi = a.dp(1), atas = 10, bawah = 10)
                kartu.tambah(butir(qa.first, qa.second))
            }
            isi.tambah(kartu)
        }
    }

    private fun butir(tanya: String, jawab: String): LinearLayout {
        val judul = ui.teks("$tanya  ▸", 14f, tebal = true)
        val teks = ui.mute(jawab, 13f).apply { visibility = View.GONE }
        judul.setOnClickListener {
            val buka = teks.visibility != View.VISIBLE
            teks.visibility = if (buka) View.VISIBLE else View.GONE
            judul.text = if (buka) "$tanya  ▾" else "$tanya  ▸"
        }
        return LinearLayout(a).apply {
            orientation = LinearLayout.VERTICAL
            tambah(judul)
            tambah(teks, atas = 6)
        }
    }
}
