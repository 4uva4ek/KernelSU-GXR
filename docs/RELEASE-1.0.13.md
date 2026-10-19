# KernelSU GXR 1.0.13

Manager version code: 32679.

- Retired modules: when a module's update feed contains `"retired": true`, the module card shows a deprecation banner with the feed's `retiredReason` (or a default notice) and a Remove button.
- Remove uses the existing confirmation dialog and `ksud module uninstall`, which restores live patches before marking the module for removal. Nothing is removed without user confirmation.
- `deprecatedBy` (successor hint) keeps precedence over `retired`. Older Managers ignore `retired`.

No device installation or test is claimed here. The signed build is produced by the GXR release workflow.
