package id.catatanscalping.app

import android.content.Context
import android.graphics.Canvas
import android.graphics.DashPathEffect
import android.graphics.Paint
import android.graphics.Path
import android.view.View
import uniffi.scalping_ffi.TitikProfitUi

/**
 * Grafik akumulasi profit trade tertutup (FR-22): satu garis tebal, garis putus-putus = Rp 0.
 * T6.5: mode dua garis (saldo dan modal aktif) dihapus. Hanya menggambar; angka dihitung di Rust.
 * Nilai diubah ke Double HANYA untuk koordinat gambar.
 */
class GrafikView(ctx: Context, t: Tema) : View(ctx) {
    private var titikProfit: List<TitikProfitUi> = emptyList()

    private val garisSaldo = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = t.accent; style = Paint.Style.STROKE; strokeWidth = dp(3f)
        strokeJoin = Paint.Join.ROUND; strokeCap = Paint.Cap.ROUND
    }
    private val garisModal = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = t.mute; style = Paint.Style.STROKE; strokeWidth = dp(2f)
        pathEffect = DashPathEffect(floatArrayOf(dp(6f), dp(5f)), 0f)
    }
    private val teksLabel = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = t.mute; textSize = dp(11f)
    }

    private fun dp(n: Float) = n * resources.displayMetrics.density

    /** Kurva akumulasi profit trade tertutup. */
    fun setProfit(data: List<TitikProfitUi>) {
        titikProfit = data
        invalidate()
    }

    override fun onMeasure(lebarSpec: Int, tinggiSpec: Int) {
        setMeasuredDimension(getDefaultSize(suggestedMinimumWidth, lebarSpec), dp(150f).toInt())
    }

    override fun onDraw(c: Canvas) {
        super.onDraw(c)
        gambarProfit(c)
    }

    private fun gambarProfit(c: Canvas) {
        if (titikProfit.isEmpty()) {
            c.drawText("Belum ada data grafik", dp(8f), height / 2f, teksLabel)
            return
        }
        val nilai = titikProfit.map { it.profit.toDoubleOrNull() ?: 0.0 }
        // Rp 0 selalu ikut dalam rentang supaya garis nol terlihat
        val maks0 = maxOf(nilai.max(), 0.0)
        val min0 = minOf(nilai.min(), 0.0)
        val ruang = if (maks0 - min0 < 1.0) 1.0 else (maks0 - min0) * 0.1
        val maks = maks0 + ruang
        val min = min0 - ruang

        val kiri = dp(8f)
        val kanan = width - dp(8f)
        val atas = dp(18f)
        val bawah = height - dp(8f)
        val t0 = titikProfit.first().waktuMs
        val t1 = titikProfit.last().waktuMs
        fun x(i: Int): Float =
            if (t1 == t0) (kiri + kanan) / 2f
            else kiri + ((titikProfit[i].waktuMs - t0).toDouble() / (t1 - t0)).toFloat() * (kanan - kiri)
        fun y(v: Double): Float = (bawah - ((v - min) / (maks - min)).toFloat() * (bawah - atas))

        // garis Rp 0 (putus-putus)
        val nol = Path()
        nol.moveTo(kiri, y(0.0))
        nol.lineTo(kanan, y(0.0))
        c.drawPath(nol, garisModal)

        val p = Path()
        p.moveTo(x(0), y(nilai[0]))
        for (i in 1 until titikProfit.size) p.lineTo(x(i), y(nilai[i]))
        if (titikProfit.size == 1) p.lineTo(kanan, y(nilai[0]))
        c.drawPath(p, garisSaldo)

        c.drawText(Format.untung(maks0.toLong().toString()), kiri, dp(12f), teksLabel)
        c.drawText(Format.untung(min0.toLong().toString()), kiri, height - dp(1f), teksLabel)
    }
}
