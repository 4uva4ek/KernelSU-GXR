# KernelSU GXR 1.0.6

Manager version `samsung-gxr-v1.0.6`, Android version code `32672`.

## Repository sources and updates

Add a public GitHub repository under Modules -> repository icon -> Sources.
The Manager reads its modules.json catalogue or stable Release ZIP assets, offers
multiple selection, downloads/verifies the selected archives, then installs them
sequentially. There is no five-module ID allowlist and no ten-module selection
limit. New catalogue entries do not require rebuilding the Manager.

Installed packages use their own updateJson. Older packages obtain feed URLs
from the configured Git catalogues; conflicting ID mappings are not guessed.
OFF modules are checked too, and refresh requests recheck the remote version.

Any valid GXR module ID can use the disabled-package installer if its archive
meets the documented lifecycle contract. The installer checks identity, payload
hashes, safe paths and startup layout; stops/restores an existing module; retains
an inert backup and persistent guard/config files; and commits the replacement
DISABLED. Failed or ambiguous restoration blocks replacement. It does not execute
downloaded install scripts, regenerate root/startup metadata, enable modules or
restart Android. Non-GXR modules retain their standard installation path.
See [package contract and recovery](MODULE-UPDATES.md).

## Themes

Shared Material controls inside Miuix now receive a palette derived from the
active Miuix colors and app appearance preference. Sources cards, checkboxes,
dropdowns, text fields and dialogs follow light/dark/system mode and custom
accents instead of using Material's default light palette on a dark screen.
The repository screen paints its themed surface explicitly. AMOLED mode keeps
Material background surfaces black. The existing Material appearance remains.

## Userspace

Includes the previously locally tested 1.0.5 change: before an intentional soft
restart, notify module workers, close verified third-party OpenXR app clients,
then stop Android. System services and ambiguous package ownership are excluded;
failed shutdown cancels before Android stop. No app is relaunched automatically.
Module installation/control and soft restart share a volatile operation lock.
Kernel payload, original-file protection and standard checkbox semantics remain.

## Validation and limits

Android Rust check, clippy with warnings denied and formatting passed. Host
tests cover OpenXR shutdown, restart/service guards, stage dispatch and GXR
replacement. The new regression installs and upgrades twelve previously unknown
IDs while retaining disabled state. Manager tests cover twelve Git catalogue
entries, remote feed discovery, conflict rejection and ZIP identity/version checks.
Theme/source code builds in the release Manager.

The initial preview APK was installed and its exact bytes read back. Launch was
authorized, but a sleeping headset and later disconnected USB prevented source
UI verification. This is not a claim that live module replacement has been tested.
The 1.0.5 soft-restart device evidence is recorded separately in TESTED.md and does
not establish every later binary's behavior. No module activation, restart,
native throwing or full power-cycle validation is implied by this release.
