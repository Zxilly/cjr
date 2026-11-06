"""Additional product acceptance tests using SDKs acquired during feasibility research."""

import hashlib
import json
import os
import pathlib
import shutil
import signal
import subprocess
import sys
import time

R = pathlib.Path(__file__).resolve().parent
W = os.name == "nt"
platform = "win32-x64" if W else "linux-x64"
O = R / "implementation-artifacts" / platform / "edges"
O.mkdir(parents=True, exist_ok=True)
C = pathlib.Path(sys.argv[1]).resolve()
manifest = json.loads((R / "linux/manifest.json").read_text())
channel, a = next(
    (k, v["versions"]["1.2.0"][platform])
    for k, v in manifest["channels"].items()
    if "1.2.0" in v["versions"] and platform in v["versions"]["1.2.0"]
)
sdk = (
    (R / "windows/sdks/1.2.0/cangjie")
    if W
    else pathlib.Path("/root/.cache/cjr-research-linux/1.2.0/cangjie")
)
layout = "windows_x86_64_cjnative" if W else "linux_x86_64_cjnative"
main = O / "main"
cjv = O / "managed-cjv"
install = cjv / "toolchains/sts"
(main / "metadata").mkdir(parents=True, exist_ok=True)
(main / "metadata/versions.json").write_text(json.dumps(manifest))
(main / ".cjr-store.json").write_text(
    json.dumps(dict(schema_version=1, kind="cjr-runtime-store"))
)
(cjv / ".cjv").mkdir(parents=True, exist_ok=True)
(cjv / ".cjv/layout-v2").touch()
for n in [".lifecycle.lock", ".lifecycle.lookup.lock"]:
    (cjv / n).touch()
(install / ".cjv").mkdir(parents=True, exist_ok=True)
(install / ".cjv/toolchain.toml").write_text(
    f'release = "{channel}-1.2.0"\ntuple = "{platform}"\nsha256 = "{a["sha256"]}"\n'
)
if not (install / "runtime/lib" / layout).exists():
    shutil.copytree(
        sdk / "runtime/lib" / layout, install / "runtime/lib" / layout, symlinks=True
    )
if (sdk / "LICENSE").exists():
    shutil.copy2(sdk / "LICENSE", install / "LICENSE")
e = dict(os.environ, CJR_HOME=str(main), CJV_HOME=str(cjv))
for k in list(e):
    if k.startswith(("CANGJIE", "LD_", "DYLD_")):
        e.pop(k)
e["PATH"] = (
    os.environ.get("SystemRoot", r"C:\Windows") + r"\System32" if W else "/usr/bin:/bin"
)
rows = []


def record(name, p, expected=0, contains=None):
    row = {
        "name": name,
        "exit": p.returncode,
        "expected": expected,
        "stdout": p.stdout.decode(errors="replace")[:3000],
        "stderr": p.stderr.decode(errors="replace")[:3000],
    }
    row["passed"] = p.returncode == expected and (
        contains is None or contains in row["stdout"] + row["stderr"]
    )
    rows.append(row)
    (O / "results.json").write_text(
        json.dumps(rows, indent=2, ensure_ascii=False), encoding="utf-8"
    )
    print(name, row["passed"], row["exit"], row["stderr"][:160], flush=True)
    return row


def run(name, args, expected=0, contains=None, env=None):
    return record(
        name,
        subprocess.run(
            [str(C), *map(str, args)], env=env or e, capture_output=True, timeout=180
        ),
        expected,
        contains,
    )


target = (
    (R / "windows/builds/1.2.0/dynamic-std/workload.exe")
    if W
    else pathlib.Path("/root/.cache/cjr-research-linux/artifacts/1.2.0/probe-dy-std")
)
run(
    "managed installation snapshot", ["--offline", "--verbose", target], contains="PASS"
)
receipt = json.loads(next((main / "runtimes").glob("*/receipt.json")).read_text())
assert receipt["provenance"] == "cjv-managed"

# Compile a correct official-SDK fixture exercising stdin, argv, exit status and sleep.
(O / "source").mkdir(exist_ok=True)
src = O / "source/arguments.cj"
src.write_text("""import std.sync.*
main(args: Array<String>): Int64 {
    for (arg in args) { println("ARG=[${arg}]") }
    if (args.size > 0 && args[0] == "wait") { sleep(10 * Duration.second) }
    return 37
}
""")
binary = O / ("arguments.exe" if W else "arguments")
compiler = sdk / "bin" / ("cjc.exe" if W else "cjc")
compiled = subprocess.run(
    [str(compiler), str(src), "--dy-std", "-o", str(binary)],
    cwd=src.parent,
    capture_output=True,
)
record("compile argv fixture", compiled)
arguments = ["--help", "--offline", "--", "a b"] + ([] if W else ["中文"])
run(
    "argument boundary / exit status",
    ["--offline", binary, *arguments],
    37,
    contains="ARG=[--]",
)

# Runtime use protection while the target is alive; also inspect real image paths.
p = subprocess.Popen(
    [str(C), "--offline", str(binary), "wait"],
    env=e,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
)
runtime = main / "runtimes" / receipt["bundle_id"]
loaded = []
child_pid = None
for _ in range(2400):
    if p.poll() is not None:
        break
    if W:
        import ctypes
        from ctypes import wintypes as wt

        class PE(ctypes.Structure):
            _fields_ = [
                ("size", wt.DWORD),
                ("usage", wt.DWORD),
                ("pid", wt.DWORD),
                ("heap", ctypes.c_size_t),
                ("module", wt.DWORD),
                ("threads", wt.DWORD),
                ("parent", wt.DWORD),
                ("priority", ctypes.c_long),
                ("flags", wt.DWORD),
                ("exe", wt.WCHAR * 260),
            ]

        k = ctypes.windll.kernel32
        k.CreateToolhelp32Snapshot.argtypes = [wt.DWORD, wt.DWORD]
        k.CreateToolhelp32Snapshot.restype = wt.HANDLE
        k.Process32FirstW.argtypes = [wt.HANDLE, ctypes.POINTER(PE)]
        k.Process32NextW.argtypes = [wt.HANDLE, ctypes.POINTER(PE)]
        k.CloseHandle.argtypes = [wt.HANDLE]
        h = k.CreateToolhelp32Snapshot(2, 0)
        entry = PE()
        entry.size = ctypes.sizeof(entry)
        ok = k.Process32FirstW(h, ctypes.byref(entry))
        while ok:
            if entry.parent == p.pid:
                child_pid = entry.pid
                break
            ok = k.Process32NextW(h, ctypes.byref(entry))
        k.CloseHandle(h)
        if child_pid:
            sys.path.insert(0, str(R / "windows"))
            from probe import loaded_modules

            loaded = loaded_modules(child_pid)
    else:
        children = pathlib.Path(f"/proc/{p.pid}/task/{p.pid}/children")
        ids = children.read_text().split() if children.exists() else []
        if ids:
            child_pid = int(ids[0])
            maps = pathlib.Path(f"/proc/{child_pid}/maps")
            if maps.exists():
                loaded = sorted(
                    {
                        line.split()[-1]
                        for line in maps.read_text().splitlines()
                        if "libcangjie" in line
                    }
                )
    if loaded:
        break
    time.sleep(0.025)
assert loaded, "no actual runtime images observed"
assert all(
    str(runtime.resolve()).replace("\\", "/").lower() in s.replace("\\", "/").lower()
    for s in loaded
), loaded
(O / "loaded-images.json").write_text(json.dumps(loaded, indent=2))
run("clean while in use", ["cache", "clean", "--all"], contains="skipped: active")
assert runtime.exists()
if W:
    # Preserve the native application's nonzero exit, including suspended/job setup.
    out, err = p.communicate(timeout=30)
    record(
        "wait/exit status", subprocess.CompletedProcess([], p.returncode, out, err), 37
    )
else:
    p.send_signal(signal.SIGTERM)
    out, err = p.communicate(timeout=10)
    record(
        "forward parent SIGTERM",
        subprocess.CompletedProcess([], p.returncode, out, err),
        -signal.SIGTERM,
    )
run("clean after exit", ["cache", "clean", "--all"])
assert not runtime.exists()

# Dynamic application dependencies built with the same SDK (no mixed-SDK tests).
shared_env = dict(
    e,
    CJR_HOME=str(R / "implementation-artifacts" / platform / "main"),
    CJV_HOME=str(R / "implementation-artifacts" / platform / "cjv"),
)
for v in ["1.0.5", "1.1.3", "1.2.0"]:
    if W:
        app = R / "windows-dylib/builds" / v / "app.exe"
    else:
        app = (
            pathlib.Path("/root/.cache/cjr-research-linux/artifacts")
            / v
            / "probe-dynamic"
        )
        shared_env["LD_LIBRARY_PATH"] = str(app.parent)
    run(
        "same SDK dynamic library " + v,
        ["--offline", app],
        contains="cross-library exception" if W else "PASS",
        env=shared_env,
    )
if not W:
    shared_env.pop("LD_LIBRARY_PATH", None)
    for mode, expected in [("old-rpath", 1), ("runpath", 0)]:
        app = (
            pathlib.Path("/root/.cache/cjr-research-linux/artifacts/1.0.5")
            / f"probe-{mode}"
        )
        if app.exists():
            run(mode, ["--offline", app], expected, env=shared_env)
print("passed", sum(x["passed"] for x in rows), "of", len(rows))
sys.exit(0 if all(x["passed"] for x in rows) else 1)
