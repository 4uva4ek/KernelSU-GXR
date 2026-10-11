#!/usr/bin/env bash
# Linux/WSL2 build using the GXR Manager and userspace source.
set -euo pipefail
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo"
GXR_VERSION_NAME=${GXR_VERSION_NAME:-$(git describe --tags --match 'gxr-*' --always)}
case "$GXR_VERSION_NAME" in
    samsung-gxr-v*) ;;
    gxr-v*) GXR_VERSION_NAME="samsung-$GXR_VERSION_NAME" ;;
    gxr-*) GXR_VERSION_NAME="samsung-gxr-v${GXR_VERSION_NAME#gxr-}" ;;
    *) GXR_VERSION_NAME="samsung-gxr-vdev-$GXR_VERSION_NAME" ;;
esac
[[ "$GXR_VERSION_NAME" =~ ^samsung-gxr-v[A-Za-z0-9][A-Za-z0-9._-]*$ ]] || { echo 'Invalid GXR version' >&2; exit 1; }
: "${ANDROID_NDK_HOME:?Set Android NDK r29 path}"
: "${ANDROID_HOME:?Set Android SDK path}"
: "${GXR_KEYSTORE:?Set persistent signing keystore path}"
: "${GXR_STORE_PASS:?Set keystore password}"
: "${GXR_KEY_PASS:?Set key password}"
: "${GXR_KEY_ALIAS:?Set key alias}"
export GXR_STORE_PASS GXR_KEY_PASS
bt="$ANDROID_HOME/build-tools/${GXR_BUILD_TOOLS:-37.0.0}"
out="$repo/out/gxr"
mkdir -p "$out"
out=$(realpath "$out")
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
keytool -exportcert -alias "$GXR_KEY_ALIAS" -keystore "$GXR_KEYSTORE" \
    -storepass:env GXR_STORE_PASS -file "$work/cert.der"
cert_size=$(stat -c %s "$work/cert.der")
cert_hash=$(sha256sum "$work/cert.der" | cut -d ' ' -f 1)
docker run --rm -e GIT_CONFIG_COUNT=1 -e GIT_CONFIG_KEY_0=safe.directory \
    -e GIT_CONFIG_VALUE_0=/work -v "$repo:/work" -w /work \
    ghcr.io/ylarod/ddk-min@sha256:272ada1e61c525116032246df2a790d1ccec1804b7a9e0b09203c5e650dcb78e \
    bash scripts/build-xrvst2-lkm.sh "KSU_EXPECTED_SIZE2=$cert_size" "KSU_EXPECTED_HASH2=$cert_hash"
cd "$repo"
cp out/xrvst2/android12-5.10_kernelsu.ko userspace/ksud/bin/aarch64/
source .github/scripts/setup-rust-build.sh aarch64-linux-android 26
builtins="$($CLANG_PATH --print-resource-dir)/lib/linux/libclang_rt.builtins-aarch64-android.a"
test -f "$builtins"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$repo/target}"
RUSTFLAGS="-C target-feature=+crt-static -C link-arg=-Wl,-z,max-page-size=16384 -C link-arg=-Wno-unused-command-line-argument -C link-arg=$builtins" \
    cargo build --locked --package ksuinit --target aarch64-linux-android --release
cp "$CARGO_TARGET_DIR/aarch64-linux-android/release/ksuinit" userspace/ksud/bin/aarch64/
# rust-embed does not always notice changed embedded binaries.
cargo clean --package ksud --target aarch64-linux-android --release
cargo build --locked --package ksud --target aarch64-linux-android --release
ksud="$CARGO_TARGET_DIR/aarch64-linux-android/release/ksud"
mkdir -p manager/app/src/main/jniLibs/arm64-v8a
cp "$ksud" manager/app/src/main/jniLibs/arm64-v8a/libksud.so
(cd manager && bash gradlew clean assembleRelease \
    -Pandroid.injected.build.abi=arm64-v8a -Pandroid.injected.testOnly=false "-PKSU_VERSION_NAME=$GXR_VERSION_NAME")
mapfile -t manager_apks < <(find manager/app/build/intermediates/apk/release -maxdepth 1 -name '*.apk')
[[ ${#manager_apks[@]} == 1 ]]
cp "${manager_apks[0]}" "$work/unsigned.apk"
"$bt/zipalign" -f -P 16 4 "$work/unsigned.apk" "$work/aligned.apk"
apk="$out/KernelSU-GXR_no-DEFEX.apk"
"$bt/apksigner" sign --ks "$GXR_KEYSTORE" --ks-key-alias "$GXR_KEY_ALIAS" \
    --ks-pass env:GXR_STORE_PASS --key-pass env:GXR_KEY_PASS \
    --v1-signing-enabled false --v2-signing-enabled true \
    --v3-signing-enabled false --v4-signing-enabled false \
    --out "$apk" "$work/aligned.apk"
"$bt/apksigner" verify --verbose --print-certs "$apk"
"$bt/zipalign" -c -P 16 4 "$apk"
"$(dirname "$CLANG_PATH")/llvm-strip" --strip-unneeded "$ksud" -o "$work/ksud-stripped"
python3 scripts/gxr-apk.py verify-source "$apk" "$work/ksud-stripped"
"$bt/aapt2" dump badging "$apk" > "$out/apk-badging.txt"
grep -Fx "application-label:'KernelSU GXR'" "$out/apk-badging.txt"
grep -F "versionName='$GXR_VERSION_NAME'" "$out/apk-badging.txt"
cp out/xrvst2/android12-5.10_kernelsu.ko "$out/"
cp "$ksud" "$out/ksud"
cd "$out"
sha256sum KernelSU-GXR_no-DEFEX.apk android12-5.10_kernelsu.ko ksud > SHA256SUMS.txt
echo "Built: $apk"
