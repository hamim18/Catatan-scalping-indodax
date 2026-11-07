package id.catatanscalping.app

import android.app.Activity
import android.app.AlertDialog
import android.content.Intent
import android.net.Uri
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.TimeZone
import uniffi.scalping_ffi.RingkasanCadanganUi
import uniffi.scalping_ffi.eksporCadangan
import uniffi.scalping_ffi.periksaCadangan
import uniffi.scalping_ffi.pulihkanCadangan

/**
 * U5: cadangan data ke penyimpanan lokal lewat pemilih berkas sistem (SAF).
 * Hanya menyalin berkas; pembuatan salinan, pemeriksaan, dan penggantian database dilakukan Rust.
 * Kunci API tidak ada di database, jadi tidak ikut dicadangkan.
 */
object Cadangan {
    const val KODE_EKSPOR = 21
    const val KODE_IMPOR = 22

    private fun waktuNama(): String =
        SimpleDateFormat("yyyyMMdd-HHmm", Locale.US)
            .apply { timeZone = TimeZone.getTimeZone("Asia/Jakarta") }
            .format(Date())

    /** Buka pemilih lokasi simpan. Hasil diterima MainActivity.onActivityResult. */
    @Suppress("DEPRECATION")
    fun mintaEkspor(a: Activity) {
        val i = Intent(Intent.ACTION_CREATE_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = "application/octet-stream"
            putExtra(Intent.EXTRA_TITLE, "catatan-scalping-${waktuNama()}.db")
        }
        a.startActivityForResult(i, KODE_EKSPOR)
    }

    /** Buka pemilih berkas cadangan. Hasil diterima MainActivity.onActivityResult. */
    @Suppress("DEPRECATION")
    fun mintaImpor(a: Activity) {
        val i = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = "*/*"
        }
        a.startActivityForResult(i, KODE_IMPOR)
    }

    private fun ringkas(r: RingkasanCadanganUi): String =
        "${r.jumlahTrade} trade, ${r.jumlahArusKas} deposit/withdraw, ${r.jumlahKoreksi} koreksi saldo"

    /** Ekspor: Rust membuat salinan di cache, lalu disalin ke berkas pilihan pengguna. */
    fun ekspor(a: MainActivity, uri: Uri, selesai: (String) -> Unit) {
        val db = a.pathDb
        Latar.baca({
            val tmp = File(a.cacheDir, "ekspor.db")
            tmp.delete()
            try {
                val r = eksporCadangan(db, tmp.path)
                val keluar = a.contentResolver.openOutputStream(uri, "w")
                    ?: throw IllegalStateException("Tidak bisa menulis ke lokasi yang dipilih")
                keluar.use { out -> tmp.inputStream().use { it.copyTo(out) } }
                r
            } finally {
                tmp.delete()
            }
        }) { r ->
            r.onSuccess { selesai("Cadangan tersimpan: ${ringkas(it)}.") }
            r.onFailure { selesai("Ekspor gagal: ${Latar.pesan(it)}") }
        }
    }

    /**
     * Impor, dua langkah: (1) salin berkas ke cache dan periksa, (2) setelah pengguna setuju,
     * ganti database. Pemanggil harus memastikan pantauan sudah mati.
     */
    fun impor(a: MainActivity, uri: Uri, selesai: (pesan: String, berhasil: Boolean) -> Unit) {
        Latar.baca({
            val tmp = File(a.cacheDir, "impor.db")
            tmp.delete()
            try {
                val masuk = a.contentResolver.openInputStream(uri)
                    ?: throw IllegalStateException("Tidak bisa membaca berkas yang dipilih")
                masuk.use { inp -> tmp.outputStream().use { inp.copyTo(it) } }
                Pair(tmp, periksaCadangan(tmp.path))
            } catch (e: Throwable) {
                tmp.delete()
                throw e
            }
        }) { r ->
            r.onFailure { selesai("Impor dibatalkan: ${Latar.pesan(it)}", false) }
            r.onSuccess { (tmp, ringkasan) ->
                konfirmasi(a, tmp, ringkasan, selesai)
            }
        }
    }

    private fun konfirmasi(
        a: MainActivity,
        tmp: File,
        ringkasan: RingkasanCadanganUi,
        selesai: (String, Boolean) -> Unit,
    ) {
        AlertDialog.Builder(a)
            .setTitle("Ganti data dengan cadangan?")
            .setMessage(
                "Berkas berisi ${ringkas(ringkasan)}.\n\n" +
                    "Semua data di aplikasi sekarang akan diganti. Salinan data lama disimpan otomatis di folder privat aplikasi.\n\n" +
                    "Kunci API tidak berubah."
            )
            .setPositiveButton("Ganti") { _, _ ->
                val db = a.pathDb
                Latar.baca({
                    try {
                        pulihkanCadangan(db, tmp.path)
                    } finally {
                        tmp.delete()
                    }
                }) { r ->
                    r.onSuccess { selesai("Impor selesai: ${ringkas(it)}.", true) }
                    r.onFailure { selesai("Impor gagal, data lama tidak berubah: ${Latar.pesan(it)}", false) }
                }
            }
            .setNegativeButton("Batal") { _, _ ->
                tmp.delete()
                selesai("Impor dibatalkan.", false)
            }
            .setOnCancelListener {
                tmp.delete()
                selesai("Impor dibatalkan.", false)
            }
            .show()
    }
}
