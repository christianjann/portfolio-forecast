# Portfolio Forecast — justfile
# https://github.com/casey/just

# Set ANDROID_STUDIO_HOME to your Android Studio install to use its JBR for Gradle.
# e.g.  export ANDROID_STUDIO_HOME=~/opt/Android/android-studio
# or override inline:  just android ANDROID_STUDIO_HOME=~/opt/Android/android-studio
export ANDROID_STUDIO_HOME := env_var_or_default("ANDROID_STUDIO_HOME", "")

# Show available recipes
default:
    @just --list

# ── Rust checks ───────────────────────────────────────────────────────────────

# Check host-compatible crates (common + desktop); android/ios require cross-compilation
check:
    cargo check -p portfolio-forecast-common -p portfolio-forecast-desktop

# Check the Android crate for aarch64-linux-android
check-android:
    cargo check -p portfolio-forecast-android --target aarch64-linux-android

# Check the common crate only
check-common:
    cargo check -p portfolio-forecast-common

# Run clippy on all host crates
lint:
    cargo clippy --workspace -- -D warnings

# ── Desktop ───────────────────────────────────────────────────────────────────

# Build and run the desktop app (debug)
desktop:
    cargo run -p portfolio-forecast-desktop

# Build the desktop app in release mode
desktop-release:
    cargo build -p portfolio-forecast-desktop --release

# ── Android ───────────────────────────────────────────────────────────────────

# Build debug APK and install on connected device
android:
    cd android && ./build.sh android --device

# Build debug APK only (no install)
android-build:
    cd android && ./build.sh android --device --no-run

# Build release APK and install on connected device
android-release:
    cd android && ./build.sh android --device --release

# Build release APK only (no install)
android-release-build:
    cd android && ./build.sh android --device --release --no-run

# Build debug APK and install on emulator
android-emu:
    cd android && ./build.sh android --emulator

# Clean Android build artifacts (Rust + Gradle)
android-clean:
    cd android && ./build.sh android --clean --no-run

# Build the Rust .so only (no Gradle / APK step)
android-so:
    cargo ndk -t arm64-v8a \
        -o android/gradle/app/src/main/jniLibs \
        --platform 31 \
        build -p portfolio-forecast-android

# Build the Rust .so in release mode only
android-so-release:
    cargo ndk -t arm64-v8a \
        -o android/gradle/app/src/main/jniLibs \
        --platform 31 \
        build -p portfolio-forecast-android --release

# ── iOS (macOS only) ──────────────────────────────────────────────────────────

# Build iOS static lib for a physical device
ios-lib:
    cargo build -p portfolio-forecast-ios --target aarch64-apple-ios --release

# Build iOS static lib for the simulator
ios-lib-sim:
    cargo build -p portfolio-forecast-ios --target aarch64-apple-ios-sim --release

# Generate Xcode project (requires xcodegen)
ios-xcodegen:
    cd ios && xcodegen generate

# Full iOS device build: lib + xcodegen + xcodebuild (macOS only)
ios:
    just ios-lib
    just ios-xcodegen
    cd ios && xcodebuild \
        -project PortfolioForecast.xcodeproj \
        -scheme PortfolioForecast \
        -configuration Release \
        -destination 'generic/platform=iOS' \
        build

# Full iOS simulator build (macOS only)
ios-sim:
    just ios-lib-sim
    just ios-xcodegen
    cd ios && xcodebuild \
        -project PortfolioForecast.xcodeproj \
        -scheme PortfolioForecast \
        -configuration Debug \
        -destination 'platform=iOS Simulator,name=iPhone 16' \
        build

# ── Utilities ─────────────────────────────────────────────────────────────────

# Remove all build artifacts
clean:
    cargo clean

# Install required Rust targets
setup:
    rustup target add aarch64-linux-android
    rustup target add aarch64-apple-ios       || true
    rustup target add aarch64-apple-ios-sim   || true
    cargo install cargo-ndk --locked          || true
    cargo install just --locked               || true
