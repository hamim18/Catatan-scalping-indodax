package id.catatanscalping.app

import android.app.AlarmManager
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.os.SystemClock
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.TimeZone

/**
 * T8.1: penjaga pantauan. Alarm (setAndAllowWhileIdle, tiap sekitar 5 menit) membangunkan receiver ini
 * walau layanan atau prosesnya sudah mati. Bila pengguna ingin pantauan menyala (PREF_AKTIF) tetapi
 * pantauan mati, atau terputus lebih dari 5 menit, dan keadaan itu bertahan sekitar 4 menit lagi,
 * muncul satu notifikasi peringatan (paling sering sekali per 30 menit). Penjaga TIDAK menyalakan layanan
 * sendiri (Android 12+ melarang foreground service dimulai dari latar); pengguna cukup membuka aplikasi.
 * Berhenti sendiri saat pantauan dihentikan pengguna.
 */
class PenjagaPantauan : BroadcastReceiver() {
    override fun onReceive(ctx: Context, intent: Intent?) {
        try {
            periksa(ctx.applicationContext)
        } catch (e: Exception) {
            LogDiag.catat("Penjaga galat: ${e.message}")
        }
    }

    companion object {
        private const val KUNCI_TERPUTUS = "terputus_sejak_ms"
        private const val KUNCI_BURUK = "buruk_sejak_ms"
        private const val KUNCI_PERINGATAN = "peringatan_ms"
        private const val SALURAN_PERINGATAN = "peringatan"
        private const val ID_PERINGATAN = 1003
        private const val JEDA_CEK_MENIT = 5L
        private const val PUTUS_LAMA_MS = 5 * 60_000L
        private const val BURUK_BERTAHAN_MS = 4 * 60_000L
        private const val JEDA_PERINGATAN_MS = 30 * 60_000L

        private fun prefs(ctx: Context) = ctx.getSharedPreferences(PantauBersama.PREF, Context.MODE_PRIVATE)

        private fun pi(ctx: Context): PendingIntent = PendingIntent.getBroadcast(
            ctx, 77, Intent(ctx, PenjagaPantauan::class.java),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )

        /** Jadwalkan pemeriksaan berikutnya (menimpa jadwal lama). */
        fun jadwalkan(ctx: Context) {
            val c = ctx.applicationContext
            c.getSystemService(AlarmManager::class.java).setAndAllowWhileIdle(
                AlarmManager.ELAPSED_REALTIME_WAKEUP,
                SystemClock.elapsedRealtime() + JEDA_CEK_MENIT * 60_000L,
                pi(c),
            )
        }

        /** Hentikan penjaga dan bersihkan penanda/peringatan (pantauan dihentikan pengguna atau berhenti). */
        fun batal(ctx: Context) {
            val c = ctx.applicationContext
            c.getSystemService(AlarmManager::class.java).cancel(pi(c))
            bersihkan(c)
            prefs(c).edit().remove(KUNCI_TERPUTUS).apply()
        }

        fun tandaiTersambung(ctx: Context) {
            prefs(ctx).edit().remove(KUNCI_TERPUTUS).apply()
            bersihkan(ctx)
        }

        fun tandaiTerputus(ctx: Context) {
            val p = prefs(ctx)
            if (p.getLong(KUNCI_TERPUTUS, 0L) == 0L) p.edit().putLong(KUNCI_TERPUTUS, System.currentTimeMillis()).apply()
        }

        private fun bersihkan(ctx: Context) {
            prefs(ctx).edit().remove(KUNCI_BURUK).apply()
            ctx.getSystemService(NotificationManager::class.java).cancel(ID_PERINGATAN)
        }

        private fun jam(ms: Long): String =
            SimpleDateFormat("HH:mm", Locale.US).apply { timeZone = TimeZone.getTimeZone("Asia/Jakarta") }.format(Date(ms))

        private fun periksa(ctx: Context) {
            val p = prefs(ctx)
            if (!p.getBoolean(PantauBersama.PREF_AKTIF, false)) {
                bersihkan(ctx) // pantauan memang dimatikan pengguna: rantai berhenti
                return
            }
            val sekarang = System.currentTimeMillis()
            val jalan = PantauBersama.pemantau.sedangJalan()
            val terputusSejak = p.getLong(KUNCI_TERPUTUS, 0L)
            val putusLama = jalan && terputusSejak > 0L && sekarang - terputusSejak >= PUTUS_LAMA_MS
            if (jalan && !putusLama) {
                bersihkan(ctx)
                jadwalkan(ctx)
                return
            }
            val burukSejak = p.getLong(KUNCI_BURUK, 0L).takeIf { it > 0L }
                ?: sekarang.also { p.edit().putLong(KUNCI_BURUK, it).apply() }
            if (sekarang - burukSejak >= BURUK_BERTAHAN_MS &&
                sekarang - p.getLong(KUNCI_PERINGATAN, 0L) >= JEDA_PERINGATAN_MS
            ) {
                val pesan = if (!jalan) {
                    "Pantauan mati. Notifikasi order tidak akan muncul. Buka aplikasi untuk menyalakan lagi."
                } else {
                    "Pantauan terputus sejak ${jam(terputusSejak)}. Notifikasi order bisa terlambat."
                }
                LogDiag.catat("Penjaga: $pesan")
                p.edit().putLong(KUNCI_PERINGATAN, sekarang).apply()
                kirimPeringatan(ctx, pesan)
            }
            jadwalkan(ctx)
        }

        private fun kirimPeringatan(ctx: Context, pesan: String) {
            val nm = ctx.getSystemService(NotificationManager::class.java)
            nm.createNotificationChannel(
                NotificationChannel(SALURAN_PERINGATAN, "Peringatan pantauan", NotificationManager.IMPORTANCE_HIGH)
                    .apply { description = "Muncul bila pantauan order mati atau terputus lama." }
            )
            val buka = PendingIntent.getActivity(
                ctx, 4,
                Intent(ctx, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP),
                PendingIntent.FLAG_IMMUTABLE,
            )
            val n = Notification.Builder(ctx, SALURAN_PERINGATAN)
                .setSmallIcon(android.R.drawable.stat_notify_error)
                .setContentTitle("Catatan Scalping")
                .setContentText(pesan)
                .setStyle(Notification.BigTextStyle().bigText(pesan))
                .setAutoCancel(true)
                .setContentIntent(buka)
                .build()
            nm.notify(ID_PERINGATAN, n)
        }
    }
}
