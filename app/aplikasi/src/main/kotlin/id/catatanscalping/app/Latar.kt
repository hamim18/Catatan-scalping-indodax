package id.catatanscalping.app

import android.os.Handler
import android.os.Looper
import uniffi.scalping_ffi.ErrorApp
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

/**
 * Menjalankan panggilan Rust (memblokir) di luar thread UI.
 * - `baca`: antrean satu thread untuk membaca database lokal. Berurutan, jadi hasil tiba sesuai urutan
 *   permintaan dan antar-panggilan tidak saling berebut kunci SQLite.
 * - `jaringan`: untuk panggilan yang menghubungi Indodax (bisa puluhan detik), tidak menahan antrean baca.
 */
object Latar {
    private val utama = Handler(Looper.getMainLooper())
    private val antreanBaca: ExecutorService = Executors.newSingleThreadExecutor()
    val jaringan: ExecutorService = Executors.newCachedThreadPool()

    /** Jalankan di thread UI (langsung bila sudah di sana). */
    fun ui(blok: () -> Unit) {
        if (Looper.myLooper() == Looper.getMainLooper()) blok() else utama.post { blok() }
    }

    /** `kerja` jalan di thread latar; `hasil` dipanggil di thread UI. */
    fun <T> baca(kerja: () -> T, hasil: (Result<T>) -> Unit) {
        antreanBaca.execute {
            val r = runCatching(kerja)
            utama.post { hasil(r) }
        }
    }

    fun pesan(e: Throwable): String = (e as? ErrorApp.Gagal)?.pesan ?: (e.message ?: "Gagal")
}
