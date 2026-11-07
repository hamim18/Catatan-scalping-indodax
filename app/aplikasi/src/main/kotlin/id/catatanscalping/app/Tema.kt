package id.catatanscalping.app

import android.content.Context
import android.content.res.Configuration
import android.graphics.Color

/**
 * Warna sesuai prototype (terang dan gelap). Dipilih sekali saat Activity dibuat;
 * bila tema sistem berganti, Android membuat ulang Activity sehingga warna ikut berganti.
 */
class Tema(val gelap: Boolean) {
    private fun w(terang: String, gelapHex: String): Int = Color.parseColor(if (gelap) gelapHex else terang)

    val bg = w("#eef1f4", "#0f161d")
    val card = w("#ffffff", "#18222c")
    val ink = w("#14202b", "#e8eef3")
    val mute = w("#5d6b78", "#93a1ad")
    val line = w("#d9e0e6", "#27343f")
    val gain = w("#0b8a68", "#3dc79c")
    val loss = w("#cf4338", "#ff7a6e")
    val accent = w("#2f4fd1", "#8aa0ff")
    val gainBg = w("#dff3ec", "#12332b")
    val lossBg = w("#fae3e0", "#3a1f1d")
    val soft = w("#f4f6f8", "#1e2a35")

    /** Tidak ada di prototype: untuk status "menyambung". */
    val warn = w("#b7791f", "#e0a84a")

    /** Warna teks di atas tombol utama (accent). */
    val teksAksen = if (gelap) Color.parseColor("#0f161d") else Color.WHITE

    /** Hijau bila angka positif, merah bila negatif, abu-abu bila nol atau bukan angka. */
    fun warnaTanda(angka: String): Int = when {
        Format.negatif(angka) -> loss
        Format.nol(angka) -> mute
        else -> gain
    }

    companion object {
        fun dari(ctx: Context): Tema {
            val mode = ctx.resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK
            return Tema(mode == Configuration.UI_MODE_NIGHT_YES)
        }
    }
}
