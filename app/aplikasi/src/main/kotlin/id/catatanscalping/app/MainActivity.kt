package id.catatanscalping.app

import android.app.Activity
import android.os.Bundle
import android.text.InputType
import android.view.View
import android.view.WindowInsets
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import uniffi.scalping_ffi.ErrorApp
import uniffi.scalping_ffi.StatusKoneksi
import uniffi.scalping_ffi.cekKoneksi
import uniffi.scalping_ffi.versiCore

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

    private fun dp(n: Int) = (n * resources.displayMetrics.density).toInt()

    private fun teks(isi: String, ukuran: Float, tebal: Boolean = false) = TextView(this).apply {
        text = isi
        textSize = ukuran
        if (tebal) setTypeface(typeface, android.graphics.Typeface.BOLD)
        setPadding(0, dp(6), 0, dp(6))
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
    }

    private fun sibuk(ya: Boolean) {
        tombolSimpan.isEnabled = !ya
        tombolTes.isEnabled = !ya && kunci.ujungKey() != null
        tombolHapus.isEnabled = !ya && kunci.ujungKey() != null
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

    private fun hapus() {
        kunci.hapus()
        info.text = "Kunci dihapus dari perangkat."
        segarkan()
    }
}
