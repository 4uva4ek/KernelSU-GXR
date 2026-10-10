# KernelSU GXR: tested configuration

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
alignment, embedded ksud and preserved Manager DEX. A successful CI build
does **not** mean that its exact APK has been tested on a headset. The table
above describes the specific local APK tested on the reference device.
Each generated package includes `BUILD-INFO.txt` with its source commit and
workflow run, and `SHA256SUMS.txt` for the compiled files.

Use [DFRoot](https://github.com/diabl0w/DFRoot) to load the module.
Do not flash the APK as a boot image.
