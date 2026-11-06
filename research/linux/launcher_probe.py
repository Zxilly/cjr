"""Research-only ELF prototype: exact SDK detection, verified runtime-only cache, clean launch.
Not production cjr: x86_64 little-endian ELF only; readelf used for DT_NEEDED.
"""

import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys
import tarfile

from run_matrix import ROOT, STORE, VERSIONS, inspect, run

MANIFEST = json.loads((ROOT / "manifest.json").read_text())
RUNTIME_CACHE = STORE / "runtime-only"


def needed(binary):
    p = subprocess.run(
        ["readelf", "-dW", str(binary)], capture_output=True, text=True, check=True
    )
    return re.findall(r"\(NEEDED\).*\[(.*?)\]", p.stdout)


def requirement(binary):
    versions = set()
    visited = set()
    pending = [binary]
    requires_runtime = False
    while pending:
        path = pending.pop().resolve()
        if path in visited:
            continue
        visited.add(path)
        md = inspect(path)
        versions.update(md.get("unique_versions", []))
        for lib in needed(path):
            if lib == "libcangjie-runtime.so" or lib.startswith("libcangjie-std-"):
                requires_runtime = True
            local = path.parent / lib
            if local.is_file() and lib.startswith("libprobelib"):
                pending.append(local)
    if not requires_runtime:
        return None
    if len(versions) != 1:
        raise ValueError("MixedSdkVersions " + str(sorted(versions)))
    return versions.pop()


def prepare(version):
    matches = []
    for channel, ch in MANIFEST["channels"].items():
        x = ch["versions"].get(version, {}).get("linux-x64")
        if x:
            matches.append(x)
    if len(matches) != 1:
        raise ValueError("unknown or ambiguous official release")
    x = matches[0]
    root = RUNTIME_CACHE / version
    runtime = root / "cangjie/runtime/lib/linux_x86_64_cjnative"
    if runtime.is_dir():
        return runtime
    archive = ROOT / "sdk-archives" / x["name"]
    with archive.open("rb") as f:
        actual = hashlib.file_digest(f, "sha256").hexdigest()
    if actual != x["sha256"].lower():
        raise ValueError("archive digest mismatch")
    root.mkdir(parents=True, exist_ok=True)
    with tarfile.open(archive, "r:gz") as t:
        members = [
            m
            for m in t.getmembers()
            if m.name.startswith("cangjie/runtime/lib/linux_x86_64_cjnative/")
            or m.name == "cangjie/LICENSE"
        ]
        t.extractall(root, members=members, filter="data")
    return runtime


def execute(binary, debug=False):
    version = requirement(binary)
    e = {"PATH": "/usr/bin:/bin", "LANG": "C.UTF-8", "HOME": str(pathlib.Path.home())}
    if version:
        e["LD_LIBRARY_PATH"] = str(binary.parent) + ":" + str(prepare(version))
    if debug:
        e["LD_DEBUG"] = "libs"
    r = run([binary], env=e, cwd=binary.parent)
    r.update(
        binary=str(binary),
        detected_version=version,
        runtime_dir=str(prepare(version)) if version else None,
    )
    return r


if __name__ == "__main__":
    p = pathlib.Path(sys.argv[1]).resolve()
    r = execute(p)
    print(json.dumps(r, ensure_ascii=False, indent=2))
    sys.exit(r["exit"] if isinstance(r["exit"], int) and r["exit"] >= 0 else 1)
