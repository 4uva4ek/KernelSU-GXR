"""Prepare GXR resources and check the APK contract required by apk_sign.c."""
import re
import shutil
import struct
import sys
import zipfile
from pathlib import Path


def verify_v2(data):
    # Locate a valid EOCD, including its variable-length comment.
    eocd = len(data) - 22
    while eocd >= max(0, len(data) - 22 - 65535):
        if (data[eocd:eocd + 4] == b"PK\x05\x06"
                and eocd + 22 + struct.unpack_from("<H", data, eocd + 20)[0] == len(data)):
            break
        eocd -= 1
    else:
        raise ValueError("Missing EOCD")
    cd = struct.unpack_from("<I", data, eocd + 16)[0]
    assert data[cd - 16:cd] == b"APK Sig Block 42"
    size = struct.unpack_from("<Q", data, cd - 24)[0]
    pos, end = cd - size - 8, cd - 24
    assert pos >= 0 and struct.unpack_from("<Q", data, pos)[0] == size
    pos += 8
    ids = []
    while pos < end:
        pair_size, block_id = struct.unpack_from("<QI", data, pos)
        assert pair_size >= 4 and pos + 8 + pair_size <= end
        ids.append(block_id)
        pos += 8 + pair_size
    assert pos == end
    # KernelSU rejects v3 and other non-padding blocks, even alongside v2.
    assert ids.count(0x7109871A) == 1 and set(ids) <= {0x7109871A, 0x42726577}, ids


def main():
    if sys.argv[1] == "prepare":
        root, ksud = map(Path, sys.argv[2:4])
        count = 0
        for path in root.glob("res/values*/strings.xml"):
            value, hits = re.subn(r'(<string name="app_name">)[^<]*(</string>)',
                                 r'\g<1>KernelSU GXR\2', path.read_text())
            if hits:
                path.write_text(value)
                count += hits
        assert count, "Missing app_name resource"
        shutil.copy2(ksud, root / "lib/arm64-v8a/libksud.so")
    elif sys.argv[1] == "verify":
        apk, base, ksud = map(Path, sys.argv[2:5])
        verify_v2(apk.read_bytes())
        with zipfile.ZipFile(apk) as built, zipfile.ZipFile(base) as original:
            assert built.testzip() is None
            assert built.read("lib/arm64-v8a/libksud.so") == ksud.read_bytes()
            for name in original.namelist():
                if name.endswith(".dex"):
                    assert original.read(name) == built.read(name), name
        print("PASS: v2-only signing block, embedded ksud and unchanged Manager DEX")
    else:
        raise SystemExit("Use prepare DECODED_APK KSUD or verify APK BASE_APK KSUD")


if __name__ == "__main__":
    main()
