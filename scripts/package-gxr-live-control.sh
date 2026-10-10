#!/usr/bin/env bash
# Repackage an existing GXR Manager with a separately built/tested ksud.
# No kernel compilation, flashing, installation or device commands.
set -euo pipefail
: "${GXR_BASE_APK:?Existing GXR APK}"
: "${GXR_KSUD:?Tested arm64 ksud}"
: "${GXR_APKTOOL:?Apktool jar}"
: "${ANDROID_HOME:?Android SDK}"
: "${GXR_KEYSTORE:?Existing GXR signing key}"
: "${GXR_KEY_ALIAS:?Signing alias}"
: "${GXR_STORE_PASS:?Keystore password}"
: "${GXR_KEY_PASS:?Key password}"
export GXR_STORE_PASS GXR_KEY_PASS
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
out=${GXR_OUT_DIR:-$repo/out/live-control}
mkdir -p "$out"
out=$(realpath "$out")
bt="$ANDROID_HOME/build-tools/${GXR_BUILD_TOOLS:-36.0.0}"
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
java -jar "$GXR_APKTOOL" d -s "$GXR_BASE_APK" -o "$work/manager"
cp "$GXR_KSUD" "$work/manager/lib/arm64-v8a/libksud.so"
java -jar "$GXR_APKTOOL" b "$work/manager" -o "$work/unsigned.apk"
"$bt/zipalign" -f -P 16 4 "$work/unsigned.apk" "$work/aligned.apk"
apk="$out/KernelSU-GXR-live-control.apk"
"$bt/apksigner" sign --ks "$GXR_KEYSTORE" --ks-key-alias "$GXR_KEY_ALIAS" \
 --ks-pass env:GXR_STORE_PASS --key-pass env:GXR_KEY_PASS \
 --v1-signing-enabled false --v2-signing-enabled true \
 --v3-signing-enabled false --v4-signing-enabled false \
 --out "$apk" "$work/aligned.apk"
"$bt/apksigner" verify --verbose --print-certs "$apk"
"$bt/zipalign" -c -P 16 4 "$apk"
python3 "$repo/scripts/gxr-apk.py" verify "$apk" "$GXR_BASE_APK" "$GXR_KSUD"
sha256sum "$apk" > "$apk.sha256"
