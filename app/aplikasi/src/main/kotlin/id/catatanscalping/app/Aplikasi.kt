package id.catatanscalping.app

import android.app.ActivityManager
import android.app.Application
import android.app.ApplicationExitInfo
import android.os.Build

/**
 * T8.1: dijalankan setiap proses aplikasi dimulai (juga saat dihidupkan alarm, boot, atau START_STICKY).
 * Mencatat awal proses, alasan proses sebelumnya berakhir (Android 11+), dan crash tak tertangani.
 */
class Aplikasi : Application() {
    override fun onCreate() {
        super.onCreate()
        LogDiag.pasang(this)
        PantauBersama.muatAwal(LogDiag.baca(40))

        val lama = Thread.getDefaultUncaughtExceptionHandler()
        Thread.setDefaultUncaughtExceptionHandler { t, e ->
            try {
                LogDiag.catat("CRASH di ${t.name}: ${e.stackTraceToString().replace('\n', ' ').take(1500)}")
            } catch (_: Throwable) {
            }
            lama?.uncaughtException(t, e)
        }

        catatAlasanProsesSebelumnya()
        LogDiag.catat("Proses dimulai · versi ${versi()} · Android ${Build.VERSION.SDK_INT}")

        // Proses dihidupkan ulang (sticky, alarm, boot) sementara pantauan seharusnya menyala: pastikan penjaga terjadwal.
        if (getSharedPreferences(PantauBersama.PREF, MODE_PRIVATE).getBoolean(PantauBersama.PREF_AKTIF, false)) {
            PenjagaPantauan.jadwalkan(this)
        }
    }

    private fun versi(): String = try {
        packageManager.getPackageInfo(packageName, 0).versionName ?: "?"
    } catch (_: Exception) {
        "?"
    }

    private fun catatAlasanProsesSebelumnya() {
        try {
            val prefs = getSharedPreferences("diag", MODE_PRIVATE)
            val terakhir = prefs.getLong("alasan_terakhir_ms", 0L)
            val am = getSystemService(ActivityManager::class.java)
            var maks = terakhir
            for (x in am.getHistoricalProcessExitReasons(packageName, 0, 10).sortedBy { it.timestamp }) {
                if (x.timestamp <= terakhir) continue
                val nama = when (x.reason) {
                    ApplicationExitInfo.REASON_LOW_MEMORY -> "memori sistem habis"
                    ApplicationExitInfo.REASON_USER_REQUESTED -> "dihentikan pengguna"
                    ApplicationExitInfo.REASON_USER_STOPPED -> "force stop"
                    ApplicationExitInfo.REASON_EXCESSIVE_RESOURCE_USAGE -> "pemakaian sumber daya berlebih"
                    ApplicationExitInfo.REASON_CRASH -> "crash"
                    ApplicationExitInfo.REASON_CRASH_NATIVE -> "crash native"
                    ApplicationExitInfo.REASON_ANR -> "ANR"
                    ApplicationExitInfo.REASON_SIGNALED -> "dibunuh sinyal (biasanya sistem/baterai)"
                    ApplicationExitInfo.REASON_DEPENDENCY_DIED -> "dependensi mati"
                    ApplicationExitInfo.REASON_PERMISSION_CHANGE -> "izin berubah"
                    ApplicationExitInfo.REASON_INITIALIZATION_FAILURE -> "gagal inisialisasi"
                    ApplicationExitInfo.REASON_EXIT_SELF -> "keluar sendiri"
                    ApplicationExitInfo.REASON_FREEZER -> "dibekukan sistem"
                    ApplicationExitInfo.REASON_PACKAGE_STATE_CHANGE -> "status paket berubah"
                    ApplicationExitInfo.REASON_PACKAGE_UPDATED -> "aplikasi diperbarui"
                    else -> "lainnya (${x.reason})"
                }
                LogDiag.catat("Proses sebelumnya berakhir: $nama${x.description?.let { " · $it" } ?: ""}")
                if (x.timestamp > maks) maks = x.timestamp
            }
            prefs.edit().putLong("alasan_terakhir_ms", maks).apply()
        } catch (_: Exception) {
        }
    }
}
