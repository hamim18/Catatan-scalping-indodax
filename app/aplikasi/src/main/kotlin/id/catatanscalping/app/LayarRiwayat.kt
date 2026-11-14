package id.catatanscalping.app

import android.view.Gravity
import android.view.View
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.Button
import android.widget.LinearLayout
import uniffi.scalping_ffi.BarisArusKas
import uniffi.scalping_ffi.BarisTertutup
import uniffi.scalping_ffi.RiwayatUi
import uniffi.scalping_ffi.StatistikUi
import uniffi.scalping_ffi.daftarArusKas
import uniffi.scalping_ffi.riwayatTertutup
import uniffi.scalping_ffi.statistikTrading

/**
 * Riwayat (T10.2): urutan statistik (tetap di atas, tidak ikut tergulir) lalu deposit/withdraw
 * (dropdown, hanya riwayat) lalu daftar trade tertutup dengan harga beli dan jual per trade.
 * Hanya tampilan; semua angka dari Rust.
 */
class LayarRiwayat(a: MainActivity) : Layar(a) {
    private var jumlahTampil = 30
    private var rentang = "semua"
    private var jumlahArus = 10

    private val daftar = LinearLayout(a).apply { orientation = LinearLayout.VERTICAL }
    private val kartuStatistik = ui.kartu().apply { setPadding(a.dp(14), a.dp(12), a.dp(14), a.dp(12)) }
    private val tombolRentang = LinkedHashMap<String, Button>()
    private lateinit var tombolLagi: Button
    private val daftarArus = LinearLayout(a).apply { orientation = LinearLayout.VERTICAL }
    private lateinit var tombolLagiArus: Button

    init {
        // --- Bagian tetap: judul, pilihan rentang, statistik ---
        header("Riwayat", ke = tetap)
        val barisRentang = LinearLayout(a).apply { orientation = LinearLayout.HORIZONTAL }
        for ((nama, kode) in listOf("Hari ini" to "hari", "Minggu ini" to "minggu", "Semua" to "semua")) {
            val b = ui.tombol(nama) { rentang = kode; gayaRentang(); muatStatistik() }
            b.setPadding(a.dp(8), a.dp(6), a.dp(8), a.dp(6))
            tombolRentang[kode] = b
            barisRentang.tambah(b, bobot = 1f, lebar = 0, kanan = 3, kiri = 3)
        }
        tetap.tambah(barisRentang, bawah = 8)
        tetap.tambah(kartuStatistik)
        tetap.visibility = View.VISIBLE
        gayaRentang()

        // --- Deposit dan withdraw (dropdown; T6.4: hanya riwayat) ---
        val kartuArus = ui.kartu()
        val judulArus = ui.teks("Deposit dan withdraw  ▸", 14f, tebal = true)
        val isiArus = LinearLayout(a).apply { orientation = LinearLayout.VERTICAL }
        isiArus.visibility = View.GONE
        judulArus.setOnClickListener {
            val buka = isiArus.visibility != View.VISIBLE
            isiArus.visibility = if (buka) View.VISIBLE else View.GONE
            judulArus.text = if (buka) "Deposit dan withdraw  ▾" else "Deposit dan withdraw  ▸"
        }
        kartuArus.tambah(judulArus)
        isiArus.tambah(daftarArus, atas = 10)
        tombolLagiArus = ui.tombol("Tampilkan lebih banyak") { jumlahArus += 10; muatArus() }
        isiArus.tambah(tombolLagiArus, atas = 10)
        kartuArus.tambah(isiArus)
        isi.tambah(kartuArus)

        // --- Trade tertutup ---
        isi.tambah(daftar, atas = 12)
        tombolLagi = ui.tombol("Tampilkan lebih banyak") { jumlahTampil += 30; muatRiwayat() }
        isi.tambah(tombolLagi, atas = 12)
    }

    override fun segarkan() {
        muatRiwayat()
        muatStatistik()
        muatArus()
    }

    private fun gayaRentang() {
        for ((kode, b) in tombolRentang) ui.gayaTombol(b, kode == rentang)
    }

    // ---------- daftar trade ----------

    private fun muatRiwayat() {
        val db = a.pathDb
        val n = jumlahTampil.toUInt()
        Latar.baca({ riwayatTertutup(db, n) }) { r ->
            r.onSuccess { tampilRiwayat(it) }
            r.onFailure { tampilGalat(Latar.pesan(it)) }
        }
    }

    private fun tampilGalat(pesan: String) {
        daftar.removeAllViews()
        daftar.tambah(ui.kartu().apply { tambah(ui.mute("Riwayat belum bisa dihitung: $pesan")) })
        tombolLagi.visibility = View.GONE
    }

    private fun tampilRiwayat(r: RiwayatUi) {
        daftar.removeAllViews()
        if (r.baris.isEmpty()) {
            daftar.tambah(ui.kartu().apply { tambah(ui.mute("Belum ada trade tertutup.")) })
        }
        for ((i, x) in r.baris.withIndex()) daftar.tambah(itemTrade(x), atas = if (i == 0) 0 else 12)
        tombolLagi.visibility = if (r.totalTrade.toInt() > r.baris.size) View.VISIBLE else View.GONE
    }

    private fun itemTrade(x: BarisTertutup): LinearLayout {
        val kiri = LinearLayout(a).apply {
            orientation = LinearLayout.VERTICAL
            tambah(ui.teks(Format.namaKoin(x.simbol), 15f, tebal = true))
            tambah(ui.mute("${Format.waktuSingkat(x.waktuTutupMs)} · modal ${Format.rupiah(x.modal)}"))
            tambah(ui.mute("Beli ${x.hargaBeli}  →  Jual ${x.hargaJual}"), atas = 2)
        }
        val positif: Boolean? = when {
            Format.negatif(x.profit) -> false
            Format.nol(x.profit) -> null
            else -> true
        }
        val kanan = LinearLayout(a).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.END
            tambah(
                ui.teks(Format.untung(x.profit), 15f, t.warnaTanda(x.profit), tebal = true),
                lebar = WRAP_CONTENT,
            )
            tambah(ui.pill(Format.persenBertanda(x.persen), positif), atas = 4, lebar = WRAP_CONTENT)
        }
        return LinearLayout(a).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            background = ui.bulat(t.card, t.line, 12)
            setPadding(a.dp(14), a.dp(12), a.dp(14), a.dp(12))
            tambah(kiri, bobot = 1f, lebar = 0)
            tambah(kanan, lebar = WRAP_CONTENT)
        }
    }

    // ---------- statistik (ringkas, tetap di atas) ----------

    private fun muatStatistik() {
        val db = a.pathDb
        val kode = rentang
        Latar.baca({ statistikTrading(db, kode) }) { r ->
            r.onSuccess { tampilStatistik(it) }
            r.onFailure {
                kartuStatistik.removeAllViews()
                kartuStatistik.tambah(ui.mute("Statistik belum bisa dihitung: ${Latar.pesan(it)}"))
            }
        }
    }

    /** Tiga baris: jumlah trade dan hasil bersih; win rate dan fee; rata-rata untung dan rugi. */
    private fun tampilStatistik(s: StatistikUi) {
        kartuStatistik.removeAllViews()
        kartuStatistik.tambah(
            ui.baris(
                ui.mute("${s.jumlah} trade", 13f),
                ui.teks(Format.untung(s.hasilBersih), 17f, t.warnaTanda(s.hasilBersih), tebal = true),
            )
        )
        val winRate = Format.persen(s.winRate) + "  (${s.untung}/${s.rugi}" +
            (if (s.impas > 0u) "/${s.impas}" else "") + ")"
        kartuStatistik.tambah(
            ui.baris(
                ui.teks("Win rate $winRate", 13f),
                ui.mute("fee ${Format.rupiah(s.totalFee)}", 13f),
            ),
            atas = 6,
        )
        kartuStatistik.tambah(
            ui.baris(
                ui.mute("Rata-rata untung", 13f),
                ui.teks(Format.untung(s.rataUntung), 13f, t.warnaTanda(s.rataUntung), tebal = true),
            ),
            atas = 6,
        )
        kartuStatistik.tambah(
            ui.baris(
                ui.mute("Rata-rata rugi", 13f),
                ui.teks(Format.untung(s.rataRugi), 13f, t.warnaTanda(s.rataRugi), tebal = true),
            ),
            atas = 4,
        )
    }

    // ---------- deposit dan withdraw (riwayat) ----------

    private fun muatArus() {
        val db = a.pathDb
        val n = jumlahArus
        Latar.baca({ daftarArusKas(db, n.toUInt()) }) { r ->
            r.onSuccess { tampilArus(it, n) }
            r.onFailure {
                daftarArus.removeAllViews()
                daftarArus.tambah(ui.mute("Riwayat deposit/withdraw belum bisa dibaca: ${Latar.pesan(it)}"))
                tombolLagiArus.visibility = View.GONE
            }
        }
    }

    private fun tampilArus(d: List<BarisArusKas>, diminta: Int) {
        daftarArus.removeAllViews()
        if (d.isEmpty()) {
            daftarArus.tambah(ui.mute("Belum ada deposit atau withdraw tercatat."))
        }
        for ((i, x) in d.withIndex()) daftarArus.tambah(itemArus(x), atas = if (i == 0) 0 else 10)
        tombolLagiArus.visibility = if (d.size >= diminta) View.VISIBLE else View.GONE
    }

    private fun itemArus(x: BarisArusKas): LinearLayout {
        val deposit = x.jenis.equals("deposit", ignoreCase = true)
        val kiri = LinearLayout(a).apply {
            orientation = LinearLayout.VERTICAL
            tambah(ui.teks(if (deposit) "Deposit" else "Withdraw", 14f, tebal = true))
            tambah(ui.mute(Format.tanggalJam(x.waktuMs), 12f))
        }
        return LinearLayout(a).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            tambah(kiri, bobot = 1f, lebar = 0)
            tambah(ui.teks(Format.rupiah(x.jumlah), 14f, t.ink, tebal = true), lebar = WRAP_CONTENT)
        }
    }
}
