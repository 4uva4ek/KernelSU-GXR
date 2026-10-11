# KernelSU GXR: tested configuration

## Current release: GXR 1.0.4

Manager `samsung-gxr-v1.0.4`, code `32669`. The exact signed release APK
was installed and tested; its SHA-256 is
`5c0d71f7034eeb94c8d54990cdd1325e80906f96b7cb7aea228d9575ba74317a`.
Embedded and deployed daemon SHA-256:
`dfe94b55316418a312c1e676033cdabbc76cce433e660f747cbfd7bf7051db19`.

During an intentional same-version reinstall in a soft restart, the running
executable became `/data/adb/ksud (deleted)`. All four module supervisors
successfully launched through `/proc/self/exe`. Haptics MAIN, module-owned
Whitening and Timestamp were verified live. Velocity's worker was alive and
waiting for an OpenXR app. Framebuffer10 was disabled and removed; it was
not activated in this test. The kernel boot ID was unchanged, with no restart
error. The kernel/root payload remains unchanged.

Android check, clippy with warnings denied, formatting, deleted-executable
regression, lifecycle guards and six stage-error propagation tests passed.
Full power-cycle and active-app Velocity validation were not performed.
See [1.0.4 details](docs/RELEASE-1.0.4.md).

## Historical release: GXR 1.0.3

See [1.0.3 changes and test scope](docs/RELEASE-1.0.3.md) and the
[restart API](docs/COORDINATED-RESTART.md). The Manager version is
`samsung-gxr-v1.0.3`, code `32668`; the original GXR kernel payload is unchanged.

The local test APK SHA-256 was
`719c9bebff288f4ce09db3a7de8936ee9edfc3ef9befa95e81188526e15f5475`.
Its embedded/deployed ksud SHA-256 was
`3b54400ff1b5928e8871c89ffaaa5545691fb52810ccae26e0760704fe98dc1f`.
Two consecutive coordinated soft restarts passed with active Framebuffer10
0.3.1 and Whitening 0.1.3; Haptics was also verified active, and Velocity was
waiting for an OpenXR target. An earlier cycle passed on 1.0.2. This is runtime
lifecycle evidence, not a visual/physical quality or cold-boot validation.

The public APK uses the same daemon but removes the test-only installation flag.
Its exact signed bytes have not been device-tested. Release hashes and build
provenance are supplied separately. The sections below retain historical tests
and must not be interpreted as tests of every later binary.

## Reference device test

| Item | Tested value |
| --- | --- |
| Device | Samsung Galaxy XR SM-I610 |
| Codename | xrvst2 |
| Device firmware / build | `I610UEU2AZF3` |
| Running kernel | `5.10.240-android12-9-perf-33775878-abI610UEU2AZF3` |
| Module loader | [DFRoot 4.1](https://github.com/diabl0w/DFRoot) |
| KernelSU source base | `diabl0w/KernelSU`, `samsung-v1.0`, commit `10aa21cbb6a2b05d7792f60cde76565b99d89a63` |
| Tested local APK | `KernelSU-GXR_32664_no-DEFEX.apk` |
| APK SHA256 | `106c1646a3a03652ba2946b6e927299c49a8abb705ddb4ea58cad6166e5a3576` |
| Profile | KDP=y, DEFEX=n, RKP_NO_PATCH_TEXT=y; SCS=y, CFI=y |
| Manager signing | APK v2 only; matching certificate compiled into the module |

**Confirmed:** the GXR module loaded, and the Manager launched two shell
processes with UID 0. This resolved the original module's missing-DEFEX error
and the initial incorrectly signed Manager's authentication failure.

**Not yet verified:** root grants to other applications, persistence after
a full reboot, long-term stability, other firmware builds or other devices.
The Apply on boot observation was described by the tester as resembling a
soft reboot; it is not evidence of reliable boot persistence.

The older `AZD8` dump was used for analysis only. It is not the firmware on
which the successful reference test was performed.

## Scope of automated releases

CI builds this GXR profile and checks the module, APK signature scheme,
alignment, embedded ksud and the source-built Manager. A successful CI build
does **not** mean that its exact APK has been tested on a headset. The table
above describes the specific local APK tested on the reference device.
Each generated package includes `BUILD-INFO.txt` with its source commit and
workflow run, and `SHA256SUMS.txt` for the compiled files.

Use [DFRoot](https://github.com/diabl0w/DFRoot) to load the module.
Do not flash the APK as a boot image.

## GXR 1.0.1

The Manager version is `samsung-gxr-v1.0.1` (Android version code 32666).
Soft-restart requests run off the UI thread and report submission failures.
The daemon serializes restart requests and acquires a kernel-expiring,
180-second suspend blocker before stopping Android. Standard module startup
and the Action button are retained; no live-checkbox extension is included.

The preceding `gxr-1.0.1-test1` build passed native checks, five guard tests
and a clean Manager release build. The owner tested it on the reference
headset and reported the Manager working well. Version 1.0.1 retains that
daemon and kernel payload; it updates Manager branding and release metadata.
This report does not establish module health or authorize a soft reboot.
