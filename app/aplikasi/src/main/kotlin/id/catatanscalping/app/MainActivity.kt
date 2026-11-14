package id.catatanscalping.app

import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Typeface
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.Gravity
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.WindowInsets
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView

/**
 * Kerangka aplikasi: lima tab (Beranda, Posisi, Riwayat, Setelan, Screener) dengan navigasi bawah seperti prototype.
 * Hanya tampilan; semua logika ada di Rust.
 */
class MainActivity : Activity() {
    lateinit var tema: Tema
        private set
    lateinit var ui: Ui
        private set
    lateinit var kunci: KunciAman
        private set

    val prefs get() = getSharedPreferences(PantauBersama.PREF, MODE_PRIVATE)
    val pathDb: String get() = Sinkron.pathDb(this)

    private val namaTab = listOf("Beranda", "Posisi", "Riwayat", "Setelan", "Screener")
    private lateinit var layar: List<Layar>
    private val tombolNav = ArrayList<TextView>()
    private var aktif = 0
    private val utama = Handler(Looper.getMainLooper())
    private val segarkanTunda = Runnable { layar[aktif].segarkan() }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // Edge-to-edge di semua versi (targetSdk 36 memaksa di Android 15+); jarak diatur sendiri di bawah.
        window.setDecorFitsSystemWindows(false)
        tema = Tema.dari(this)
        ui = Ui(this, tema)
        kunci = KunciAman(this)

        layar = listOf(LayarBeranda(this), LayarPosisi(this), LayarRiwayat(this), LayarSetelan(this), LayarScreener(this))
        val konten = FrameLayout(this)
        for (l in layar) konten.addView(l.root, FrameLayout.LayoutParams(MATCH_PARENT, MATCH_PARENT))

        val baris = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(8), dp(4), dp(8), dp(4))
        }
        for ((i, nama) in namaTab.withIndex()) {
            val b = TextView(this).apply {
                text = nama
                textSize = 12f
                gravity = Gravity.CENTER
                setPadding(0, dp(12), 0, dp(12))
                setOnClickListener { pilih(i) }
            }
            tombolNav.add(b)
            baris.tambah(b, bobot = 1f, lebar = 0)
        }
        val nav = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(tema.card)
            tambah(View(this@MainActivity).apply { setBackgroundColor(tema.line) }, tinggi = dp(1))
            tambah(baris)
        }

        val akar = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(tema.bg)
            tambah(konten, bobot = 1f, tinggi = 0)
            tambah(nav)
            setOnApplyWindowInsetsListener { v, insets ->
                val b = insets.getInsets(WindowInsets.Type.systemBars() or WindowInsets.Type.ime())
                v.setPadding(b.left, b.top, b.right, b.bottom)
                // Papan ketik terbuka: sembunyikan navigasi supaya tidak ikut terdorong ke atas.
                nav.visibility = if (insets.isVisible(WindowInsets.Type.ime())) View.GONE else View.VISIBLE
                insets
            }
        }
        setContentView(akar)

        // Pertama kali (belum ada kunci) langsung ke Setelan.
        // (data dimuat di onResume, jadi tidak dimuat dua kali)
        pilih(savedInstanceState?.getInt(KUNCI_TAB) ?: if (kunci.ujungKey() == null) 3 else 0, muat = false)
    }

    private fun pilih(i: Int, muat: Boolean = true) {
        val ganti = i != aktif || !layar[i].root.isShown
        aktif = i
        for ((n, l) in layar.withIndex()) l.root.visibility = if (n == i) View.VISIBLE else View.GONE
        for ((n, b) in tombolNav.withIndex()) {
            b.setTextColor(if (n == i) tema.accent else tema.mute)
            b.typeface = Typeface.create(Typeface.DEFAULT, if (n == i) 700 else 500, false)
        }
        if (ganti) layar[i].root.scrollTo(0, 0)
        if (muat) layar[i].segarkan()
    }

    /** Dipanggil setelah data berubah (sinkron selesai, setelan disimpan, dan seterusnya). */
    fun dataBerubah() {
        layar[aktif].segarkan()
    }

    override fun onResume() {
        super.onResume()
        // Layanan memberi tahu lewat callback ini (dari thread mana pun); status diperbarui seketika,
        // data layar aktif dimuat ulang setelah jeda singkat supaya beruntun tidak membebani.
        PantauBersama.pendengarUi = {
            runOnUiThread {
                for (l in layar) l.pantauBerubah()
                utama.removeCallbacks(segarkanTunda)
                utama.postDelayed(segarkanTunda, 1200)
            }
        }
        layar[aktif].segarkan()
        Sinkron.otomatis(this) { dataBerubah() }
    }

    override fun onPause() {
        PantauBersama.pendengarUi = null
        utama.removeCallbacks(segarkanTunda)
        super.onPause()
    }

    override fun onSaveInstanceState(outState: Bundle) {
        super.onSaveInstanceState(outState)
        outState.putInt(KUNCI_TAB, aktif)
    }

    override fun onRequestPermissionsResult(requestCode: Int, permissions: Array<out String>, grantResults: IntArray) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode == KODE_NOTIF) {
            (layar[TAB_SETELAN] as LayarSetelan)
                .izinNotifSelesai(grantResults.firstOrNull() == PackageManager.PERMISSION_GRANTED)
        }
    }

    /** U5: hasil pemilih berkas sistem (ekspor dan impor cadangan). */
    @Suppress("DEPRECATION")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (resultCode != RESULT_OK) return
        val uri = data?.data ?: return
        val setelan = layar[TAB_SETELAN] as LayarSetelan
        when (requestCode) {
            Cadangan.KODE_EKSPOR -> setelan.cadanganEkspor(uri)
            Cadangan.KODE_IMPOR -> setelan.cadanganImpor(uri)
        }
    }

    companion object {
        const val KODE_NOTIF = 7
        const val TAB_SETELAN = 3
        private const val KUNCI_TAB = "tab"
    }
}
