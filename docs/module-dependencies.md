# Declarative module conditions

GXR modules declare installation requirements in their own `module.prop`.
KernelSU contains no registry of suite names or constituent module IDs.

- `gxrReplaces=gxr_old_a,gxr_old_b`: installed listed modules must be restored
  and removed before installation. The order is the restoration order.
- `gxrRequiresDisabled=gxr_conflict`: listed installed modules must be restored
  and left disabled. Their files remain installed.
- `gxrDeprecatedBy=gxr_successor`: optional retirement hint in an old module.
  The update feed can also supply `deprecatedBy` for already installed releases.

Both requirement lists are optional, comma-separated, contain at most 16
unique GXR IDs, and cannot include the declaring module or overlap. Empty or
malformed declarations fail validation. Absent modules require no action.
The current guarded lifecycle contract supports GXR modules only.

Manager reads these declarations from the selected ZIP and presents the
required actions. Cancellation leaves modules unchanged. Approval is bound
to the SHA-256 of the reviewed ZIP. The daemon independently reads metadata,
checks the exact approved installed conflict sets and verifies package hashes.
A CLI install without matching explicit consent fails when conflicts exist.

Before changing modules, the daemon checks the incoming package, target version,
pending updates and all installed conflicting packages. Restoration failure
aborts installation. Removed modules are moved to inert recovery directories
outside `/data/adb/modules`; a failed installation restores those directories,
keeping modules disabled. After successful installation the temporary migration
copies are deleted. Modules required disabled remain disabled on failure.
The replacement installs disabled. Activation or a soft restart is separate.
An installed declaring module also prevents enabling conflicting modules again.

Available in KernelSU GXR 1.0.11. Modules install disabled; activation and a
soft restart remain separate user actions.

Manager requirement dialogs and retirement notices use English/Russian Android
resources. Returning to the module page reloads the installed list after migration.
