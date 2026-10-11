# KernelSU GXR for Samsung Galaxy XR

Experimental SM-I610 (xrvst2) build based on diabl0w/KernelSU
`samsung-v1.0`, commit `10aa21cbb6a2b05d7792f60cde76565b99d89a63`.

## Tested result

On firmware I610UEU2AZF3, kernel
`5.10.240-android12-9-perf-33775878-abI610UEU2AZF3`, DFRoot 4.1 started
ksud as UID 0, but the original Samsung module failed with
`Samsung DEFEX credential functions unavailable` and `-ENOENT`.

The older AZD8 kernel dump contains KDP/RKP symbols but no DEFEX symbols.
The running AZF3 configuration enables KDP/RKP, SCS and CFI and contains no
DEFEX settings. Building with the following options allowed the module to load:

```text
CONFIG_KSU_SAMSUNG_KDP=y
CONFIG_KSU_SAMSUNG_DEFEX=n
CONFIG_KSU_RKP_NO_PATCH_TEXT=y
```

The DDK must keep `CONFIG_SHADOW_CALL_STACK=y` and `CONFIG_CFI_CLANG=y`.
The script checks these settings rather than applying the unrelated A14 workaround.

Manager signing is also essential: this source's `kernel/manager/apk_sign.c`
requires exactly one APK v2 signature block and rejects v3 blocks. An initial
v3-only APK installed in Android but was not recognized by KernelSU; its driver
request was blocked by seccomp. With v2-only signing and the matching certificate
compiled into the module, the Manager launched two shell processes with UID 0.
Root grants to other apps, reboot persistence and long-term stability have not
been validated. This is not a full kernel or boot image replacement.

## Build on Linux / WSL2

Required: Docker, Git, Python 3, JDK 21, Rust stable with the
`aarch64-linux-android` target, libclang, Android NDK r29, Android SDK build-tools
37.0.0, platform android-37.0 and CMake 3.22.1. Use an LF checkout on the Linux filesystem and a full
Git history. The DDK image is pinned by digest in `scripts/build-gxr.sh`.

The build compiles the GXR Manager from source with the label **KernelSU GXR**,
plus ksuinit, ksud and the module. This preserves the GXR soft-restart UI fixes
and gives both the home and About screens the GXR release version.

Create and retain your own RSA signing keystore outside the repository. Set:

```bash
export ANDROID_HOME=/path/to/android-sdk
export ANDROID_NDK_HOME=/path/to/android-ndk-r29
export GXR_VERSION_NAME=gxr-v1.0.3
export GXR_KEYSTORE=/private/path/gxr.jks
export GXR_KEY_ALIAS=gxr
read -rs -p 'Keystore password: ' GXR_STORE_PASS; echo
read -rs -p 'Key password: ' GXR_KEY_PASS; echo
export GXR_STORE_PASS GXR_KEY_PASS
bash scripts/build-gxr.sh
```

The script exports the certificate and builds its size/hash into the module as
`EXPECTED_SIZE2`/`EXPECTED_HASH2`; signature authentication remains enabled.
It builds the module and userspace, signs v2-only, verifies APK signatures,
alignment, name, version and embedded ksud. Outputs and SHA256 sums
are under `out/gxr/`. No signing keys or headset logs are included in this fork.

## Installation

An APK signed by a different key cannot update an existing Manager in place.
Back up needed settings before removing the old Manager. Updates signed with
the same retained GXR key can use `adb install -r`.
Use DFRoot to load the module; do not flash this APK as a boot image.

## Automatic GXR releases

The `KernelSU GXR release` workflow builds this profile only. Push a `gxr-*`
tag from the `galaxyxr` branch to build and publish a release with the APK,
module, ksud, build metadata and checksums. A manual Actions run builds the
same files as an artifact when `release_tag` is empty. To publish from the
Actions UI, enter a new `gxr-*` version in `release_tag`; the tag and release
are created only after the build and checks succeed.

The application label is `KernelSU GXR`. APK `versionName` uses
`samsung-gxr-v<release version>` (tag `gxr-v1.0.0` becomes
`samsung-gxr-v1.0.0`), including tags supplied through the
Actions UI. Local repacks can set `GXR_VERSION_NAME`; source Manager builds
can set `-PKSU_VERSION_NAME=gxr-v1.0.0`. Without an override, builds use the
nearest `gxr-*` tag or `samsung-gxr-vdev-<commit>` when no GXR tag is available.
Direct Gradle builds default to the release version and Android version code
in `manager/gradle.properties`; bump both when preparing a new release.

Configure repository Actions secrets `GXR_KEYSTORE_BASE64`, `GXR_STORE_PASS`,
`GXR_KEY_PASS` and `GXR_KEY_ALIAS` using the retained signing key. Missing
secrets fail the build; CI never generates a replacement signing identity.
The old Samsung release workflow is disabled in this fork.

The Manager is built from this repository's source. It retains upstream update
links; obtain GXR releases from this fork's Releases page. Direct local Gradle
release builds must set `-Pandroid.injected.testOnly=false` so ordinary APK
installation does not require `adb install -t`.

## Coordinated module restarts

See [Coordinated Android restart stages](docs/COORDINATED-RESTART.md) for the GXR
1.0.3 userspace lifecycle, why it was introduced, hook contracts, cold/soft boot
behavior, duplicate prevention and third-party compatibility limits.
