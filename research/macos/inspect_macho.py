import json
import pathlib
import struct
import subprocess

ROOT = pathlib.Path(__file__).resolve().parent


def inspect(path):
    b = path.read_bytes()
    magic = struct.unpack_from("<I", b)[0]
    if magic != 0xFEEDFACF:
        return {
            "path": str(path.relative_to(ROOT)),
            "magic": hex(magic),
            "supported": False,
        }
    _, cpu, subcpu, ft, ncmd, cmdsize, flags, reserved = struct.unpack_from(
        "<IIIIIIII", b
    )
    segs = []
    sections = []
    dylibs = []
    rpaths = []
    commands = []
    off = 32
    for _ in range(ncmd):
        cmd, size = struct.unpack_from("<II", b, off)
        assert size >= 8 and off + size <= len(b)
        commands.append(hex(cmd))
        if cmd == 0x19:
            name = b[off + 8 : off + 24].split(b"\0")[0].decode()
            vm, vms, fo, fs = struct.unpack_from("<QQQQ", b, off + 24)
            nsec = struct.unpack_from("<I", b, off + 64)[0]
            segs.append((vm, fs, fo, name))
            for i in range(nsec):
                q = off + 72 + 80 * i
                sn = b[q : q + 16].split(b"\0")[0].decode()
                sg = b[q + 16 : q + 32].split(b"\0")[0].decode()
                addr, sz, ofs = struct.unpack_from("<QQI", b, q + 32)
                sections.append(
                    {
                        "name": sn,
                        "segment": sg,
                        "address": addr,
                        "size": sz,
                        "offset": ofs,
                    }
                )
        elif cmd in [0xC, 0xD, 0x80000018, 0x8000001F, 0x20, 0x80000023]:
            pos = off + struct.unpack_from("<I", b, off + 8)[0]
            s = b[pos : b.index(0, pos, off + size)].decode()
            dylibs.append({"command": hex(cmd), "path": s})
        elif cmd == 0x8000001C:
            pos = off + struct.unpack_from("<I", b, off + 8)[0]
            rpaths.append(b[pos : b.index(0, pos, off + size)].decode())
        off += size

    def mapva(va):
        for vm, fs, fo, name in segs:
            if vm <= va < vm + fs:
                return fo + va - vm
        raise ValueError(f"unmapped {va:x}")

    versions = []
    errors = []
    for s in sections:
        if s["name"] == "__cjsdkversion":
            for pos in range(s["offset"], s["offset"] + s["size"], 8):
                va = struct.unpack_from("<Q", b, pos)[0]
                try:
                    f = mapva(va)
                    end = b.index(0, f, min(f + 256, len(b)))
                    v = b[f:end].decode("ascii")
                    versions.append(v)
                except Exception as e:
                    errors.append({"offset": pos, "raw": hex(va), "error": str(e)})
    return {
        "path": str(path.relative_to(ROOT)),
        "bytes": len(b),
        "cpu": hex(cpu),
        "cpu_subtype": subcpu,
        "filetype": ft,
        "flags": hex(flags),
        "universal": False,
        "metadata_sections": [
            s for s in sections if s["name"] in ["__cjsdkversion", "__cjmetaheader"]
        ],
        "versions": sorted(set(versions)),
        "version_entries": len(versions),
        "decode_errors": errors,
        "dependencies": dylibs,
        "rpaths": rpaths,
        "has_chained_fixups": "0x80000034" in commands,
        "has_dyld_info": "0x80000022" in commands,
        "code_signature": "0x1d" in commands,
    }


results = []
for sdk in sorted((ROOT / "extracted").iterdir()):
    root = sdk / "cangjie"
    candidates = [root / "tools/bin/cjpm", root / "bin/cjc"]
    candidates += list((root / "runtime/lib").glob("*/libcangjie-std-core.dylib"))
    candidates += list((root / "runtime/lib").glob("*/libcangjie-runtime.dylib"))
    for p in candidates:
        r = inspect(p)
        results.append(r)
        print(
            p.name,
            sdk.name,
            r.get("versions"),
            r.get("version_entries"),
            len(r.get("decode_errors", [])),
            flush=True,
        )
    p = root / "tools/bin/cjpm"
    out = ROOT / "stripped" / sdk.name / "cjpm"
    out.parent.mkdir(parents=True, exist_ok=True)
    s = subprocess.run(
        ["llvm-strip", "--strip-all", "-o", str(out), str(p)],
        capture_output=True,
        text=True,
    )
    if s.returncode == 0:
        r = inspect(out)
        r["strip_stderr"] = s.stderr
        results.append(r)
        print(
            "strip",
            sdk.name,
            r["versions"],
            r["version_entries"],
            len(r["decode_errors"]),
            flush=True,
        )
    else:
        results.append(
            {"strip_failed": str(p), "stderr": s.stderr, "exit": s.returncode}
        )
(ROOT / "inspection.json").write_text(json.dumps(results, ensure_ascii=False, indent=2))
