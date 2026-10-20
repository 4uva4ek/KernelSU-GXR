# KernelSU GXR 1.0.16

Module source catalogues can mark an entry with `"experimental": true`.
The Manager shows a red Experimental label in its source card and installation
confirmation (localized in English and Russian). Selection and installation
remain available.

The field is optional and defaults to false. It must be a JSON boolean.
Existing schemaVersion 1 catalogues remain compatible. The new Manager also
reads optional `modules-experimental.json` and merges it with `modules.json`.
Entries from the experimental catalogue are always marked experimental, and
duplicate IDs across files are rejected. Older Managers read only `modules.json`,
so modules listed exclusively in the experimental file stay hidden from them. This label describes catalogue entries;
it does not change module activation or mark installed modules automatically.

Version code: 32682. Kernel, root acquisition, daemon and module lifecycle code
are unchanged. The signed release is built by the retained GXR signing workflow.
No headset installation or visual device test is performed for this release.
