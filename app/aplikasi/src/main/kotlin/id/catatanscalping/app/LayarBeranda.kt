package id.catatanscalping.app

import android.os.Handler
import android.os.Looper
import android.view.Gravity
import android.view.View
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.LinearLayout
import android.widget.TextView
import uniffi.scalping_ffi.FloatingUi
import uniffi.scalping_ffi.HargaKoinUi
import uniffi.scalping_ffi.RingkasanProfitUi
import uniffi.scalping_ffi.TargetProfitUi
import uniffi.scalping_ffi.TitikProfitUi
import uniffi.scalping_ffi.TotalAsetUi
import uniffi.scalping_ffi.dataGrafikProfit
import uniffi.scalping_ffi.floatingPosisi
import uniffi.scalping_ffi.ringkasanProfit
import uniffi.scalping_ffi.totalAset

/**
 * Beranda (T6.2): satu kartu "Total aset" (nilai pasar; IDR berlabel "Buying power", koin per aset),
 * baris "Profit trading", grafik akumulasi profit trade tertutup, dan target harian/mingguan.
 * Hanya tampilan; semua angka dari Rust (total_aset, ringkasan_profit, data_grafik_profit).
 *
 * Total aset: jaringan, saat layar dibuka dan tiap 10 detik selama Beranda terlihat dan aplikasi
 * di depan. Profit, target, dan grafik: dari database lokal (trade tertutup), bebas dari harga;
 * total aset hanya dipakai sebagai pembagi persen target.
 *
 * T6.3: satu baris "Floating" (koin yang sedang dipegang, nilai pasar dikurangi modal, SEBELUM fee
 * jual) di bawah Profit trading. Harga diambil dari total aset yang sama (tanpa permintaan
 * jaringan tambahan). Floating TIDAK masuk target dan grafik.
 */
class LayarBeranda(a: MainActivity) : Layar(a) {
    private val chip = ui.Chip()
    private val grafik = GrafikView(a, t)
    private val infoGrafik = ui.mute("", 12f)
    private val kartuAset = ui.kartu()
    private val kartuTarget = ui.kartu()
    private val kartuPesan = ui.kartu()
    private val teksPesan = ui.mute("")

    // total aset (jaringan)
    private var asetTerakhir: TotalAsetUi? = null
    private var asetWaktuMs = 0L
    private var asetGalat: String? = null
    private var asetMemuat = false
    private var asetMulaiMs = 0L

    // floating posisi terbuka (database + harga dari total aset)
    private var floating: FloatingUi? = null

    // profit trade tertutup (database lokal)
    private var profit: RingkasanProfitUi? = null
    private var grafikProfit: List<TitikProfitUi> = emptyList()
    private var galatProfit: String? = null

    private val pengatur = Handler(Looper.getMainLooper())
    private var pernahTerpasang = false
    private val putaranAset = object : Runnable {
        override fun run() {
            if (root.isAttachedToWindow) {
                pernahTerpasang = true
            } else if (pernahTerpasang) {
                return // layar sudah dilepas: hentikan putaran
            }
            if (root.isShown && root.windowVisibility == View.VISIBLE) muatAset()
            pengatur.postDelayed(this, JEDA_PUTARAN_MS)
        }
    }

    init {
        header("Catatan trading", chip.root)

        kartuPesan.tambah(teksPesan)
        kartuPesan.visibility = View.GONE
        isi.tambah(kartuPesan, bawah = 12)

        kartuAset.visibility = View.GONE
        isi.tambah(kartuAset)

        isi.tambah(ui.judulBagian("Target"), atas = 22, bawah = 10)
        isi.tambah(kartuTarget)
        segarkanChip()
        pengatur.postDelayed(putaranAset, JEDA_PUTARAN_MS)
    }

    override fun pantauBerubah() = segarkanChip()

    /** Status koneksi di pojok kanan atas. */
    private fun segarkanChip() {
        if (a.kunci.ujungKey() == null) {
            chip.atur("Belum ada kunci", t.loss)
            return
        }
        if (!Sinkron.pantauAktif(a)) {
            chip.atur("Hanya baca · pantauan mati", t.mute)
            return
        }
        val s = PantauBersama.status
        when {
            s.startsWith("Tersambung") -> chip.atur("Terhubung · hanya baca", t.gain)
            s.startsWith("Terputus") || s.startsWith("Menghubungkan") -> chip.atur("Menyambung ulang", t.warn)
            s.startsWith("Gagal") || s.startsWith("Berhenti") -> chip.atur("Pantauan berhenti", t.loss)
            else -> chip.atur("Hanya baca · pantauan mati", t.mute)
        }
    }

    override fun segarkan() {
        segarkanChip()
        muatAset()
        muatProfit()
        muatFloating() // posisi bisa berubah (beli/jual); memakai harga terakhir yang sudah ada
    }

    /**
     * Floating dari posisi di database dan harga koin pada total aset terakhir. Tanpa jaringan.
     * Gagal (misal belum ada setelan atau snapshot) = baris Floating disembunyikan.
     */
    private fun muatFloating() {
        val d = asetTerakhir ?: return
        val harga = d.koin.map { HargaKoinUi(koin = it.koin, harga = it.hargaAngka) }
        val db = a.pathDb
        Latar.baca({ floatingPosisi(db, harga) }) { r ->
            r.onSuccess { floating = it }
            r.onFailure { floating = null }
            render()
        }
    }

    /**
     * Profit, target, dan grafik dari database. Persen target memakai total aset terakhir yang
     * sudah diterima (None bila belum ada: persen kosong, rupiah tetap tampil).
     */
    private fun muatProfit() {
        val db = a.pathDb
        val total = asetTerakhir?.totalAngka
        Latar.baca({
            val r = ringkasanProfit(db, total)
            val g = runCatching { dataGrafikProfit(db, 120u) }.getOrDefault(emptyList())
            Pair(r, g)
        }) { hasil ->
            hasil.onSuccess { (r, g) ->
                profit = r
                grafikProfit = g
                galatProfit = null
            }
            hasil.onFailure {
                profit = null
                grafikProfit = emptyList()
                galatProfit = Latar.pesan(it)
            }
            render()
        }
    }

    /**
     * Total aset: dua panggilan jaringan di Rust (/account dan ticker_all). Satu permintaan pada
     * satu waktu, jeda minimal 5 detik antar-permintaan (segarkan() bisa dipanggil sering).
     */
    private fun muatAset() {
        val kunci = a.kunci.ambil()
        if (kunci == null) {
            render()
            return
        }
        val sekarang = System.currentTimeMillis()
        if (asetMemuat || sekarang - asetMulaiMs < JEDA_MINIMAL_MS) return
        asetMemuat = true
        asetMulaiMs = sekarang
        Latar.jaringan.execute {
            val r = runCatching { totalAset(kunci.first, kunci.second) }
            Latar.ui {
                asetMemuat = false
                r.onSuccess {
                    asetTerakhir = it
                    asetWaktuMs = System.currentTimeMillis()
                    asetGalat = null
                }
                r.onFailure { asetGalat = Latar.pesan(it) }
                render()
                // total aset baru = pembagi persen target baru
                if (r.isSuccess) {
                    muatProfit()
                    muatFloating()
                }
            }
        }
    }

    private fun render() {
        val g = galatProfit
        if (g != null) {
            teksPesan.text = "Belum ada data profit untuk ditampilkan: $g\n\n" +
                "Buka Setelan, simpan kunci API, isi TP/SL, lalu tekan \"Sinkron sekarang\"."
            kartuPesan.visibility = View.VISIBLE
        } else {
            kartuPesan.visibility = View.GONE
        }
        tampilKartuAset()
        tampilTarget()
    }

    /** Satu kartu: total aset, buying power, koin, profit trading, grafik. */
    private fun tampilKartuAset() {
        kartuAset.visibility = View.VISIBLE
        kartuAset.removeAllViews()
        kartuAset.tambah(ui.mute("Total aset"))
        val d = asetTerakhir
        if (d == null) {
            val g = asetGalat
            val pesan = when {
                a.kunci.ambil() == null -> "Total aset butuh kunci API (Setelan)."
                g != null -> "Gagal memuat total aset: $g"
                else -> "Memuat…"
            }
            kartuAset.tambah(ui.mute(pesan), atas = 4)
        } else {
            kartuAset.tambah(ui.teks(d.total, 32f, tebal = true), atas = 2)
            kartuAset.tambah(ui.baris(ui.mute("Buying power"), ui.teks(d.idr, 14f, tebal = true)), atas = 6)
            for (k in d.koin) {
                kartuAset.tambah(ui.baris(ui.teks(k.koin, 14f, tebal = true), ui.teks(k.nilai, 14f)), atas = 10)
                kartuAset.tambah(ui.mute("${k.jumlah} koin × Rp ${k.harga}", 12f))
            }
            if (d.tanpaHarga.isNotEmpty()) {
                kartuAset.tambah(
                    ui.mute("Tanpa harga IDR, tidak dihitung: ${d.tanpaHarga.joinToString(", ")}", 12f),
                    atas = 10,
                )
            }
            val status = StringBuilder("Diperbarui ${Format.waktuSingkat(asetWaktuMs)}")
            asetGalat?.let { status.append(" · gagal memperbarui: ").append(it) }
            kartuAset.tambah(ui.mute(status.toString(), 12f), atas = 10)
        }

        val p = profit ?: return
        val warna = t.warnaTanda(p.profitTotal)
        kartuAset.tambah(
            ui.baris(ui.teks("Profit trading", 14f, tebal = true), ui.teks(Format.untung(p.profitTotal), 15f, warna, tebal = true)),
            atas = 16,
        )
        kartuAset.tambah(ui.mute("${p.jumlahTrade} trade tertutup sejak aplikasi dipasang", 12f))
        tampilFloating()
        kartuAset.tambah(grafik, atas = 12)
        grafik.setProfit(grafikProfit)
        val legenda = LinearLayout(a).apply {
            orientation = LinearLayout.HORIZONTAL
            tambah(ui.teks("Akumulasi profit", 13f, t.accent, tebal = true), lebar = WRAP_CONTENT)
            tambah(ui.mute("Rp 0 (putus-putus)").apply { gravity = Gravity.END }, bobot = 1f, lebar = 0)
        }
        kartuAset.tambah(legenda, atas = 6)
        infoGrafik.text = if (grafikProfit.size >= 2) {
            "${grafikProfit.size} titik · ${Format.tanggalJam(grafikProfit.first().waktuMs)} sampai ${Format.tanggalJam(grafikProfit.last().waktuMs)}"
        } else {
            ""
        }
        kartuAset.tambah(infoGrafik, atas = 4)
        kartuAset.tambah(
            ui.mute("Harga koin = transaksi terakhir. Profit dan target hanya dari trade yang sudah tertutup.", 12f),
            atas = 6,
        )
    }

    /** Satu baris Floating di bawah Profit trading; hilang bila tidak ada posisi terbuka. */
    private fun tampilFloating() {
        val fl = floating ?: return
        if (fl.posisi.isEmpty()) return
        val total = fl.totalAngka
        if (total != null) {
            val persen = fl.totalPersen
            val teks = Format.untung(total) + (if (persen != null) " (${Format.persenBertanda(persen)})" else "")
            kartuAset.tambah(
                ui.baris(ui.teks("Floating", 14f, tebal = true), ui.teks(teks, 15f, t.warnaTanda(total), tebal = true)),
                atas = 10,
            )
        }
        val ket = StringBuilder("${fl.posisi.size} posisi terbuka · sebelum fee jual · tidak masuk target dan grafik")
        if (fl.tanpaHarga.isNotEmpty()) ket.append(" · tanpa harga: ").append(fl.tanpaHarga.joinToString(", "))
        kartuAset.tambah(ui.mute(ket.toString(), 12f))
    }

    private fun tampilTarget() {
        kartuTarget.removeAllViews()
        val p = profit ?: return
        kartuTarget.tambah(blokTarget("Hari ini", p.harian))
        kartuTarget.tambah(blokTarget("Minggu ini", p.mingguan), atas = 16)
        kartuTarget.tambah(
            ui.mute("Persen = profit periode ÷ (total aset − profit periode). Deposit dan withdraw tidak dihitung.", 12f),
            atas = 12,
        )
    }

    private fun blokTarget(nama: String, p: TargetProfitUi): LinearLayout {
        val kolom = LinearLayout(a).apply { orientation = LinearLayout.VERTICAL }
        val pers = p.persen
        val target = p.targetPersen
        val progres = p.progressPersen
        val warna = t.warnaTanda(pers ?: p.profit)

        val kanan: TextView = when {
            pers != null && target != null ->
                ui.teks("${Format.persenBertanda(pers)} dari ${Format.persen(target)}", 14f, warna, tebal = true)
            pers != null -> ui.teks(Format.persenBertanda(pers), 14f, warna, tebal = true)
            else -> ui.teks(Format.untung(p.profit), 14f, warna, tebal = true)
        }
        kolom.tambah(ui.baris(ui.teks(nama, 14f, tebal = true), kanan))

        when {
            target == null ->
                kolom.tambah(ui.mute("Target belum diatur (Setelan) · ${Format.untung(p.profit)}"), atas = 6)
            progres == null || pers == null ->
                kolom.tambah(ui.mute("Menunggu total aset · ${Format.untung(p.profit)}"), atas = 6)
            else -> {
                val angka = progres.toBigDecimalOrNull()?.toInt() ?: 0
                kolom.tambah(ui.bar(angka, if (p.tercapai) t.gain else t.accent), atas = 8, bawah = 4, tinggi = a.dp(10))
                val ket = when {
                    p.tercapai -> "TERCAPAI · ${Format.untung(p.profit)}"
                    Format.negatif(pers) -> "Belum ada hasil bersih · ${Format.untung(p.profit)}"
                    else -> "${Format.ribuan(progres)}% tercapai · ${Format.untung(p.profit)}"
                }
                kolom.tambah(ui.mute(ket))
            }
        }
        return kolom
    }

    private companion object {
        const val JEDA_PUTARAN_MS = 10_000L
        const val JEDA_MINIMAL_MS = 5_000L
    }
}
