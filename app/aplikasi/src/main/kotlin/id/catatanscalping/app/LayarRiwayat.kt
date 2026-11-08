package id.catatanscalping.app

import android.view.Gravity
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
 * Riwayat (prototype; disederhanakan di T7.2): ringkasan hari ini, daftar trade tertutup, statistik (T4.1),
 * dan riwayat deposit/withdraw IDR di bagian bawah (hanya riwayat). Hanya tampilan; semua angka dari Rust.
 */
class LayarRiwayat(a: MainActivity) : Layar(a) {
    private var jumlahTampil = 30
    private var rentang = "semua"
    private var jumlahArus = 10

    private val daftar = LinearLayout(a).apply { orientation = LinearLayout.VERTICAL }
    private val kartuRingkasan = ui.kartu()
    private val kartuStatistik = ui.kartu()
    private val tombolRentang = LinkedHashMap<String, Button>()
    private lateinit var tombolLagi: Button
    private val daftarArus = LinearLayout(a).apply { orientation = LinearLayout.VERTICAL }
    private lateinit var tombolLagiArus: Button

    init {
        header("Riwayat")

        isi.tambah(kartuRingkasan, bawah = 12)
        isi.tambah(daftar)
        tombolLagi = ui.tombol("Tampilkan lebih banyak") { jumlahTampil += 30; muatRiwayat() }
        isi.tambah(tombolLagi, atas = 12)

        // --- Statistik ---
        isi.tambah(ui.judulBagian("Statistik"), atas = 22, bawah = 10)
        val barisRentang = LinearLayout(a).apply { orientation = LinearLayout.HORIZONTAL }
        for ((nama, kode) in listOf("Hari ini" to "hari", "Minggu ini" to "minggu", "Semua" to "semua")) {
            val b = ui.tombol(nama) { rentang = kode; gayaRentang(); muatStatistik() }
            tombolRentang[kode] = b
            barisRentang.tambah(b, bobot = 1f, lebar = 0, kanan = 3, kiri = 3)
        }
        isi.tambah(barisRentang, bawah = 10)
        isi.tambah(kartuStatistik)
        gayaRentang()

        // --- Deposit dan withdraw (T6.4: hanya riwayat) ---
        isi.tambah(ui.judulBagian("Deposit dan withdraw"), atas = 22, bawah = 10)
        isi.tambah(daftarArus)
        tombolLagiArus = ui.tombol("Tampilkan lebih banyak") { jumlahArus += 10; muatArus() }
        isi.tambah(tombolLagiArus, atas = 12)
    }

    override fun segarkan() {
        muatRiwayat()
        muatStatistik()
        muatArus()
    }

    private fun gayaRentang() {
        for ((kode, b) in tombolRentang) ui.gayaTombol(b, kode == rentang)
    }

    // ---------- ringkasan dan daftar ----------

    private fun muatRiwayat() {
        val db = a.pathDb
        val n = jumlahTampil.toUInt()
        Latar.baca({ Pair(riwayatTertutup(db, n), statistikTrading(db, "hari")) }) { r ->
            r.onSuccess { (rw, sh) -> tampilRiwayat(rw, sh) }
            r.onFailure { tampilGalat(Latar.pesan(it)) }
        }
    }

    private fun tampilGalat(pesan: String) {
        kartuRingkasan.removeAllViews()
        kartuRingkasan.tambah(ui.mute("Riwayat belum bisa dihitung: $pesan"))
        daftar.removeAllViews()
        tombolLagi.visibility = android.view.View.GONE
    }

    private fun tampilRiwayat(r: RiwayatUi, hari: StatistikUi) {
        kartuRingkasan.removeAllViews()
        kartuRingkasan.tambah(
            ui.baris(
                ui.mute("Hasil bersih hari ini"),
                ui.teks(Format.untung(r.hariIniProfit), 15f, t.warnaTanda(r.hariIniProfit), tebal = true),
            )
        )
        kartuRingkasan.tambah(
            ui.mute("${r.hariIniJumlah} trade · ${r.hariIniUntung} untung · fee ${Format.rupiah(hari.totalFee)}"),
            atas = 4,
        )

        daftar.removeAllViews()
        if (r.baris.isEmpty()) {
            daftar.tambah(ui.kartu().apply { tambah(ui.mute("Belum ada trade tertutup.")) })
        }
        for ((i, x) in r.baris.withIndex()) daftar.tambah(itemTrade(x), atas = if (i == 0) 0 else 12)
        tombolLagi.visibility =
            if (r.totalTrade.toInt() > r.baris.size) android.view.View.VISIBLE else android.view.View.GONE
    }

    private fun itemTrade(x: BarisTertutup): LinearLayout {
        val kiri = LinearLayout(a).apply {
            orientation = LinearLayout.VERTICAL
            tambah(ui.teks(Format.namaKoin(x.simbol), 15f, tebal = true))
            tambah(ui.mute("${Format.waktuSingkat(x.waktuTutupMs)} · modal ${Format.rupiah(x.modal)}"))
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

    // ---------- statistik ----------

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

    private fun barisStat(label: String, nilai: String, warna: Int = t.ink): LinearLayout =
        ui.baris(ui.mute(label, 13f), ui.teks(nilai, 14f, warna, tebal = true))

    private fun tampilStatistik(s: StatistikUi) {
        val judul = when (s.rentang) { "hari" -> "Hari ini"; "minggu" -> "Minggu ini"; else -> "Semua riwayat" }
        kartuStatistik.removeAllViews()
        kartuStatistik.tambah(ui.teks("$judul · ${s.jumlah} trade", 14f, tebal = true))
        val baris = listOf(
            barisStat(
                "Win rate",
                Format.persen(s.winRate) + "  (${s.untung} untung · ${s.rugi} rugi" +
                    (if (s.impas > 0u) " · ${s.impas} impas" else "") + ")",
            ),
            barisStat("Rata-rata untung", Format.untung(s.rataUntung), t.warnaTanda(s.rataUntung)),
            barisStat("Rata-rata rugi", Format.untung(s.rataRugi), t.warnaTanda(s.rataRugi)),
            barisStat("Hasil bersih", Format.untung(s.hasilBersih), t.warnaTanda(s.hasilBersih)),
            barisStat(
                "Fee",
                Format.rupiah(s.totalFee) + (if (s.persenFee != "-") " (${Format.persen(s.persenFee)} dari profit)" else ""),
            ),
        )
        for (b in baris) kartuStatistik.tambah(b, atas = 8)
    }

    // ---------- deposit dan withdraw (riwayat) ----------

    private fun muatArus() {
        val db = a.pathDb
        val n = jumlahArus
        Latar.baca({ daftarArusKas(db, n.toUInt()) }) { r ->
            r.onSuccess { tampilArus(it, n) }
            r.onFailure {
                daftarArus.removeAllViews()
                daftarArus.tambah(ui.kartu().apply { tambah(ui.mute("Riwayat deposit/withdraw belum bisa dibaca: ${Latar.pesan(it)}")) })
                tombolLagiArus.visibility = android.view.View.GONE
            }
        }
    }

    private fun tampilArus(d: List<BarisArusKas>, diminta: Int) {
        daftarArus.removeAllViews()
        if (d.isEmpty()) {
            daftarArus.tambah(ui.kartu().apply { tambah(ui.mute("Belum ada deposit atau withdraw tercatat.")) })
        }
        for ((i, x) in d.withIndex()) daftarArus.tambah(itemArus(x), atas = if (i == 0) 0 else 12)
        tombolLagiArus.visibility = if (d.size >= diminta) android.view.View.VISIBLE else android.view.View.GONE
    }

    private fun itemArus(x: BarisArusKas): LinearLayout {
        val deposit = x.jenis.equals("deposit", ignoreCase = true)
        val kiri = LinearLayout(a).apply {
            orientation = LinearLayout.VERTICAL
            tambah(ui.teks(if (deposit) "Deposit" else "Withdraw", 15f, tebal = true))
            tambah(ui.mute(Format.tanggalJam(x.waktuMs)))
        }
        val kanan = LinearLayout(a).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.END
            tambah(
                ui.teks(Format.rupiah(x.jumlah), 15f, t.ink, tebal = true),
                lebar = WRAP_CONTENT,
            )
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
}
