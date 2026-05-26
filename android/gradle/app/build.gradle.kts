// App module build.gradle.kts for the GPUI Mobile Android Example.
//
// This module packages the pre-compiled Rust native library into an APK
// that uses Android's NativeActivity to host the GPUI application.
//
// The Rust library must be compiled separately and placed into
// app/src/main/jniLibs/<abi>/ before building the APK.
//
// Quick start:
//   cd <repo-root>
//   cargo ndk -t arm64-v8a -o example/android_app/gradle/app/src/main/jniLibs \
//       build --example android_app --release
//   cd example/android_app/gradle
//   ./gradlew assembleDebug

plugins {
    id("com.android.application")
}

android {
    namespace = "dev.gpui.portfolio.forecast"
    compileSdk = 34

    defaultConfig {
        applicationId = "dev.gpui.portfolio.forecast"
        minSdk = 26          // Vulkan 1.0 is mandatory from API 26+
        targetSdk = 34
        versionCode = 1
        versionName = "1.0.0"

        // Tell NativeActivity which .so to load.
        // This must match the cdylib / example output name.
        ndk {
            abiFilters += listOf("arm64-v8a")
        }

        // Forward the library name to the manifest via a placeholder.
        manifestPlaceholders["nativeLibraryName"] = "portfolio_forecast"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
            // Sign with the debug keystore so `assembleRelease` produces a
            // fully-signed APK (app-release.apk) that adb can install directly.
            // Replace with a proper release keystore for production.
            signingConfig = signingConfigs.getByName("debug")
        }
        debug {
            isDebuggable = true
            isJniDebuggable = true
        }
    }

    // We do NOT use CMake / ndk-build — the native library is compiled
    // externally via cargo-ndk and placed directly into jniLibs.
    //
    // Disable the built-in native build system so Gradle doesn't look for
    // a CMakeLists.txt or Android.mk.
    externalNativeBuild {
        // Intentionally left empty.
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }

    // Tell Gradle where the pre-built .so files live.
    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }

    packaging {
        // Prevent stripping of the Rust library — cargo already strips in
        // release mode and stripping again can break backtraces.
        jniLibs {
            keepDebugSymbols += listOf(
                "*/arm64-v8a/libportfolio_forecast.so",
                "*/armeabi-v7a/libportfolio_forecast.so",
                "*/x86_64/libportfolio_forecast.so",
                "*/x86/libportfolio_forecast.so"
            )
        }
    }

    // Lint configuration — relaxed for an example project.
    lint {
        abortOnError = false
        checkReleaseBuilds = false
    }
}

dependencies {
    // AndroidX core for NotificationCompat (used by GpuiNotifications)
    implementation("androidx.core:core:1.12.0")
    // AndroidX SplashScreen compat (used by GpuiActivity to hold splash until native init)
    implementation("androidx.core:core-splashscreen:1.0.1")
    // AndroidX Biometric for BiometricPrompt (used by GpuiAuthActivity)
    implementation("androidx.biometric:biometric:1.1.0")
    // AndroidX Media for MediaSessionCompat (used by GpuiMediaSession for system controls)
    implementation("androidx.media:media:1.7.1")
}
