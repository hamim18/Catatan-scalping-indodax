package id.catatanscalping.app

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

/**
 * T5.1: menyalakan lagi pantauan setelah HP restart.
 * Hanya bila pengguna terakhir meninggalkan pantauan dalam keadaan MENYALA (preferensi "aktif" = true).
 * Bila pengguna menekan "Hentikan pantauan", preferensi itu false dan pantauan tetap mati.
 * Android 15 melarang beberapa tipe foreground service dimulai dari BOOT_COMPLETED
 * (dataSync, mediaPlayback, dan lain-lain); tipe specialUse yang dipakai LayananLatar tidak termasuk.
 */
class PenerimaBoot : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent?) {
        if (intent?.action != Intent.ACTION_BOOT_COMPLETED) return
        val app = context.applicationContext
        val prefs = app.getSharedPreferences(PantauBersama.PREF, Context.MODE_PRIVATE)
        if (!prefs.getBoolean(PantauBersama.PREF_AKTIF, false)) return

        fun matikanPenanda(alasan: String) {
            // Supaya layar tidak menampilkan "Berjalan" padahal layanan tidak hidup.
            prefs.edit().putBoolean(PantauBersama.PREF_AKTIF, false).apply()
            PantauBersama.status = "Pantauan mati"
            PantauBersama.catat("Pantauan tidak menyala otomatis: $alasan")
        }

        if (KunciAman(app).ujungKey() == null) {
            matikanPenanda("kunci API tidak ada")
            return
        }
        if (!prefs.getBoolean(PantauBersama.PREF_PERNAH_SINKRON, false)) {
            matikanPenanda("belum pernah sinkron")
            return
        }
        if (PantauBersama.pemantau.sedangJalan()) return
        try {
            LayananLatar.mulai(app)
            PantauBersama.catat("Pantauan dinyalakan otomatis setelah HP restart")
        } catch (e: Exception) {
            matikanPenanda(e.message ?: e.javaClass.simpleName)
        }
    }
}
