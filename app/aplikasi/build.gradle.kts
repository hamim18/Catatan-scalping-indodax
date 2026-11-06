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
        versionCode = 1
        versionName = "0.1.0"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
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
