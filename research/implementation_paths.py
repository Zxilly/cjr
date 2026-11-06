"""Offline fallback, corruption and concurrent publication tests of the product CLI."""

import hashlib
import json
import os
import pathlib
import subprocess
import sys

R = pathlib.Path(__file__).resolve().parent
W = os.name == "nt"
key = "win32-x64" if W else "linux-x64"
O = R / "implementation-artifacts" / key / "paths"
O.mkdir(parents=True, exist_ok=True)
C = pathlib.Path(sys.argv[1]).resolve()
manifest = json.loads((R / "linux/manifest.json").read_text())
a = next(
    ch["versions"]["1.2.0"][key]
    for ch in manifest["channels"].values()
    if "1.2.0" in ch["versions"] and key in ch["versions"]["1.2.0"]
)
archive = R / ("windows/archives" if W else "linux/sdk-archives") / a["name"]
digest = a["sha256"].lower()
binary = (
    R / "windows/builds/1.2.0/default/workload.exe"
    if W
    else pathlib.Path("/root/.cache/cjr-research-linux/artifacts/1.2.0/probe-default")
)
home = O / "owned"
cjv = O / "corrupt-cjv"
for p in [home / "metadata", home / "downloads", cjv / "cache"]:
    p.mkdir(parents=True, exist_ok=True)
(home / ".cjr-store.json").write_text(
    json.dumps(dict(schema_version=1, kind="cjr-runtime-store"))
)
(home / "metadata/versions.json").write_text(json.dumps(manifest))
dest = home / "downloads" / digest
if not dest.exists():
    os.link(archive, dest)
cachekey = hashlib.sha256(
    ("artifact\0" + a["url"] + "\0" + digest).encode()
).hexdigest()
(cjv / "cache" / f"{cachekey}.data").write_bytes(b"damaged archive")
(cjv / "cache" / f"{cachekey}.json").write_text(
    json.dumps(
        dict(kind="artifact", key=cachekey, url=a["url"], sha256=digest, size=15)
    )
)
e = dict(os.environ, CJR_HOME=str(home), CJV_HOME=str(cjv))
e.pop("LD_LIBRARY_PATH", None)
e["PATH"] = (
    os.environ.get("SystemRoot", r"C:\Windows") + "\\System32" if W else "/usr/bin:/bin"
)
rows = []


def record(name, p, expected):
    rows.append(
        dict(
            name=name,
            exit=p.returncode,
            expected=expected,
            passed=p.returncode == expected,
            stdout=p.stdout.decode(errors="replace")[:2000],
            stderr=p.stderr.decode(errors="replace")[:2000],
        )
    )
    (O / "results.json").write_text(json.dumps(rows, indent=2))
    assert p.returncode == expected, rows[-1]


processes = [
    subprocess.Popen(
        [str(C), "--offline", "--verbose", str(binary)],
        env=e,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    for _ in range(2)
]
for i, p in enumerate(processes):
    out, err = p.communicate(timeout=180)
    record(
        f"concurrent publication {i}",
        subprocess.CompletedProcess([], p.returncode, out, err),
        0,
    )
receipts = list((home / "runtimes").glob("*/receipt.json"))
assert len(receipts) == 1
r = json.loads(receipts[0].read_text())
assert r["provenance"] == "archive-verified"
assert pathlib.Path(r["source"]).resolve() == dest.resolve()
# The selected runtime's integrity check must reject tampering before launch.
runtime = receipts[0].parent / "payload" / r["runtime_dir"]
lib = runtime / ("libcangjie-runtime.dll" if W else "libcangjie-runtime.so")
with lib.open("r+b") as f:
    f.seek(-1, 2)
    b = f.read(1)
    f.seek(-1, 2)
    f.write(bytes([b[0] ^ 1]))
p = subprocess.run(
    [str(C), "--offline", str(binary)], env=e, capture_output=True, timeout=60
)
record("corrupt runtime rejected", p, 1)
assert b"CorruptCache" in p.stderr
p = subprocess.run(
    [str(C), "cache", "clean", "--all"], env=e, capture_output=True, timeout=60
)
record("corrupt inactive cache clean", p, 0)
assert not receipts[0].parent.exists()
p = subprocess.run(
    [str(C), "--offline", str(binary)], env=e, capture_output=True, timeout=60
)
record("offline miss after clean", p, 1)
assert b"OfflineCacheMiss" in p.stderr
print("passed", len(rows), "fallback/concurrency/corruption checks")
