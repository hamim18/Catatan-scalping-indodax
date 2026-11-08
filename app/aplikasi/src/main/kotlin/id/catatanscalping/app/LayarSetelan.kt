package id.catatanscalping.app

import android.Manifest
import android.app.AlertDialog
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.PowerManager
import android.provider.Settings
import android.view.Gravity
import android.view.View
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import uniffi.scalping_ffi.StatusKoneksi
import uniffi.scalping_ffi.bacaSetelan
import uniffi.scalping_ffi.bacaTarget
import uniffi.scalping_ffi.cekKoneksi
import uniffi.scalping_ffi.daftarArusKas
import uniffi.scalping_ffi.simpanSetelan
import uniffi.scalping_ffi.simpanTarget
import uniffi.scalping_ffi.versiCore

/**
 * Layar Setelan (prototype): TP/SL per trade, target periode, koneksi Indodax (kunci, pantauan realtime,
 * sinkron), deposit/withdraw. Hanya tampilan; validasi dan penyimpanan dilakukan Rust.
 */
class LayarSetelan(a: MainActivity) : Layar(a) {
    private val isiTp = ui.input("contoh 1").apply { setText("1") }
    private val isiSl = ui.input("contoh 1").apply { setText("1") }
    private val isiHarian = ui.input("contoh 1,5")
    private val isiMingguan = ui.input("contoh 5")
    private val infoSimpan = ui.mute("")

    private val nilaiKunci = ui.teks("", 14f, tebal = true)
    private val nilaiPantau = ui.teks("", 14f, tebal = true)
    private val statusPantau = ui.mute("", 12f)
    private val infoKoneksi = ui.mute("")
    private val formKunci = LinearLayout(a)
    private val isiKey = ui.input("API key", angka = false, rahasia = true)
    private val isiSecret = ui.input("API secret", angka = false, rahasia = true)
    private lateinit var tombolSinkron: Button
    private lateinit var tombolGanti: Button
    private lateinit var tombolPantau: Button
    private lateinit var tombolBaterai: Button
    private lateinit var tombolSimpanKunci: Button
    private lateinit var tombolTes: Button
    private lateinit var tombolHapus: Button
    private var formTerbuka = false

    private val infoArusKas = ui.mono("")
    private lateinit var tombolArusKas: Button
    private lateinit var tombolEkspor: Button
    private lateinit var tombolImpor: Button
    private val infoCadangan = ui.mute("", 12f)
    private val teksLog = ui.mono("")

    init {
        header("Setelan")

        // --- Target per trade ---
        val kTrade = ui.kartu()
        kTrade.tambah(ui.teks("Target per trade", 14f, tebal = true))
        kTrade.tambah(ui.teks("Take profit (TP)", 13f, tebal = true), atas = 14)
        kTrade.tambah(barisPersen(isiTp), atas = 6)
        kTrade.tambah(ui.teks("Stop loss (SL)", 13f, tebal = true), atas = 14)
        kTrade.tambah(barisPersen(isiSl), atas = 6)
        kTrade.tambah(
            ui.mute("TP dan SL dihitung dari modal efektif, yaitu harga beli ditambah fee beli. Perubahan berlaku untuk posisi yang dibuka sesudahnya.", 12f),
            atas = 10,
        )
        isi.tambah(kTrade)

        // --- Target periode ---
        val kPeriode = ui.kartu()
        kPeriode.tambah(ui.teks("Target periode", 14f, tebal = true))
        kPeriode.tambah(ui.teks("Target harian", 13f, tebal = true), atas = 14)
        kPeriode.tambah(barisPersen(isiHarian), atas = 6)
        kPeriode.tambah(ui.teks("Target mingguan", 13f, tebal = true), atas = 14)
        kPeriode.tambah(barisPersen(isiMingguan), atas = 6)
        kPeriode.tambah(
            ui.mute("Persen dari saldo awal periode. Hari mulai 00.00 WIB, minggu mulai Senin. Kosongkan keduanya bila belum ingin memakai target.", 12f),
            atas = 10,
        )
        isi.tambah(kPeriode, atas = 12)

        isi.tambah(ui.tombol("Simpan setelan", primer = true) { simpanSemua() }, atas = 12)
        isi.tambah(infoSimpan, atas = 8)

        // --- Koneksi Indodax ---
        val kKoneksi = ui.kartu()
        kKoneksi.tambah(ui.teks("Koneksi Indodax", 14f, tebal = true))
        kKoneksi.tambah(ui.baris(ui.teks("Kunci API (izin baca saja)", 14f), nilaiKunci), atas = 10)
        kKoneksi.tambah(ui.baris(ui.teks("Pantau order realtime", 14f), nilaiPantau), atas = 8)
        kKoneksi.tambah(statusPantau, atas = 4)

        tombolSinkron = ui.tombol("Sinkron sekarang") { sinkronManual() }
        tombolGanti = ui.tombol("Ganti kunci") { formTerbuka = !formTerbuka; segarkanStatus() }
        kKoneksi.tambah(barisTombol(tombolSinkron, tombolGanti), atas = 12)

        tombolPantau = ui.tombol("Mulai pantauan") { togglePantau() }
        tombolBaterai = ui.tombol("Matikan optimasi baterai") { mintaAbaikanBaterai() }
        kKoneksi.tambah(barisTombol(tombolPantau, tombolBaterai), atas = 8)

        // Formulir kunci: tampil bila belum ada kunci atau setelah "Ganti kunci".
        formKunci.orientation = LinearLayout.VERTICAL
        formKunci.tambah(ui.mute("Gunakan key TAPI v2 dengan izin BACA SAJA. Aplikasi menolak key yang punya izin trade atau withdraw.", 12f))
        formKunci.tambah(isiKey, atas = 8)
        formKunci.tambah(isiSecret, atas = 8)
        tombolSimpanKunci = ui.tombol("Tes dan simpan kunci", primer = true) { simpanKunciBaru() }
        tombolTes = ui.tombol("Tes kunci tersimpan") { tesTersimpan() }
        tombolHapus = ui.tombol("Hapus kunci") { konfirmasiHapus() }
        formKunci.tambah(tombolSimpanKunci, atas = 10)
        formKunci.tambah(barisTombol(tombolTes, tombolHapus), atas = 8)
        kKoneksi.tambah(formKunci, atas = 14)

        kKoneksi.tambah(infoKoneksi, atas = 10)
        isi.tambah(kKoneksi, atas = 12)

        // --- Deposit dan withdraw ---
        val kArus = ui.kartu()
        kArus.tambah(ui.teks("Deposit dan withdraw IDR", 14f, tebal = true))
        kArus.tambah(
            ui.mute("Hanya riwayat (daftarnya ada di layar Riwayat); tidak memengaruhi total aset, target, atau grafik. Dilengkapi otomatis; tombol ini untuk memaksa.", 12f),
            atas = 6,
        )
        tombolArusKas = ui.tombol("Sinkron deposit/withdraw") { sinkronArusKasManual() }
        kArus.tambah(tombolArusKas, atas = 10)
        kArus.tambah(infoArusKas, atas = 6)
        isi.tambah(kArus, atas = 12)

        // --- Cadangan data (U5) ---
        val kCadangan = ui.kartu()
        kCadangan.tambah(ui.teks("Cadangan data", 14f, tebal = true))
        kCadangan.tambah(
            ui.mute("Simpan atau pulihkan catatan trade ke penyimpanan HP (berkas .db). Kunci API tidak ikut dicadangkan; setelah impor di HP baru, masukkan kunci lagi.", 12f),
            atas = 6,
        )
        tombolEkspor = ui.tombol("Ekspor data") { Cadangan.mintaEkspor(a) }
        tombolImpor = ui.tombol("Impor data") { Cadangan.mintaImpor(a) }
        kCadangan.tambah(barisTombol(tombolEkspor, tombolImpor), atas = 10)
        kCadangan.tambah(infoCadangan, atas = 8)
        isi.tambah(kCadangan, atas = 12)

        // --- Log pantauan ---
        val kLog = ui.kartu()
        kLog.tambah(ui.teks("Log pantauan", 14f, tebal = true))
        kLog.tambah(teksLog, atas = 8)
        isi.tambah(kLog, atas = 12)

        isi.tambah(ui.mute("Inti Rust versi ${versiCore()}", 12f).apply { gravity = Gravity.CENTER }, atas = 16)

        muatNilaiTersimpan()
        segarkanStatus()
    }

    // ---------- bangun tampilan ----------

    private fun barisPersen(e: EditText): LinearLayout = LinearLayout(a).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        tambah(e, bobot = 1f, lebar = 0)
        tambah(ui.teks("%", 16f, t.mute), kiri = 8, lebar = WRAP_CONTENT)
    }

    private fun barisTombol(kiri: Button, kanan: Button): LinearLayout = LinearLayout(a).apply {
        orientation = LinearLayout.HORIZONTAL
        tambah(kiri, bobot = 1f, lebar = 0, kanan = 4)
        tambah(kanan, bobot = 1f, lebar = 0, kiri = 4)
    }

    // ---------- memuat dan menyegarkan ----------

    /** Isi kolom dari database (sekali, saat layar dibuat). */
    private fun muatNilaiTersimpan() {
        val db = a.pathDb
        Latar.baca({ Pair(bacaSetelan(db), bacaTarget(db)) }) { r ->
            r.onSuccess { (s, tg) ->
                s.tpPersen?.let { isiTp.setText(Format.komaDesimal(it)) }
                s.slPersen?.let { isiSl.setText(Format.komaDesimal(it)) }
                tg.harianPersen?.let { isiHarian.setText(Format.komaDesimal(it)) }
                tg.mingguanPersen?.let { isiMingguan.setText(Format.komaDesimal(it)) }
            }
            r.onFailure { infoSimpan.text = "Gagal membaca setelan: ${Latar.pesan(it)}" }
        }
    }

    override fun segarkan() {
        segarkanStatus()
        muatArusKas()
    }

    override fun pantauBerubah() = segarkanStatus()

    private fun abaikanBaterai(): Boolean =
        a.getSystemService(PowerManager::class.java).isIgnoringBatteryOptimizations(a.packageName)

    /** Status kunci, pantauan dan keadaan tombol. Tidak memuat data berat. */
    private fun segarkanStatus() {
        val ujung = a.kunci.ujungKey()
        val punyaKunci = ujung != null
        val pantau = Sinkron.pantauAktif(a)

        nilaiKunci.text = if (punyaKunci) "Aktif · …$ujung" else "Belum ada"
        nilaiKunci.setTextColor(if (punyaKunci) t.gain else t.loss)
        nilaiPantau.text = if (pantau) "Berjalan" else "Mati"
        nilaiPantau.setTextColor(if (pantau) t.gain else t.mute)
        statusPantau.text = if (pantau) PantauBersama.status else "Pantauan mati"

        tombolPantau.text = if (pantau) "Hentikan pantauan" else "Mulai pantauan"
        tombolPantau.aktif(punyaKunci)
        tombolSinkron.text = if (pantau) "Sinkron (matikan pantauan dulu)" else "Sinkron sekarang"
        tombolSinkron.aktif(Sinkron.bolehSinkron(a))
        tombolGanti.text = if (formTerbuka && punyaKunci) "Tutup formulir" else "Ganti kunci"
        tombolGanti.aktif(punyaKunci)

        val bateraiAman = abaikanBaterai()
        tombolBaterai.text = if (bateraiAman) "Baterai: sudah dimatikan" else "Matikan optimasi baterai"
        tombolBaterai.aktif(!bateraiAman)

        formKunci.visibility = if (!punyaKunci || formTerbuka) View.VISIBLE else View.GONE
        tombolTes.aktif(punyaKunci)
        tombolHapus.aktif(punyaKunci)
        tombolArusKas.aktif(punyaKunci && !Sinkron.arusKasBerjalan)
        tombolImpor.aktif(!pantau && !Sinkron.sedangSinkron() && !Sinkron.imporBerjalan)
        teksLog.text = PantauBersama.logTeks()
    }

    private fun muatArusKas() {
        val db = a.pathDb
        Latar.baca({ daftarArusKas(db, 1000u).size }) { r ->
            r.onSuccess { n ->
                infoArusKas.text =
                    if (n == 0) "(belum ada deposit/withdraw tercatat)" else "$n tercatat. Lihat daftarnya di layar Riwayat."
            }
            r.onFailure { infoArusKas.text = "Gagal membaca arus kas: ${Latar.pesan(it)}" }
        }
    }

    // ---------- simpan setelan ----------

    private fun simpanSemua() {
        val tp = isiTp.text.toString()
        val sl = isiSl.text.toString()
        val harian = isiHarian.text.toString().trim()
        val mingguan = isiMingguan.text.toString().trim()
        if (tp.isBlank() || sl.isBlank()) {
            infoSimpan.text = "Isi TP dan SL dulu."
            return
        }
        if (harian.isEmpty() != mingguan.isEmpty()) {
            infoSimpan.text = "Isi target harian dan mingguan sekaligus, atau kosongkan keduanya."
            return
        }
        infoSimpan.text = "Menyimpan..."
        val db = a.pathDb
        Latar.baca({
            simpanSetelan(db, tp, sl)
            if (harian.isNotEmpty()) simpanTarget(db, harian, mingguan)
            Unit
        }) { r ->
            r.onSuccess {
                infoSimpan.text = "Setelan tersimpan."
                a.dataBerubah()
            }
            r.onFailure { infoSimpan.text = "Tidak disimpan: ${Latar.pesan(it)}" }
        }
    }

    // ---------- cadangan data (U5) ----------

    fun cadanganEkspor(uri: Uri) {
        infoCadangan.text = "Menyalin data..."
        Cadangan.ekspor(a, uri) { infoCadangan.text = it }
    }

    fun cadanganImpor(uri: Uri) {
        if (Sinkron.pantauAktif(a)) {
            infoCadangan.text = "Hentikan pantauan dulu sebelum impor."
            return
        }
        if (Sinkron.sedangSinkron() || Sinkron.imporBerjalan) {
            infoCadangan.text = "Tunggu sinkron selesai dulu sebelum impor."
            return
        }
        infoCadangan.text = "Memeriksa berkas..."
        Cadangan.impor(a, uri) { pesan, berhasil ->
            infoCadangan.text = pesan
            if (berhasil) {
                muatNilaiTersimpan()
                a.dataBerubah()
            }
            segarkanStatus()
        }
    }

    // ---------- sinkron ----------

    private fun sinkronManual() {
        if (!Sinkron.bolehSinkron(a)) return
        val mulai = Sinkron.sinkronTrade(a, false) { h ->
            infoKoneksi.text = h.pesan
            segarkanStatus()
            a.dataBerubah()
        }
        if (mulai) {
            infoKoneksi.text = "Sinkron berjalan, bisa beberapa puluh detik..."
            segarkanStatus()
        } else {
            infoKoneksi.text = "Kunci tersimpan tidak bisa dibuka. Masukkan ulang."
        }
    }

    private fun sinkronArusKasManual() {
        val mulai = Sinkron.jalankanArusKas(a) { h ->
            infoKoneksi.text = h.pesan
            segarkanStatus()
            a.dataBerubah()
        }
        if (mulai) {
            infoKoneksi.text = "Menarik riwayat deposit/withdraw..."
            segarkanStatus()
        } else if (Sinkron.imporBerjalan) {
            infoKoneksi.text = "Impor data sedang berjalan. Coba lagi sebentar."
        } else if (!Sinkron.arusKasBerjalan) {
            infoKoneksi.text = "Kunci tersimpan tidak bisa dibuka. Masukkan ulang."
        }
    }


    // ---------- pantauan realtime ----------

    private fun togglePantau() {
        if (Sinkron.pantauAktif(a)) {
            LayananLatar.henti(a)
            infoKoneksi.text = "Pantauan dihentikan."
            segarkanStatus()
            return
        }
        if (Sinkron.imporBerjalan) {
            infoKoneksi.text = "Impor data sedang berjalan. Mulai pantauan setelah selesai."
            return
        }
        if (a.kunci.ambil() == null) {
            infoKoneksi.text = "Kunci tersimpan tidak bisa dibuka. Masukkan ulang."
            return
        }
        // Snapshot saldo awal dibuat oleh sinkron pertama; pantauan sebelum itu membuat data tidak konsisten.
        if (!Sinkron.pernahSinkron(a)) {
            infoKoneksi.text = "Jalankan \"Sinkron sekarang\" satu kali dulu, baru mulai pantauan."
            return
        }
        if (Build.VERSION.SDK_INT >= 33 &&
            a.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
        ) {
            a.requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), MainActivity.KODE_NOTIF)
            return
        }
        mulaiPantau()
    }

    private fun mulaiPantau() {
        // Notifikasi TP/SL butuh setelan persen: simpan dari kolom TP/SL (tidak menambah catatan bila sama).
        val tp = isiTp.text.toString()
        val sl = isiSl.text.toString()
        if (tp.isBlank() || sl.isBlank()) {
            infoKoneksi.text = "Isi TP dan SL dulu, lalu mulai pantauan."
            return
        }
        infoKoneksi.text = "Menyimpan setelan TP/SL..."
        val db = a.pathDb
        Latar.baca({ simpanSetelan(db, tp, sl) }) { r ->
            r.onSuccess {
                LayananLatar.mulai(a)
                infoKoneksi.text = "Memulai pantauan (TP ${tp}% / SL ${sl}%)..."
                segarkanStatus()
            }
            r.onFailure { infoKoneksi.text = "Setelan TP/SL tidak valid: ${Latar.pesan(it)}" }
        }
    }

    /** Hasil permintaan izin notifikasi dari MainActivity. */
    fun izinNotifSelesai(diberi: Boolean) {
        if (!diberi) {
            infoKoneksi.text = "Izin notifikasi ditolak: pantauan tetap jalan, tapi notifikasinya tidak terlihat."
        }
        mulaiPantau()
    }

    private fun mintaAbaikanBaterai() {
        if (abaikanBaterai()) return
        try {
            a.startActivity(
                Intent(Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS, Uri.parse("package:${a.packageName}"))
            )
        } catch (e: Exception) {
            // Sebagian HP tidak punya dialog ini; buka daftar pengaturan baterai.
            a.startActivity(Intent(Settings.ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS))
        }
    }

    // ---------- kunci API ----------

    private fun sibuk(ya: Boolean) {
        tombolSimpanKunci.aktif(!ya)
        if (ya) {
            tombolTes.aktif(false)
            tombolHapus.aktif(false)
            infoKoneksi.text = "Menghubungi Indodax..."
        } else {
            segarkanStatus()
        }
    }

    private fun jalankanTes(k: String, s: String, simpanBila: Boolean) {
        sibuk(true)
        Latar.jaringan.execute {
            val hasil = runCatching { cekKoneksi(k, s) }
            Latar.ui { selesaiTes(hasil, k, s, simpanBila) }
        }
    }

    private fun selesaiTes(hasil: Result<StatusKoneksi>, k: String, s: String, simpanBila: Boolean) {
        sibuk(false)
        val h = hasil.getOrElse {
            infoKoneksi.text = "Gagal: ${Latar.pesan(it)}"
            return
        }
        val t = StringBuilder()
        t.append("Tersambung. Jenis akun: ${h.jenisAkun}\n")
        t.append("Jumlah aset di daftar saldo: ${h.jumlahAset}\n")
        t.append("Aset IDR ada: ${if (h.idrAda) "ya" else "TIDAK"}\n")
        if (h.bacaSaja) {
            t.append("Izin key: baca saja (benar).")
            if (simpanBila) {
                a.kunci.simpan(k, s)
                isiKey.setText("")
                isiSecret.setText("")
                formTerbuka = false
                t.append("\nKunci disimpan terenkripsi.")
            }
        } else {
            t.append("PERINGATAN: key ini punya izin trade/withdraw. Kunci TIDAK disimpan. Buat key baru dengan izin BACA SAJA.")
        }
        infoKoneksi.text = t.toString()
        segarkanStatus()
        a.dataBerubah()
    }

    private fun simpanKunciBaru() {
        val k = isiKey.text.toString().trim()
        val s = isiSecret.text.toString().trim()
        if (k.isEmpty() || s.isEmpty()) {
            infoKoneksi.text = "Isi API key dan API secret dulu."
            return
        }
        jalankanTes(k, s, simpanBila = true)
    }

    private fun tesTersimpan() {
        val p = a.kunci.ambil()
        if (p == null) {
            infoKoneksi.text = "Kunci tersimpan tidak bisa dibuka. Masukkan ulang."
            return
        }
        jalankanTes(p.first, p.second, simpanBila = false)
    }

    private fun konfirmasiHapus() {
        AlertDialog.Builder(a)
            .setTitle("Hapus kunci?")
            .setMessage("Kunci dihapus dari perangkat dan pantauan dihentikan. Catatan trade tidak ikut terhapus.")
            .setPositiveButton("Hapus") { _, _ ->
                if (Sinkron.pantauAktif(a)) LayananLatar.henti(a) // tanpa kunci pantauan tidak bisa jalan
                a.kunci.hapus()
                infoKoneksi.text = "Kunci dihapus dari perangkat."
                formTerbuka = false
                segarkanStatus()
                a.dataBerubah()
            }
            .setNegativeButton("Batal", null)
            .show()
    }
}
