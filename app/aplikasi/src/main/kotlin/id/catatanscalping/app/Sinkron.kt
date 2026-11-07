package id.catatanscalping.app

import android.content.Context
import uniffi.scalping_ffi.sinkronArusKas
import uniffi.scalping_ffi.sinkronSekarang

/**
 * Sinkron manual dan otomatis (FR-10), dipakai bersama oleh layar. Logikanya sama dengan layar uji lama:
 * sinkron trade hanya bila pantauan mati (cegah dua koneksi SQLite menulis bersamaan), arus kas
 * boleh kapan saja. Semua hitungan tetap di Rust; di sini hanya urutan pemanggilan dan pesan.
 */
object Sinkron {
    class Hasil(val berhasil: Boolean, val pesan: String)

    @Volatile var tradeBerjalan = false
    @Volatile var arusKasBerjalan = false
    @Volatile private var terakhirOtomatisMs = 0L
    @Volatile private var terakhirArusKasMs = 0L

    fun pathDb(ctx: Context): String {
        val f = ctx.getDatabasePath("catatan.db") // sama dengan LayananLatar
        f.parentFile?.mkdirs()
        return f.absolutePath
    }

    private fun prefs(ctx: Context) = ctx.getSharedPreferences(PantauBersama.PREF, Context.MODE_PRIVATE)

    fun pantauAktif(ctx: Context): Boolean =
        prefs(ctx).getBoolean(PantauBersama.PREF_AKTIF, false) || PantauBersama.pemantau.sedangJalan()

    fun pernahSinkron(ctx: Context): Boolean = prefs(ctx).getBoolean(PantauBersama.PREF_PERNAH_SINKRON, false)

    fun bolehSinkron(ctx: Context): Boolean =
        KunciAman(ctx).ujungKey() != null && !pantauAktif(ctx) && !tradeBerjalan

    /**
     * Sinkron trade (dan arus kas). Mengembalikan false bila tidak bisa dimulai (kunci tidak ada
     * atau sedang berjalan). `selesai` dipanggil di thread UI.
     */
    fun sinkronTrade(ctx: Context, otomatis: Boolean, selesai: (Hasil) -> Unit): Boolean {
        val app = ctx.applicationContext
        val kunci = KunciAman(app).ambil() ?: return false
        if (tradeBerjalan) return false
        tradeBerjalan = true
        val db = pathDb(app)
        Latar.jaringan.execute {
            val hasil = try {
                val h = sinkronSekarang(kunci.first, kunci.second, db, 7u, emptyList())
                prefs(app).edit().putBoolean(PantauBersama.PREF_PERNAH_SINKRON, true).apply()
                val t = StringBuilder()
                t.append(if (otomatis) "Sinkron otomatis selesai.\n" else "Sinkron selesai.\n")
                t.append("Koin dipindai: ${h.koinDipindai.joinToString(", ").ifEmpty { "(tidak ada)" }}\n")
                t.append("Trade baru: ${h.tradeBaru}, total tersimpan: ${h.tradeTotal}\n")
                if (h.koinDilewati.isNotEmpty()) {
                    t.append("Dilewati (simbol tidak dikenal Indodax): ${h.koinDilewati.joinToString(", ")}\n")
                }
                if (h.snapshotAwalDibuat) t.append("Snapshot saldo awal dibuat.\n")
                // Arus kas (deposit/withdraw): kegagalan di sini tidak membatalkan sinkron trade.
                if (!arusKasBerjalan) {
                    arusKasBerjalan = true
                    try {
                        val a = sinkronArusKas(kunci.first, kunci.second, db)
                        t.append("Arus kas: ${a.baru} baru, ${a.total} tersimpan.")
                    } catch (e: Exception) {
                        t.append("Arus kas gagal: ${Latar.pesan(e)}")
                    } finally {
                        arusKasBerjalan = false
                    }
                }
                Hasil(true, t.toString().trimEnd())
            } catch (e: Exception) {
                Hasil(false, "Sinkron gagal: ${Latar.pesan(e)}")
            }
            tradeBerjalan = false
            Latar.ui { selesai(hasil) }
        }
        return true
    }

    /** Sinkron deposit/withdraw IDR saja. Aman dijalankan saat pantauan hidup. */
    fun jalankanArusKas(ctx: Context, selesai: (Hasil) -> Unit): Boolean {
        val app = ctx.applicationContext
        val kunci = KunciAman(app).ambil() ?: return false
        if (arusKasBerjalan) return false
        arusKasBerjalan = true
        val db = pathDb(app)
        Latar.jaringan.execute {
            val hasil = try {
                val h = sinkronArusKas(kunci.first, kunci.second, db)
                Hasil(true, "Arus kas: ${h.baru} baru, ${h.total} tersimpan, ${h.dilewati} dilewati (bukan sukses/IDR).")
            } catch (e: Exception) {
                Hasil(false, "Arus kas gagal: ${Latar.pesan(e)}")
            }
            arusKasBerjalan = false
            Latar.ui { selesai(hasil) }
        }
        return true
    }

    /**
     * Dipanggil tiap layar dibuka (FR-10). Pantauan mati: sinkron trade (sekalian arus kas).
     * Pantauan hidup: hanya arus kas, karena pantau sudah merekonsiliasi trade sendiri.
     * Jeda minimal 60 detik supaya tidak berulang saat layar bolak-balik.
     */
    fun otomatis(ctx: Context, selesai: (Hasil) -> Unit) {
        val app = ctx.applicationContext
        if (KunciAman(app).ujungKey() == null || !pernahSinkron(app)) return
        val sekarang = System.currentTimeMillis()
        if (pantauAktif(app)) {
            if (arusKasBerjalan || sekarang - terakhirArusKasMs < 60_000) return
            terakhirArusKasMs = sekarang
            jalankanArusKas(app, selesai)
        } else {
            if (tradeBerjalan || sekarang - terakhirOtomatisMs < 60_000) return
            terakhirOtomatisMs = sekarang
            sinkronTrade(app, true, selesai)
        }
    }
}
