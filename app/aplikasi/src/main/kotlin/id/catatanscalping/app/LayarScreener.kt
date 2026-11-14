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
import uniffi.scalping_ffi.SetelanLonjakanUi
import uniffi.scalping_ffi.SetelanScreenerUi
import uniffi.scalping_ffi.bacaSetelanLonjakan
import uniffi.scalping_ffi.bacaSetelanScreener
import uniffi.scalping_ffi.jalankanScreener
import uniffi.scalping_ffi.pindaiLonjakan
import uniffi.scalping_ffi.setelanLonjakanBawaan
import uniffi.scalping_ffi.setelanScreenerBawaan
import uniffi.scalping_ffi.simpanSetelanLonjakan
import uniffi.scalping_ffi.simpanSetelanScreener

/**
 * T9.2: layar Screener (tab ke-5). HANYA informasi: daftar pair IDR yang lolos kriteria pengguna,
 * tanpa teks penjelas (gaya Fase 7). Tidak memengaruhi posisi, profit, target, grafik, atau statistik.
 * Data publik tanpa key, jadi layar ini jalan walau kunci API belum diisi.
 * Penyegaran otomatis hanya saat layar terlihat (FR-30). Hanya tampilan; semua hitungan di Rust.
 * Mengetuk baris tidak melakukan apa pun (tombol buka pair dibatalkan).
 *
 * T9.4: tombol mode "Daftar / Lonjakan". Mode Lonjakan memolling ringkasan pasar tiap beberapa detik
 * (interval dari setelan) HANYA saat layar terlihat dan mode aktif; riwayat harga ada di memori Rust.
 * Tanpa layanan latar dan tanpa notifikasi.
 */
class LayarScreener(a: MainActivity) : Layar(a) {
    private val isiModal = ui.input("Modal trade ini (Rp)")
    private val statusTeks = ui.mute("", 12f)
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
    private lateinit var tombolMode: Button
    private var modeLonjakan = false
    private var intervalMs = 20_000L

    // --- keadaan pemindaian ---
    private var hasil: HasilScreenerUi? = null
    private var galat: String? = null
    private var hasilL: HasilLonjakanUi? = null
    private var galatL: String? = null
    private var memuat = false
    private var mulaiMs = 0L

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
            pengatur.postDelayed(this, if (modeLonjakan) intervalMs else JEDA_PUTARAN_MS)
        }
    }

    init {
        header("Screener")

        val barisModal = LinearLayout(a).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            tambah(isiModal, bobot = 1f, lebar = 0)
            tambah(ui.tombol("Segarkan") { muat(JEDA_MANUAL_MIN_MS) }, kiri = 8, lebar = WRAP_CONTENT)
        }
        isi.tambah(barisModal)
        tombolMode = ui.tombol("Mode: Daftar") {
            modeLonjakan = !modeLonjakan
            perbaruiTombolMode()
            render()
            muat(0L)
        }
        isi.tambah(tombolMode, atas = 8)
        isi.tambah(statusTeks, atas = 8, bawah = 10)
        isi.tambah(kartuDaftar)

        // --- kartu ambang (tersembunyi sampai judul diketuk) ---
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

        isiAmbang.tambah(ui.mute("Kosong = filter mati.", 12f), bawah = 4)
        isiAmbang.tambah(barisIsian("Harga minimum (Rp)", fHargaMin))
        isiAmbang.tambah(barisIsian("Harga maksimum (Rp)", fHargaMaks))
        isiAmbang.tambah(barisIsian("Volume 24 jam minimum (Rp)", fVolMin))
        isiAmbang.tambah(barisIsian("Volume 24 jam maksimum (Rp)", fVolMaks))
        isiAmbang.tambah(barisIsian("Trx/mnt minimum", fTrxMin))
        isiAmbang.tambah(barisIsian("Jendela (menit)", fJendela))
        isiAmbang.tambah(barisIsian("Spread maksimum (%)", fSpread))
        isiAmbang.tambah(barisIsian("Range 24 jam minimum (%)", fRange24))
        isiAmbang.tambah(barisIsian("Usia transaksi maksimum (detik)", fUsia))
        isiAmbang.tambah(barisIsian("Gerak jendela minimum (%)", fGerak))
        isiAmbang.tambah(barisIsian("Menit aktif minimum (%)", fMenitAktif))
        isiAmbang.tambah(barisIsian("Arah jendela minimum (%)", fArah))
        isiAmbang.tambah(barisIsian("Kedalaman: kelipatan modal", fKedKelipatan))
        isiAmbang.tambah(barisIsian("Kedalaman: jarak (%)", fKedJarak))
        isiAmbang.tambah(barisIsian("Tanda dominasi (%)", fDominasi))
        isiAmbang.tambah(barisIsian("Biaya round-trip (%)", fBiaya))

        isiAmbang.tambah(ui.teks("Lonjakan", 14f, tebal = true), atas = 14, bawah = 4)
        isiAmbang.tambah(ui.mute("Dasar = rata-rata harga dalam periode di bawah. Tanpa notifikasi.", 12f), bawah = 4)
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
        isiAmbang.tambah(barisArah, bawah = 4)
        isiAmbang.tambah(barisIsian("Lonjakan minimum (%)", fLX))
        isiAmbang.tambah(barisIsian("Trx/mnt minimum", fLM))
        isiAmbang.tambah(barisIsian("Jendela trx/mnt (menit)", fLJendela))
        isiAmbang.tambah(barisIsian("Rata-rata harga dasar (menit)", fLDasar))
        isiAmbang.tambah(barisIsian("Pemanasan minimum (menit)", fLPemanasan))
        isiAmbang.tambah(barisIsian("Maks pair diperiksa", fLMaks))
        isiAmbang.tambah(barisIsian("Interval (detik)", fLInterval))

        tombolUrutan = ui.tombol("Urutan: Skor") {
            urutan = if (urutan == "skor") "trx" else "skor"
            perbaruiTombolUrutan()
        }
        isiAmbang.tambah(tombolUrutan, atas = 10)

        val barisTombol = LinearLayout(a).apply {
            orientation = LinearLayout.HORIZONTAL
            tambah(ui.tombol("Simpan ambang", primer = true) { simpanAmbang() }, bobot = 1f, lebar = 0, kanan = 4)
            tambah(ui.tombol("Nilai awal") {
                isiKolom(setelanScreenerBawaan())
                isiKolomLonjakan(setelanLonjakanBawaan())
            }, bobot = 1f, lebar = 0, kiri = 4)
        }
        isiAmbang.tambah(barisTombol, atas = 10)
        isiAmbang.tambah(infoAmbang, atas = 8)
        isi.tambah(kartuAmbang, atas = 12)

        perbaruiTombolArah()
        muatAmbang()
        render()
        pengatur.postDelayed(putaran, JEDA_PUTARAN_MS)
    }

    /** Dipanggil saat tab dibuka atau data aplikasi berubah (bisa sering): ada jeda minimal antar-pemindaian. */
    override fun segarkan() {
        muat(jedaOtomatisMin())
    }

    private fun jedaOtomatisMin(): Long =
        if (modeLonjakan) (intervalMs - 1_000L).coerceAtLeast(5_000L) else JEDA_OTOMATIS_MIN_MS

    // ---------- pemindaian ----------

    private fun muat(jedaMinMs: Long) {
        val sekarang = System.currentTimeMillis()
        if (memuat || sekarang - mulaiMs < jedaMinMs) return
        memuat = true
        mulaiMs = sekarang
        renderStatus()
        val db = a.pathDb
        val modal = isiModal.text.toString().trim()
        val lonjakan = modeLonjakan
        Latar.jaringan.execute {
            if (lonjakan) {
                val r = runCatching { pindaiLonjakan(db) }
                Latar.ui {
                    memuat = false
                    r.onSuccess {
                        hasilL = it
                        galatL = null
                    }
                    r.onFailure { galatL = Latar.pesan(it) }
                    selesai(lonjakan)
                }
            } else {
                val r = runCatching { jalankanScreener(db, modal) }
                Latar.ui {
                    memuat = false
                    r.onSuccess {
                        hasil = it
                        galat = null
                    }
                    r.onFailure { galat = Latar.pesan(it) }
                    selesai(lonjakan)
                }
            }
        }
    }

    /** Mode bisa berganti saat pemindaian berjalan: tampilkan hasil mode aktif, lalu muat bila mode berubah. */
    private fun selesai(modeSaatMulai: Boolean) {
        render()
        if (modeSaatMulai != modeLonjakan) muat(0L)
    }

    // ---------- tampilan ----------

    private fun render() {
        kartuDaftar.removeAllViews()
        if (modeLonjakan) renderLonjakan() else renderDaftar()
        renderStatus()
    }

    private fun renderLonjakan() {
        val h = hasilL
        if (h == null) {
            kartuDaftar.tambah(ui.mute(if (memuat) "Memuat..." else "–"))
        } else if (h.baris.isEmpty()) {
            kartuDaftar.tambah(
                ui.mute(if (h.jumlahPunyaDasar == 0) "Menunggu data harga dasar..." else "Tidak ada lonjakan."),
            )
        } else {
            for ((i, b) in h.baris.withIndex()) {
                if (i > 0) kartuDaftar.tambah(View(a).apply { setBackgroundColor(t.line) }, tinggi = a.dp(1))
                kartuDaftar.tambah(barisLonjakanView(b))
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
        if (modeLonjakan) {
            renderStatusLonjakan()
            return
        }
        val h = hasil
        val g = galat
        statusTeks.text = when {
            g != null -> "Gagal memuat: $g" + (h?.let { " · daftar ${Format.waktuSingkat(it.waktuMs)}" } ?: "")
            h == null -> if (memuat) "Memuat..." else ""
            else -> buildString {
                append(Format.waktuSingkat(h.waktuMs))
                append(" · ").append(h.baris.size).append(" pair")
                if (h.jumlahGagal > 0) append(" · ").append(h.jumlahGagal).append(" gagal")
                if (h.jumlahDiperiksa < h.jumlahKandidat) {
                    append(" · diperiksa ").append(h.jumlahDiperiksa).append(" dari ").append(h.jumlahKandidat)
                }
            }
        }
        statusTeks.setTextColor(if (g != null) t.loss else t.mute)
    }

    private fun renderStatusLonjakan() {
        val h = hasilL
        val g = galatL
        statusTeks.text = when {
            g != null -> "Gagal memuat: $g" + (h?.let { " · daftar ${Format.waktuSingkat(it.waktuMs)}" } ?: "")
            h == null -> if (memuat) "Memuat..." else ""
            else -> buildString {
                append(Format.waktuSingkat(h.waktuMs))
                if (h.jumlahPunyaDasar == 0) {
                    append(" · pemanasan ").append(h.rentangMenit).append(" dari ").append(h.pemanasanMenit).append(" mnt")
                } else {
                    append(" · ").append(h.baris.size).append(" lonjakan")
                    append(" · data ").append(h.rentangMenit).append(" mnt")
                    if (h.jumlahDiperiksa < h.jumlahPemicu) {
                        append(" · diperiksa ").append(h.jumlahDiperiksa).append(" dari ").append(h.jumlahPemicu)
                    }
                }
                if (h.jumlahGagal > 0) append(" · ").append(h.jumlahGagal).append(" gagal")
            }
        }
        statusTeks.setTextColor(if (g != null) t.loss else t.mute)
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
            "dasar ${b.dasar} (${b.lamaDasarMenit} mnt)",
        ).joinToString("  ·  ")
        v.tambah(ui.mute(rincian, 12f), atas = 3)
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

    private fun perbaruiTombolMode() {
        tombolMode.text = if (modeLonjakan) "Mode: Lonjakan" else "Mode: Daftar"
    }

    private fun perbaruiTombolArah() {
        tombolNaik.text = if (lNaik) "Naik ▲: ya" else "Naik ▲: tidak"
        tombolTurun.text = if (lTurun) "Turun ▼: ya" else "Turun ▼: tidak"
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
    }

    private fun simpanAmbang() {
        val s = ambilKolom()
        val l = ambilKolomLonjakan()
        val db = a.pathDb
        infoAmbang.text = "Menyimpan..."
        Latar.baca({
            simpanSetelanScreener(db, s)
            simpanSetelanLonjakan(db, l)
        }) { r ->
            r.onSuccess {
                infoAmbang.text = "Ambang tersimpan."
                l.intervalDetik.toLongOrNull()?.let { intervalMs = it * 1000L }
                muat(0L)
            }
            r.onFailure { infoAmbang.text = "Tidak disimpan: ${Latar.pesan(it)}" }
        }
    }

    companion object {
        /** Putaran penyegaran saat layar terlihat. */
        private const val JEDA_PUTARAN_MS = 45_000L

        /** Jeda minimal antar-pemindaian otomatis (segarkan() bisa dipanggil sering). */
        private const val JEDA_OTOMATIS_MIN_MS = 40_000L

        /** Jeda minimal untuk tombol Segarkan. */
        private const val JEDA_MANUAL_MIN_MS = 10_000L
    }
}
