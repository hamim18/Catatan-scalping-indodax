package id.catatanscalping.app

import android.content.Context
import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.graphics.drawable.RippleDrawable
import android.text.Editable
import android.text.InputType
import android.text.TextWatcher
import android.text.method.DigitsKeyListener
import android.view.Gravity
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.Button
import android.widget.CheckBox
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView

fun Context.dp(n: Int): Int = (n * resources.displayMetrics.density).toInt()

fun Int.denganAlpha(a: Int): Int = (this and 0x00FFFFFF) or (a shl 24)

/** Tambah anak ke LinearLayout dengan ukuran dan jarak (dalam dp). Mengembalikan anak itu. */
fun <T : View> LinearLayout.tambah(
    v: T,
    atas: Int = 0,
    bawah: Int = 0,
    kiri: Int = 0,
    kanan: Int = 0,
    bobot: Float = 0f,
    lebar: Int = MATCH_PARENT,
    tinggi: Int = WRAP_CONTENT,
): T {
    val p = LinearLayout.LayoutParams(lebar, tinggi, bobot)
    p.setMargins(context.dp(kiri), context.dp(atas), context.dp(kanan), context.dp(bawah))
    addView(v, p)
    return v
}

/** Tombol nonaktif tampak pudar. Pakai ini, bukan isEnabled langsung. */
fun Button.aktif(ya: Boolean) {
    isEnabled = ya
    alpha = if (ya) 1f else 0.45f
}

/**
 * T10.6: kolom nominal (harga, volume, modal) menampilkan titik pemisah ribuan saat diketik:
 * 2000000000 menjadi 2.000.000.000. Koma tetap pemisah desimal; titik selalu pemisah ribuan.
 * Pakai `teksAngka()` untuk membaca nilainya (titik dibuang) sebelum dikirim ke Rust.
 */
fun EditText.ribuan(): EditText {
    // Pendengar tombol bawaan angka-desimal hanya menerima koma (locale id); titik perlu diterima juga.
    @Suppress("DEPRECATION")
    keyListener = object : DigitsKeyListener() {
        override fun getAcceptedChars(): CharArray = "0123456789.,".toCharArray()
    }
    setRawInputType(InputType.TYPE_CLASS_NUMBER or InputType.TYPE_NUMBER_FLAG_DECIMAL)
    addTextChangedListener(object : TextWatcher {
        private var sibuk = false
        override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) {}
        override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) {}
        override fun afterTextChanged(e: Editable) {
            if (sibuk) return
            val mentah = e.toString()
            val hasil = formatRibuan(mentah)
            if (hasil == mentah) return
            val posisi = selectionStart.coerceIn(0, mentah.length)
            val penting = mentah.substring(0, posisi).count { it.isDigit() || it == ',' }
            sibuk = true
            e.replace(0, e.length, hasil)
            var n = 0
            var idx = 0
            while (idx < hasil.length && n < penting) {
                if (hasil[idx].isDigit() || hasil[idx] == ',') n++
                idx++
            }
            setSelection(idx.coerceIn(0, hasil.length))
            sibuk = false
        }
    })
    return this
}

/** "2000000" menjadi "2.000.000"; "1234,5" menjadi "1.234,5". Titik di masukan dianggap sisa pemisah ribuan. */
fun formatRibuan(masuk: String): String {
    val koma = masuk.indexOf(',')
    val bulat = (if (koma >= 0) masuk.substring(0, koma) else masuk).filter { it.isDigit() }
    val desimal = if (koma >= 0) masuk.substring(koma + 1).filter { it.isDigit() } else null
    val sb = StringBuilder()
    for ((i, c) in bulat.withIndex()) {
        if (i > 0 && (bulat.length - i) % 3 == 0) sb.append('.')
        sb.append(c)
    }
    if (desimal != null) sb.append(',').append(desimal)
    return sb.toString()
}

/** Nilai kolom nominal tanpa titik ribuan (koma desimal tetap), siap dikirim ke Rust. */
fun EditText.teksAngka(): String = text.toString().trim().replace(".", "")

/** Pabrik komponen bertema (kartu, tombol, kolom isian, dan seterusnya). Hanya tampilan. */
class Ui(val ctx: Context, val t: Tema) {
    fun dp(n: Int): Int = ctx.dp(n)

    fun bulat(isi: Int, garis: Int? = null, radius: Int = 14): GradientDrawable =
        GradientDrawable().apply {
            setColor(isi)
            cornerRadius = dp(radius).toFloat()
            if (garis != null) setStroke(dp(1), garis)
        }

    fun teks(isi: CharSequence, ukuran: Float = 14f, warna: Int = t.ink, tebal: Boolean = false): TextView =
        TextView(ctx).apply {
            text = isi
            textSize = ukuran
            setTextColor(warna)
            if (tebal) typeface = Typeface.create(Typeface.DEFAULT, 700, false)
            fontFeatureSettings = "tnum" // angka lebar sama, kolom rata
            setLineSpacing(0f, 1.12f)
        }

    fun mute(isi: CharSequence, ukuran: Float = 13f): TextView = teks(isi, ukuran, t.mute)

    fun mono(isi: CharSequence, ukuran: Float = 12f): TextView = teks(isi, ukuran, t.mute).apply {
        typeface = Typeface.MONOSPACE
        fontFeatureSettings = null
    }

    fun kartu(): LinearLayout = LinearLayout(ctx).apply {
        orientation = LinearLayout.VERTICAL
        background = bulat(t.card, t.line, 14)
        setPadding(dp(16), dp(16), dp(16), dp(16))
    }

    fun judulBagian(isi: String): TextView = teks(isi, 15f, tebal = true)

    fun gayaTombol(b: Button, primer: Boolean) {
        b.setTextColor(if (primer) t.teksAksen else t.ink)
        val isi = if (primer) t.accent else t.card
        val garis = if (primer) t.accent else t.line
        b.background = RippleDrawable(
            ColorStateList.valueOf(t.ink.denganAlpha(0x22)),
            bulat(isi, garis, 10),
            bulat(Color.WHITE, null, 10),
        )
    }

    fun tombol(label: String, primer: Boolean = false, klik: () -> Unit): Button = Button(ctx).apply {
        text = label
        isAllCaps = false
        textSize = 14f
        typeface = Typeface.create(Typeface.DEFAULT, 700, false)
        stateListAnimator = null
        minHeight = 0
        minimumHeight = 0
        setPadding(dp(12), dp(10), dp(12), dp(10))
        gayaTombol(this, primer)
        setOnClickListener { klik() }
    }

    /** Kolom isian. `angka` = desimal (koma atau titik), `rahasia` = teks disamarkan untuk key dan secret. */
    fun input(petunjuk: String, angka: Boolean = true, rahasia: Boolean = false): EditText = EditText(ctx).apply {
        hint = petunjuk
        setHintTextColor(t.mute)
        setTextColor(t.ink)
        textSize = 16f
        inputType = when {
            angka -> InputType.TYPE_CLASS_NUMBER or InputType.TYPE_NUMBER_FLAG_DECIMAL
            rahasia -> InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD
            else -> InputType.TYPE_CLASS_TEXT
        }
        setSingleLine()
        importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_NO
        background = bulat(t.soft, t.line, 10)
        setPadding(dp(12), dp(10), dp(12), dp(10))
    }

    fun cek(label: String): CheckBox = CheckBox(ctx).apply {
        text = label
        textSize = 13f
        setTextColor(t.ink)
        buttonTintList = ColorStateList.valueOf(t.accent)
    }

    fun pill(isi: String, positif: Boolean?): TextView = teks(
        isi, 13f,
        when (positif) { true -> t.gain; false -> t.loss; null -> t.mute },
        tebal = true,
    ).apply {
        background = bulat(
            when (positif) { true -> t.gainBg; false -> t.lossBg; null -> t.soft },
            null, 8,
        )
        setPadding(dp(8), dp(3), dp(8), dp(3))
    }

    /** Batang progress 0..100 (di luar rentang dipotong). Beri tinggi 10dp lewat `tambah(..., tinggi = dp(10))`. */
    fun bar(persen: Int, warna: Int = t.accent): LinearLayout {
        val p = persen.coerceIn(0, 100)
        return LinearLayout(ctx).apply {
            orientation = LinearLayout.HORIZONTAL
            weightSum = 100f
            background = bulat(t.soft, null, 6)
            addView(
                View(ctx).apply { background = bulat(warna, null, 6) },
                LinearLayout.LayoutParams(0, MATCH_PARENT, p.toFloat()),
            )
        }
    }

    /** Dua teks sejajar: kiri sesuai isi, kanan mengisi sisa dan rata kanan. */
    fun baris(kiri: View, kanan: View): LinearLayout = LinearLayout(ctx).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        tambah(kiri, lebar = WRAP_CONTENT)
        tambah(kanan, kiri = 8, bobot = 1f, lebar = 0)
        if (kanan is TextView) kanan.gravity = Gravity.END
    }

    /** Indikator kecil (titik berwarna + teks) seperti "Terhubung · hanya baca" di prototype. */
    inner class Chip {
        private val titik = View(ctx)
        private val label = teks("", 12f, t.mute)
        val root: LinearLayout = LinearLayout(ctx).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            background = bulat(t.card, t.line, 99)
            setPadding(dp(10), dp(4), dp(10), dp(4))
            tambah(titik, kanan = 6, lebar = dp(8), tinggi = dp(8))
            tambah(label, lebar = WRAP_CONTENT)
        }

        /** `warnaTitik` null = tanpa titik. */
        fun atur(isi: String, warnaTitik: Int?) {
            label.text = isi
            if (warnaTitik == null) {
                titik.visibility = View.GONE
            } else {
                titik.visibility = View.VISIBLE
                titik.background = GradientDrawable().apply { shape = GradientDrawable.OVAL; setColor(warnaTitik) }
            }
        }
    }
}
