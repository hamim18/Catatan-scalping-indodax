package id.catatanscalping.app

import android.view.Gravity
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.LinearLayout
import android.widget.ScrollView

/**
 * Dasar tiap layar tab: bagian tetap di atas (`tetap`, kosong bawaan) dan satu ScrollView berisi
 * kolom vertikal (`isi`). Hanya tampilan; angka dihitung di Rust. `segarkan()` memuat ulang data
 * (dipanggil tiap tab dibuka dan saat data berubah).
 *
 * T10.1: `root` kini LinearLayout (bagian tetap + gulir) supaya layar bisa punya bagian yang tidak ikut
 * tergulir (dipakai Riwayat). Tiap header (kecuali layar menu) punya tombol ☰ di kanan.
 */
abstract class Layar(protected val a: MainActivity) {
    protected val ui: Ui = a.ui
    protected val t: Tema = a.tema

    protected val isi: LinearLayout = LinearLayout(a).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(a.dp(16), a.dp(16), a.dp(16), a.dp(24))
    }

    /** Bagian di atas yang tidak ikut tergulir. Tersembunyi sampai ada isinya. */
    protected val tetap: LinearLayout = LinearLayout(a).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(a.dp(16), a.dp(16), a.dp(16), a.dp(8))
        visibility = View.GONE
    }

    private val gulir: ScrollView = ScrollView(a).apply {
        isVerticalScrollBarEnabled = false
        addView(isi)
    }

    val root: LinearLayout = LinearLayout(a).apply {
        orientation = LinearLayout.VERTICAL
        visibility = View.GONE
        addView(tetap, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))
        addView(gulir, LinearLayout.LayoutParams(MATCH_PARENT, 0, 1f))
    }

    fun gulirKeAtas() = gulir.scrollTo(0, 0)

    /**
     * Judul layar di kiri, penanda (chip) opsional, lalu tombol ☰ di kanan.
     * `kembali` = layar menu (Setelan, Panduan): tombol ‹ di kiri judul, tanpa ☰.
     * `ke` = kolom tujuan (isi, atau `tetap` untuk bagian yang tidak tergulir).
     */
    protected fun header(
        judul: String,
        kanan: View? = null,
        kembali: Boolean = false,
        ke: LinearLayout = isi,
    ) {
        val baris = LinearLayout(a).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            if (kembali) {
                val k = ui.teks("‹", 28f, t.accent, tebal = true).apply {
                    setPadding(0, 0, a.dp(14), a.dp(2))
                    setOnClickListener { a.kembaliDariMenu() }
                }
                tambah(k, lebar = WRAP_CONTENT)
            }
            tambah(ui.teks(judul, 20f, tebal = true), bobot = 1f, lebar = 0)
            if (kanan != null) tambah(kanan, lebar = WRAP_CONTENT)
            if (!kembali) {
                val menu = ui.teks("☰", 22f, t.ink).apply {
                    setPadding(a.dp(14), a.dp(4), a.dp(2), a.dp(4))
                    setOnClickListener { a.bukaMenu() }
                }
                tambah(menu, lebar = WRAP_CONTENT)
            }
        }
        ke.tambah(baris, bawah = 14)
    }

    /** Muat ulang data layar ini. Dipanggil di thread UI. */
    open fun segarkan() {}

    /** Status pantauan atau log berubah (bisa sering). Hanya ubah teks status; jangan memuat data berat. */
    open fun pantauBerubah() {}
}
