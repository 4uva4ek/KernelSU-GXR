# KernelSU GXR 1.0.4

Module service supervisors now re-execute `/proc/self/exe` without resolving the
symlink. This retains the running executable inode if its installed pathname is
removed or replaced during a restart. The resolved source and script path are
logged for diagnosis; no procfs or system file is modified.

Service and boot-completed dispatch errors propagate to the restart transaction
instead of being warnings followed by success. Failures retain their context in
`/dev/ksu-restart-stage/error`. Successful dispatch is explicitly distinguished
from verified runtime module activation; use each module's Action report.
Existing concurrency leases, same-boot failure guards and reboot safety remain.

Local validation: Android check, clippy with warnings denied and formatting;
deleted-executable regression; service/restart guards; six stage-dispatch tests.
Headset validation: signed 1.0.4 installed. During the authorized soft restart,
a same-version install replaced `/data/adb/ksud`; the running process and all
four supervisor launch logs showed `/data/adb/ksud (deleted)`. Dispatch succeeded.
Haptics MAIN, module-owned Whitening and Timestamp were verified live; Velocity
worker was waiting with no OpenXR app. No restart error, unchanged kernel boot.
Framebuffer10 was disabled before the test and removed at the user's request.
No firmware, original system files or partition changes. Full power-cycle and
Velocity activation in a running app were not tested in this session.
