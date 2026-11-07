package id.catatanscalping.app

import android.view.View
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.LinearLayout
import android.widget.TextView
import uniffi.scalping_ffi.PencapaianUi
import uniffi.scalping_ffi.RingkasanTargetUi
import uniffi.scalping_ffi.TitikGrafikUi
import uniffi.scalping_ffi.dataGrafik
import uniffi.scalping_ffi.hitungTarget

/**
 * Beranda (prototype): saldo versi aplikasi, hasil terhadap modal aktif, grafik, dan progress target
 * harian/mingguan. Hanya tampilan; angka dari Rust (hitung_target, data_grafik).
 */
class LayarBeranda(a: MainActivity) : Layar(a) {
    private val chip = ui.Chip()
    private val nilaiSaldo = ui.teks("", 32f, tebal = true)
    private val nilaiHasil = ui.teks("", 15f, tebal = true)
    private val nilaiModal = ui.mute("")
    private val grafik = GrafikView(a, t)
    private val infoGrafik = ui.mute("", 12f)
    private val kartuTarget = ui.kartu()
    private val kartuPesan = ui.kartu()
    private val teksPesan = ui.mute("")
    private val kartuSaldo = ui.kartu()

    init {
        header("Catatan trading", chip.root)

        kartuPesan.tambah(teksPesan)
        kartuPesan.visibility = View.GONE
        isi.tambah(kartuPesan, bawah = 12)

        kartuSaldo.tambah(ui.mute("Saldo"))
        kartuSaldo.tambah(nilaiSaldo, atas = 2)
        kartuSaldo.tambah(nilaiHasil)
        kartuSaldo.tambah(nilaiModal)
        kartuSaldo.tambah(grafik, atas = 12)
        val legenda = LinearLayout(a).apply {
            orientation = LinearLayout.HORIZONTAL
            tambah(ui.teks("Saldo", 13f, t.accent, tebal = true), lebar = WRAP_CONTENT)
            tambah(ui.mute("Modal aktif (putus-putus)").apply { gravity = android.view.Gravity.END }, bobot = 1f, lebar = 0)
        }
        kartuSaldo.tambah(legenda, atas = 6)
        kartuSaldo.tambah(infoGrafik, atas = 4)
        isi.tambah(kartuSaldo)

        isi.tambah(ui.judulBagian("Target"), atas = 22, bawah = 10)
        isi.tambah(kartuTarget)
        segarkanChip()
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
        val db = a.pathDb
        Latar.baca({
            val r = hitungTarget(db)
            val g = runCatching { dataGrafik(db, 120u) }.getOrDefault(emptyList())
            Pair(r, g)
        }) { hasil ->
            hasil.onSuccess { (r, g) -> tampil(r, g) }
            hasil.onFailure { tampilGalat(Latar.pesan(it)) }
        }
    }

    private fun tampilGalat(pesan: String) {
        teksPesan.text = "Belum ada data untuk ditampilkan: $pesan\n\n" +
            "Buka Setelan, simpan kunci API, isi TP/SL, lalu tekan \"Sinkron sekarang\"."
        kartuPesan.visibility = View.VISIBLE
        kartuSaldo.visibility = View.GONE
        kartuTarget.removeAllViews()
    }

    private fun tampil(r: RingkasanTargetUi, g: List<TitikGrafikUi>) {
        kartuPesan.visibility = View.GONE
        kartuSaldo.visibility = View.VISIBLE
        nilaiSaldo.text = Format.rupiah(r.saldoSekarang)
        nilaiHasil.text = "${Format.untung(r.selisihModal)} (${Format.persenBertanda(r.hasilPersen)})"
        nilaiHasil.setTextColor(t.warnaTanda(r.selisihModal))
        nilaiModal.text = "dari modal aktif ${Format.rupiah(r.modalAktif)}"
        grafik.setData(g)
        infoGrafik.text = if (g.size >= 2) {
            "${g.size} titik · ${Format.tanggalJam(g.first().waktuMs)} sampai ${Format.tanggalJam(g.last().waktuMs)}"
        } else {
            ""
        }

        kartuTarget.removeAllViews()
        kartuTarget.tambah(blokTarget("Hari ini", r.harian))
        kartuTarget.tambah(blokTarget("Minggu ini", r.mingguan), atas = 16)
    }

    private fun blokTarget(nama: String, p: PencapaianUi): LinearLayout {
        val kolom = LinearLayout(a).apply { orientation = LinearLayout.VERTICAL }
        val target = p.targetPersen
        val progres = p.progressPersen
        val warnaPersen = t.warnaTanda(p.persen)

        val kanan: TextView = if (target != null) {
            ui.teks("${Format.persenBertanda(p.persen)} dari ${Format.persen(target)}", 14f, warnaPersen, tebal = true)
        } else {
            ui.teks(Format.persenBertanda(p.persen), 14f, warnaPersen, tebal = true)
        }
        kolom.tambah(ui.baris(ui.teks(nama, 14f, tebal = true), kanan))

        if (target == null || progres == null) {
            kolom.tambah(ui.mute("Target belum diatur (Setelan) · ${Format.untung(p.profit)}"), atas = 6)
        } else {
            val angka = progres.toBigDecimalOrNull()?.toInt() ?: 0
            kolom.tambah(ui.bar(angka, if (p.tercapai) t.gain else t.accent), atas = 8, bawah = 4, tinggi = a.dp(10))
            val ket = when {
                p.tercapai -> "TERCAPAI · ${Format.untung(p.profit)}"
                Format.negatif(p.persen) -> "Belum ada hasil bersih · ${Format.untung(p.profit)}"
                else -> "${Format.ribuan(progres)}% tercapai · ${Format.untung(p.profit)}"
            }
            kolom.tambah(ui.mute(ket))
        }
        if (!Format.nol(p.setoranBersih)) {
            kolom.tambah(ui.mute("Setoran bersih ${Format.untung(p.setoranBersih)} tidak dihitung sebagai hasil", 12f), atas = 2)
        }
        return kolom
    }
}
