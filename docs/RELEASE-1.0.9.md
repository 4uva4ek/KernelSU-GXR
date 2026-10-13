# KernelSU GXR 1.0.9

Manager `samsung-gxr-v1.0.9`, Android version code `32674`.

## Fix

Installing Steam Link Pose Time 0.1.7 with Manager 1.0.8 failed with
`Unexpected payload directory`: the reviewed GXR installer allowed nested
files only under `bin/`, although KernelSU WebUI uses `webroot/`.
The installer now also accepts checksummed regular files under `webroot/`,
including nested UI assets. Existing module archives need no repackaging.

Unsafe paths, unsupported ZIP entry types, unlisted or altered files, system
payloads and early-boot hooks remain rejected. Installation does not execute
WebUI files or new package scripts. Replacements still require restoration of
the old live patch and finish disabled, preserving guard/config files.

## Validation

Android arm64 check, clippy with warnings denied, and formatting passed.
All 13 host regressions passed, covering fresh WebUI installation and
replacement, nested assets, disabled replacement, checksum tampering, missing
checksums, traversal and lookalike directory names. The real eight-module CI
package set, including Pose Time 0.1.7, passed validation against the Rust installer.
The release workflow builds and verifies the signed APK and update manifest.
Live module replacement and WebUI interaction on the headset are not validated
by these host checks.

## Update

Use the Manager's existing home-screen update card to install 1.0.9, then reopen
the Manager so its bundled userspace is refreshed. Retry the Pose Time update.
Module enablement and any soft restart remain separate user actions.
