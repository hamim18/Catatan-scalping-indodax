package id.catatanscalping.app

import android.view.Gravity
import android.view.View
import android.widget.LinearLayout
import android.widget.ScrollView

/**
 * Dasar tiap layar tab: satu ScrollView berisi kolom vertikal (`isi`). Hanya tampilan; angka
 * dihitung di Rust. `segarkan()` memuat ulang data (dipanggil tiap tab dibuka dan saat data berubah).
 */
abstract class Layar(protected val a: MainActivity) {
    protected val ui: Ui = a.ui
    protected val t: Tema = a.tema

    protected val isi: LinearLayout = LinearLayout(a).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(a.dp(16), a.dp(16), a.dp(16), a.dp(24))
    }

    val root: ScrollView = ScrollView(a).apply {
        isVerticalScrollBarEnabled = false
        visibility = View.GONE
        addView(isi)
    }

    /** Judul layar di kiri, penanda (chip) opsional di kanan. */
    protected fun header(judul: String, kanan: View? = null) {
        val baris = LinearLayout(a).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            tambah(ui.teks(judul, 20f, tebal = true), bobot = 1f, lebar = 0)
            if (kanan != null) tambah(kanan, lebar = android.view.ViewGroup.LayoutParams.WRAP_CONTENT)
        }
        isi.tambah(baris, bawah = 14)
    }

    /** Muat ulang data layar ini. Dipanggil di thread UI. */
    open fun segarkan() {}

    /** Status pantauan atau log berubah (bisa sering). Hanya ubah teks status; jangan memuat data berat. */
    open fun pantauBerubah() {}
}
