"""Verify cjv's Go/OS lock protocol excludes a cjr installed-SDK snapshot."""

import json
import os
import pathlib
import subprocess
import sys
import tempfile
import time

root = pathlib.Path(__file__).resolve().parent
w = os.name == "nt"
platform = "win32-x64" if w else "linux-x64"
base = root / "implementation-artifacts" / platform
home = pathlib.Path(tempfile.mkdtemp(prefix="lock-cjr-", dir=base))
cjv = base / "edges/managed-cjv"
(home / "metadata").mkdir(parents=True, exist_ok=True)
(home / ".cjr-store.json").write_text(
    json.dumps(dict(schema_version=1, kind="cjr-runtime-store"))
)
(home / "metadata/versions.json").write_bytes(
    (root / "linux/manifest.json").read_bytes()
)
helper = base / ("lock-holder.exe" if w else "lock-holder")
subprocess.run(
    ["go", "build", "-o", str(helper), "."],
    cwd=root / "implementation-lock",
    check=True,
)
cjr = pathlib.Path(sys.argv[1]).resolve()
target = (
    root / "windows/builds/1.2.0/default/workload.exe"
    if w
    else pathlib.Path("/root/.cache/cjr-research-linux/artifacts/1.2.0/probe-default")
)
env = dict(os.environ, CJR_HOME=str(home), CJV_HOME=str(cjv))
env.pop("LD_LIBRARY_PATH", None)
holder = subprocess.Popen(
    [str(helper), str(cjv / ".lifecycle.lock"), str(cjv / ".lifecycle.lookup.lock")],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
)
assert holder.stdout.readline() == b"locked\n"
start = time.monotonic()
try:
    p = subprocess.run(
        [str(cjr), "--offline", "--verbose", str(target)],
        env=env,
        capture_output=True,
        timeout=30,
    )
    assert (
        p.returncode == 1
        and b"CacheBusy" in p.stderr
        and b"OfflineCacheMiss" in p.stderr
    ), p.stderr
finally:
    holder.communicate(input=b"x", timeout=10)
first = dict(
    name="Go-held cjv locks reject snapshot",
    exit=p.returncode,
    seconds=round(time.monotonic() - start, 3),
    passed=True,
)
p = subprocess.run(
    [str(cjr), "--offline", "--verbose", str(target)],
    env=env,
    capture_output=True,
    timeout=120,
)
assert p.returncode == 0 and b"PASS" in p.stdout, p.stderr
rows = [
    first,
    dict(name="snapshot after Go lock release", exit=p.returncode, passed=True),
]
(base / "lock-results.json").write_text(json.dumps(rows, indent=2))
print(json.dumps(rows, indent=2))
