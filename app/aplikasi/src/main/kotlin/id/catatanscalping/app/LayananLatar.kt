package id.catatanscalping.app

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.graphics.drawable.Icon
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import uniffi.scalping_ffi.ErrorApp
import uniffi.scalping_ffi.IsiNotifOrder
import uniffi.scalping_ffi.isiNotifikasi
import uniffi.scalping_ffi.ringkasanPosisiNotif
import uniffi.scalping_ffi.KejadianUi
import uniffi.scalping_ffi.Pemantau
import uniffi.scalping_ffi.PendengarPantau
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.TimeZone
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicInteger

/**
 * Status pantauan yang dipakai bersama oleh layanan dan layar.
 * Hanya penampung tampilan; semua logika pantauan ada di Rust (Pemantau).
 */
object PantauBersama {
    const val PREF = "pantau"
    const val PREF_AKTIF = "aktif"                 // pengguna ingin pantauan menyala
    const val PREF_PERNAH_SINKRON = "pernah_sinkron"

    /** Satu pengendali untuk seluruh proses. */
    val pemantau: Pemantau by lazy { Pemantau() }

    @Volatile var status: String = "Pantauan mati"
    /** Dipanggil (dari thread mana pun) saat status/log berubah; layar memindahkannya ke thread UI. */
    @Volatile var pendengarUi: (() -> Unit)? = null

    private val log = ArrayDeque<String>()
    private const val MAKS_LOG = 12

    @Synchronized
    fun catat(baris: String) {
        val f = SimpleDateFormat("HH:mm:ss", Locale.US).apply { timeZone = TimeZone.getTimeZone("Asia/Jakarta") }
        log.addFirst("${f.format(Date())}  $baris")
        while (log.size > MAKS_LOG) log.removeLast()
        pendengarUi?.invoke()
    }

    @Synchronized
    fun logTeks(): String = if (log.isEmpty()) "(belum ada kejadian)" else log.joinToString("\n")
}

/**
 * Foreground service dengan satu notifikasi tetap (FR-06). Hanya menjaga proses tetap hidup
 * dan meneruskan kejadian dari Rust; pencatatan trade dilakukan Rust (pantau).
 */
class LayananLatar : Service() {
    private val utama = Handler(Looper.getMainLooper())
    @Volatile private var hidup = false
    private var statusKoneksi = "Menghubungkan..."
    private var orderTerakhir: String? = null

    // Satu thread kerja: menyusun isi notifikasi memakai database, tanpa menahan thread pantau
    // dan menjaga urutan notifikasi sesuai urutan order.
    private val pekerja = Executors.newSingleThreadExecutor()

    // Callback dari thread Rust. Tidak boleh melempar exception ke Rust.
    private val penerima = object : PendengarPantau {
        override fun kejadian(k: KejadianUi) {
            try {
                tangani(k)
            } catch (e: Exception) {
                PantauBersama.catat("Galat penanganan kejadian: ${e.message}")
            }
        }
    }

    private val pathDb: String
        get() {
            val f = getDatabasePath("catatan.db") // sama dengan MainActivity
            f.parentFile?.mkdirs()
            return f.absolutePath
        }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        hidup = true
        val nm = getSystemService(NotificationManager::class.java)
        val saluran = NotificationChannel(SALURAN, "Pantauan Indodax", NotificationManager.IMPORTANCE_LOW)
        saluran.description = "Satu notifikasi tetap selama pantauan order berjalan. Boleh disenyapkan."
        nm.createNotificationChannel(saluran)
        // Saluran terpisah, prioritas tinggi, untuk notifikasi beli/jual (FR-15, FR-16)
        val saluranOrder = NotificationChannel(SALURAN_ORDER, "Order terisi", NotificationManager.IMPORTANCE_HIGH)
        saluranOrder.description = "Muncul saat order beli atau jual terisi: TP/SL atau hasil bersih."
        nm.createNotificationChannel(saluranOrder)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == AKSI_HENTI) {
            // Dari tombol di notifikasi atau layar. Dikirim lewat startService, jadi
            // tidak wajib memanggil startForeground.
            PantauBersama.catat("Pantauan dihentikan pengguna")
            tandaiAktif(false)
            PantauBersama.pemantau.berhenti()
            PantauBersama.status = "Pantauan mati"
            stopForeground(STOP_FOREGROUND_REMOVE)
            stopSelf()
            return START_NOT_STICKY
        }

        // Wajib segera setelah startForegroundService (batas 5 detik).
        statusKoneksi = "Menghubungkan..."
        PantauBersama.status = statusKoneksi
        tampilkanForeground()

        val kunci = KunciAman(this).ambil()
        if (kunci == null) {
            gagalMulai("Kunci API tidak ditemukan. Simpan kunci di aplikasi dulu.")
            return START_NOT_STICKY
        }
        mulaiDenganUlang(kunci.first, kunci.second, 0)
        // START_STICKY: bila sistem mematikan layanan, Android menghidupkannya lagi
        // (intent null) dan pantauan dimulai ulang beserta rekonsiliasi REST.
        return START_STICKY
    }

    /**
     * Pantauan lama bisa masih dalam proses berhenti (Rust menolak mulai ganda),
     * jadi coba lagi beberapa kali sebelum menyerah.
     */
    private fun mulaiDenganUlang(key: String, secret: String, percobaan: Int) {
        try {
            PantauBersama.pemantau.mulai(key, secret, pathDb, penerima)
            tandaiAktif(true)
            PantauBersama.catat("Pantauan dimulai")
        } catch (e: ErrorApp.Gagal) {
            if (percobaan < 5 && hidup) {
                utama.postDelayed({ if (hidup) mulaiDenganUlang(key, secret, percobaan + 1) }, 1500)
            } else {
                gagalMulai(e.pesan)
            }
        }
    }

    private fun gagalMulai(pesan: String) {
        PantauBersama.catat("Gagal memulai: $pesan")
        PantauBersama.status = "Gagal memulai: $pesan"
        tandaiAktif(false)
        notifGalat("Pantauan gagal dimulai: $pesan")
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    private fun tandaiAktif(ya: Boolean) {
        getSharedPreferences(PantauBersama.PREF, MODE_PRIVATE).edit().putBoolean(PantauBersama.PREF_AKTIF, ya).apply()
        PantauBersama.pendengarUi?.invoke()
    }

    private fun tangani(k: KejadianUi) {
        if (!hidup) return // layanan sudah dihentikan; jangan memunculkan notifikasi lagi
        when (k) {
            is KejadianUi.Tersambung -> {
                statusKoneksi = "Tersambung, memantau order"
                PantauBersama.catat("Tersambung")
            }
            is KejadianUi.Terputus -> {
                statusKoneksi = "Terputus (${k.alasan}), coba lagi ${k.cobaLagiDetik} dtk"
                PantauBersama.catat(statusKoneksi)
            }
            is KejadianUi.OrderSelesai -> {
                orderTerakhir = "${k.sisi.uppercase()} ${k.simbol} (trade baru ${k.tradeBaru})"
                PantauBersama.catat("Order selesai: $orderTerakhir")
                // trade_baru 0 = order ini sudah tercatat sebelumnya (contoh oleh rekonsiliasi): jangan notifikasi ganda
                if (k.tradeBaru > 0u) pekerja.execute { notifOrder(k.simbol, k.sisi) }
            }
            is KejadianUi.Rekonsiliasi -> {
                PantauBersama.catat("Rekonsiliasi REST: ${k.tradeBaru} trade baru")
                // Trade yang terlewat saat koneksi putus: koinnya tidak diketahui, tampilkan TP/SL semua posisi
                if (k.tradeBaru > 0u) pekerja.execute { notifRekonsiliasi(k.tradeBaru) }
            }
            is KejadianUi.Galat -> PantauBersama.catat("Galat: ${k.pesan}")
            is KejadianUi.Berhenti -> {
                val g = k.galat
                if (g == null) {
                    PantauBersama.catat("Pantauan berhenti")
                } else {
                    // Galat fatal (contoh: kunci dicabut). Pantau sudah mencoba menyambung ulang sendiri.
                    PantauBersama.catat("Pantauan berhenti: $g")
                    PantauBersama.status = "Berhenti: $g"
                    tandaiAktif(false)
                    notifGalat("Pantauan berhenti: $g")
                    stopForeground(STOP_FOREGROUND_REMOVE)
                    stopSelf()
                    return
                }
            }
        }
        PantauBersama.status = statusKoneksi
        perbaruiNotif()
    }

    // ---- notifikasi ----

    private fun buatNotif(): Notification {
        val buka = PendingIntent.getActivity(
            this, 0,
            Intent(this, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP),
            PendingIntent.FLAG_IMMUTABLE
        )
        val henti = PendingIntent.getService(
            this, 1,
            Intent(this, LayananLatar::class.java).setAction(AKSI_HENTI),
            PendingIntent.FLAG_IMMUTABLE
        )
        val rinci = statusKoneksi + (orderTerakhir?.let { "\nOrder terakhir: $it" } ?: "")
        return Notification.Builder(this, SALURAN)
            .setSmallIcon(android.R.drawable.stat_notify_sync)
            .setContentTitle("Catatan Scalping")
            .setContentText(statusKoneksi)
            .setStyle(Notification.BigTextStyle().bigText(rinci))
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setCategory(Notification.CATEGORY_SERVICE)
            .setContentIntent(buka)
            .addAction(
                Notification.Action.Builder(
                    Icon.createWithResource(this, android.R.drawable.ic_menu_close_clear_cancel),
                    "Hentikan", henti
                ).build()
            )
            .build()
    }

    private fun tampilkanForeground() {
        val n = buatNotif()
        if (Build.VERSION.SDK_INT >= 34) {
            startForeground(ID_NOTIF, n, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE)
        } else {
            startForeground(ID_NOTIF, n) // tipe diambil dari manifest
        }
    }

    private fun perbaruiNotif() {
        getSystemService(NotificationManager::class.java).notify(ID_NOTIF, buatNotif())
    }

    /** Notifikasi biasa (bisa digeser) untuk kegagalan, terpisah dari notifikasi layanan. */
    private fun notifGalat(pesan: String) {
        val n = Notification.Builder(this, SALURAN)
            .setSmallIcon(android.R.drawable.stat_notify_error)
            .setContentTitle("Catatan Scalping")
            .setContentText(pesan)
            .setStyle(Notification.BigTextStyle().bigText(pesan))
            .setAutoCancel(true)
            .setContentIntent(
                PendingIntent.getActivity(this, 2, Intent(this, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE)
            )
            .build()
        getSystemService(NotificationManager::class.java).notify(ID_GALAT, n)
    }

    // ---- notifikasi order (T1.5) ----

    private fun ambilIsi(buat: () -> IsiNotifOrder): IsiNotifOrder? {
        // Percobaan kedua setelah jeda singkat: database bisa sedang sibuk menulis dari pantau.
        for (percobaan in 0..1) {
            try {
                return buat()
            } catch (e: ErrorApp.Gagal) {
                if (percobaan == 1) {
                    PantauBersama.catat("Isi notifikasi gagal: ${e.pesan}")
                    return IsiNotifOrder("Order tercatat", e.pesan, e.pesan)
                }
                Thread.sleep(1500)
            } catch (e: Exception) {
                PantauBersama.catat("Isi notifikasi gagal: ${e.message}")
                return null
            }
        }
        return null
    }

    private fun notifOrder(simbol: String, sisi: String) {
        val isi = ambilIsi { isiNotifikasi(pathDb, simbol, sisi) } ?: return
        kirimNotifOrder(isi)
    }

    private fun notifRekonsiliasi(tradeBaru: UInt) {
        val isi = ambilIsi { ringkasanPosisiNotif(pathDb, tradeBaru) } ?: return
        kirimNotifOrder(isi)
    }

    /** Notifikasi biasa (bisa digeser), satu per order, terpisah dari notifikasi layanan. */
    private fun kirimNotifOrder(isi: IsiNotifOrder) {
        val n = Notification.Builder(this, SALURAN_ORDER)
            .setSmallIcon(android.R.drawable.stat_notify_sync)
            .setContentTitle(isi.judul)
            .setContentText(isi.teks)
            .setStyle(Notification.BigTextStyle().bigText(isi.rinci))
            .setCategory(Notification.CATEGORY_STATUS)
            .setAutoCancel(true)
            .setContentIntent(
                PendingIntent.getActivity(
                    this, 3,
                    Intent(this, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP),
                    PendingIntent.FLAG_IMMUTABLE
                )
            )
            .build()
        getSystemService(NotificationManager::class.java).notify(idOrder.getAndIncrement(), n)
    }

    override fun onDestroy() {
        hidup = false
        pekerja.shutdown() // tugas yang sudah antre tetap selesai
        utama.removeCallbacksAndMessages(null)
        // Bila dimatikan sistem, preferensi "aktif" tetap true supaya START_STICKY menyambung lagi.
        PantauBersama.pemantau.berhenti()
        super.onDestroy()
    }

    companion object {
        const val AKSI_HENTI = "id.catatanscalping.app.HENTI"
        private const val SALURAN = "pantau"
        private const val SALURAN_ORDER = "order"
        private val idOrder = AtomicInteger(2000) // satu ID per notifikasi order supaya bertumpuk, tidak saling timpa
        private const val ID_NOTIF = 1001
        private const val ID_GALAT = 1002

        /** Dipanggil dari layar. Catatan: startForegroundService hanya boleh dari app yang tampil. */
        fun mulai(ctx: Context) {
            ctx.getSharedPreferences(PantauBersama.PREF, Context.MODE_PRIVATE)
                .edit().putBoolean(PantauBersama.PREF_AKTIF, true).apply()
            ctx.startForegroundService(Intent(ctx, LayananLatar::class.java))
        }

        fun henti(ctx: Context) {
            ctx.getSharedPreferences(PantauBersama.PREF, Context.MODE_PRIVATE)
                .edit().putBoolean(PantauBersama.PREF_AKTIF, false).apply()
            ctx.startService(Intent(ctx, LayananLatar::class.java).setAction(AKSI_HENTI))
        }
    }
}
