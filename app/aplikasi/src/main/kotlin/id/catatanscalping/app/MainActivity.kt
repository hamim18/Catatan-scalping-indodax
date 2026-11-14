package id.catatanscalping.app

import android.app.Activity
import android.app.AlertDialog
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Typeface
import android.os.Build
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
import android.window.OnBackInvokedCallback
import android.window.OnBackInvokedDispatcher

/**
 * Kerangka aplikasi: empat tab di navigasi bawah (Beranda, Posisi, Riwayat, Screener) dan menu ☰ di
 * pojok kanan atas tiap layar (Setelan, Panduan). Layar menu tidak punya tab; tombol kembali sistem
 * atau ‹ membawa pengguna ke tab sebelumnya.
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

    // Urutan layar: 0..3 = tab navigasi, 4 = Setelan, 5 = Panduan (dibuka dari menu ☰).
    private val namaTab = listOf("Beranda", "Posisi", "Riwayat", "Screener")
    private lateinit var layar: List<Layar>
    private val tombolNav = ArrayList<TextView>()
    private var aktif = 0
    private var tabSebelumMenu = 0
    private val utama = Handler(Looper.getMainLooper())
    private val segarkanTunda = Runnable { layar[aktif].segarkan() }

    private var callbackKembali: Any? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // Edge-to-edge di semua versi (targetSdk 36 memaksa di Android 15+); jarak diatur sendiri di bawah.
        window.setDecorFitsSystemWindows(false)
        tema = Tema.dari(this)
        ui = Ui(this, tema)
        kunci = KunciAman(this)

        layar = listOf(
            LayarBeranda(this), LayarPosisi(this), LayarRiwayat(this), LayarScreener(this),
            LayarSetelan(this), LayarPanduan(this),
        )
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
        val awal = savedInstanceState?.getInt(KUNCI_TAB) ?: if (kunci.ujungKey() == null) TAB_SETELAN else 0
        tabSebelumMenu = savedInstanceState?.getInt(KUNCI_TAB_SEBELUM) ?: 0
        pilih(awal, muat = false)
    }

    private fun pilih(i: Int, muat: Boolean = true) {
        val ganti = i != aktif || !layar[i].root.isShown
        aktif = i
        for ((n, l) in layar.withIndex()) l.root.visibility = if (n == i) View.VISIBLE else View.GONE
        for ((n, b) in tombolNav.withIndex()) {
            b.setTextColor(if (n == i) tema.accent else tema.mute)
            b.typeface = Typeface.create(Typeface.DEFAULT, if (n == i) 700 else 500, false)
        }
        if (ganti) layar[i].gulirKeAtas()
        aturTombolKembali(i >= TAB_SETELAN)
        if (muat) layar[i].segarkan()
    }

    /** Menu ☰: dipanggil dari header tiap layar tab. */
    fun bukaMenu() {
        val daftar = arrayOf("Setelan", "Panduan")
        AlertDialog.Builder(this)
            .setItems(daftar) { _, n ->
                if (aktif < TAB_SETELAN) tabSebelumMenu = aktif
                pilih(if (n == 0) TAB_SETELAN else TAB_PANDUAN)
            }
            .show()
    }

    /** Dari layar menu kembali ke tab terakhir. */
    fun kembaliDariMenu() {
        if (aktif >= TAB_SETELAN) pilih(tabSebelumMenu.coerceIn(0, namaTab.size - 1))
    }

    /** Pada Android 13+ tombol kembali predictif tidak lagi memanggil onBackPressed: daftarkan callback hanya di layar menu. */
    private fun aturTombolKembali(layarMenu: Boolean) {
        if (Build.VERSION.SDK_INT < 33) return
        val d = onBackInvokedDispatcher
        val lama = callbackKembali as? OnBackInvokedCallback
        if (layarMenu && lama == null) {
            val c = OnBackInvokedCallback { kembaliDariMenu() }
            callbackKembali = c
            d.registerOnBackInvokedCallback(OnBackInvokedDispatcher.PRIORITY_DEFAULT, c)
        } else if (!layarMenu && lama != null) {
            d.unregisterOnBackInvokedCallback(lama)
            callbackKembali = null
        }
    }

    @Suppress("DEPRECATION", "OVERRIDE_DEPRECATION")
    override fun onBackPressed() {
        if (aktif >= TAB_SETELAN) kembaliDariMenu() else super.onBackPressed()
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
        outState.putInt(KUNCI_TAB_SEBELUM, tabSebelumMenu)
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
        const val TAB_SETELAN = 4
        const val TAB_PANDUAN = 5
        private const val KUNCI_TAB = "tab"
        private const val KUNCI_TAB_SEBELUM = "tab_sebelum"
    }
}
