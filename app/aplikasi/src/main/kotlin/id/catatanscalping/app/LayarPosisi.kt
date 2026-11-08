package id.catatanscalping.app

import android.graphics.drawable.GradientDrawable
import android.os.Handler
import android.os.Looper
import android.view.Gravity
import android.view.View
import android.widget.LinearLayout
import uniffi.scalping_ffi.FloatingPosisiUi
import uniffi.scalping_ffi.PosisiUi
import uniffi.scalping_ffi.ambilHargaTerakhir
import uniffi.scalping_ffi.daftarPosisi
import uniffi.scalping_ffi.floatingPosisi

/**
 * Open posisi (prototype; disederhanakan di T7.1): per posisi tampil tangga SL/modal efektif/TP,
 * satu baris harga + floating, dan satu baris estimasi bersih. Tanpa teks penjelas.
 * Tanpa tombol Jual (keputusan 7 Nov 2026: aplikasi tidak membuat order; posisi tertutup otomatis
 * saat order jual terdeteksi). Semua teks sudah berformat dari Rust (daftar_posisi).
 *
 * T6.3: harga sekarang (transaksi terakhir) tampil sebagai penanda di tangga, merah bila harga sudah
 * di bawah SL, beserta floating P/L per posisi SEBELUM fee jual. Harga diambil dari ticker_all
 * (jaringan, thread latar) saat layar dibuka dan tiap 10 detik selama layar terlihat; floating
 * dihitung di Rust. Gagal ambil harga tidak mengganggu daftar posisi.
 */
class LayarPosisi(a: MainActivity) : Layar(a) {
    private val chip = ui.Chip()
    private val daftar = LinearLayout(a).apply { orientation = LinearLayout.VERTICAL }

    // posisi dari database dan floating terakhir per simbol
    private var posisiTerakhir: List<PosisiUi> = emptyList()
    private var floatingPerSimbol: Map<String, FloatingPosisiUi> = emptyMap()
    private var galatDaftar: String? = null

    // harga (jaringan)
    private var hargaMemuat = false
    private var hargaMulaiMs = 0L
    private var hargaGalat: String? = null

    private val pengatur = Handler(Looper.getMainLooper())
    private var pernahTerpasang = false
    private val putaranHarga = object : Runnable {
        override fun run() {
            if (root.isAttachedToWindow) {
                pernahTerpasang = true
            } else if (pernahTerpasang) {
                return // layar sudah dilepas: hentikan putaran
            }
            if (root.isShown && root.windowVisibility == View.VISIBLE) muatHarga()
            pengatur.postDelayed(this, JEDA_PUTARAN_MS)
        }
    }

    init {
        chip.atur("0 posisi", null)
        header("Open posisi", chip.root)
        isi.tambah(daftar)
        pengatur.postDelayed(putaranHarga, JEDA_PUTARAN_MS)
    }

    override fun segarkan() {
        val db = a.pathDb
        Latar.baca({ daftarPosisi(db) }) { r ->
            r.onSuccess {
                galatDaftar = null
                posisiTerakhir = it
                // buang floating posisi yang sudah tidak ada; nilai posisi yang berubah menyusul di muatHarga
                val ada = it.map { p -> p.simbol }.toSet()
                floatingPerSimbol = floatingPerSimbol.filterKeys { k -> k in ada }
                render()
                muatHarga()
            }
            r.onFailure {
                galatDaftar = Latar.pesan(it)
                posisiTerakhir = emptyList()
                floatingPerSimbol = emptyMap()
                render()
            }
        }
    }

    /**
     * Harga terakhir lewat jaringan (satu permintaan pada satu waktu, jeda minimal 5 detik), lalu
     * floating dihitung di Rust dari database. Tanpa kunci atau tanpa posisi: tidak ada yang dilakukan.
     */
    private fun muatHarga() {
        val kunci = a.kunci.ambil() ?: return
        val koin = posisiTerakhir.map { it.nama }
        if (koin.isEmpty()) return
        val sekarang = System.currentTimeMillis()
        if (hargaMemuat || sekarang - hargaMulaiMs < JEDA_MINIMAL_MS) return
        hargaMemuat = true
        hargaMulaiMs = sekarang
        val db = a.pathDb
        Latar.jaringan.execute {
            val r = runCatching { ambilHargaTerakhir(kunci.first, kunci.second, koin) }
            Latar.ui {
                r.onSuccess { harga ->
                    Latar.baca({ floatingPosisi(db, harga) }) { hasil ->
                        hargaMemuat = false
                        hasil.onSuccess { f ->
                            floatingPerSimbol = f.posisi.associateBy { p -> p.simbol }
                            hargaGalat = null
                        }
                        hasil.onFailure { hargaGalat = Latar.pesan(it) }
                        render()
                    }
                }
                r.onFailure {
                    hargaMemuat = false
                    hargaGalat = Latar.pesan(it)
                    render()
                }
            }
        }
    }

    private fun render() {
        val galat = galatDaftar
        if (galat != null) {
            tampilGalat(galat)
        } else {
            tampil(posisiTerakhir)
        }
    }

    private fun tampilGalat(pesan: String) {
        daftar.removeAllViews()
        chip.atur("-", null)
        val k = ui.kartu()
        k.tambah(
            ui.mute(
                "Posisi belum bisa dihitung: $pesan\n\n" +
                    "Pastikan kunci sudah disimpan, TP/SL diisi di Setelan, dan sinkron sudah dijalankan sekali."
            )
        )
        daftar.tambah(k)
    }

    private fun tampil(posisi: List<PosisiUi>) {
        daftar.removeAllViews()
        chip.atur("${posisi.size} posisi", null)
        if (posisi.isEmpty()) {
            daftar.tambah(ui.kartu().apply { tambah(ui.mute("Belum ada posisi terbuka.")) })
            return
        }
        for ((i, p) in posisi.withIndex()) {
            daftar.tambah(kartuPosisi(p, floatingPerSimbol[p.simbol]), atas = if (i == 0) 0 else 12)
        }
        // hanya galat yang ditandai (angka terakhir tetap tampil)
        if (hargaGalat != null) daftar.tambah(ui.teks("Gagal memperbarui harga", 12f, t.warn), atas = 12)
    }

    private fun kartuPosisi(p: PosisiUi, f: FloatingPosisiUi?): LinearLayout {
        val k = ui.kartu()
        k.tambah(
            ui.baris(
                ui.teks(p.nama, 17f, tebal = true),
                ui.mute("${p.koin} koin · modal Rp ${p.totalModal}"),
            )
        )
        k.tambah(tangga(p, f), atas = 16, bawah = 6, kiri = 6, kanan = 6)
        tambahFloating(k, f)
        k.tambah(
            ui.baris(
                ui.mute("Bersih  TP ${p.estimasiTpPersen} · SL ${p.estimasiSlPersen}", 12f),
                ui.mute(Format.tanggalJam(p.waktuBukaMs), 12f),
            ),
            atas = 10,
        )
        return k
    }

    /** Baris harga sekarang dan floating di bawah tangga. Tanpa data harga: tidak menambah apa pun. */
    private fun tambahFloating(k: LinearLayout, f: FloatingPosisiUi?) {
        if (f == null) return
        // salin ke variabel lokal: properti record dari Rust adalah var, tidak bisa di-smart-cast
        val harga = f.hargaSekarang
        val rp = f.floatingAngka
        val persen = f.floatingPersen
        if (harga == null || rp == null || persen == null) return // tanpa harga IDR: tidak ditampilkan
        val warna = t.warnaTanda(rp)
        k.tambah(
            ui.baris(
                ui.teks("Harga $harga", 13f, tebal = true),
                ui.teks("${Format.untung(rp)} (${Format.persenBertanda(persen)})", 13f, warna, tebal = true),
            ),
            atas = 8,
        )
    }

    /**
     * Tangga harga: SL di kiri, modal efektif di tengah, TP di kanan, garis merah ke hijau di bawahnya.
     * Dengan harga sekarang: titik penanda di bawah garis (posisi dari Rust: 0 = SL, 0,5 = modal, 1 = TP).
     */
    private fun tangga(p: PosisiUi, f: FloatingPosisiUi?): LinearLayout {
        fun kolom(harga: String, label: String, rata: Int, warnaTanda: Int): LinearLayout =
            LinearLayout(a).apply {
                orientation = LinearLayout.VERTICAL
                gravity = rata
                tambah(ui.teks(harga, 12f, tebal = true), lebar = android.view.ViewGroup.LayoutParams.WRAP_CONTENT)
                tambah(ui.mute(label, 11f), lebar = android.view.ViewGroup.LayoutParams.WRAP_CONTENT)
                tambah(
                    View(a).apply { setBackgroundColor(warnaTanda) },
                    atas = 3, lebar = a.dp(2), tinggi = a.dp(12),
                )
            }

        val atas = LinearLayout(a).apply {
            orientation = LinearLayout.HORIZONTAL
            tambah(kolom(p.hargaSl, "SL −${p.slPersen}%", Gravity.START, t.loss), bobot = 1f, lebar = 0)
            tambah(kolom(p.modalEfektif, "Modal efektif", Gravity.CENTER_HORIZONTAL, t.ink), bobot = 1f, lebar = 0)
            tambah(kolom(p.hargaTp, "TP +${p.tpPersen}%", Gravity.END, t.gain), bobot = 1f, lebar = 0)
        }

        val r = a.dp(2).toFloat()
        val garis = LinearLayout(a).apply {
            orientation = LinearLayout.HORIZONTAL
            tambah(
                View(a).apply {
                    background = GradientDrawable().apply {
                        setColor(t.loss); cornerRadii = floatArrayOf(r, r, 0f, 0f, 0f, 0f, r, r)
                    }
                },
                bobot = 1f, lebar = 0, tinggi = a.dp(4),
            )
            tambah(
                View(a).apply {
                    background = GradientDrawable().apply {
                        setColor(t.gain); cornerRadii = floatArrayOf(0f, 0f, r, r, r, r, 0f, 0f)
                    }
                },
                bobot = 1f, lebar = 0, tinggi = a.dp(4),
            )
        }
        return LinearLayout(a).apply {
            orientation = LinearLayout.VERTICAL
            tambah(atas)
            tambah(garis)
            val skala = f?.skala
            if (f != null && skala != null) tambah(penanda(skala, f), atas = 4)
        }
    }

    /** Baris penanda harga sekarang: spacer berbobot di kiri dan kanan titik, sehingga titik jatuh di `skala`. */
    private fun penanda(skala: Double, f: FloatingPosisiUi): LinearLayout {
        val s = skala.coerceIn(0.0, 1.0).toFloat()
        val warna = when {
            f.diBawahSl -> t.loss
            f.diAtasTp -> t.gain
            else -> t.accent
        }
        val ukuran = a.dp(10)
        return LinearLayout(a).apply {
            orientation = LinearLayout.HORIZONTAL
            tambah(View(a), bobot = s, lebar = 0, tinggi = ukuran)
            tambah(
                View(a).apply {
                    background = GradientDrawable().apply { shape = GradientDrawable.OVAL; setColor(warna) }
                },
                lebar = ukuran, tinggi = ukuran,
            )
            tambah(View(a), bobot = 1f - s, lebar = 0, tinggi = ukuran)
        }
    }

    private companion object {
        const val JEDA_PUTARAN_MS = 10_000L
        const val JEDA_MINIMAL_MS = 5_000L
    }
}
