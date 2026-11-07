package id.catatanscalping.app

import android.Manifest
import android.app.Activity
import android.app.AlertDialog
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
import uniffi.scalping_ffi.PencapaianUi
import uniffi.scalping_ffi.StatusKoneksi
import uniffi.scalping_ffi.aturAmountSudahBersih
import uniffi.scalping_ffi.daftarArusKas
import uniffi.scalping_ffi.hitungTarget
import uniffi.scalping_ffi.SelisihSaldoUi
import uniffi.scalping_ffi.cekSelisihSaldo
import uniffi.scalping_ffi.daftarKoreksiSaldo
import uniffi.scalping_ffi.dataGrafik
import uniffi.scalping_ffi.riwayatTertutup
import uniffi.scalping_ffi.statistikTrading
import uniffi.scalping_ffi.sinkronArusKas
import uniffi.scalping_ffi.terapkanKoreksiSaldo
import uniffi.scalping_ffi.bacaTarget
import uniffi.scalping_ffi.cekKoneksi
import uniffi.scalping_ffi.hitungPosisi
import uniffi.scalping_ffi.simpanSetelan
import uniffi.scalping_ffi.simpanTarget
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
    private lateinit var isiTargetHarian: EditText
    private lateinit var isiTargetMingguan: EditText
    private lateinit var tombolTarget: Button
    private lateinit var isiTarget: TextView
    private lateinit var tombolArusKas: Button
    private lateinit var cekBersih: CheckBox
    private lateinit var tombolProgress: Button
    private lateinit var isiProgress: TextView
    private lateinit var grafik: GrafikView
    private lateinit var isiGrafik: TextView
    private lateinit var tombolRiwayat: Button
    private lateinit var isiRiwayat: TextView
    private lateinit var isiStatistik: TextView
    @Volatile private var rentangStatistik = "semua"
    private lateinit var tombolCekSelisih: Button
    private lateinit var tombolKoreksi: Button
    private lateinit var isiKoreksi: TextView
    @Volatile private var selisihTerakhir: SelisihSaldoUi? = null

    // true selama sinkron (manual atau otomatis) berjalan
    @Volatile private var sinkronBerjalan = false

    // true selama sinkron arus kas (T2.2) berjalan
    @Volatile private var arusKasBerjalan = false

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
        isi.addView(teks("Target harian dan mingguan (T2.1)", 16f, tebal = true))
        isi.addView(teks("Dalam persen dari saldo awal periode. Progress dihitung di T2.3.", 13f))
        isiTargetHarian = angka("Target harian % (contoh 1,5)", "")
        isiTargetMingguan = angka("Target mingguan % (contoh 5)", "")
        isi.addView(isiTargetHarian)
        isi.addView(isiTargetMingguan)
        tombolTarget = Button(this).apply { text = "Simpan target"; setOnClickListener { simpanTargetUi() } }
        isi.addView(tombolTarget)
        isiTarget = teks("", 13f)
        isi.addView(isiTarget)
        isi.addView(teks("Arus kas dan progress target (T2.2 sampai T2.4)", 16f, tebal = true))
        isi.addView(teks("Deposit/withdraw IDR dikeluarkan dari pencapaian. Periode WIB, minggu mulai Senin. Butuh sinkron trade dan \"Hitung posisi\" (TP/SL tersimpan) sekali.", 13f))
        tombolArusKas = Button(this).apply { text = "Sinkron arus kas (deposit/withdraw)"; setOnClickListener { jalankanArusKas(false) } }
        cekBersih = CheckBox(this).apply {
            text = "Nominal (amount) sudah bersih: fee TIDAK dikurangkan lagi"
            setOnClickListener { ubahModeFee() }
        }
        tombolProgress = Button(this).apply { text = "Hitung progress target"; setOnClickListener { tampilkanProgress() } }
        isi.addView(tombolArusKas)
        isi.addView(cekBersih)
        isi.addView(tombolProgress)
        isiProgress = teks("", 13f)
        isiProgress.typeface = android.graphics.Typeface.MONOSPACE
        isi.addView(isiProgress)

        isi.addView(teks("Grafik saldo (T3.2)", 16f, tebal = true))
        isi.addView(teks("Garis tebal: saldo versi aplikasi. Putus-putus: modal aktif. Jarak keduanya = hasil trading.", 13f))
        grafik = GrafikView(this)
        isi.addView(grafik, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT))
        isiGrafik = teks("", 12f)
        isi.addView(isiGrafik)

        isi.addView(teks("Riwayat trade tertutup (T3.1)", 16f, tebal = true))
        tombolRiwayat = Button(this).apply { text = "Tampilkan riwayat"; setOnClickListener { tampilkanRiwayat() } }
        isi.addView(tombolRiwayat)
        isiRiwayat = teks("", 13f)
        isiRiwayat.typeface = android.graphics.Typeface.MONOSPACE
        isi.addView(isiRiwayat)

        isi.addView(teks("Statistik trading (T4.1)", 16f, tebal = true))
        val barisRentang = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL }
        for ((nama, kode) in listOf("Hari ini" to "hari", "Minggu ini" to "minggu", "Semua" to "semua")) {
            barisRentang.addView(
                Button(this).apply { text = nama; setOnClickListener { rentangStatistik = kode; tampilkanStatistik() } },
                LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f),
            )
        }
        isi.addView(barisRentang)
        isiStatistik = teks("", 13f)
        isiStatistik.typeface = android.graphics.Typeface.MONOSPACE
        isi.addView(isiStatistik)

        isi.addView(teks("Koreksi saldo (T3.3)", 16f, tebal = true))
        isi.addView(teks("Bandingkan saldo aplikasi dengan saldo IDR asli di Indodax. Sinkron trade dulu; jangan cek saat ada order terisi sebagian. Selisih dicatat sebagai penyesuaian (tidak dihitung sebagai profit), tidak menghapus catatan apa pun.", 13f))
        tombolCekSelisih = Button(this).apply { text = "Cek selisih saldo"; setOnClickListener { cekSelisih() } }
        tombolKoreksi = Button(this).apply { text = "Catat koreksi"; isEnabled = false; setOnClickListener { konfirmasiKoreksi() } }
        isi.addView(tombolCekSelisih)
        isi.addView(tombolKoreksi)
        isiKoreksi = teks("", 13f)
        isiKoreksi.typeface = android.graphics.Typeface.MONOSPACE
        isi.addView(isiKoreksi)
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
        muatTarget()
        tampilkanProgress()
        tampilkanRiwayat()
        tampilkanStatistik()
        tampilkanKoreksi()
        tampilkanGrafik()
    }

    /** T3.1: trade tertutup terbaru dan ringkasan hari ini. Tanpa jaringan. */
    private fun tampilkanRiwayat() {
        Thread {
            try {
                val r = riwayatTertutup(pathDb, 15u)
                val t = StringBuilder()
                t.append("Hari ini: ${r.hariIniJumlah} trade, ${r.hariIniUntung} untung, hasil Rp ${r.hariIniProfit}\n")
                t.append("Semua: ${r.totalTrade} trade, hasil Rp ${r.totalProfit}\n\n")
                if (r.baris.isEmpty()) t.append("(belum ada trade tertutup)")
                for (x in r.baris) {
                    t.append("${waktuWib(x.waktuTutupMs)} ${x.simbol}\n")
                    t.append("  modal ${x.modal} diterima ${x.diterima}\n")
                    t.append("  hasil ${x.profit} (${x.persen}%), buka ${waktuWib(x.waktuBukaMs)}\n")
                }
                runOnUiThread { isiRiwayat.text = t.toString() }
            } catch (e: Exception) {
                runOnUiThread { isiRiwayat.text = "Riwayat belum bisa dihitung: ${pesanGalat(e)}" }
            }
        }.start()
    }

    /** T4.1: statistik trade tertutup untuk rentang terpilih. Tanpa jaringan. */
    private fun tampilkanStatistik() {
        val rentang = rentangStatistik
        Thread {
            try {
                val s = statistikTrading(pathDb, rentang)
                val judul = when (s.rentang) { "hari" -> "Hari ini"; "minggu" -> "Minggu ini"; else -> "Semua riwayat" }
                val t = StringBuilder()
                t.append("$judul: ${s.jumlah} trade\n")
                t.append("Untung ${s.untung} · rugi ${s.rugi} · impas ${s.impas}\n")
                t.append("Win rate: ${s.winRate}%\n")
                t.append("Rata-rata untung: Rp ${s.rataUntung}\n")
                t.append("Rata-rata rugi: Rp ${s.rataRugi}\n")
                t.append("Hasil bersih: Rp ${s.hasilBersih}\n")
                t.append("Total fee: Rp ${s.totalFee}\n")
                t.append("Profit kotor: Rp ${s.profitKotor}\n")
                t.append("Profit termakan fee: ${s.persenFee}%")
                runOnUiThread { isiStatistik.text = t.toString() }
            } catch (e: Exception) {
                runOnUiThread { isiStatistik.text = "Statistik belum bisa dihitung: ${pesanGalat(e)}" }
            }
        }.start()
    }

    /** T3.2: grafik saldo dan modal aktif. Tanpa jaringan. */
    private fun tampilkanGrafik() {
        Thread {
            try {
                val data = dataGrafik(pathDb, 120u)
                runOnUiThread {
                    grafik.setData(data)
                    isiGrafik.text = "${data.size} titik, dari ${waktuWib(data.first().waktuMs)} sampai ${waktuWib(data.last().waktuMs)}"
                }
            } catch (e: Exception) {
                runOnUiThread { grafik.setData(emptyList()); isiGrafik.text = "Grafik belum bisa dibuat: ${pesanGalat(e)}" }
            }
        }.start()
    }

    private fun teksKoreksiTersimpan(): String {
        val daftar = daftarKoreksiSaldo(pathDb, 5u)
        if (daftar.isEmpty()) return "Belum ada koreksi dicatat."
        val t = StringBuilder("Koreksi tercatat:\n")
        for (k in daftar) t.append("${waktuWib(k.waktuMs)}  Rp ${k.jumlah}  ${k.catatan}\n")
        return t.toString()
    }

    private fun tampilkanKoreksi(awalan: String = "") {
        Thread {
            val isi = try { teksKoreksiTersimpan() } catch (e: Exception) { "Gagal membaca koreksi: ${pesanGalat(e)}" }
            runOnUiThread { isiKoreksi.text = if (awalan.isEmpty()) isi else awalan + "\n\n" + isi }
        }.start()
    }

    private fun cekSelisih() {
        val p = kunci.ambil()
        if (p == null) {
            isiKoreksi.text = "Kunci tersimpan tidak bisa dibuka. Masukkan ulang."
            return
        }
        tombolCekSelisih.isEnabled = false
        tombolKoreksi.isEnabled = false
        isiKoreksi.text = "Mengambil saldo asli dari Indodax..."
        Thread {
            try {
                val s = cekSelisihSaldo(p.first, p.second, pathDb)
                selisihTerakhir = s
                val t = StringBuilder()
                t.append("Saldo IDR asli Indodax : Rp ${s.idrAsli}\n")
                t.append("IDR menurut aplikasi   : Rp ${s.idrMenurutAplikasi}\n")
                t.append("Modal posisi terbuka   : Rp ${s.nilaiPosisi}\n")
                t.append("Saldo versi aplikasi   : Rp ${s.saldoAplikasi}\n")
                t.append("SELISIH (asli - aplikasi): Rp ${s.selisih}\n")
                t.append(if (s.perluKoreksi) "Ada selisih. Bila yakin semua trade sudah tersinkron, tekan \"Catat koreksi\"." else "Cocok (selisih di bawah Rp 1).")
                val perlu = s.perluKoreksi
                runOnUiThread {
                    tombolCekSelisih.isEnabled = true
                    tombolKoreksi.isEnabled = perlu
                    tampilkanKoreksi(t.toString())
                }
            } catch (e: Exception) {
                runOnUiThread {
                    tombolCekSelisih.isEnabled = true
                    isiKoreksi.text = "Cek selisih gagal: ${pesanGalat(e)}"
                }
            }
        }.start()
    }

    private fun konfirmasiKoreksi() {
        val s = selisihTerakhir ?: return
        AlertDialog.Builder(this)
            .setTitle("Catat koreksi saldo?")
            .setMessage("Selisih Rp ${s.selisih} akan dicatat sebagai penyesuaian. Saldo dan modal aktif ikut berubah sebesar itu, hasil trading (profit) tidak berubah. Saldo asli diambil ulang saat dicatat.")
            .setPositiveButton("Catat") { _, _ -> terapkanKoreksi() }
            .setNegativeButton("Batal", null)
            .show()
    }

    private fun terapkanKoreksi() {
        val p = kunci.ambil() ?: return
        tombolKoreksi.isEnabled = false
        Thread {
            val pesan = try {
                val j = terapkanKoreksiSaldo(p.first, p.second, pathDb, "Koreksi manual")
                if (j == null) "Tidak ada selisih yang perlu dikoreksi." else "Koreksi dicatat: Rp $j."
            } catch (e: Exception) {
                "Koreksi gagal: ${pesanGalat(e)}"
            }
            selisihTerakhir = null
            runOnUiThread {
                tampilkanKoreksi(pesan)
                tampilkanProgress()
                tampilkanGrafik()
            }
        }.start()
    }

    private fun pesanGalat(e: Exception): String = (e as? ErrorApp.Gagal)?.pesan ?: (e.message ?: "Gagal")

    /** T2.2: tarik deposit/withdraw IDR (baca saja). Aman dijalankan saat pantauan hidup. */
    private fun jalankanArusKas(otomatis: Boolean) {
        val p = kunci.ambil()
        if (p == null) {
            if (!otomatis) isiProgress.text = "Kunci tersimpan tidak bisa dibuka. Masukkan ulang."
            return
        }
        if (arusKasBerjalan) return
        arusKasBerjalan = true
        tombolArusKas.isEnabled = false
        if (!otomatis) isiProgress.text = "Menarik riwayat deposit/withdraw..."
        Thread {
            val pesan = try {
                val h = sinkronArusKas(p.first, p.second, pathDb)
                "Arus kas: ${h.baru} baru, ${h.total} tersimpan, ${h.dilewati} dilewati (bukan sukses/IDR)."
            } catch (e: Exception) {
                "Arus kas gagal: ${pesanGalat(e)}"
            }
            runOnUiThread {
                arusKasBerjalan = false
                tombolArusKas.isEnabled = kunci.ujungKey() != null
                tampilkanProgress(pesan)
                tampilkanGrafik()
            }
        }.start()
    }

    /** Aplikasi dibuka saat pantauan hidup: sinkron trade dimatikan, jadi arus kas dilengkapi di sini. */
    private fun arusKasOtomatis() {
        if (kunci.ujungKey() == null || !pantauAktif() || arusKasBerjalan) return
        if (!prefs.getBoolean(PantauBersama.PREF_PERNAH_SINKRON, false)) return
        val sekarang = System.currentTimeMillis()
        if (sekarang - terakhirArusKasMs < 60_000) return
        terakhirArusKasMs = sekarang
        jalankanArusKas(true)
    }

    private fun ubahModeFee() {
        val ya = cekBersih.isChecked
        Thread {
            try {
                aturAmountSudahBersih(pathDb, ya)
                runOnUiThread { tampilkanProgress() }
            } catch (e: Exception) {
                runOnUiThread { isiProgress.text = "Mode fee tidak disimpan: ${pesanGalat(e)}" }
            }
        }.start()
    }

    private fun barisPencapaian(judul: String, p: PencapaianUi): String {
        val sb = StringBuilder()
        sb.append("$judul (sejak ${waktuWib(p.mulaiMs)})\n")
        sb.append("  pencapaian ${p.persen}%  hasil Rp ${p.profit}  setoran bersih Rp ${p.setoranBersih}\n")
        // variabel lokal: properti hasil binding tidak bisa di-smart-cast
        val target = p.targetPersen
        val progres = p.progressPersen
        if (target == null || progres == null) {
            sb.append("  target belum diatur\n")
        } else {
            val isi = (progres.toBigDecimalOrNull()?.toInt() ?: 0).coerceIn(0, 100) / 10
            sb.append("  target $target%  [${"█".repeat(isi)}${"░".repeat(10 - isi)}] $progres%")
            if (p.tercapai) sb.append("  TERCAPAI")
            sb.append("\n")
        }
        return sb.toString()
    }

    /** T2.3/T2.4: saldo versi aplikasi, pencapaian dan progress. Hitungan di Rust, tanpa jaringan. */
    private fun tampilkanProgress(awalan: String = "") {
        Thread {
            try {
                val r = hitungTarget(pathDb)
                val arus = daftarArusKas(pathDb, 5u)
                val t = StringBuilder()
                if (awalan.isNotEmpty()) t.append(awalan).append("\n\n")
                t.append("Saldo awal (snapshot ${waktuWib(r.snapshotAwalMs)}): Rp ${r.saldoAwal}\n")
                t.append("Modal aktif: Rp ${r.modalAktif}\n")
                t.append("Saldo versi aplikasi: Rp ${r.saldoSekarang} (hasil sejak awal: Rp ${r.selisihModal})\n\n")
                t.append(barisPencapaian("HARI INI", r.harian)).append("\n")
                t.append(barisPencapaian("MINGGU INI", r.mingguan)).append("\n")
                t.append("Arus kas tersimpan: ${r.jumlahArusKas}\n")
                for (a in arus) {
                    t.append("${waktuWib(a.waktuMs)} ${a.jenis.uppercase()} amount ${a.jumlah} fee ${a.biaya} -> saldo ${a.efekSaldo}\n")
                }
                runOnUiThread {
                    cekBersih.isChecked = r.amountSudahBersih
                    isiProgress.text = t.toString()
                }
            } catch (e: Exception) {
                val awal = if (awalan.isNotEmpty()) awalan + "\n\n" else ""
                runOnUiThread { isiProgress.text = awal + "Progress belum bisa dihitung: ${pesanGalat(e)}" }
            }
        }.start()
    }

    /** Isi kolom target dari database (bila sudah pernah disimpan). */
    private fun muatTarget() {
        Thread {
            try {
                val t = bacaTarget(pathDb)
                // variabel lokal: properti hasil binding bisa var, jadi tidak bisa di-smart-cast
                val h = t.harianPersen?.replace('.', ',')
                val m = t.mingguanPersen?.replace('.', ',')
                runOnUiThread {
                    h?.let { isiTargetHarian.setText(it) }
                    m?.let { isiTargetMingguan.setText(it) }
                    isiTarget.text = if (h == null || m == null) "Target belum diatur."
                    else "Tersimpan: harian $h%, mingguan $m%"
                }
            } catch (e: Exception) {
                val pesan = (e as? ErrorApp.Gagal)?.pesan ?: (e.message ?: "Gagal")
                runOnUiThread { isiTarget.text = "Gagal membaca target: $pesan" }
            }
        }.start()
    }

    private fun simpanTargetUi() {
        val h = isiTargetHarian.text.toString()
        val m = isiTargetMingguan.text.toString()
        Thread {
            try {
                simpanTarget(pathDb, h, m)
                runOnUiThread { muatTarget() }
            } catch (e: Exception) {
                val pesan = (e as? ErrorApp.Gagal)?.pesan ?: (e.message ?: "Gagal")
                runOnUiThread { isiTarget.text = "Target tidak disimpan: $pesan" }
            }
        }.start()
    }

    private fun segarkan() {
        val ujung = kunci.ujungKey()
        status.text = if (ujung != null) "Kunci tersimpan (berakhiran $ujung)" else "Belum ada kunci"
        tombolTes.isEnabled = ujung != null
        tombolHapus.isEnabled = ujung != null
        tombolSinkron.isEnabled = bolehSinkron()
        tombolArusKas.isEnabled = ujung != null && !arusKasBerjalan
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
        arusKasOtomatis()
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
        // Notifikasi TP/SL butuh setelan persen: simpan dari kolom TP%/SL% (tidak menambah catatan bila sama).
        val tp = isiTp.text.toString()
        val sl = isiSl.text.toString()
        info.text = "Menyimpan setelan TP/SL..."
        Thread {
            try {
                simpanSetelan(pathDb, tp, sl)
                runOnUiThread {
                    LayananLatar.mulai(this)
                    info.text = "Memulai pantauan (TP ${tp}% / SL ${sl}%)..."
                    segarkan()
                }
            } catch (e: Exception) {
                val pesan = (e as? ErrorApp.Gagal)?.pesan ?: (e.message ?: "Gagal")
                runOnUiThread { info.text = "Setelan TP/SL tidak valid: $pesan" }
            }
        }.start()
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
        @Volatile var terakhirArusKasMs = 0L
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
                if (h.snapshotAwalDibuat) ringkas.append("Snapshot saldo awal dibuat.\n")
                // T2.2: lengkapi arus kas (deposit/withdraw). Kegagalan di sini tidak membatalkan sinkron trade.
                if (!arusKasBerjalan) {
                    arusKasBerjalan = true
                    try {
                        val a = sinkronArusKas(p.first, p.second, pathDb)
                        ringkas.append("Arus kas: ${a.baru} baru, ${a.total} tersimpan.")
                    } catch (e: Exception) {
                        ringkas.append("Arus kas gagal: ${pesanGalat(e)}")
                    } finally {
                        arusKasBerjalan = false
                    }
                }
                runOnUiThread {
                    sinkronBerjalan = false
                    sibuk(false)
                    info.text = ringkas.toString()
                    tampilkanOrder()
                    segarkan()
                    tampilkanProgress()
                    tampilkanRiwayat()
                    tampilkanStatistik()
                    tampilkanGrafik()
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
