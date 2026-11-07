package id.catatanscalping.app

import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.DashPathEffect
import android.graphics.Paint
import android.graphics.Path
import android.view.View
import uniffi.scalping_ffi.TitikGrafikUi

/**
 * Grafik dua garis (FR-22): saldo versi aplikasi (garis tebal) dan modal aktif (putus-putus,
 * berundak karena hanya berubah oleh setoran/koreksi). Jarak antara keduanya adalah profit.
 * Hanya menggambar; angka sudah dihitung di Rust. Nilai diubah ke Double HANYA untuk koordinat gambar.
 */
class GrafikView(ctx: Context) : View(ctx) {
    private var titik: List<TitikGrafikUi> = emptyList()

    private val garisSaldo = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#4F8CFF"); style = Paint.Style.STROKE; strokeWidth = dp(3f)
        strokeJoin = Paint.Join.ROUND; strokeCap = Paint.Cap.ROUND
    }
    private val garisModal = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#9AA5B1"); style = Paint.Style.STROKE; strokeWidth = dp(2f)
        pathEffect = DashPathEffect(floatArrayOf(dp(6f), dp(5f)), 0f)
    }
    private val teksLabel = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#9AA5B1"); textSize = dp(11f)
    }

    private fun dp(n: Float) = n * resources.displayMetrics.density

    fun setData(data: List<TitikGrafikUi>) {
        titik = data
        invalidate()
    }

    override fun onMeasure(lebarSpec: Int, tinggiSpec: Int) {
        setMeasuredDimension(getDefaultSize(suggestedMinimumWidth, lebarSpec), dp(180f).toInt())
    }

    override fun onDraw(c: Canvas) {
        super.onDraw(c)
        if (titik.isEmpty()) {
            c.drawText("Belum ada data grafik", dp(8f), height / 2f, teksLabel)
            return
        }
        val saldo = titik.map { it.saldo.toDoubleOrNull() ?: 0.0 }
        val modal = titik.map { it.modalAktif.toDoubleOrNull() ?: 0.0 }
        val maks0 = (saldo + modal).max()
        val min0 = (saldo + modal).min()
        // beri ruang supaya garis datar tidak menempel di tepi
        val ruang = if (maks0 - min0 < 1.0) 1.0 else (maks0 - min0) * 0.1
        val maks = maks0 + ruang
        val min = min0 - ruang

        val kiri = dp(8f)
        val kanan = width - dp(8f)
        val atas = dp(18f)
        val bawah = height - dp(8f)
        val t0 = titik.first().waktuMs
        val t1 = titik.last().waktuMs
        fun x(i: Int): Float =
            if (t1 == t0) (kiri + kanan) / 2f
            else kiri + ((titik[i].waktuMs - t0).toDouble() / (t1 - t0)).toFloat() * (kanan - kiri)
        fun y(v: Double): Float = (bawah - ((v - min) / (maks - min)).toFloat() * (bawah - atas))

        // modal aktif: berundak (nilai bertahan sampai titik berikutnya)
        val pm = Path()
        pm.moveTo(x(0), y(modal[0]))
        for (i in 1 until titik.size) {
            pm.lineTo(x(i), y(modal[i - 1]))
            pm.lineTo(x(i), y(modal[i]))
        }
        c.drawPath(pm, garisModal)

        val ps = Path()
        ps.moveTo(x(0), y(saldo[0]))
        for (i in 1 until titik.size) ps.lineTo(x(i), y(saldo[i]))
        if (titik.size == 1) ps.lineTo(kanan, y(saldo[0]))
        c.drawPath(ps, garisSaldo)

        c.drawText("Rp ${maks0.toLong()}", kiri, dp(12f), teksLabel)
        c.drawText("Rp ${min0.toLong()}", kiri, height - dp(1f), teksLabel)
    }
}
