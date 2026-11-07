package id.catatanscalping.app

import android.app.AlertDialog
import android.view.Gravity
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.Button
import android.widget.LinearLayout
import uniffi.scalping_ffi.BarisTertutup
import uniffi.scalping_ffi.RiwayatUi
import uniffi.scalping_ffi.SelisihSaldoUi
import uniffi.scalping_ffi.StatistikUi
import uniffi.scalping_ffi.cekSelisihSaldo
import uniffi.scalping_ffi.daftarKoreksiSaldo
import uniffi.scalping_ffi.riwayatTertutup
import uniffi.scalping_ffi.statistikTrading
import uniffi.scalping_ffi.terapkanKoreksiSaldo

/**
 * Riwayat (prototype): ringkasan hari ini dan daftar trade tertutup. Ditambah statistik (T4.1)
 * dan koreksi saldo (T3.3). Hanya tampilan; semua angka dari Rust.
 */
class LayarRiwayat(a: MainActivity) : Layar(a) {
    private var jumlahTampil = 30
    private var rentang = "semua"
    private var selisihTerakhir: SelisihSaldoUi? = null
    private var awalanKoreksi = "" // hasil cek/koreksi terakhir, dipertahankan saat layar disegarkan

    private val daftar = LinearLayout(a).apply { orientation = LinearLayout.VERTICAL }
    private val kartuRingkasan = ui.kartu()
    private val kartuStatistik = ui.kartu()
    private val tombolRentang = LinkedHashMap<String, Button>()
    private lateinit var tombolLagi: Button
    private lateinit var tombolCek: Button
    private lateinit var tombolKoreksi: Button
    private val teksKoreksi = ui.mono("", 12f)

    init {
        header("Riwayat", ui.Chip().apply { atur("Hari ini", null) }.root)

        isi.tambah(kartuRingkasan, bawah = 12)
        isi.tambah(daftar)
        tombolLagi = ui.tombol("Tampilkan lebih banyak") { jumlahTampil += 30; muatRiwayat() }
        isi.tambah(tombolLagi, atas = 12)

        // --- Statistik ---
        isi.tambah(ui.judulBagian("Statistik"), atas = 22, bawah = 10)
        val barisRentang = LinearLayout(a).apply { orientation = LinearLayout.HORIZONTAL }
        for ((nama, kode) in listOf("Hari ini" to "hari", "Minggu ini" to "minggu", "Semua" to "semua")) {
            val b = ui.tombol(nama) { rentang = kode; gayaRentang(); muatStatistik() }
            tombolRentang[kode] = b
            barisRentang.tambah(b, bobot = 1f, lebar = 0, kanan = 3, kiri = 3)
        }
        isi.tambah(barisRentang, bawah = 10)
        isi.tambah(kartuStatistik)
        gayaRentang()

        // --- Koreksi saldo ---
        isi.tambah(ui.judulBagian("Koreksi saldo"), atas = 22, bawah = 10)
        val kKoreksi = ui.kartu()
        kKoreksi.tambah(
            ui.mute(
                "Bandingkan saldo aplikasi dengan saldo IDR asli di Indodax. Sinkron dulu dan jangan cek saat ada " +
                    "order terisi sebagian. Selisih dicatat sebagai penyesuaian (bukan profit); catatan lain tidak dihapus.",
                12f,
            )
        )
        tombolCek = ui.tombol("Cek selisih saldo") { cekSelisih() }
        tombolKoreksi = ui.tombol("Catat koreksi", primer = true) { konfirmasiKoreksi() }
        tombolKoreksi.aktif(false)
        val br = LinearLayout(a).apply {
            orientation = LinearLayout.HORIZONTAL
            tambah(tombolCek, bobot = 1f, lebar = 0, kanan = 4)
            tambah(tombolKoreksi, bobot = 1f, lebar = 0, kiri = 4)
        }
        kKoreksi.tambah(br, atas = 10)
        kKoreksi.tambah(teksKoreksi, atas = 10)
        isi.tambah(kKoreksi)
    }

    override fun segarkan() {
        muatRiwayat()
        muatStatistik()
        muatKoreksi(awalanKoreksi)
    }

    private fun gayaRentang() {
        for ((kode, b) in tombolRentang) ui.gayaTombol(b, kode == rentang)
    }

    // ---------- ringkasan dan daftar ----------

    private fun muatRiwayat() {
        val db = a.pathDb
        val n = jumlahTampil.toUInt()
        Latar.baca({ Pair(riwayatTertutup(db, n), statistikTrading(db, "hari")) }) { r ->
            r.onSuccess { (rw, sh) -> tampilRiwayat(rw, sh) }
            r.onFailure { tampilGalat(Latar.pesan(it)) }
        }
    }

    private fun tampilGalat(pesan: String) {
        kartuRingkasan.removeAllViews()
        kartuRingkasan.tambah(ui.mute("Riwayat belum bisa dihitung: $pesan"))
        daftar.removeAllViews()
        tombolLagi.visibility = android.view.View.GONE
    }

    private fun tampilRiwayat(r: RiwayatUi, hari: StatistikUi) {
        kartuRingkasan.removeAllViews()
        kartuRingkasan.tambah(
            ui.baris(
                ui.mute("Hasil bersih hari ini"),
                ui.teks(Format.untung(r.hariIniProfit), 15f, t.warnaTanda(r.hariIniProfit), tebal = true),
            )
        )
        kartuRingkasan.tambah(
            ui.mute("${r.hariIniJumlah} trade · ${r.hariIniUntung} untung · fee ${Format.rupiah(hari.totalFee)}"),
            atas = 4,
        )
        kartuRingkasan.tambah(
            ui.mute("Semua: ${r.totalTrade} trade · hasil ${Format.untung(r.totalProfit)}", 12f),
            atas = 2,
        )

        daftar.removeAllViews()
        if (r.baris.isEmpty()) {
            daftar.tambah(ui.kartu().apply { tambah(ui.mute("Belum ada trade tertutup.")) })
        }
        for ((i, x) in r.baris.withIndex()) daftar.tambah(itemTrade(x), atas = if (i == 0) 0 else 12)
        tombolLagi.visibility =
            if (r.totalTrade.toInt() > r.baris.size) android.view.View.VISIBLE else android.view.View.GONE
    }

    private fun itemTrade(x: BarisTertutup): LinearLayout {
        val kiri = LinearLayout(a).apply {
            orientation = LinearLayout.VERTICAL
            tambah(ui.teks(Format.namaKoin(x.simbol), 15f, tebal = true))
            tambah(ui.mute("${Format.waktuSingkat(x.waktuTutupMs)} · modal ${Format.rupiah(x.modal)}"))
        }
        val positif: Boolean? = when {
            Format.negatif(x.profit) -> false
            Format.nol(x.profit) -> null
            else -> true
        }
        val kanan = LinearLayout(a).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.END
            tambah(
                ui.teks(Format.untung(x.profit), 15f, t.warnaTanda(x.profit), tebal = true),
                lebar = WRAP_CONTENT,
            )
            tambah(ui.pill(Format.persenBertanda(x.persen), positif), atas = 4, lebar = WRAP_CONTENT)
        }
        return LinearLayout(a).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            background = ui.bulat(t.card, t.line, 12)
            setPadding(a.dp(14), a.dp(12), a.dp(14), a.dp(12))
            tambah(kiri, bobot = 1f, lebar = 0)
            tambah(kanan, lebar = WRAP_CONTENT)
        }
    }

    // ---------- statistik ----------

    private fun muatStatistik() {
        val db = a.pathDb
        val kode = rentang
        Latar.baca({ statistikTrading(db, kode) }) { r ->
            r.onSuccess { tampilStatistik(it) }
            r.onFailure {
                kartuStatistik.removeAllViews()
                kartuStatistik.tambah(ui.mute("Statistik belum bisa dihitung: ${Latar.pesan(it)}"))
            }
        }
    }

    private fun barisStat(label: String, nilai: String, warna: Int = t.ink): LinearLayout =
        ui.baris(ui.mute(label, 13f), ui.teks(nilai, 14f, warna, tebal = true))

    private fun tampilStatistik(s: StatistikUi) {
        val judul = when (s.rentang) { "hari" -> "Hari ini"; "minggu" -> "Minggu ini"; else -> "Semua riwayat" }
        kartuStatistik.removeAllViews()
        kartuStatistik.tambah(ui.teks("$judul · ${s.jumlah} trade", 14f, tebal = true))
        val baris = listOf(
            barisStat("Untung · rugi · impas", "${s.untung} · ${s.rugi} · ${s.impas}"),
            barisStat("Win rate", Format.persen(s.winRate)),
            barisStat("Rata-rata untung", Format.untung(s.rataUntung), t.warnaTanda(s.rataUntung)),
            barisStat("Rata-rata rugi", Format.untung(s.rataRugi), t.warnaTanda(s.rataRugi)),
            barisStat("Hasil bersih", Format.untung(s.hasilBersih), t.warnaTanda(s.hasilBersih)),
            barisStat("Total fee", Format.rupiah(s.totalFee)),
            barisStat("Profit kotor", Format.rupiah(s.profitKotor)),
            barisStat("Profit termakan fee", Format.persen(s.persenFee)),
        )
        for (b in baris) kartuStatistik.tambah(b, atas = 8)
    }

    // ---------- koreksi saldo ----------

    private fun muatKoreksi(awalan: String) {
        awalanKoreksi = awalan
        val db = a.pathDb
        Latar.baca({ daftarKoreksiSaldo(db, 5u) }) { r ->
            val daftarTeks = r.fold(
                onSuccess = { d ->
                    if (d.isEmpty()) "Belum ada koreksi dicatat."
                    else "Koreksi tercatat:\n" + d.joinToString("\n") {
                        "${Format.tanggalJam(it.waktuMs)}  ${Format.untung(it.jumlah)}  ${it.catatan}"
                    }
                },
                onFailure = { "Gagal membaca koreksi: ${Latar.pesan(it)}" },
            )
            teksKoreksi.text = if (awalan.isEmpty()) daftarTeks else "$awalan\n\n$daftarTeks"
        }
    }

    private fun cekSelisih() {
        val p = a.kunci.ambil()
        if (p == null) {
            teksKoreksi.text = "Kunci tersimpan tidak bisa dibuka. Masukkan ulang di Setelan."
            return
        }
        tombolCek.aktif(false)
        tombolKoreksi.aktif(false)
        teksKoreksi.text = "Mengambil saldo asli dari Indodax..."
        val db = a.pathDb
        Latar.jaringan.execute {
            val hasil = runCatching { cekSelisihSaldo(p.first, p.second, db) }
            Latar.ui {
                tombolCek.aktif(true)
                hasil.onSuccess { s ->
                    selisihTerakhir = s
                    tombolKoreksi.aktif(s.perluKoreksi)
                    val t = StringBuilder()
                    t.append("Saldo IDR asli Indodax  : ${Format.rupiah(s.idrAsli)}\n")
                    t.append("IDR menurut aplikasi    : ${Format.rupiah(s.idrMenurutAplikasi)}\n")
                    t.append("Modal posisi terbuka    : ${Format.rupiah(s.nilaiPosisi)}\n")
                    t.append("Saldo versi aplikasi    : ${Format.rupiah(s.saldoAplikasi)}\n")
                    t.append("SELISIH (asli - aplikasi): ${Format.untung(s.selisih)}\n")
                    t.append(
                        if (s.perluKoreksi) "Ada selisih. Bila yakin semua trade sudah tersinkron, tekan \"Catat koreksi\"."
                        else "Cocok (selisih di bawah Rp 1)."
                    )
                    muatKoreksi(t.toString())
                }
                hasil.onFailure { teksKoreksi.text = "Cek selisih gagal: ${Latar.pesan(it)}" }
            }
        }
    }

    private fun konfirmasiKoreksi() {
        val s = selisihTerakhir ?: return
        AlertDialog.Builder(a)
            .setTitle("Catat koreksi saldo?")
            .setMessage(
                "Selisih ${Format.untung(s.selisih)} akan dicatat sebagai penyesuaian. Saldo dan modal aktif ikut " +
                    "berubah sebesar itu, hasil trading (profit) tidak berubah. Saldo asli diambil ulang saat dicatat."
            )
            .setPositiveButton("Catat") { _, _ -> terapkanKoreksi() }
            .setNegativeButton("Batal", null)
            .show()
    }

    private fun terapkanKoreksi() {
        val p = a.kunci.ambil() ?: return
        tombolKoreksi.aktif(false)
        val db = a.pathDb
        Latar.jaringan.execute {
            val pesan = try {
                val j = terapkanKoreksiSaldo(p.first, p.second, db, "Koreksi manual")
                if (j == null) "Tidak ada selisih yang perlu dikoreksi." else "Koreksi dicatat: ${Format.untung(j)}."
            } catch (e: Exception) {
                "Koreksi gagal: ${Latar.pesan(e)}"
            }
            selisihTerakhir = null
            Latar.ui {
                muatKoreksi(pesan)
                a.dataBerubah()
            }
        }
    }
}
