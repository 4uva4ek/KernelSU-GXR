# KernelSU GXR 1.0.12

Manager version code: 32678.

- Merge PR #1 by BlueScorpioVR: read process metadata as bytes so non-UTF-8 names and mappings (observed with Tailscale) do not abort soft restart.
- Preserve OpenXR runtime, UID ownership, system-process exclusions and process identity checks. Ignore ESRCH as well as ENOENT when a process exits during inspection; other errors still abort.
- Retain all three new regression tests under cfg(test); they are excluded from release binaries.

The PR's Android arm64 check, clippy, formatting, 41 host tests and daemon build passed in https://github.com/BlueScorpioVR/KernelSU-GXR/actions/runs/39150840330 . The author reports a successful headset soft restart with Tailscale running. This is prior candidate evidence, not a device test of the 1.0.12 APK. Kernel/root acquisition code and module preferences are unchanged by this patch.

The signed 1.0.12 build is validated separately by the GXR build workflow. No device installation or restart is claimed here.
