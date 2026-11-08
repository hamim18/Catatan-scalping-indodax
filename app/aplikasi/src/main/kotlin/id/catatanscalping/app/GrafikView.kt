package id.catatanscalping.app

import android.content.Context
import android.graphics.Canvas
import android.graphics.DashPathEffect
import android.graphics.Paint
import android.graphics.Path
import android.view.View
import uniffi.scalping_ffi.TitikGrafikUi
import uniffi.scalping_ffi.TitikProfitUi

/**
 * Grafik dua garis (FR-22): saldo versi aplikasi (garis tebal) dan modal aktif (putus-putus,
 * berundak karena hanya berubah oleh setoran/koreksi). Jarak antara keduanya adalah profit.
 * T6.2: mode kedua `setProfit` menggambar kurva akumulasi profit trade tertutup (satu garis tebal,
 * garis putus-putus = Rp 0). Hanya menggambar; angka sudah dihitung di Rust. Nilai diubah ke Double HANYA untuk koordinat gambar.
 */
class GrafikView(ctx: Context, t: Tema) : View(ctx) {
    private var titik: List<TitikGrafikUi> = emptyList()
    private var titikProfit: List<TitikProfitUi> = emptyList()
    private var modeProfit = false

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

    fun setData(data: List<TitikGrafikUi>) {
        titik = data
        modeProfit = false
        invalidate()
    }

    /** T6.2: kurva akumulasi profit trade tertutup. */
    fun setProfit(data: List<TitikProfitUi>) {
        titikProfit = data
        modeProfit = true
        invalidate()
    }

    override fun onMeasure(lebarSpec: Int, tinggiSpec: Int) {
        setMeasuredDimension(getDefaultSize(suggestedMinimumWidth, lebarSpec), dp(150f).toInt())
    }

    override fun onDraw(c: Canvas) {
        super.onDraw(c)
        if (modeProfit) {
            gambarProfit(c)
            return
        }
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
        if (titik.size == 1) pm.lineTo(kanan, y(modal[0]))
        c.drawPath(pm, garisModal)

        val ps = Path()
        ps.moveTo(x(0), y(saldo[0]))
        for (i in 1 until titik.size) ps.lineTo(x(i), y(saldo[i]))
        if (titik.size == 1) ps.lineTo(kanan, y(saldo[0]))
        c.drawPath(ps, garisSaldo)

        c.drawText(Format.rupiah(maks0.toLong().toString()), kiri, dp(12f), teksLabel)
        c.drawText(Format.rupiah(min0.toLong().toString()), kiri, height - dp(1f), teksLabel)
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
