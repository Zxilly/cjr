import json
import os
import pathlib
import re
import shutil
import struct
import subprocess
import time

ROOT = pathlib.Path("/mnt/e/Project/CS_Project/2026/cjr/research/linux")
STORE = pathlib.Path((ROOT / "sdk-store.txt").read_text())
OUT = STORE / "artifacts"
OUT.mkdir(exist_ok=True)
VERSIONS = ["1.0.5", "1.1.3", "1.2.0"]
results = {
    "environment": {
        "uname": subprocess.getoutput("uname -a"),
        "glibc": subprocess.getoutput("ldd --version").splitlines()[0],
    },
    "builds": [],
    "runs": [],
    "metadata": [],
}


def run(cmd, env=None, cwd=None, timeout=120):
    start = time.monotonic()
    try:
        p = subprocess.run(
            [str(x) for x in cmd],
            cwd=cwd,
            env=env,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        return {
            "exit": p.returncode,
            "stdout": p.stdout[:5000],
            "stderr": p.stderr[:200000]
            if env and env.get("LD_DEBUG")
            else p.stderr[:5000],
            "seconds": round(time.monotonic() - start, 3),
        }
    except subprocess.TimeoutExpired as e:
        return {
            "exit": "timeout",
            "stdout": str(e.stdout or "")[:1000],
            "stderr": str(e.stderr or "")[:1000],
        }


def runtime(v):
    return STORE / v / "cangjie/runtime/lib/linux_x86_64_cjnative"


def env(v=None, lib=None, debug=False):
    e = {"PATH": "/usr/bin:/bin", "LANG": "C.UTF-8", "HOME": str(pathlib.Path.home())}
    paths = []
    if lib:
        paths.append(str(lib))
    if v:
        paths.append(str(runtime(v)))
    if paths:
        e["LD_LIBRARY_PATH"] = ":".join(paths)
    if debug:
        e["LD_DEBUG"] = "libs"
    return e


def inspect(p):
    b = p.read_bytes()
    assert b[:6] == b"\x7fELF\x02\x01"
    hdr = struct.unpack_from("<HHIQQQIHHHHHH", b, 16)
    shoff = hdr[5]
    shsize = hdr[10]
    shnum = hdr[11]
    namesidx = hdr[12]
    sections = [
        struct.unpack_from("<IIQQQQIIQQ", b, shoff + i * shsize) for i in range(shnum)
    ]
    ns = sections[namesidx]
    names = b[ns[4] : ns[4] + ns[5]]
    named = {names[s[0] : names.find(b"\0", s[0])].decode(): s for s in sections}

    def off(va):
        for s in sections:
            if s[1] != 8 and s[3] <= va < s[3] + s[5]:
                return s[4] + va - s[3]
        raise ValueError("VA unmapped " + hex(va))

    m = named.get(".cjmetadata")
    if not m:
        return {"path": str(p), "versions": [], "error": "no .cjmetadata"}
    raw = b[m[4] : m[4] + m[5]]
    magic, flags, unk, total, start, end = struct.unpack_from("<IIIIII", raw)
    assert magic == 0x12345678 and 0 <= start <= end <= len(raw)
    rel = {}
    for s in sections:
        if s[1] == 4:
            for a in range(s[4], s[4] + s[5], s[9]):
                addr, info, addend = struct.unpack_from("<QQq", b, a)
                if info & 0xFFFFFFFF == 8:
                    rel[addr] = addend
    versions = []
    for j in range(start, end, 8):
        va = rel.get(m[3] + j, struct.unpack_from("<Q", raw, j)[0])
        if va == 0:
            continue  # aligned version table can end with a null padding slot
        pos = off(va)
        endstr = b.find(b"\0", pos, pos + 256)
        if endstr < 0:
            raise ValueError("unterminated version")
        versions.append(b[pos:endstr].decode())
    return {
        "path": str(p),
        "elf_type": hdr[0],
        "section_size": m[5],
        "magic": hex(magic),
        "flags": hex(flags),
        "version_table_offset": start,
        "version_table_end": end,
        "versions": versions,
        "unique_versions": sorted(set(versions)),
        "relocations_used": sum(m[3] + j in rel for j in range(start, end, 8)),
    }


def main():
    for v in VERSIONS:
        d = OUT / v
        d.mkdir(exist_ok=True)
        cjc = STORE / v / "cangjie/bin/cjc"
        for mode, flags in [
            ("default", []),
            ("dy-std", ["--dy-std"]),
            ("static", ["--static"]),
        ]:
            p = d / f"probe-{mode}"
            r = run([cjc, ROOT / "probe.cj", *flags, "-o", p], cwd=d)
            r.update(version=v, mode=mode)
            results["builds"].append(r)
            print("build", v, mode, r["exit"], r["stderr"][:200], flush=True)
            if r["exit"] != 0:
                continue
            results["metadata"].append(inspect(p))
            stripped = d / f"probe-{mode}-stripped"
            shutil.copy2(p, stripped)
            subprocess.run(["strip", "--strip-all", str(stripped)], check=True)
            results["metadata"].append(inspect(stripped))
            (ROOT / f"{v}-{mode}-readelf.txt").write_text(
                subprocess.getoutput(f"readelf -dW {p}")
            )
            for rv in [None, *VERSIONS]:
                rr = run([stripped], env=env(rv), cwd=d, timeout=35)
                rr.update(binary_version=v, mode=mode, runtime_version=rv or "none")
                results["runs"].append(rr)
                print(
                    "run",
                    v,
                    mode,
                    rv,
                    rr["exit"],
                    rr["stdout"][:100],
                    rr["stderr"][:100],
                    flush=True,
                )
            dbg = run([stripped], env=env(v, debug=True), cwd=d, timeout=35)
            (ROOT / f"{v}-{mode}-loader.txt").write_text(dbg["stderr"])
        # Shared Cangjie library and application
        r = run(
            [
                cjc,
                ROOT / "probelib.cj",
                "--output-type=dylib",
                "-o",
                d / "libprobelib.so",
            ],
            cwd=d,
        )
        r.update(version=v, mode="library")
        results["builds"].append(r)
        print("library", v, r["exit"], r["stderr"][:200], flush=True)
        if r["exit"] == 0:
            results["metadata"].append(inspect(d / "libprobelib.so"))
            r = run(
                [
                    cjc,
                    ROOT / "dynamic.cj",
                    "--import-path",
                    d,
                    "--dy-libs",
                    "-L",
                    d,
                    "-lprobelib",
                    "-o",
                    d / "probe-dynamic",
                ],
                cwd=d,
            )
            r.update(version=v, mode="dynamic-app")
            results["builds"].append(r)
            print("dynamic-app", v, r["exit"], r["stderr"][:500], flush=True)
            if r["exit"] == 0:
                results["metadata"].append(inspect(d / "probe-dynamic"))
                for rv in [None, *VERSIONS]:
                    rr = run(
                        [d / "probe-dynamic"], env=env(rv, lib=d), cwd=d, timeout=35
                    )
                    rr.update(
                        binary_version=v,
                        mode="dynamic-app",
                        runtime_version=rv or "none",
                    )
                    results["runs"].append(rr)
                    print(
                        "dynamic-run", v, rv, rr["exit"], rr["stderr"][:100], flush=True
                    )
        (ROOT / "results.json").write_text(
            json.dumps(results, ensure_ascii=False, indent=2)
        )
    # Library compiled under previous compiler, import from newer compiler
    for appv, libv in [("1.2.0", "1.1.3"), ("1.1.3", "1.0.5")]:
        d = OUT / appv
        lib = OUT / libv
        p = d / f"mixed-with-{libv}"
        rr = run(
            [
                STORE / appv / "cangjie/bin/cjc",
                ROOT / "dynamic.cj",
                "--import-path",
                lib,
                "--dy-libs",
                "-L",
                lib,
                "-lprobelib",
                "-o",
                p,
            ],
            cwd=d,
        )
        rr.update(version=appv, library_version=libv, mode="mixed-app")
        results["builds"].append(rr)
        print("mixed-build", appv, libv, rr["exit"], rr["stderr"][:500], flush=True)
        if rr["exit"] == 0:
            results["metadata"].append(inspect(p))
            rr = run([p], env=env(appv, lib), cwd=d, timeout=35)
            rr.update(
                binary_version=appv,
                library_version=libv,
                mode="mixed-app",
                runtime_version=appv,
            )
            results["runs"].append(rr)
    (ROOT / "results.json").write_text(
        json.dumps(results, ensure_ascii=False, indent=2)
    )


if __name__ == "__main__":
    main()
