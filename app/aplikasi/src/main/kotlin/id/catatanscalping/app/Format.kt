package id.catatanscalping.app

import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.TimeZone

/**
 * Format tampilan Indonesia untuk angka yang sudah dihitung di Rust (teks seperti "1482350", "-2940", "0.58").
 * Hanya mengubah teks (titik ribuan, koma desimal, tanda); tidak ada perhitungan uang di sini.
 * Teks yang bukan angka (contoh "-" dari statistik) dikembalikan apa adanya.
 */
object Format {
    private val ANGKA = Regex("^[-−]?\\d+(\\.\\d+)?$")
    private val WIB: TimeZone = TimeZone.getTimeZone("Asia/Jakarta")

    private fun mutlak(a: String): String = a.trim().trimStart('-', '−')

    fun nol(a: String): Boolean {
        val s = a.trim()
        return !ANGKA.matches(s) || mutlak(s).all { it == '0' || it == '.' }
    }

    fun negatif(a: String): Boolean {
        val s = a.trim()
        return ANGKA.matches(s) && (s.startsWith("-") || s.startsWith("−")) && !nol(s)
    }

    /** "1482350" -> "1.482.350", "-2940" -> "−2.940", "12.5" -> "12,5". */
    fun ribuan(angka: String): String {
        val s = angka.trim()
        if (!ANGKA.matches(s)) return angka
        val neg = negatif(s)
        val tanpaTanda = mutlak(s)
        val bulat = tanpaTanda.substringBefore('.')
        val desimal = if ('.' in tanpaTanda) tanpaTanda.substringAfter('.') else null
        val sb = StringBuilder()
        if (neg) sb.append('−')
        val n = bulat.length
        for ((i, c) in bulat.withIndex()) {
            if (i > 0 && (n - i) % 3 == 0) sb.append('.')
            sb.append(c)
        }
        if (desimal != null) sb.append(',').append(desimal)
        return sb.toString()
    }

    /** "65526" -> "Rp 65.526", "-2940" -> "−Rp 2.940". */
    fun rupiah(angka: String): String {
        if (!ANGKA.matches(angka.trim())) return angka
        val r = ribuan(angka)
        return if (r.startsWith("−")) "−Rp ${r.drop(1)}" else "Rp $r"
    }

    /** Hasil dengan tanda: "+Rp 380", "−Rp 2.940", "Rp 0". */
    fun untung(angka: String): String {
        if (!ANGKA.matches(angka.trim())) return angka
        return when {
            nol(angka) -> "Rp 0"
            negatif(angka) -> "−Rp ${ribuan(mutlak(angka))}"
            else -> "+Rp ${ribuan(mutlak(angka))}"
        }
    }

    /** Buang nol di belakang koma: "0.90" -> "0.9", "5.00" -> "5". */
    private fun rapi(a: String): String {
        var s = mutlak(a)
        if ('.' in s) s = s.trimEnd('0').trimEnd('.')
        return s.ifEmpty { "0" }
    }

    /** "1.5" -> "1,5%", "-0.98" -> "−0,98%". */
    fun persen(angka: String): String {
        if (!ANGKA.matches(angka.trim())) return angka
        val tanda = if (negatif(angka)) "−" else ""
        return "$tanda${ribuan(rapi(angka))}%"
    }

    /** "0.58" -> "+0,58%", "-0.98" -> "−0,98%", "0" -> "0%". */
    fun persenBertanda(angka: String): String {
        if (!ANGKA.matches(angka.trim())) return angka
        return when {
            nol(angka) -> "0%"
            negatif(angka) -> "−${ribuan(rapi(angka))}%"
            else -> "+${ribuan(rapi(angka))}%"
        }
    }

    /** Teks dari kolom isian dengan koma desimal ("1.5" -> "1,5"). */
    fun komaDesimal(angka: String): String = angka.replace('.', ',')

    /** "uaiidr" -> "UAI" (sama dengan nama_koin di Rust). */
    fun namaKoin(simbol: String): String =
        simbol.lowercase().removeSuffix("idr").trimEnd('_').uppercase()

    private fun format(pola: String, ms: Long): String =
        SimpleDateFormat(pola, Locale("id", "ID")).apply { timeZone = WIB }.format(Date(ms))

    /** "07 Nov 17:07" (WIB). */
    fun tanggalJam(ms: Long): String = format("dd MMM HH:mm", ms)

    /** "17:07" bila hari ini (WIB), selain itu "07 Nov 17:07". */
    fun waktuSingkat(ms: Long): String {
        val hari = "yyyyMMdd"
        return if (format(hari, ms) == format(hari, System.currentTimeMillis())) format("HH:mm", ms) else tanggalJam(ms)
    }
}
