package id.catatanscalping.app

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * Menyimpan API key dan secret terenkripsi (AES-GCM, kunci di Android Keystore).
 * Tidak ada logika bisnis di sini; hanya penyimpanan.
 */
class KunciAman(context: Context) {
    private val prefs = context.getSharedPreferences("kunci_aman", Context.MODE_PRIVATE)

    private fun kunciAes(): SecretKey {
        val ks = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (ks.getKey(ALIAS, null) as? SecretKey)?.let { return it }
        val gen = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
        gen.init(
            KeyGenParameterSpec.Builder(
                ALIAS,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .build()
        )
        return gen.generateKey()
    }

    private fun enkripsi(teks: String): String {
        val c = Cipher.getInstance(TRANSFORMASI)
        c.init(Cipher.ENCRYPT_MODE, kunciAes())
        val data = c.doFinal(teks.toByteArray(Charsets.UTF_8))
        return Base64.encodeToString(c.iv, Base64.NO_WRAP) + ":" +
            Base64.encodeToString(data, Base64.NO_WRAP)
    }

    private fun dekripsi(tersimpan: String?): String? {
        if (tersimpan == null) return null
        return try {
            val (iv, data) = tersimpan.split(":", limit = 2)
            val c = Cipher.getInstance(TRANSFORMASI)
            c.init(
                Cipher.DECRYPT_MODE,
                kunciAes(),
                GCMParameterSpec(128, Base64.decode(iv, Base64.NO_WRAP))
            )
            String(c.doFinal(Base64.decode(data, Base64.NO_WRAP)), Charsets.UTF_8)
        } catch (e: Exception) {
            null
        }
    }

    fun simpan(key: String, secret: String) {
        prefs.edit()
            .putString("key", enkripsi(key))
            .putString("secret", enkripsi(secret))
            .apply()
    }

    /** Pasangan (key, secret), atau null bila belum ada / tidak bisa dibuka. */
    fun ambil(): Pair<String, String>? {
        val k = dekripsi(prefs.getString("key", null)) ?: return null
        val s = dekripsi(prefs.getString("secret", null)) ?: return null
        return k to s
    }

    /** Empat karakter terakhir key, untuk ditampilkan. Secret tidak pernah ditampilkan. */
    fun ujungKey(): String? = ambil()?.first?.takeLast(4)

    fun hapus() {
        prefs.edit().clear().apply()
    }

    private companion object {
        const val ALIAS = "catatan_scalping_kunci"
        const val TRANSFORMASI = "AES/GCM/NoPadding"
    }
}
