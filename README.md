> **GXR 1.1.0: Live control** ? compatible modules apply/restore their patches
> immediately when toggled. Standard modules and the Action button retain their
> existing behavior. See [feature and safety notes](GALAXYXR.md#live-module-checkbox-control-gxr-110).

# KernelSU GXR

![DANGER: Changing data with root privileges is extremely dangerous. It can cause data loss or leave the headset unable to boot. You perform all actions entirely at your own risk.](root-risk-warning.svg)

> [!CAUTION]
> **Changing data with root privileges is extremely dangerous. You perform all actions entirely at your own risk.**
> Incorrect changes can cause permanent data loss or leave the headset unable to boot.
> Temporary root does not undo persistent writes; a reboot may not restore the device.

Experimental KernelSU build for **Samsung Galaxy XR SM-I610 (xrvst2)**,
based on [diabl0w/KernelSU](https://github.com/diabl0w/KernelSU).

**Reference device test: firmware `I610UEU2AZF3`, [DFRoot 4.1](https://github.com/diabl0w/DFRoot).**
The module loaded and the Manager launched shell processes with UID 0.
Reboot persistence and long-term stability have not been verified.

- [Exact tested firmware, kernel and APK](TESTED.md)
- [GXR build instructions and investigation](GALAXYXR.md)
- [GXR releases](https://github.com/4uva4ek/KernelSU-GXR/releases)
- [Build workflow](https://github.com/4uva4ek/KernelSU-GXR/actions/workflows/gxr-release.yml)

This profile enables Samsung KDP/RKP, disables DEFEX, retains SCS/CFI and
uses APK v2-only signing with a matching Manager certificate in the module.
It is a Manager APK with a loadable module, not a replacement boot image.
Load the module using [DFRoot](https://github.com/diabl0w/DFRoot).

The default branch is `galaxyxr`. Tags beginning with `gxr-` automatically
build and publish GXR prereleases. The generic Samsung release is disabled
in this fork. The repackaged Manager retains its upstream update links;
download GXR updates from this fork's Releases page.

Upstream documentation is available in [docs/README.md](docs/README.md).
Kernel code is GPL-2.0-only; other components retain their upstream licenses.
