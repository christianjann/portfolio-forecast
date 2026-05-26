#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────────────────────
# build.sh — Build & run PortfolioForecast on Android
#
# Usage:
#   ./build.sh android [--device | --emulator] [--release] [--clean] [--no-run]
#
# Options:
#   --device      Target a physical device (default).
#   --emulator    Target an Android emulator.
#   --release     Build in release mode (default is debug).
#   --clean       Run a clean build.
#   --no-run      Build only — do not install or launch the app.
#   -h, --help    Show this help message.
#
# Prerequisites:
#   Android SDK + NDK, cargo-ndk (cargo install cargo-ndk),
#   rustup target aarch64-linux-android
# ──────────────────────────────────────────────────────────────────────────────
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# SCRIPT_DIR = portfolio-forecast/android/
APP_DIR="$SCRIPT_DIR"
ANDROID_GRADLE_DIR="$APP_DIR/gradle"

if [ -t 1 ]; then
    BOLD='\033[1m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    RED='\033[0;31m'
    CYAN='\033[0;36m'
    RESET='\033[0m'
else
    BOLD='' GREEN='' YELLOW='' RED='' CYAN='' RESET=''
fi

info()  { echo -e "${GREEN}▸${RESET} $*"; }
warn()  { echo -e "${YELLOW}⚠${RESET} $*"; }
error() { echo -e "${RED}✘${RESET} $*" >&2; }
step()  { echo -e "\n${BOLD}${CYAN}══ $* ══${RESET}\n"; }

# If ANDROID_STUDIO_HOME is set, use its bundled JBR as JAVA_HOME so that
# Gradle's toolchain detection finds a fully-recognised JDK (the system
# java-latest-openjdk may lack IMAGE_TYPE=JDK in its release file).
if [[ -n "${ANDROID_STUDIO_HOME:-}" && -d "${ANDROID_STUDIO_HOME}/jbr" ]]; then
    export JAVA_HOME="${ANDROID_STUDIO_HOME}/jbr"
    info "Using JDK from ANDROID_STUDIO_HOME: $JAVA_HOME"
fi

usage() {
    cat <<'EOF'
Usage:
  ./build.sh android [--device|--emulator] [--release] [--clean] [--no-run]

Options:
  --device      Target a physical device (default)
  --emulator    Target an Android emulator
  --release     Release build (default: debug)
  --clean       Clean before building
  --no-run      Build only — skip install & launch
  -h, --help    Show this help
EOF
}

PLATFORM=""
TARGET_KIND="device"
PROFILE="debug"
CLEAN=false
NO_RUN=false

if [[ $# -lt 1 ]]; then
    usage
    exit 1
fi

case "${1:-}" in
    android) PLATFORM="$1"; shift ;;
    -h|--help) usage; exit 0 ;;
    *) error "Unknown subcommand: $1"; usage; exit 1 ;;
esac

while [[ $# -gt 0 ]]; do
    case "$1" in
        --device)   TARGET_KIND="device"   ;;
        --emulator) TARGET_KIND="emulator" ;;
        --release)  PROFILE="release"      ;;
        --clean)    CLEAN=true             ;;
        --no-run)   NO_RUN=true            ;;
        -h|--help)  usage; exit 0          ;;
        *) error "Unknown option: $1"; usage; exit 1 ;;
    esac
    shift
done

build_android() {
    step "Android — ${PROFILE} — ${TARGET_KIND}"

    if ! command -v cargo-ndk &>/dev/null; then
        error "cargo-ndk not found. Install it with: cargo install cargo-ndk"
        exit 1
    fi

    if [[ -z "${ANDROID_HOME:-}" && -z "${ANDROID_SDK_ROOT:-}" ]]; then
        if [[ -d "$HOME/Library/Android/sdk" ]]; then
            export ANDROID_HOME="$HOME/Library/Android/sdk"
        elif [[ -d "$HOME/Android/Sdk" ]]; then
            export ANDROID_HOME="$HOME/Android/Sdk"
        else
            warn "ANDROID_HOME / ANDROID_SDK_ROOT not set. Gradle may fail."
        fi
    fi

    local rust_target="aarch64-linux-android"
    local ndk_abi="arm64-v8a"

    local cargo_profile_flag=""
    if [[ "$PROFILE" == "release" ]]; then
        cargo_profile_flag="--release"
    fi

    local gradle_task
    if [[ "$PROFILE" == "release" ]]; then
        gradle_task="assembleRelease"
    else
        gradle_task="assembleDebug"
    fi

    local apk_variant
    if [[ "$PROFILE" == "release" ]]; then
        apk_variant="release"
    else
        apk_variant="debug"
    fi

    info "Ensuring Rust target ${rust_target} is installed..."
    rustup target add "$rust_target" 2>/dev/null || true

    if $CLEAN; then
        info "Cleaning Rust build artifacts..."
        cd "$APP_DIR"
        cargo clean --target "$rust_target" 2>/dev/null || true

        info "Cleaning Gradle..."
        cd "$ANDROID_GRADLE_DIR"
        ./gradlew clean 2>/dev/null || true
    fi

    step "Building Rust shared library for ${ndk_abi} (${PROFILE})"

    local jni_libs_dir="$ANDROID_GRADLE_DIR/app/src/main/jniLibs"

    cd "$APP_DIR"
    cargo ndk \
        -t "$ndk_abi" \
        -o "$jni_libs_dir" \
        --platform 31 \
        build \
        $cargo_profile_flag \
        2>&1

    local so_path="$jni_libs_dir/${ndk_abi}/libportfolio_forecast.so"
    if [[ ! -f "$so_path" ]]; then
        error "Shared library not found at: $so_path"
        exit 1
    fi
    info "Shared library: $so_path ($(du -h "$so_path" | cut -f1))"

    step "Assembling APK (${gradle_task})"

    cd "$ANDROID_GRADLE_DIR"
    ./gradlew "$gradle_task" 2>&1

    local apk_path="$ANDROID_GRADLE_DIR/app/build/outputs/apk/${apk_variant}/app-${apk_variant}.apk"
    if [[ ! -f "$apk_path" ]]; then
        local unsigned="$ANDROID_GRADLE_DIR/app/build/outputs/apk/${apk_variant}/app-${apk_variant}-unsigned.apk"
        if [[ -f "$unsigned" ]]; then
            warn "Signed APK not found; using unsigned APK."
            apk_path="$unsigned"
        else
            error "APK not found at: $apk_path"
            exit 1
        fi
    fi
    info "APK: $apk_path ($(du -h "$apk_path" | cut -f1))"

    if $NO_RUN; then
        info "Skipping install & launch (--no-run)."
        return 0
    fi

    _android_install_and_launch "$apk_path"
}

_android_install_and_launch() {
    local apk_path="$1"

    step "Installing & launching on Android"

    if ! command -v adb &>/dev/null; then
        local adb_candidate="${ANDROID_HOME:-}/platform-tools/adb"
        if [[ -x "$adb_candidate" ]]; then
            export PATH="${ANDROID_HOME}/platform-tools:$PATH"
        else
            error "adb not found. Make sure Android SDK platform-tools are in your PATH."
            info "APK is at: $apk_path"
            exit 1
        fi
    fi

    local device_count
    device_count=$(adb devices 2>/dev/null | grep -cE '\t(device|emulator)') || true

    if [[ "$device_count" -eq 0 ]]; then
        if [[ "$TARGET_KIND" == "emulator" ]]; then
            warn "No running emulator found. Attempting to start one..."
            local avd_name
            avd_name=$(emulator -list-avds 2>/dev/null | head -1) || true
            if [[ -n "$avd_name" ]]; then
                info "Starting emulator: $avd_name"
                emulator -avd "$avd_name" -no-snapshot-load &
                info "Waiting for emulator to boot..."
                adb wait-for-device
                sleep 10
            else
                error "No AVDs found. Create one with Android Studio or avdmanager."
                exit 1
            fi
        else
            error "No connected Android device found. Connect a device and try again."
            info "APK is at: $apk_path"
            exit 1
        fi
    fi

    info "Installing APK..."
    adb install -r "$apk_path" 2>&1

    info "Launching app..."
    adb shell am start \
        -n "dev.gpui.portfolio.forecast/dev.gpui.mobile.GpuiActivity" \
        -a android.intent.action.MAIN \
        -c android.intent.category.LAUNCHER \
        2>&1

    info "App launched on Android!"

    echo ""
    info "View logs with:  adb logcat -s portfolio-forecast:D"
}

case "$PLATFORM" in
    android) build_android ;;
esac

echo ""
info "${BOLD}Done!${RESET}"
