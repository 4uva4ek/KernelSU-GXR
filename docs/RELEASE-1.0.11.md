# KernelSU GXR 1.0.11

Public release of the tested 1.0.11-dev module migration and language changes. Version code 32677 also updates 1.0.11-dev (32676).

- Review module-declared replacements and conflicts before installation. Consent is tied to the reviewed ZIP checksum.
- Restore old live patches before migration; install replacements disabled. Failed installation restores old directories disabled.
- Remove temporary migration backups after success and refresh the installed module list when returning to it.
- Show requirement and retirement dialogs in English or Russian.

Update the Manager first, then install Controller Suite and Steam Link Suite from Sources. Review the replacement prompt. Enable the installed suites separately and use the Manager soft restart; Action reports live status. Haptic MAIN remains separate.

Validation: the development build passed 19 installer tests, compiled Manager metadata-parser checks and a clean release build. The signed 1.0.11-dev APK was installed and its version verified; the user confirmed the latest Manager works and authorized publication on 2026-10-18. CI rebuilds the public version with the retained signing certificate. This is not a new device test of the CI APK or a claim of measured tracking improvement. Kernel/root behavior is unchanged by this source change.
