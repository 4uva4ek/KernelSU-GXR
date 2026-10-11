# Coordinated Android restart stages (introduced in 1.0.2, updated in 1.0.3)

## GXR 1.0.3 diagnostics and device follow-up

Each hook now saves stdout/stderr (up to 64 KiB), its generation, exit status and
elapsed time in `/dev/ksu-restart-stage/logs/<module>-<phase>.log`. Further output
is drained without storage, preventing a verbose hook from blocking on a full
pipe. The coordinator does not wait for background descendants to close their
output descriptors. These logs are replaced per phase; copy them before another
test. A new transaction clears the previous `error` file so an old failure is
not mistaken for the current result. Hook failures and timeouts still abort.

On the headset, the original 1.0.2/Framebuffer10 0.3.0 failure was an exit code 1
from `restart-started.sh`, immediately after native ENABLED, followed by a clean
abort restoration. Its output was discarded by the daemon, so the original
failing command cannot be established retrospectively. With Framebuffer10 0.3.1
phase tracing, three subsequent soft restarts in one kernel boot passed, two
with active Whitening 0.1.3 and 1.0.3 userspace. This is evidence of those tested
cycles, not proof of the original transient's cause or all third-party modules.

This userspace extension was created because a display module restarted Android
from `service.sh`, after other modules had already patched its services. Haptics
successfully patched one controller-service process, then lost it during the
display module's second framework restart. A readiness timeout cannot prevent a
later restart. The solution is one KernelSU-owned stop/start, with preparation
between stop and start and ordinary module services dispatched after verification.

No framebuffer name, pixel format or library path is built into ksud.

## Participant contract

An enabled module opts in by shipping all four files below. Disabled/removed
modules are excluded from the initial participant snapshot. Partial contracts
are errors. Hooks run sequentially in sorted module-path order, with ordinary
KernelSU script environment plus `KSU_RESTART_GENERATION` (a transaction UUID).

| Hook | Android state | Required behavior |
| --- | --- | --- |
| `restart-prepare.sh` | Running, before stop | Validate compatibility and arrange handoff of an existing session. Do not restart Android. |
| `restart-stopped.sh` | KernelSU has issued stop | Verify the particular service is stopped; prepare reversible startup changes. Return only when KernelSU may start. |
| `restart-started.sh` | Android has reported boot completion | Verify the result and clear temporary startup substitutions/selectors. Do not restart Android on the successful path. |
| `restart-abort.sh` | Depends on failure point | Undo preparation and verify a safe baseline. Must tolerate partial preparation and a disabled checkbox. |

Each foreground hook must return zero only on success. Limits are 30 seconds for
prepare/abort, 45 for stopped and 70 for started, further bounded by the existing
180-second restart transaction. On timeout ksud kills only that hook's dedicated
process group. A module that modifies system runtime state must provide an
independent recovery guardian in a separate session. Aborting one participant
must not prevent abort notification of the remaining participants.

`/dev/ksu-restart-stage` contains the current generation, busy/ready state and an
error report. It is volatile, not a checkbox or hidden activation preference.
Service and boot-completed callbacks are deferred while busy. An unsuccessful
transaction does not dispatch ordinary services or silently retry itself.

## Order and cold boot

The order is: participant preflight; standard emulated-soft-reboot notifications;
Android stop; standard post-fs-data setup; stopped hooks; one Android start;
boot-completed wait; started hooks; ordinary service and boot-completed scripts.
The kernel, ksuinit, firmware, original system files and partition protections
are not changed by this feature.

A native cold-boot service callback with enabled participants enters this same
coordinator before ordinary service scripts. With no participants the standard
path remains. Galaxy XR temporary-root late-load still does not dispatch modules:
the existing explicit KernelSU soft-reboot workflow remains necessary. Installing
the APK or checking a module does not itself request a reboot.

Repeated intentional soft reboots get new transaction UUIDs. Framebuffer10 hands
its previous guardian off at the stopped phase, then prepares the next session
before KernelSU starts Android. Successful sessions are reusable. Interrupted
native sessions remain quarantined in that kernel boot; cold boot clears /dev.

## Ordinary modules and duplicate protection

Ordinary modules need no restart hooks. KernelSU supervises `service.sh`,
`boot-completed.sh`, and common service/boot-completed directory scripts using a
per-script volatile lease and a Linux child subreaper. The supervisor retains
ownership while descendants survive, including double-fork/setsid daemons.
Repeated callbacks do not launch a second live script tree. Completed scripts
run at most once per generation and may run again in a later soft-reboot
generation. A failed script or lost supervisor is not blindly retried; its
quarantine lasts until a cold boot. Preferences are not toggled by lease reset.

This prevents duplicate launches; it cannot repair arbitrary third-party code
that does not tolerate Android service replacement. Processes created externally
(for example through init, another daemon, or an older ksud before upgrade) are
not retroactively adopted. Begin validation from a clean cold-root session when
testing migration. Supervisors do not kill arbitrary module workers or guess
how to restore their patches. Existing lifecycle hooks remain available.

## Validation and deployment

Rust Android check/clippy and host tests exercise phase ordering, checked hook
failures, timeout process-group containment, abort fan-out, generation leases,
and setsid descendants. Framebuffer production native simulations cover one
external start, timeout rollback and coordinated handoff. These are local tests,
not proof of a complete headset cold/soft-reboot sequence. Install replacement
modules disabled. Enable and actual reboot require separate user authorization.
