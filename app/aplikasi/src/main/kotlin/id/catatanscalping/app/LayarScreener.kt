package id.catatanscalping.app

import android.os.Handler
import android.os.Looper
import android.text.InputType
import android.view.Gravity
import android.view.View
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import uniffi.scalping_ffi.BarisLonjakanUi
import uniffi.scalping_ffi.BarisScreenerUi
import uniffi.scalping_ffi.HasilLonjakanUi
import uniffi.scalping_ffi.HasilScreenerUi
import uniffi.scalping_ffi.InfoGlobalUi
import uniffi.scalping_ffi.SetelanGlobalUi
import uniffi.scalping_ffi.SetelanLonjakanUi
import uniffi.scalping_ffi.SetelanScreenerUi
import uniffi.scalping_ffi.bacaSetelanGlobal
import uniffi.scalping_ffi.bacaSetelanLonjakan
import uniffi.scalping_ffi.bacaSetelanScreener
import uniffi.scalping_ffi.jalankanScreener
import uniffi.scalping_ffi.pindaiLonjakan
import uniffi.scalping_ffi.setelanGlobalBawaan
import uniffi.scalping_ffi.setelanLonjakanBawaan
import uniffi.scalping_ffi.setelanScreenerBawaan
import uniffi.scalping_ffi.simpanSetelanGlobal
import uniffi.scalping_ffi.simpanSetelanLonjakan
import uniffi.scalping_ffi.simpanSetelanScreener

/**
 * Screener (T9.2, T9.4 sampai T9.6; T10.3): dua bagian pada satu layar, tanpa saklar mode:
 * "Lonjakan" di atas (pair yang menyimpang dari rata-rata harga dasar) dan "Daftar" di bawah (pair yang
 * lolos kriteria). HANYA informasi, tidak memengaruhi posisi, profit, target, grafik, atau statistik.
 * Data publik tanpa key, jadi layar ini jalan walau kunci API belum diisi.
 * Penyegaran otomatis hanya saat layar terlihat (FR-30): tiap siklus memindai Lonjakan, dan Daftar
 * ikut dipindai bila sudah lewat jeda daftar (satu thread jaringan, berurutan). Tanpa layanan latar
 * dan tanpa notifikasi. Hanya tampilan; semua hitungan di Rust.
 *
 * Harga global (T9.5, T9.6): tiap baris memuat harga wajar dari Binance, Bybit, OKX, selisih Indodax,
 * dan arah harga N menit terakhir (kosong sampai data cukup). Baris tanpa pembanding tidak menampilkan
 * apa pun; status koneksi exchange hanya dirinci bila ada yang bermasalah.
 */
class LayarScreener(a: MainActivity) : Layar(a) {
    private val isiModal = ui.input("Modal trade ini (Rp)")
    private val statusLonjakan = ui.mute("", 12f)
    private val statusDaftar = ui.mute("", 12f)
    private val kartuLonjakan = ui.kartu()
    private val kartuDaftar = ui.kartu()

    // --- ambang (semua dari Rust; kosong = filter mati) ---
    private val fHargaMin = ui.input("mati")
    private val fHargaMaks = ui.input("mati")
    private val fVolMin = ui.input("mati")
    private val fVolMaks = ui.input("mati")
    private val fTrxMin = ui.input("mati")
    private val fJendela = ui.input("wajib")
    private val fSpread = ui.input("mati")
    private val fRange24 = ui.input("mati")
    private val fUsia = ui.input("mati")
    private val fGerak = ui.input("mati")
    private val fMenitAktif = ui.input("mati")
    private val fArah = ui.input("mati").apply { inputType = inputType or InputType.TYPE_NUMBER_FLAG_SIGNED }
    private val fKedKelipatan = ui.input("mati")
    private val fKedJarak = ui.input("wajib")
    private val fDominasi = ui.input("mati")
    private val fBiaya = ui.input("wajib")
    private var urutan = "skor"
    private lateinit var tombolUrutan: Button
    private val infoAmbang = ui.mute("", 12f)

    // --- ambang Lonjakan (T9.4) ---
    private val fLX = ui.input("wajib")
    private val fLM = ui.input("wajib")
    private val fLJendela = ui.input("wajib")
    private val fLDasar = ui.input("wajib")
    private val fLPemanasan = ui.input("wajib")
    private val fLMaks = ui.input("wajib")
    private val fLInterval = ui.input("wajib")
    private var lNaik = true
    private var lTurun = true
    private lateinit var tombolNaik: Button
    private lateinit var tombolTurun: Button
    private var intervalMs = 20_000L

    // --- ambang Harga global (T9.5) ---
    private val fGSebaran = ui.input("wajib")
    private val fGCuriga = ui.input("wajib")
    private val fGKurs = ui.input("wajib")
    private val fGInterval = ui.input("wajib")
    private val fGArahMenit = ui.input("wajib")
    private val fGArahAmbang = ui.input("wajib")
    private var gAktif = true
    private lateinit var tombolGlobal: Button
    private val statusGlobal = ui.mute("", 12f)

    // --- keadaan pemindaian ---
    private var hasil: HasilScreenerUi? = null
    private var galat: String? = null
    private var hasilL: HasilLonjakanUi? = null
    private var galatL: String? = null
    private var memuat = false
    private var mulaiMs = 0L
    private var daftarMs = 0L

    private val pengatur = Handler(Looper.getMainLooper())
    private var pernahTerpasang = false
    private val putaran = object : Runnable {
        override fun run() {
            if (root.isAttachedToWindow) {
                pernahTerpasang = true
            } else if (pernahTerpasang) {
                return // layar sudah dilepas: hentikan putaran
            }
            if (root.isShown && root.windowVisibility == View.VISIBLE) muat(jedaOtomatisMin())
            pengatur.postDelayed(this, intervalMs)
        }
    }

    init {
        header("Screener")

        val barisModal = LinearLayout(a).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            tambah(isiModal, bobot = 1f, lebar = 0)
            tambah(ui.tombol("Segarkan") { muat(JEDA_MANUAL_MIN_MS, paksaDaftar = true) }, kiri = 8, lebar = WRAP_CONTENT)
        }
        isi.tambah(barisModal)
        isi.tambah(statusGlobal, atas = 8)

        // --- Lonjakan ---
        isi.tambah(ui.judulBagian("Lonjakan"), atas = 12)
        isi.tambah(statusLonjakan, atas = 2, bawah = 6)
        isi.tambah(kartuLonjakan)

        // --- Daftar ---
        isi.tambah(ui.judulBagian("Daftar"), atas = 18)
        isi.tambah(statusDaftar, atas = 2, bawah = 6)
        isi.tambah(kartuDaftar)

        // --- kartu ambang (tersembunyi sampai judul diketuk; tiga bagian lipat) ---
        val kartuAmbang = ui.kartu()
        val judul = ui.teks("Ambang  ▸", 14f, tebal = true)
        val isiAmbang = LinearLayout(a).apply { orientation = LinearLayout.VERTICAL }
        isiAmbang.visibility = View.GONE
        judul.setOnClickListener {
            val buka = isiAmbang.visibility != View.VISIBLE
            isiAmbang.visibility = if (buka) View.VISIBLE else View.GONE
            judul.text = if (buka) "Ambang  ▾" else "Ambang  ▸"
        }
        kartuAmbang.tambah(judul)
        kartuAmbang.tambah(isiAmbang, atas = 6)

        val bagianDaftar = bagianLipat(isiAmbang, "Daftar", atas = 8)
        bagianDaftar.tambah(barisIsian("Harga minimum (Rp)", fHargaMin))
        bagianDaftar.tambah(barisIsian("Harga maksimum (Rp)", fHargaMaks))
        bagianDaftar.tambah(barisIsian("Volume 24 jam minimum (Rp)", fVolMin))
        bagianDaftar.tambah(barisIsian("Volume 24 jam maksimum (Rp)", fVolMaks))
        bagianDaftar.tambah(barisIsian("Trx/mnt minimum", fTrxMin))
        bagianDaftar.tambah(barisIsian("Jendela (menit)", fJendela))
        bagianDaftar.tambah(barisIsian("Spread maksimum (%)", fSpread))
        bagianDaftar.tambah(barisIsian("Range 24 jam minimum (%)", fRange24))
        bagianDaftar.tambah(barisIsian("Usia transaksi maksimum (detik)", fUsia))
        bagianDaftar.tambah(barisIsian("Gerak jendela minimum (%)", fGerak))
        bagianDaftar.tambah(barisIsian("Menit aktif minimum (%)", fMenitAktif))
        bagianDaftar.tambah(barisIsian("Arah jendela minimum (%)", fArah))
        bagianDaftar.tambah(barisIsian("Kedalaman: kelipatan modal", fKedKelipatan))
        bagianDaftar.tambah(barisIsian("Kedalaman: jarak (%)", fKedJarak))
        bagianDaftar.tambah(barisIsian("Tanda dominasi (%)", fDominasi))
        bagianDaftar.tambah(barisIsian("Biaya round-trip (%)", fBiaya))
        tombolUrutan = ui.tombol("Urutan: Skor") {
            urutan = if (urutan == "skor") "trx" else "skor"
            perbaruiTombolUrutan()
        }
        bagianDaftar.tambah(tombolUrutan, atas = 8)

        val bagianLonjakan = bagianLipat(isiAmbang, "Lonjakan", atas = 10)
        tombolNaik = ui.tombol("Naik ▲: ya") {
            lNaik = !lNaik
            perbaruiTombolArah()
        }
        tombolTurun = ui.tombol("Turun ▼: ya") {
            lTurun = !lTurun
            perbaruiTombolArah()
        }
        val barisArah = LinearLayout(a).apply {
            orientation = LinearLayout.HORIZONTAL
            tambah(tombolNaik, bobot = 1f, lebar = 0, kanan = 4)
            tambah(tombolTurun, bobot = 1f, lebar = 0, kiri = 4)
        }
        bagianLonjakan.tambah(barisArah, bawah = 4)
        bagianLonjakan.tambah(barisIsian("Lonjakan minimum (%)", fLX))
        bagianLonjakan.tambah(barisIsian("Trx/mnt minimum", fLM))
        bagianLonjakan.tambah(barisIsian("Jendela trx/mnt (menit)", fLJendela))
        bagianLonjakan.tambah(barisIsian("Rata-rata harga dasar (menit)", fLDasar))
        bagianLonjakan.tambah(barisIsian("Pemanasan minimum (menit)", fLPemanasan))
        bagianLonjakan.tambah(barisIsian("Maks pair diperiksa", fLMaks))
        bagianLonjakan.tambah(barisIsian("Interval (detik)", fLInterval))

        val bagianGlobal = bagianLipat(isiAmbang, "Harga global", atas = 10)
        tombolGlobal = ui.tombol("Harga global: ya") {
            gAktif = !gAktif
            perbaruiTombolGlobal()
        }
        bagianGlobal.tambah(tombolGlobal, bawah = 4)
        bagianGlobal.tambah(barisIsian("Sebaran maksimum antar exchange (%)", fGSebaran))
        bagianGlobal.tambah(barisIsian("Curiga bila selisih di atas (%)", fGCuriga))
        bagianGlobal.tambah(barisIsian("Tanda selisih kurs di atas (%)", fGKurs))
        bagianGlobal.tambah(barisIsian("Interval ambil data (detik)", fGInterval))
        bagianGlobal.tambah(barisIsian("Jendela arah harga (menit)", fGArahMenit))
        bagianGlobal.tambah(barisIsian("Tanda arah: selisih minimum (%)", fGArahAmbang))

        val barisTombol = LinearLayout(a).apply {
            orientation = LinearLayout.HORIZONTAL
            tambah(ui.tombol("Simpan ambang", primer = true) { simpanAmbang() }, bobot = 1f, lebar = 0, kanan = 4)
            tambah(ui.tombol("Nilai awal") {
                isiKolom(setelanScreenerBawaan())
                isiKolomLonjakan(setelanLonjakanBawaan())
                isiKolomGlobal(setelanGlobalBawaan())
            }, bobot = 1f, lebar = 0, kiri = 4)
        }
        isiAmbang.tambah(barisTombol, atas = 12)
        isiAmbang.tambah(infoAmbang, atas = 8)
        isi.tambah(kartuAmbang, atas = 18)

        perbaruiTombolArah()
        perbaruiTombolGlobal()
        muatAmbang()
        render()
        pengatur.postDelayed(putaran, JEDA_PUTARAN_MS)
    }

    /** Subbagian ambang yang bisa dilipat (tertutup bawaan). Mengembalikan kolom isinya. */
    private fun bagianLipat(induk: LinearLayout, nama: String, atas: Int): LinearLayout {
        val judul = ui.teks("$nama  ▸", 14f, tebal = true)
        val isiBagian = LinearLayout(a).apply { orientation = LinearLayout.VERTICAL }
        isiBagian.visibility = View.GONE
        judul.setOnClickListener {
            val buka = isiBagian.visibility != View.VISIBLE
            isiBagian.visibility = if (buka) View.VISIBLE else View.GONE
            judul.text = if (buka) "$nama  ▾" else "$nama  ▸"
        }
        induk.tambah(judul, atas = atas)
        induk.tambah(isiBagian, atas = 4)
        return isiBagian
    }

    /** Dipanggil saat tab dibuka atau data aplikasi berubah (bisa sering): ada jeda minimal antar-pemindaian. */
    override fun segarkan() {
        muat(jedaOtomatisMin())
    }

    private fun jedaOtomatisMin(): Long = (intervalMs - 1_000L).coerceAtLeast(5_000L)

    // ---------- pemindaian ----------

    /**
     * Satu siklus: Lonjakan selalu dipindai (snapshot harga terkumpul tiap siklus); Daftar ikut dipindai
     * bila belum pernah, sudah lewat `JEDA_DAFTAR_MS`, atau dipaksa (tombol Segarkan, simpan ambang).
     * Keduanya berurutan di satu thread jaringan; gagal satu tidak menggagalkan yang lain.
     */
    private fun muat(jedaMinMs: Long, paksaDaftar: Boolean = false) {
        val sekarang = System.currentTimeMillis()
        if (memuat || sekarang - mulaiMs < jedaMinMs) return
        memuat = true
        mulaiMs = sekarang
        val perluDaftar = paksaDaftar || hasil == null || sekarang - daftarMs >= JEDA_DAFTAR_MS
        if (perluDaftar) daftarMs = sekarang
        renderStatus()
        val db = a.pathDb
        val modal = isiModal.text.toString().trim()
        Latar.jaringan.execute {
            val rl = runCatching { pindaiLonjakan(db) }
            val rd = if (perluDaftar) runCatching { jalankanScreener(db, modal) } else null
            Latar.ui {
                memuat = false
                rl.onSuccess {
                    hasilL = it
                    galatL = null
                }
                rl.onFailure { galatL = Latar.pesan(it) }
                rd?.onSuccess {
                    hasil = it
                    galat = null
                }
                rd?.onFailure { galat = Latar.pesan(it) }
                render()
            }
        }
    }

    // ---------- tampilan ----------

    private fun render() {
        kartuLonjakan.removeAllViews()
        renderLonjakan()
        kartuDaftar.removeAllViews()
        renderDaftar()
        renderStatus()
    }

    private fun renderLonjakan() {
        val h = hasilL
        if (h == null) {
            kartuLonjakan.tambah(ui.mute(if (memuat) "Memuat..." else "–"))
        } else if (h.baris.isEmpty()) {
            kartuLonjakan.tambah(
                ui.mute(if (h.jumlahPunyaDasar == 0) "Menunggu data harga dasar..." else "Tidak ada lonjakan."),
            )
        } else {
            for ((i, b) in h.baris.withIndex()) {
                if (i > 0) kartuLonjakan.tambah(View(a).apply { setBackgroundColor(t.line) }, tinggi = a.dp(1))
                kartuLonjakan.tambah(barisLonjakanView(b))
            }
        }
    }

    private fun renderDaftar() {
        val h = hasil
        if (h == null) {
            kartuDaftar.tambah(ui.mute(if (memuat) "Memuat..." else "–"))
        } else if (h.baris.isEmpty()) {
            kartuDaftar.tambah(ui.mute("Tidak ada pair yang lolos."))
        } else {
            for ((i, b) in h.baris.withIndex()) {
                if (i > 0) kartuDaftar.tambah(View(a).apply { setBackgroundColor(t.line) }, tinggi = a.dp(1))
                kartuDaftar.tambah(barisView(b))
            }
        }
    }

    private fun renderStatus() {
        renderStatusGlobal()
        renderStatusDaftar()
        renderStatusLonjakan()
    }

    private fun renderStatusDaftar() {
        val h = hasil
        val g = galat
        statusDaftar.text = when {
            g != null -> "Gagal memuat: $g" + (h?.let { " · daftar ${Format.waktuSingkat(it.waktuMs)}" } ?: "")
            h == null -> if (memuat) "Memuat..." else ""
            else -> buildString {
                append(Format.waktuSingkat(h.waktuMs))
                append(" · ").append(h.baris.size).append(" pair")
                if (h.jumlahGagal > 0) append(" · ").append(h.jumlahGagal).append(" gagal")
            }
        }
        statusDaftar.setTextColor(if (g != null) t.loss else t.mute)
    }

    private fun renderStatusLonjakan() {
        val h = hasilL
        val g = galatL
        statusLonjakan.text = when {
            g != null -> "Gagal memuat: $g" + (h?.let { " · daftar ${Format.waktuSingkat(it.waktuMs)}" } ?: "")
            h == null -> if (memuat) "Memuat..." else ""
            else -> buildString {
                append(Format.waktuSingkat(h.waktuMs))
                if (h.jumlahPunyaDasar == 0) {
                    append(" · pemanasan ").append(h.rentangMenit).append(" dari ").append(h.pemanasanMenit).append(" mnt")
                } else {
                    append(" · ").append(h.baris.size).append(" lonjakan")
                    append(" · data ").append(h.rentangMenit).append(" mnt")
                }
                if (h.jumlahGagal > 0) append(" · ").append(h.jumlahGagal).append(" gagal")
            }
        }
        statusLonjakan.setTextColor(if (g != null) t.loss else t.mute)
    }

    /**
     * Status koneksi exchange luar (T9.5). Semua tersambung dan kurs wajar: satu baris ringkas;
     * bila ada yang gagal atau kurs menyimpang: rincian dan petunjuk.
     */
    private fun renderStatusGlobal() {
        val s = hasilL?.globalStatus ?: hasil?.globalStatus
        if (s == null) {
            statusGlobal.visibility = View.GONE
            return
        }
        statusGlobal.visibility = View.VISIBLE
        val gagal = s.exchange.filter { !it.ok }
        if (gagal.isEmpty() && !s.kursTanda) {
            statusGlobal.text = "Global ✓ ${s.exchange.size}/${s.exchange.size}"
            statusGlobal.setTextColor(t.mute)
            return
        }
        statusGlobal.text = buildString {
            append("Global: ")
            append(s.exchange.joinToString("  ") { (if (it.ok) "✓ " else "✗ ") + it.nama })
            if (s.kursIndodax.isNotEmpty() && s.kursBinance.isNotEmpty()) {
                append(" · kurs ").append(s.kursIndodax).append(" vs ").append(s.kursBinance)
                append(" (").append(persenBertanda(s.selisihKurs)).append(")")
                if (s.kursTanda) append(" ⚠")
            }
            for (g in gagal) append("\n").append(g.nama).append(": ").append(g.pesan)
            if (s.petunjukDns) append("\nNama host tidak ditemukan. Coba Private DNS HP: dns.google")
        }
        statusGlobal.setTextColor(t.warn)
    }

    /** Persen mentah dari Rust ("-1.25") menjadi teks bertanda memakai Format yang sama dengan layar lain. */
    private fun persenBertanda(s: String): String = when {
        s.isEmpty() -> "–"
        s.startsWith("-") -> "-" + Format.persen(s.substring(1))
        else -> "+" + Format.persen(s)
    }

    /**
     * Harga global per pair. Tanpa pembanding: tidak menampilkan apa pun. Satu baris wajar + selisih
     * (+ kualitas); rincian per exchange hanya bila minimal dua exchange punya harga; lalu baris arah bila ada.
     */
    private fun tambahGlobal(v: LinearLayout, g: InfoGlobalUi?) {
        if (g == null || g.wajar.isEmpty()) return
        val atas = buildString {
            append("wajar ").append(g.wajar).append(" (").append(persenBertanda(g.selisih)).append(")")
            if (g.kualitas != "tinggi") append(" · ").append(g.kualitas)
            if (g.curiga) append(" ⚠")
        }
        v.tambah(if (g.curiga) ui.teks(atas, 12f, t.warn) else ui.mute(atas, 12f), atas = 2)
        val ada = g.rincian.filter { it.harga.isNotEmpty() }
        if (ada.size >= 2) {
            val rincian = ada.joinToString("  ·  ") { "${it.nama} ${it.harga} (${persenBertanda(it.selisih)})" }
            v.tambah(ui.mute(rincian, 12f), atas = 2)
        }
        val ar = g.arah
        if (ar != null) {
            val teksArah = "${ar.menit} mnt: global ${panah(ar.global)}  ·  Indodax ${panah(ar.indodax)}" +
                "  ·  selisih ${persenBertanda(ar.selisih)}"
            v.tambah(if (ar.tanda) ui.teks(teksArah, 12f, t.warn) else ui.mute(teksArah, 12f), atas = 2)
        }
    }

    /** Perubahan harga bertanda dari Rust: ▲ bila positif, ▼ bila negatif, tanpa panah bila nol. */
    private fun panah(s: String): String = when {
        s.isEmpty() -> "–"
        s.startsWith("-") -> "▼" + Format.persen(s.substring(1))
        s == "0" -> Format.persen(s)
        else -> "▲" + Format.persen(s)
    }

    /** Dua baris per pair: nama, penanda, arah dan persen, harga; lalu trx/mnt, spread, harga dasar. */
    private fun barisLonjakanView(b: BarisLonjakanUi): LinearLayout {
        val v = LinearLayout(a).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(0, a.dp(10), 0, a.dp(10))
        }
        val atas = LinearLayout(a).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
        }
        atas.tambah(ui.teks(b.nama, 15f, tebal = true), lebar = WRAP_CONTENT)
        if (b.dominasiTinggi) atas.tambah(ui.teks("⚠", 13f, t.warn), kiri = 6, lebar = WRAP_CONTENT)
        if (b.dipegang) atas.tambah(ui.pill("Dipegang", null), kiri = 8, lebar = WRAP_CONTENT)
        val persenTeks = (if (b.naik) "▲ " else "▼ ") + Format.persen(b.perubahan)
        val penanda = if (b.naik) ui.teks(persenTeks, 15f, tebal = true) else ui.teks(persenTeks, 15f, t.loss, tebal = true)
        atas.tambah(penanda, kiri = 10, lebar = WRAP_CONTENT)
        atas.tambah(ui.teks(b.harga, 15f).apply { gravity = Gravity.END }, bobot = 1f, lebar = 0)
        v.tambah(atas)

        val rincian = listOf(
            "${Format.ribuan(b.trxMnt)} trx/mnt",
            "spread ${if (b.spread.isEmpty()) "–" else Format.persen(b.spread)}",
            "dasar ${b.dasar}",
        ).joinToString("  ·  ")
        v.tambah(ui.mute(rincian, 12f), atas = 3)
        tambahGlobal(v, b.global)
        return v
    }

    /** Dua baris per pair: nama, penanda, dan harga; lalu trx/mnt, spread, gerak, buy%. */
    private fun barisView(b: BarisScreenerUi): LinearLayout {
        val v = LinearLayout(a).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(0, a.dp(10), 0, a.dp(10))
        }
        val atas = LinearLayout(a).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
        }
        atas.tambah(ui.teks(b.nama, 15f, tebal = true), lebar = WRAP_CONTENT)
        if (b.dominasiTinggi) atas.tambah(ui.teks("⚠", 13f, t.warn), kiri = 6, lebar = WRAP_CONTENT)
        if (b.dipegang) atas.tambah(ui.pill("Dipegang", null), kiri = 8, lebar = WRAP_CONTENT)
        atas.tambah(ui.teks(b.harga, 15f).apply { gravity = Gravity.END }, bobot = 1f, lebar = 0)
        v.tambah(atas)

        val rincian = listOf(
            "${Format.ribuan(b.trxMnt)} trx/mnt",
            "spread ${if (b.spread.isEmpty()) "–" else Format.persen(b.spread)}",
            "gerak ${if (b.gerak.isEmpty()) "–" else Format.persen(b.gerak)}",
            "buy ${Format.persen(b.buyPersen)}",
        ).joinToString("  ·  ")
        v.tambah(ui.mute(rincian, 12f), atas = 3)
        tambahGlobal(v, b.global)
        return v
    }

    private fun barisIsian(label: String, e: EditText): LinearLayout = LinearLayout(a).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        tambah(ui.teks(label, 13f), bobot = 1f, lebar = 0)
        tambah(e, kiri = 8, lebar = a.dp(120))
        setPadding(0, a.dp(4), 0, a.dp(4))
    }

    // ---------- ambang ----------

    private fun perbaruiTombolUrutan() {
        tombolUrutan.text = if (urutan == "skor") "Urutan: Skor" else "Urutan: Trx/mnt"
    }

    private fun perbaruiTombolArah() {
        tombolNaik.text = if (lNaik) "Naik ▲: ya" else "Naik ▲: tidak"
        tombolTurun.text = if (lTurun) "Turun ▼: ya" else "Turun ▼: tidak"
    }

    private fun perbaruiTombolGlobal() {
        tombolGlobal.text = if (gAktif) "Harga global: ya" else "Harga global: tidak"
    }

    private fun isiKolomGlobal(s: SetelanGlobalUi) {
        fun atur(e: EditText, nilai: String) = e.setText(Format.komaDesimal(nilai))
        gAktif = s.aktif
        perbaruiTombolGlobal()
        atur(fGSebaran, s.sebaranMaks)
        atur(fGCuriga, s.curiga)
        atur(fGKurs, s.kursTanda)
        atur(fGInterval, s.intervalDetik)
        atur(fGArahMenit, s.arahMenit)
        atur(fGArahAmbang, s.arahAmbang)
    }

    private fun ambilKolomGlobal(): SetelanGlobalUi {
        fun teks(e: EditText) = e.text.toString().trim()
        return SetelanGlobalUi(
            aktif = gAktif,
            sebaranMaks = teks(fGSebaran),
            curiga = teks(fGCuriga),
            kursTanda = teks(fGKurs),
            intervalDetik = teks(fGInterval),
            arahMenit = teks(fGArahMenit),
            arahAmbang = teks(fGArahAmbang),
        )
    }

    private fun isiKolomLonjakan(s: SetelanLonjakanUi) {
        fun atur(e: EditText, nilai: String) = e.setText(Format.komaDesimal(nilai))
        lNaik = s.naik
        lTurun = s.turun
        perbaruiTombolArah()
        atur(fLX, s.ambangPersen)
        atur(fLM, s.trxMntMin)
        atur(fLJendela, s.jendelaMenit)
        atur(fLDasar, s.dasarMenit)
        atur(fLPemanasan, s.pemanasanMenit)
        atur(fLMaks, s.maksPeriksa)
        atur(fLInterval, s.intervalDetik)
        s.intervalDetik.toLongOrNull()?.let { intervalMs = it * 1000L }
    }

    private fun ambilKolomLonjakan(): SetelanLonjakanUi {
        fun teks(e: EditText) = e.text.toString().trim()
        return SetelanLonjakanUi(
            naik = lNaik,
            turun = lTurun,
            ambangPersen = teks(fLX),
            trxMntMin = teks(fLM),
            jendelaMenit = teks(fLJendela),
            dasarMenit = teks(fLDasar),
            pemanasanMenit = teks(fLPemanasan),
            maksPeriksa = teks(fLMaks),
            intervalDetik = teks(fLInterval),
        )
    }

    private fun isiKolom(s: SetelanScreenerUi) {
        fun atur(e: EditText, nilai: String) = e.setText(Format.komaDesimal(nilai))
        atur(fHargaMin, s.hargaMin)
        atur(fHargaMaks, s.hargaMaks)
        atur(fVolMin, s.volMin)
        atur(fVolMaks, s.volMaks)
        atur(fTrxMin, s.trxMntMin)
        atur(fJendela, s.jendelaMenit)
        atur(fSpread, s.spreadMaks)
        atur(fRange24, s.range24Min)
        atur(fUsia, s.usiaTrxMaks)
        atur(fGerak, s.gerakMin)
        atur(fMenitAktif, s.menitAktifMin)
        atur(fArah, s.arahMin)
        atur(fKedKelipatan, s.kedalamanKelipatan)
        atur(fKedJarak, s.kedalamanPersen)
        atur(fDominasi, s.dominasiTanda)
        atur(fBiaya, s.biayaRoundtrip)
        urutan = s.urutan
        perbaruiTombolUrutan()
    }

    private fun ambilKolom(): SetelanScreenerUi {
        fun teks(e: EditText) = e.text.toString().trim()
        return SetelanScreenerUi(
            hargaMin = teks(fHargaMin),
            hargaMaks = teks(fHargaMaks),
            volMin = teks(fVolMin),
            volMaks = teks(fVolMaks),
            trxMntMin = teks(fTrxMin),
            jendelaMenit = teks(fJendela),
            spreadMaks = teks(fSpread),
            range24Min = teks(fRange24),
            usiaTrxMaks = teks(fUsia),
            gerakMin = teks(fGerak),
            menitAktifMin = teks(fMenitAktif),
            arahMin = teks(fArah),
            kedalamanKelipatan = teks(fKedKelipatan),
            kedalamanPersen = teks(fKedJarak),
            dominasiTanda = teks(fDominasi),
            biayaRoundtrip = teks(fBiaya),
            urutan = urutan,
        )
    }

    /** Isi kolom dari database (sekali, saat layar dibuat). */
    private fun muatAmbang() {
        val db = a.pathDb
        Latar.baca({ bacaSetelanScreener(db) }) { r ->
            r.onSuccess { isiKolom(it) }
            r.onFailure {
                isiKolom(setelanScreenerBawaan())
                infoAmbang.text = "Gagal membaca ambang: ${Latar.pesan(it)}"
            }
        }
        Latar.baca({ bacaSetelanLonjakan(db) }) { r ->
            r.onSuccess { isiKolomLonjakan(it) }
            r.onFailure {
                isiKolomLonjakan(setelanLonjakanBawaan())
                infoAmbang.text = "Gagal membaca ambang Lonjakan: ${Latar.pesan(it)}"
            }
        }
        Latar.baca({ bacaSetelanGlobal(db) }) { r ->
            r.onSuccess { isiKolomGlobal(it) }
            r.onFailure {
                isiKolomGlobal(setelanGlobalBawaan())
                infoAmbang.text = "Gagal membaca ambang Harga global: ${Latar.pesan(it)}"
            }
        }
    }

    private fun simpanAmbang() {
        val s = ambilKolom()
        val l = ambilKolomLonjakan()
        val g = ambilKolomGlobal()
        val db = a.pathDb
        infoAmbang.text = "Menyimpan..."
        Latar.baca({
            simpanSetelanScreener(db, s)
            simpanSetelanLonjakan(db, l)
            simpanSetelanGlobal(db, g)
        }) { r ->
            r.onSuccess {
                infoAmbang.text = "Ambang tersimpan."
                l.intervalDetik.toLongOrNull()?.let { intervalMs = it * 1000L }
                muat(0L, paksaDaftar = true)
            }
            r.onFailure { infoAmbang.text = "Tidak disimpan: ${Latar.pesan(it)}" }
        }
    }

    companion object {
        /** Putaran penyegaran saat layar terlihat. */
        private const val JEDA_PUTARAN_MS = 45_000L

        /** Jeda minimal antar-pemindaian Daftar (Lonjakan dipindai tiap siklus). */
        private const val JEDA_DAFTAR_MS = 40_000L

        /** Jeda minimal untuk tombol Segarkan. */
        private const val JEDA_MANUAL_MIN_MS = 10_000L
    }
}
