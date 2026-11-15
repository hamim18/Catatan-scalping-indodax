plugins {
    id("com.android.application")
}

android {
    namespace = "id.catatanscalping.app"
    compileSdk = 36
    ndkVersion = "28.2.13676358"

    defaultConfig {
        applicationId = "id.catatanscalping.app"
        minSdk = 30
        targetSdk = 36
        versionCode = 3
        versionName = "0.9.0-uji"
    }

    buildTypes {
        release {
            // R8 sengaja mati: JNA dan UniFFI memakai refleksi.
            isMinifyEnabled = false
            // Rilis uji pribadi: pakai kunci debug bawaan AGP (sama dengan APK 0.8.0-uji)
            // supaya APK baru bisa menimpa yang lama tanpa uninstall (data tidak hilang).
            signingConfig = signingConfigs.getByName("debug")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    // Dibutuhkan oleh kode Kotlin hasil UniFFI.
    implementation("net.java.dev.jna:jna:5.17.0@aar")
}
