# Releasing KernelSU GXR and the Galaxy XR modules

One page for everything that must be bumped, tagged and checked. Two
repositories are involved; each has its own numbering and its own feed.

## 1. KernelSU GXR Manager (this repository)

### Version numbers — three places, bump all of them

| What | Where | Rule |
| --- | --- | --- |
| Android `versionCode` | `manager/gradle.properties` → `KSU_VERSION_CODE` | **Must increase for every release.** This property overrides the commit-count fallback in `manager/build.gradle.kts`; if it is not bumped the new APK has the old code and the Manager's self-update ignores the release (1.0.7 shipped with the 1.0.6 code 32672 for this reason and had to be re-released as 1.0.8 = 32673). |
| `versionName` | the Git tag `gxr-vX.Y.Z` | `build-gxr.sh` turns the tag into `samsung-gxr-vX.Y.Z`. Builds without a tag get `samsung-gxr-v<last tag>-N-g<sha>`. |
| Records | `TESTED.md` (new section on top) and `docs/RELEASE-X.Y.Z.md` | Describe what changed and what was and was not validated. Link the new release notes from `README.md`. |

Current code: see `KSU_VERSION_CODE` in `manager/gradle.properties`
(32674 = 1.0.9). Next release: 32675, and so on. Never reuse a code.

### Release steps

1. Commit the change set on `galaxyxr` and push it.
2. Bump `KSU_VERSION_CODE`, add `docs/RELEASE-X.Y.Z.md`, update `TESTED.md`
   and `README.md`; commit and push.
3. Tag the release commit and push the tag:

   ```sh
   git tag -a gxr-vX.Y.Z -m "KernelSU GXR X.Y.Z: <summary>"
   git push origin gxr-vX.Y.Z
   ```

4. The tag push runs `.github/workflows/gxr-release.yml`: it builds ksuinit,
   ksud and the Manager, signs the APK with the retained GXR certificate from the
   repository secrets (`GXR_KEYSTORE_BASE64`, `GXR_KEY_ALIAS`, `GXR_KEY_PASS`,
   `GXR_STORE_PASS`), and publishes the GitHub Release with these assets:
   `KernelSU-GXR-samsung-gxr-vX.Y.Z.apk`, `manager-update.json`, `SHA256SUMS.txt`,
   `BUILD-INFO.txt`, `TESTED.md`, `ksud`, `android12-5.10_kernelsu.ko`.
   There is no local keystore; local builds are for compiling only.
5. Verify the release: `manager-update.json` must show the new `versionCode`,
   the APK name and SHA-256 must match `SHA256SUMS.txt`, `certSha256` must be
   `0e74cb50361e34719035efec7f82f1048803165aae126ecadccab33a37c7b6e1`.
   A `workflow_dispatch` run without a tag builds the same signed artifact
   without publishing — use it to test a build on the headset first.

### Manager self-update (1.0.8+)

The home screen reads `https://api.github.com/repos/4uva4ek/KernelSU-GXR/releases/latest`
(drafts and prereleases are skipped) and shows the update card only when the
manifest `versionCode` is greater than the installed one. "Update" downloads the
APK and installs it through root only if the SHA-256 matches the manifest, the
package name is the Manager, every signing certificate equals the installed
Manager's and the version code increases. Therefore:

* a release with an unchanged `KSU_VERSION_CODE` is invisible to installed Managers;
* "latest" is the most recent non-prerelease by date — do not leave stray tags or
  test releases newer than the real one;
* releases are immutable: never retag or replace assets, publish the next number.

Older releases without `manager-update.json` are read from `BUILD-INFO.txt`
(`Manager: <name> (<code>)`) and `SHA256SUMS.txt`.

### GXR module installer log

`ksud module install` routes every `gxr_*` ID through `userspace/ksud/src/gxr_update.rs`.
It prints a KernelSU-style log and an "About this module" block from the optional
`install-info.txt` in the package root (UTF-8, ≤4 KiB, ≤32 lines, ≤160 chars per
line, no control characters; printed, never executed) or the `module.prop`
description. IDs with a hyphen (`gxr-…`) are not GXR packages and take the
standard installer with `customize.sh`.

## 2. Galaxy XR modules (`4uva4ek/galaxyxr-modules`)

Full rules: that repository's `docs/RELEASING.md`, `docs/UPDATES.md` and
`docs/MODULE-LAYOUT.md`. The short version:

| What | Where | Rule |
| --- | --- | --- |
| `version` and `versionCode` | `modules/NAME/module/module.prop` | Bump both for every release; `versionCode` must increase (the Manager and the feed compare it). |
| Changelog | `changelogs/NAME.md` | New section per release; local edits go under "Unreleased". |
| Package | `dist/NAME/gxr-NAME-VERSION-auto.zip` + `.zip.sha256` | Built by the module's documented command from a clean checkout; installs disabled; `SHA256SUMS` inside covers every file. Optional `install-info.txt` for the installer log. |
| Tag and release | `NAME/vVERSION` | Attach only the ZIP and its `.sha256`. `update-feed.yml` validates the asset and commits `updates/NAME.json` to `main`. |
| Catalogue | `modules.json` | Lists public modules for **Sources**; add a module only after its release and lifecycle review. |

Checks before tagging, from the repository root:

```sh
python tools/check_module_layout.py
python tools/check_catalogue.py
python tools/test_update_feed.py
```

A module's `versionCode` on the headset that equals the feed is reported as
installed; the Manager offers an update only for a strictly higher feed code.

## 3. Headset deployment notes

* Manager APK: `adb install -r` keeps module preferences; the daemon is replaced
  on the Manager's first launch (`/data/adb/ksud -V` shows the new version).
* Modules install **disabled** through Sources or the reviewed direct deployment;
  enabling and the soft restart are separate decisions.
* Record each deployment under `E:\GalaxyXR\diagnostics\` (receipts, SHA-256,
  before/after snapshots) and in `PROJECT_MEMORY.json`.
