# Reviewed GXR module updates

The Manager reads installed packages' updateJson feeds and configured GitHub catalogues.
OFF modules are checked too, and explicit list refresh rechecks the remote feed
even if the local version did not change. Older packages without updateJson use
the ID/feed mapping downloaded from their configured repositories. There is no
built-in module-ID allowlist. Conflicting source mappings are not guessed; an
installed package's own updateJson takes priority. Action stays accessible for
GXR packages while OFF.

Updates require the user's Update/install action. There is no background install,
automatic enable, implicit soft restart, or custom checkbox activation hook.

## Repository sources

Open Modules -> repository icon -> Sources, then paste a public GitHub repository
URL or `owner/repository`. Sources are saved locally and can be removed. The GXR
repository is included by default; the standard KernelSU catalogue remains in
the source picker. Private repositories/authentication are not supported.

A root `modules.json` (schemaVersion 1) lists module IDs, names, descriptions and
updateJson URLs. Without it, stable GitHub release ZIP assets are offered, using
the latest release per `module/vVERSION` tag prefix (or one ordinary release
line). Source-only repositories need a catalogue and downloadable module ZIPs.

Select packages and confirm Install. Every selected package is downloaded and
checked for identity, version and SHA-256 when supplied before installation
starts. Installation then runs sequentially and stops on the first error; it is
not an atomic batch. Already installed/current versions cannot be downgraded.
GXR replacements stay disabled and do not show the normal reboot shortcut.

## Direct installation

`ksud module install ZIP` routes any valid `gxr_*` ID through the disabled-package
installer. Eligibility depends on archive contents and the lifecycle contract,
not a list of public names. Ordinary modules retain their existing installer.

For GXR packages, the new path:

* validates bounded ZIP entries, regular files, identity, all payload checksums,
  disabled/skip-mount markers, and absence of unexpected system/early-boot files;
* rejects a pending update/removal, altered installed payload, symlinks/hardlinks,
  and a versionCode that does not increase;
* serializes installation against GXR Manager controls and soft restart;
* disables the old installation, calls its **installed** `action.sh off`, and
  requires successful `action.sh status` with verified stock/no-target output;
* stages the replacement outside the startup module directory, preserves all
  extra guard/config files, syncs files/directories, retains the old directory,
  and renames the new disabled directory into place.

Neither `customize.sh`, package install scripts, metamodule installers, preinit
regeneration nor boot/root configuration is executed by this GXR install path.
It does not execute newly downloaded code. Checksums are integrity checks, not a
publisher signature; only install trusted releases.

New GXR modules do not need a Manager rebuild. Supply a regular checksummed ZIP
with empty disable/skip_mount, service.sh and action.sh; system overlays,
early-boot payloads and unsupported filesystem entries are rejected. For update
restoration, the installed action.sh off must succeed, then action.sh status
must confirm stock runtime. New authors can emit `GXR_UPDATE_STATE=STOCK` after
verifying restoration/no owned runtime changes. Existing legacy stock reports
remain accepted. An error or remaining active report blocks replacement even
when a stock marker is also printed. A checkbox alone is not that verification.

Source catalogues are refreshed before legacy update lookup. Removing a source
removes its fallback mapping on refresh. Selection has no five/ten-module batch
cap; all selected downloads are verified before the sequential installer starts.

Backups and incomplete transactions remain inert under `/data/adb/gxr-updates/`.
They have no startup hooks. A stop/status failure leaves the old version disabled
and retains its files. Interruption between the two renames can leave the module
absent, with `old/` in the printed transaction directory. There is no unattended
recovery/retry. Inspect the transaction before manual recovery. Full power loss
does not undo an installed module update; the replacement stays disabled and
the backup is retained.

The concurrency lock covers this updated Manager/ksud and its soft-restart path.
Do not run older ksud tools or separate root-shell activation/deployment commands
concurrently; those tools do not participate in the new lock.

## Validation and deployment status

Local Android `cargo ndk check`, `clippy` with warnings denied, and formatting
are required. The host harness under `scripts/tests/gxr-update/` exercises actual
filesystem replacement, restoration failure, guard retention, downgrade refusal,
unsafe paths, concurrent locking and checksums. Its optional public-fixture test
validates the five downloaded stable archives against the Rust installer.

The Manager is built from current local source as an updates preview. The user
requested APK installation on the headset, separately from module activation.
No live module replacement, enable operation or Android restart was requested.
Host tests do not establish live
restoration of every installed historical version. Failed/ambiguous status stops
installation and requires recovery before retrying.

On 2026-10-12 the user-requested APK `samsung-gxr-v1.0.6-updates-preview`
(32671) was installed on SM-I610. The installed APK SHA-256 was read back and
matched `2a628af3378545441eee5771e0e4dbd8cf9d2ff5b79461191c40aacb808fd8b3`.
Kernel boot ID and module versions/checkbox preferences were unchanged. Three
Manager source tests, ten Rust updater tests including all five public fixtures,
and three feed tests passed, as did catalogue CI at modules commit `a272249`.
The user subsequently approved Manager startup including its userspace refresh.
Its activity launched, but the headset remained asleep with no visible window
surface and later disconnected from ADB; catalogue loading could not be verified
through the UI. A previous APK and userspace daemon are saved
under `diagnostics/module-updates-20261012/`. No live module replacement or reboot
was performed by this task.
