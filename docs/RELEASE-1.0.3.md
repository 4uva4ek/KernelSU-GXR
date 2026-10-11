# KernelSU GXR 1.0.3

- Coordinate module preparation, one successful-path Android stop/start,
  verification, then ordinary module services. No framebuffer-specific code.
- Prevent duplicate service-script trees, including detached children.
- Save bounded per-hook output, exit status and duration for restart diagnostics.
- Keep standard checkboxes and Action. Kernel payload and root acquisition are unchanged.

Android checks and 21 host tests passed. Three soft restarts passed with
Framebuffer10 0.3.1; the last two used 1.0.3 with active Whitening 0.1.3.
Haptics was active, Velocity waited for an app, and no duplicate owners were found.

The earlier Framebuffer started-hook exit 1 was not reproduced; its exact cause
remains unknown. Cold-boot and long-term stability remain unverified.
The public APK removes the test-only flag and retains the tested daemon; the
exact repackaged APK has not been device-tested. Build metadata includes hashes.

[Restart API and compatibility](COORDINATED-RESTART.md) | [Test record](../TESTED.md) |
[Module changes](https://github.com/4uva4ek/galaxyxr-modules/blob/main/docs/RELEASE-2026-10-11.md)
