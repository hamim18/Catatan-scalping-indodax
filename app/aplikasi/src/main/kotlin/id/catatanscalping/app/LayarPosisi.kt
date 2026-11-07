package id.catatanscalping.app

import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.widget.LinearLayout
import uniffi.scalping_ffi.PosisiUi
import uniffi.scalping_ffi.daftarPosisi

/**
 * Open posisi (prototype): per posisi tampil tangga SL, modal efektif, TP dan estimasi hasil bersih.
 * Tanpa tombol Jual (keputusan 7 Nov 2026: aplikasi tidak membuat order; posisi tertutup otomatis
 * saat order jual terdeteksi). Semua teks sudah berformat dari Rust (daftar_posisi).
 */
class LayarPosisi(a: MainActivity) : Layar(a) {
    private val chip = ui.Chip()
    private val daftar = LinearLayout(a).apply { orientation = LinearLayout.VERTICAL }

    init {
        chip.atur("0 posisi", null)
        header("Open posisi", chip.root)
        isi.tambah(daftar)
        isi.tambah(
            ui.mute("Posisi tertutup otomatis saat aplikasi mendeteksi order jual dari Indodax.", 12f),
            atas = 14,
        )
    }

    override fun segarkan() {
        val db = a.pathDb
        Latar.baca({ daftarPosisi(db) }) { r ->
            r.onSuccess { tampil(it) }
            r.onFailure { tampilGalat(Latar.pesan(it)) }
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
        for ((i, p) in posisi.withIndex()) daftar.tambah(kartuPosisi(p), atas = if (i == 0) 0 else 12)
    }

    private fun kartuPosisi(p: PosisiUi): LinearLayout {
        val k = ui.kartu()
        k.tambah(
            ui.baris(
                ui.teks(p.nama, 17f, tebal = true),
                ui.mute("${p.koin} koin · modal Rp ${p.totalModal}"),
            )
        )
        k.tambah(tangga(p), atas = 16, bawah = 6, kiri = 6, kanan = 6)
        k.tambah(
            ui.mute(
                "Hasil bersih bila TP: ${p.estimasiTpPersen} (${p.estimasiTpRp}). " +
                    "Bila SL: ${p.estimasiSlPersen} (${p.estimasiSlRp}). " +
                    "Setelah fee jual perkiraan 0,31%.",
                12f,
            ),
            atas = 10,
        )
        k.tambah(ui.mute("Dibuka ${Format.tanggalJam(p.waktuBukaMs)}", 12f), atas = 4)
        return k
    }

    /** Tangga harga: SL di kiri, modal efektif di tengah, TP di kanan, garis merah ke hijau di bawahnya. */
    private fun tangga(p: PosisiUi): LinearLayout {
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
        }
    }
}
