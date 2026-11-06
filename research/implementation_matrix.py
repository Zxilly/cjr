"""Exercise the product binary against the existing verified SDK research fixtures.

All writes go to implementation-artifacts. This never mutates the real cjv home.
Usage: python research/implementation_matrix.py <cjr executable>
"""

import hashlib
import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent
WINDOWS = os.name == "nt"
PLATFORM = "win32-x64" if WINDOWS else "linux-x64"
OUT = ROOT / "implementation-artifacts" / PLATFORM
OUT.mkdir(parents=True, exist_ok=True)
CJR = Path(sys.argv[1]).resolve()
MANIFEST = json.loads((ROOT / "linux/manifest.json").read_text())
RESULTS = []


def asset(v):
    return next(
        (ch, data["versions"][v][PLATFORM])
        for ch, data in MANIFEST["channels"].items()
        if v in data["versions"] and PLATFORM in data["versions"][v]
    )


def seed(home, cjv, versions):
    (home / "metadata").mkdir(parents=True, exist_ok=True)
    (home / ".cjr-store.json").write_text(
        json.dumps(dict(schema_version=1, kind="cjr-runtime-store"))
    )
    (home / "metadata/versions.json").write_text(json.dumps(MANIFEST))
    (cjv / "cache").mkdir(parents=True, exist_ok=True)
    for v in versions:
        _, a = asset(v)
        digest = a["sha256"].lower()
        key = hashlib.sha256(
            ("artifact\0" + a["url"] + "\0" + digest).encode()
        ).hexdigest()
        original = (
            ROOT / ("windows/archives" if WINDOWS else "linux/sdk-archives") / a["name"]
        )
        dest = cjv / "cache" / f"{key}.data"
        if not dest.exists():
            try:
                os.link(original, dest)
            except OSError:
                shutil.copyfile(original, dest)
        (cjv / "cache" / f"{key}.json").write_text(
            json.dumps(
                dict(
                    key=key,
                    kind="artifact",
                    url=a["url"],
                    sha256=digest,
                    size=original.stat().st_size,
                )
            )
        )


def environment(home, cjv):
    e = dict(os.environ, CJR_HOME=str(home), CJV_HOME=str(cjv))
    for k in list(e):
        if k.startswith(("CANGJIE", "LD_", "DYLD_")):
            e.pop(k)
    e["PATH"] = (
        (os.environ.get("SystemRoot", r"C:\Windows") + r"\System32")
        if WINDOWS
        else "/usr/bin:/bin"
    )
    return e


def run(name, args, e, expected=0, output=None):
    start = time.monotonic()
    p = subprocess.run(
        [str(CJR), *map(str, args)], env=e, capture_output=True, timeout=180
    )
    result = dict(
        name=name,
        exit=p.returncode,
        expected=expected,
        seconds=round(time.monotonic() - start, 3),
        stdout=p.stdout.decode("utf-8", errors="replace")[:12000],
        stderr=p.stderr.decode("utf-8", errors="replace")[:12000],
    )
    result["passed"] = p.returncode == expected and (
        output is None or output in result["stdout"]
    )
    RESULTS.append(result)
    (OUT / "results.json").write_text(
        json.dumps(RESULTS, indent=2, ensure_ascii=False), encoding="utf-8"
    )
    print(name, result["exit"], result["passed"], result["stderr"][:160], flush=True)
    return p, result


home = OUT / "main"
cjv = OUT / "cjv"
seed(home, cjv, ["1.0.5", "1.1.3", "1.2.0"])
env = environment(home, cjv)
for v in ["1.0.5", "1.1.3", "1.2.0"]:
    modes = (
        ["default", "dynamic-std", "stripped"]
        if WINDOWS
        else ["default", "dy-std", "static", "default-stripped", "dy-std-stripped"]
    )
    for mode in modes:
        p = (
            (ROOT / "windows/builds" / v / mode / "workload.exe")
            if WINDOWS
            else Path("/root/.cache/cjr-research-linux/artifacts") / v / f"probe-{mode}"
        )
        _, r = run(f"{v}/{mode}", ["--offline", "--verbose", p], env, output="PASS")
        if not r["passed"]:
            continue
        result, _ = run(f"inspect/{v}/{mode}", ["inspect", p, "--json"], env)
        assert json.loads(result.stdout)["binary"]["sdk_version"] == v

# Receipt identity works without any remaining catalog/archive/cjv source.
run("default clean", ["cache", "clean"], env)
isolated = dict(env, CJV_HOME=str(OUT / "absent-cjv"))
p = (
    (ROOT / "windows/builds/1.2.0/stripped/workload.exe")
    if WINDOWS
    else Path("/root/.cache/cjr-research-linux/artifacts/1.2.0/probe-dy-std-stripped")
)
run("receipt-only offline", ["--offline", p], isolated, output="PASS")
run("list", ["list", "--json"], env)

# Windows application directory precedence must be refused before executing.
if WINDOWS:
    p = ROOT / "windows/builds/shadowing/workload.exe"
    if p.exists():
        run("DLL shadow rejection", ["--offline", p], env, expected=1)

run("dry-run all", ["cache", "clean", "--all", "--dry-run"], env)
print(
    json.dumps(
        {"passed": sum(r["passed"] for r in RESULTS), "total": len(RESULTS)}, indent=2
    )
)
sys.exit(0 if all(r["passed"] for r in RESULTS) else 1)
