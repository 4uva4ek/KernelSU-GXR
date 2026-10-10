#!/usr/bin/env bash
# Run inside ghcr.io/ylarod/ddk-min:android12-5.10-20260828.
# Pass KSU_EXPECTED_SIZE2 / KSU_EXPECTED_HASH2 as make arguments when
# packaging with a newly signed Manager; do not disable signature checks.
set -euo pipefail
repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
: "${KDIR:?Run inside the android12-5.10 DDK container}"
test "$(basename "$KDIR")" = android12-5.10
for setting in CONFIG_SHADOW_CALL_STACK CONFIG_CFI_CLANG; do
    grep -qx "${setting}=y" "$KDIR/include/config/auto.conf" || {
        echo "DDK mismatch: SM-I610 AZF3 requires ${setting}=y" >&2
        exit 1
    }
done
cd "$repo_dir/kernel"
CONFIG_KSU=m CONFIG_KSU_SAMSUNG_KDP=y CONFIG_KSU_SAMSUNG_DEFEX=n \
    CONFIG_KSU_RKP_NO_PATCH_TEXT=y CONFIG_SECTION_MISMATCH_WARN_ONLY=y \
    CC=clang make "$@"
if strings kernelsu.ko | grep 'Samsung DEFEX credential functions unavailable' >/dev/null; then
    echo 'Unexpected DEFEX implementation in module' >&2
    exit 1
fi
strings kernelsu.ko | grep 'Samsung KDP' >/dev/null
mkdir -p "$repo_dir/out/xrvst2"
cp kernelsu.ko "$repo_dir/out/xrvst2/android12-5.10_kernelsu.ko"
llvm-strip -d "$repo_dir/out/xrvst2/android12-5.10_kernelsu.ko"
sha256sum "$repo_dir/out/xrvst2/android12-5.10_kernelsu.ko"
