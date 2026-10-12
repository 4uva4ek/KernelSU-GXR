# KernelSU GXR 1.0.8

Manager version `samsung-gxr-v1.0.8`, Android version code `32673`.

1.0.8 is the 1.0.7 change set re-released with an increased version code:
`manager/gradle.properties` pins `KSU_VERSION_CODE`, and 1.0.7 was published
with the 1.0.6 value (32672), which the Manager self-update correctly ignores.
Bump that property for every release.

## Manager self-update

The home screen checks the latest published release of
`4uva4ek/KernelSU-GXR` (drafts and prereleases are ignored) when "Check for
updates" is enabled, which is the default. A newer version shows the usual
update card with the release notes. Tapping **Update** downloads the release
APK and installs it only after all of the following hold:

* the APK's SHA-256 equals the value in the release `manager-update.json`
  (older releases: `SHA256SUMS.txt`);
* the APK package name is this Manager and every signing certificate equals the
  installed Manager's certificates (and the manifest's `certSha256`, when given);
* the version code increases.

Installation runs through root (`pm install -r --user 0`) from a world-readable
staging copy under `/data/local/tmp`, which is removed afterwards. The Manager
process is replaced by the installation and must be reopened. Nothing is
downloaded or installed without the user's tap; there is no background install.

Releases now ship `KernelSU-GXR-<version>.apk`, `manager-update.json`
(`versionCode`, `versionName`, `apk`, `sha256`, `certSha256`) and a
`SHA256SUMS.txt` that covers both. `scripts/build-gxr.sh` writes them.

## GXR module installer log

The reviewed GXR package route prints an installer log like the standard
KernelSU installer: banner, package identity, verified file count, stop and
restoration of an installed version, the DISABLED installation path, the backup
location, an "About this module" block and the enable/Action instructions. The
about block is the optional `install-info.txt` from the package root (bounded,
printable text that is printed and never executed), otherwise the `module.prop`
description. See [package contract and recovery](MODULE-UPDATES.md).

## Validation limits

Kotlin and ksud compile locally; the installer-notes unit test covers bounds and
control characters. The CI build of the release sources was installed on the
headset with the kernel boot ID and all module preferences unchanged, and the
installer log was exercised up to the version-must-increase refusal. A live
self-update from this version to the next release and the full log of a
successful installation are still pending. Module activation remains a separate
user decision.
