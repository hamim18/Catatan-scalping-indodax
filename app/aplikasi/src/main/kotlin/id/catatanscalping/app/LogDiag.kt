package id.catatanscalping.app

import android.content.Context
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.TimeZone

/**
 * T8.1: log diagnostik yang bertahan walau proses dimatikan sistem (berkas `diag.log` di penyimpanan
 * privat aplikasi, maksimal sekitar 256 KB; separuh terlama dibuang bila penuh). Hanya teks status dan
 * galat; tidak pernah menulis kunci API atau saldo. Gagal menulis tidak boleh mengganggu aplikasi.
 */
object LogDiag {
    private const val NAMA = "diag.log"
    private const val MAKS_BYTE = 256 * 1024L
    private var berkas: File? = null

    fun pasang(ctx: Context) {
        berkas = File(ctx.applicationContext.filesDir, NAMA)
    }

    private fun stempel(): String =
        SimpleDateFormat("dd/MM HH:mm:ss", Locale.US).apply { timeZone = TimeZone.getTimeZone("Asia/Jakarta") }
            .format(Date())

    /** Beri cap waktu WIB, tulis ke berkas, dan kembalikan barisnya (untuk tampilan di layar). */
    @Synchronized
    fun catat(pesan: String): String {
        val baris = "${stempel()}  ${pesan.replace('\n', ' ')}"
        val f = berkas ?: return baris
        try {
            if (f.length() > MAKS_BYTE) {
                val sisa = f.readLines().let { it.drop(it.size / 2) }
                f.writeText(sisa.joinToString("\n") + "\n")
            }
            f.appendText(baris + "\n")
        } catch (_: Exception) {
        }
        return baris
    }

    /** `maks` baris terakhir, urutan terlama ke terbaru. */
    @Synchronized
    fun baca(maks: Int): List<String> {
        val f = berkas ?: return emptyList()
        return try {
            if (f.exists()) f.readLines().takeLast(maks) else emptyList()
        } catch (_: Exception) {
            emptyList()
        }
    }
}
