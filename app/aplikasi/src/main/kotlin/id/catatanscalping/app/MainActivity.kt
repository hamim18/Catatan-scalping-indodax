package id.catatanscalping.app

import android.Manifest
import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.PowerManager
import android.provider.Settings
import android.os.Bundle
import android.text.InputType
import android.view.View
import android.view.WindowInsets
import android.widget.Button
import android.widget.CheckBox
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import uniffi.scalping_ffi.ErrorApp
import uniffi.scalping_ffi.StatusKoneksi
import uniffi.scalping_ffi.cekKoneksi
import uniffi.scalping_ffi.hitungPosisi
import uniffi.scalping_ffi.simpanSetelan
import uniffi.scalping_ffi.HasilSinkronUi
import uniffi.scalping_ffi.orderTerbaru
import uniffi.scalping_ffi.sinkronSekarang
import uniffi.scalping_ffi.versiCore
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.TimeZone

/**
 * Layar koneksi (FR-01, FR-02). Hanya tampilan; semua logika ada di Rust.
 */
class MainActivity : Activity() {
    private lateinit var kunci: KunciAman
    private lateinit var status: TextView
    private lateinit var info: TextView
    private lateinit var isiKey: EditText
    private lateinit var isiSecret: EditText
    private lateinit var tombolSimpan: Button
    private lateinit var tombolTes: Button
    private lateinit var tombolHapus: Button
    private lateinit var tombolSinkron: Button
    private lateinit var tombolOrder: Button
    private lateinit var isiOrder: TextView
    private lateinit var isiTp: EditText
    private lateinit var isiSl: EditText
    private lateinit var cekSemua: CheckBox
    private lateinit var tombolHitung: Button
    private lateinit var isiPosisi: TextView
    private lateinit var tombolPantau: Button
    private lateinit var tombolBaterai: Button
    private lateinit var isiPantau: TextView

    // true selama sinkron (manual atau otomatis) berjalan
    @Volatile private var sinkronBerjalan = false

    private val prefs get() = getSharedPreferences(PantauBersama.PREF, MODE_PRIVATE)

    private val pathDb: String
        get() {
            val f = getDatabasePath("catatan.db")
            f.parentFile?.mkdirs() // folder databases/ belum tentu ada
            return f.absolutePath
        }

    private fun dp(n: Int) = (n * resources.displayMetrics.density).toInt()

    private fun teks(isi: String, ukuran: Float, tebal: Boolean = false) = TextView(this).apply {
        text = isi
        textSize = ukuran
        if (tebal) setTypeface(typeface, android.graphics.Typeface.BOLD)
        setPadding(0, dp(6), 0, dp(6))
    }

    private fun angka(petunjuk: String, awal: String) = EditText(this).apply {
        hint = petunjuk
        setText(awal)
        inputType = InputType.TYPE_CLASS_NUMBER or InputType.TYPE_NUMBER_FLAG_DECIMAL
        setSingleLine()
    }

    private fun kolom(petunjuk: String) = EditText(this).apply {
        hint = petunjuk
        inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD
        importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_NO
        setSingleLine()
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        kunci = KunciAman(this)

        val isi = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(20), dp(20), dp(20))
        }
        isi.addView(teks("Catatan Scalping", 24f, tebal = true))
        status = teks("", 15f, tebal = true)
        isi.addView(status)
        isi.addView(teks("Aplikasi hanya membaca data. Gunakan key TAPI v2 dengan izin BACA SAJA.", 13f))
        isiKey = kolom("API key")
        isiSecret = kolom("API secret")
        isi.addView(isiKey)
        isi.addView(isiSecret)
        tombolSimpan = Button(this).apply { text = "Tes dan simpan kunci"; setOnClickListener { simpanBaru() } }
        tombolTes = Button(this).apply { text = "Tes koneksi (kunci tersimpan)"; setOnClickListener { tesTersimpan() } }
        tombolHapus = Button(this).apply { text = "Hapus kunci"; setOnClickListener { hapus() } }
        isi.addView(tombolSimpan)
        isi.addView(tombolTes)
        isi.addView(tombolHapus)
        tombolSinkron = Button(this).apply { text = "Sinkron sekarang (T1.2)"; setOnClickListener { sinkron() } }
        tombolOrder = Button(this).apply { text = "Lihat order tersimpan"; setOnClickListener { tampilkanOrder() } }
        isi.addView(tombolSinkron)
        isi.addView(tombolOrder)
        isiOrder = teks("", 13f)
        isiOrder.typeface = android.graphics.Typeface.MONOSPACE
        isi.addView(isiOrder)
        isi.addView(teks("Pantauan realtime (T1.4c)", 16f, tebal = true))
        isi.addView(teks("Menjaga koneksi Indodax di latar belakang agar trade tercatat otomatis.", 13f))
        tombolPantau = Button(this).apply { setOnClickListener { togglePantau() } }
        tombolBaterai = Button(this).apply { setOnClickListener { mintaAbaikanBaterai() } }
        isi.addView(tombolPantau)
        isi.addView(tombolBaterai)
        isiPantau = teks("", 12f)
        isiPantau.typeface = android.graphics.Typeface.MONOSPACE
        isi.addView(isiPantau)
        isi.addView(teks("Posisi dan TP/SL (T1.3)", 16f, tebal = true))
        isiTp = angka("Take profit % (contoh 1)", "1")
        isiSl = angka("Stop loss % (contoh 1)", "1")
        isi.addView(isiTp)
        isi.addView(isiSl)
        cekSemua = CheckBox(this).apply { text = "Hitung dari semua riwayat tersimpan (uji)" }
        isi.addView(cekSemua)
        tombolHitung = Button(this).apply { text = "Hitung posisi"; setOnClickListener { hitungPosisiUi() } }
        isi.addView(tombolHitung)
        isiPosisi = teks("", 13f)
        isiPosisi.typeface = android.graphics.Typeface.MONOSPACE
        isi.addView(isiPosisi)
        info = teks("", 14f)
        isi.addView(info)
        isi.addView(teks("Inti Rust versi ${versiCore()}", 12f))

        val gulir = ScrollView(this).apply {
            addView(isi)
            // targetSdk 36: layar digambar di bawah bilah sistem, beri jarak sendiri
            setOnApplyWindowInsetsListener { v, insets ->
                val b = insets.getInsets(WindowInsets.Type.systemBars())
                v.setPadding(b.left, b.top, b.right, b.bottom)
                insets
            }
        }
        setContentView(gulir)
        segarkan()
    }

    private fun segarkan() {
        val ujung = kunci.ujungKey()
        status.text = if (ujung != null) "Kunci tersimpan (berakhiran $ujung)" else "Belum ada kunci"
        tombolTes.isEnabled = ujung != null
        tombolHapus.isEnabled = ujung != null
        tombolSinkron.isEnabled = bolehSinkron()
        segarkanPantau()
    }

    // Sinkron manual dimatikan saat pantauan berjalan: pantau sudah merekonsiliasi sendiri,
    // dan dua koneksi SQLite menulis bersamaan bisa saling mengunci.
    private fun bolehSinkron() = kunci.ujungKey() != null && !pantauAktif() && !sinkronBerjalan

    private fun pantauAktif() = prefs.getBoolean(PantauBersama.PREF_AKTIF, false) || PantauBersama.pemantau.sedangJalan()

    private fun abaikanBaterai() =
        getSystemService(PowerManager::class.java).isIgnoringBatteryOptimizations(packageName)

    private fun segarkanPantau() {
        val aktif = pantauAktif()
        tombolPantau.text = if (aktif) "Hentikan pantauan realtime" else "Mulai pantauan realtime"
        tombolPantau.isEnabled = kunci.ujungKey() != null
        tombolBaterai.text =
            if (abaikanBaterai()) "Optimasi baterai: sudah dimatikan untuk app ini" else "Matikan optimasi baterai untuk app ini"
        tombolBaterai.isEnabled = !abaikanBaterai()
        tombolSinkron.text = if (aktif) "Sinkron manual (matikan pantauan dulu)" else "Sinkron sekarang (T1.2)"
        tombolSinkron.isEnabled = bolehSinkron()
        isiPantau.text = PantauBersama.status + "\n\n" + PantauBersama.logTeks()
    }

    override fun onResume() {
        super.onResume()
        // Layanan memberi tahu lewat callback ini; pindahkan ke thread UI.
        PantauBersama.pendengarUi = { runOnUiThread { segarkanPantau() } }
        segarkan()
        sinkronOtomatis()
    }

    override fun onPause() {
        PantauBersama.pendengarUi = null
        super.onPause()
    }

    private fun togglePantau() {
        if (pantauAktif()) {
            LayananLatar.henti(this)
            info.text = "Pantauan dihentikan."
            segarkan()
            return
        }
        if (kunci.ambil() == null) {
            info.text = "Kunci tersimpan tidak bisa dibuka. Masukkan ulang."
            return
        }
        // Snapshot saldo awal dibuat oleh sinkron pertama; pantauan sebelum itu membuat data tidak konsisten.
        if (!prefs.getBoolean(PantauBersama.PREF_PERNAH_SINKRON, false)) {
            info.text = "Jalankan \"Sinkron sekarang\" satu kali dulu, baru mulai pantauan."
            return
        }
        if (Build.VERSION.SDK_INT >= 33 &&
            checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
        ) {
            requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), KODE_NOTIF)
            return
        }
        mulaiPantau()
    }

    private fun mulaiPantau() {
        LayananLatar.mulai(this)
        info.text = "Memulai pantauan..."
        segarkan()
    }

    override fun onRequestPermissionsResult(requestCode: Int, permissions: Array<out String>, grantResults: IntArray) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode == KODE_NOTIF) {
            if (grantResults.firstOrNull() != PackageManager.PERMISSION_GRANTED) {
                info.text = "Izin notifikasi ditolak: pantauan tetap jalan, tapi notifikasinya tidak terlihat."
            }
            mulaiPantau()
        }
    }

    private fun mintaAbaikanBaterai() {
        if (abaikanBaterai()) return
        try {
            startActivity(
                Intent(Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS, Uri.parse("package:$packageName"))
            )
        } catch (e: Exception) {
            // Sebagian HP tidak punya dialog ini; buka daftar pengaturan baterai.
            startActivity(Intent(Settings.ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS))
        }
    }

    /** FR-10: tiap aplikasi dibuka, lengkapi trade yang terlewat (hanya bila pantauan mati). */
    private fun sinkronOtomatis() {
        val sekarang = System.currentTimeMillis()
        if (kunci.ujungKey() == null || pantauAktif() || sinkronBerjalan) return
        if (!prefs.getBoolean(PantauBersama.PREF_PERNAH_SINKRON, false)) return
        if (sekarang - terakhirOtomatisMs < 60_000) return // jangan berulang saat layar bolak-balik
        terakhirOtomatisMs = sekarang
        sinkron(otomatis = true)
    }

    private companion object {
        const val KODE_NOTIF = 7
        @Volatile var terakhirOtomatisMs = 0L
    }

    private fun sibuk(ya: Boolean) {
        tombolSimpan.isEnabled = !ya
        tombolTes.isEnabled = !ya && kunci.ujungKey() != null
        tombolHapus.isEnabled = !ya && kunci.ujungKey() != null
        tombolSinkron.isEnabled = !ya && bolehSinkron()
        if (ya) info.text = "Menghubungi Indodax..."
    }

    private fun jalankanTes(k: String, s: String, simpanBila: Boolean) {
        sibuk(true)
        Thread {
            try {
                val hasil = cekKoneksi(k, s)
                runOnUiThread { selesai(hasil, k, s, simpanBila) }
            } catch (e: Exception) {
                val pesan = (e as? ErrorApp.Gagal)?.pesan ?: (e.message ?: "Gagal")
                runOnUiThread { sibuk(false); info.text = "Gagal: $pesan" }
            }
        }.start()
    }

    private fun selesai(h: StatusKoneksi, k: String, s: String, simpanBila: Boolean) {
        sibuk(false)
        val baris = StringBuilder()
        baris.append("Tersambung. Jenis akun: ${h.jenisAkun}\n")
        baris.append("Jumlah aset di daftar saldo: ${h.jumlahAset}\n")
        baris.append("Aset IDR ada: ${if (h.idrAda) "ya" else "TIDAK"}\n")
        if (h.bacaSaja) {
            baris.append("Izin key: baca saja (benar).")
            if (simpanBila) {
                kunci.simpan(k, s)
                isiKey.setText("")
                isiSecret.setText("")
                baris.append("\nKunci disimpan terenkripsi.")
            }
        } else {
            baris.append("PERINGATAN: key ini punya izin trade/withdraw. Kunci TIDAK disimpan. Buat key baru dengan izin BACA SAJA.")
        }
        info.text = baris.toString()
        segarkan()
    }

    private fun simpanBaru() {
        val k = isiKey.text.toString().trim()
        val s = isiSecret.text.toString().trim()
        if (k.isEmpty() || s.isEmpty()) {
            info.text = "Isi API key dan API secret dulu."
            return
        }
        jalankanTes(k, s, simpanBila = true)
    }

    private fun tesTersimpan() {
        val p = kunci.ambil()
        if (p == null) {
            info.text = "Kunci tersimpan tidak bisa dibuka. Masukkan ulang."
            return
        }
        jalankanTes(p.first, p.second, simpanBila = false)
    }

    private fun waktuWib(ms: Long): String {
        val f = SimpleDateFormat("dd MMM HH:mm", Locale("id", "ID"))
        f.timeZone = TimeZone.getTimeZone("Asia/Jakarta")
        return f.format(Date(ms))
    }

    private fun sinkron(otomatis: Boolean = false) {
        val p = kunci.ambil()
        if (p == null) {
            if (!otomatis) info.text = "Kunci tersimpan tidak bisa dibuka. Masukkan ulang."
            return
        }
        sinkronBerjalan = true
        if (otomatis) {
            tombolSinkron.isEnabled = false
            info.text = "Sinkron otomatis berjalan..."
        } else {
            sibuk(true)
            info.text = "Sinkron berjalan, bisa beberapa puluh detik..."
        }
        Thread {
            try {
                val h: HasilSinkronUi = sinkronSekarang(p.first, p.second, pathDb, 7u, emptyList())
                prefs.edit().putBoolean(PantauBersama.PREF_PERNAH_SINKRON, true).apply()
                val ringkas = StringBuilder()
                ringkas.append(if (otomatis) "Sinkron otomatis selesai.\n" else "Sinkron selesai.\n")
                ringkas.append("Koin dipindai: ${h.koinDipindai.joinToString(", ").ifEmpty { "(tidak ada)" }}\n")
                ringkas.append("Trade baru: ${h.tradeBaru}, total tersimpan: ${h.tradeTotal}\n")
                if (h.koinDilewati.isNotEmpty()) ringkas.append("Dilewati (simbol tidak dikenal Indodax): ${h.koinDilewati.joinToString(", ")}\n")
                if (h.snapshotAwalDibuat) ringkas.append("Snapshot saldo awal dibuat.")
                runOnUiThread {
                    sinkronBerjalan = false
                    sibuk(false)
                    info.text = ringkas.toString()
                    tampilkanOrder()
                    segarkan()
                }
            } catch (e: Exception) {
                val pesan = (e as? ErrorApp.Gagal)?.pesan ?: (e.message ?: "Gagal")
                runOnUiThread {
                    sinkronBerjalan = false
                    sibuk(false)
                    info.text = "Sinkron gagal: $pesan"
                    segarkan()
                }
            }
        }.start()
    }

    private fun tampilkanOrder() {
        Thread {
            try {
                val daftar = orderTerbaru(pathDb, 15u)
                val t = StringBuilder()
                if (daftar.isEmpty()) t.append("Belum ada order tersimpan.")
                for (o in daftar) {
                    t.append("${waktuWib(o.waktuMs)}  ${o.sisi.uppercase()} ${o.simbol}\n")
                    t.append("   koin ${o.koin}  Rp ${o.idr}  (${o.pengisian}x isi)\n")
                }
                runOnUiThread { isiOrder.text = t.toString() }
            } catch (e: Exception) {
                val pesan = (e as? ErrorApp.Gagal)?.pesan ?: (e.message ?: "Gagal")
                runOnUiThread { isiOrder.text = "Gagal membaca order: $pesan" }
            }
        }.start()
    }

    private fun hitungPosisiUi() {
        val tp = isiTp.text.toString()
        val sl = isiSl.text.toString()
        val semua = cekSemua.isChecked
        Thread {
            try {
                simpanSetelan(pathDb, tp, sl)
                val h = hitungPosisi(pathDb, semua)
                val t = StringBuilder()
                t.append("Dihitung sejak: ${if (h.sejakMs == 0L) "semua riwayat" else waktuWib(h.sejakMs)}\n\n")
                t.append("POSISI TERBUKA (${h.posisi.size})\n")
                if (h.posisi.isEmpty()) t.append("  (tidak ada)\n")
                for (p in h.posisi) {
                    t.append("${p.simbol}  koin ${p.koin}  buka ${waktuWib(p.waktuBukaMs)}\n")
                    t.append("  modal efektif ${p.modalEfektif}  (total Rp ${p.totalModal})\n")
                    t.append("  TP ${p.hargaTp} (+${p.tpPersen}%)   SL ${p.hargaSl} (-${p.slPersen}%)\n")
                }
                t.append("\nTERTUTUP (${h.tertutup.size})  total Rp ${h.totalProfit}\n")
                for (x in h.tertutup.take(10)) {
                    t.append("${waktuWib(x.waktuTutupMs)} ${x.simbol} koin ${x.koin}\n")
                    t.append("  modal ${x.modal} diterima ${x.diterima}  hasil ${x.profit} (${x.persen}%)\n")
                }
                if (h.tertutup.size > 10) t.append("  ... dan ${h.tertutup.size - 10} lagi\n")
                t.append("\nPenyesuaian (jual tanpa modal): ${h.jumlahPenyesuaian}")
                runOnUiThread { isiPosisi.text = t.toString() }
            } catch (e: Exception) {
                val pesan = (e as? ErrorApp.Gagal)?.pesan ?: (e.message ?: "Gagal")
                runOnUiThread { isiPosisi.text = "Gagal menghitung: $pesan" }
            }
        }.start()
    }

    private fun hapus() {
        if (pantauAktif()) LayananLatar.henti(this) // tanpa kunci pantauan tidak bisa jalan
        kunci.hapus()
        info.text = "Kunci dihapus dari perangkat."
        segarkan()
    }
}
