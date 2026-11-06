"""Windows dynamic Cangjie package/version matrix; read-only installed SDKs.
Low-level PE/module enumeration helpers adapted from sibling Windows research probe.
"""

import ctypes
import hashlib
import json
import os
import shutil
import struct
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent
ctypes.windll.kernel32.SetErrorMode(0x0001 | 0x0002 | 0x8000)


def sha(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(4 * 1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def pe_info(path):
    b = Path(path).read_bytes()
    pe = struct.unpack_from("<I", b, 0x3C)[0]
    count = struct.unpack_from("<H", b, pe + 6)[0]
    opts = struct.unpack_from("<H", b, pe + 20)[0]
    magic = struct.unpack_from("<H", b, pe + 24)[0]
    if magic != 0x20B:
        raise ValueError("probe supports PE32+ only")
    base = struct.unpack_from("<Q", b, pe + 48)[0]
    sections = []
    for i in range(count):
        p = pe + 24 + opts + 40 * i
        name = b[p : p + 8].rstrip(b"\0").decode()
        size, rva, rawsize, raw = struct.unpack_from("<IIII", b, p + 8)
        sections.append((name, size, rva, rawsize, raw))

    def offset(rva):
        for _, _, start, rawsize, raw in sections:
            if start <= rva < start + rawsize:
                return raw + rva - start
        raise ValueError(f"unmapped RVA {rva:x}")

    versions = []
    for name, size, _, _, raw in sections:
        if name == ".cjsdkv":
            for p in range(raw, raw + size, 8):
                va = struct.unpack_from("<Q", b, p)[0]
                pos = offset(va - base)
                versions.append(b[pos : b.index(0, pos, pos + 256)].decode())
    out = subprocess.run(
        ["llvm-readobj", "--coff-imports", str(path)],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    imports = [
        s.strip().removeprefix("Name: ")
        for s in out.splitlines()
        if s.strip().startswith("Name: ")
    ]
    return {"versions": versions, "imports": imports, "sha256": sha(path)}


def loaded_modules(pid):
    # Native module enumeration; confirms actual loader choice, not predicted PATH.
    from ctypes import wintypes as w

    class Entry(ctypes.Structure):
        _fields_ = [
            ("dwSize", w.DWORD),
            ("th32ModuleID", w.DWORD),
            ("th32ProcessID", w.DWORD),
            ("GlblcntUsage", w.DWORD),
            ("ProccntUsage", w.DWORD),
            ("modBaseAddr", ctypes.c_void_p),
            ("modBaseSize", w.DWORD),
            ("hModule", w.HMODULE),
            ("szModule", w.WCHAR * 256),
            ("szExePath", w.WCHAR * 260),
        ]

    k = ctypes.windll.kernel32
    k.CreateToolhelp32Snapshot.restype = w.HANDLE
    k.CreateToolhelp32Snapshot.argtypes = [w.DWORD, w.DWORD]
    k.Module32FirstW.argtypes = [w.HANDLE, ctypes.POINTER(Entry)]
    k.Module32NextW.argtypes = [w.HANDLE, ctypes.POINTER(Entry)]
    k.CloseHandle.argtypes = [w.HANDLE]
    h = k.CreateToolhelp32Snapshot(0x8 | 0x10, pid)
    if h == ctypes.c_void_p(-1).value:
        return []
    e = Entry()
    e.dwSize = ctypes.sizeof(e)
    result = []
    try:
        ok = k.Module32FirstW(h, ctypes.byref(e))
        while ok:
            if any(
                name in e.szModule.lower()
                for name in ["cangjie", "boundscheck", "bridgeprobe"]
            ):
                result.append(e.szExePath)
            ok = k.Module32NextW(h, ctypes.byref(e))
    finally:
        k.CloseHandle(h)
    return result


def run(exe, runtime):
    env = {
        k: os.environ[k]
        for k in ["SYSTEMROOT", "WINDIR", "TEMP", "TMP", "USERPROFILE"]
        if k in os.environ
    }
    env["PATH"] = (str(runtime) + os.pathsep if runtime else "") + str(
        Path(env["SYSTEMROOT"]) / "System32"
    )
    start = time.monotonic()
    proc = subprocess.Popen(
        [str(exe)],
        cwd=exe.parent,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    modules = []
    for _ in range(15):
        if proc.poll() is not None:
            break
        modules = loaded_modules(proc.pid)
        if modules:
            break
        time.sleep(0.05)
    try:
        stdout, stderr = proc.communicate(timeout=25)
    except subprocess.TimeoutExpired:
        proc.kill()
        stdout, stderr = proc.communicate()
        return {"timeout": True, "stdout": stdout, "stderr": stderr, "modules": modules}
    return {
        "exit_code": proc.returncode,
        "stdout": stdout.strip(),
        "stderr": stderr.strip(),
        "modules": modules,
        "seconds": round(time.monotonic() - start, 3),
    }


def main():
    sdks = {
        v: Path.home() / ".cjv/toolchains" / n
        for v, n in [
            ("1.0.5", "lts-1.0.5"),
            ("1.1.3", "sts-1.1.3"),
            ("1.2.0", "sts-1.2.0"),
        ]
    }
    result = {
        "host": "Windows x64",
        "sdk_source": "Existing cjv official managed installs; no per-file original-archive authentication in this experiment",
        "builds": [],
        "runs": [],
        "strip_runs": [],
    }

    def save():
        (ROOT / "results.json").write_text(
            json.dumps(result, indent=2), encoding="utf-8"
        )

    for version, sdk in sdks.items():
        dest = ROOT / "builds" / version
        dest.mkdir(parents=True, exist_ok=True)
        commands = [
            [
                str(sdk / "bin/cjc.exe"),
                str(ROOT / "bridgeprobe.cj"),
                "--output-type=dylib",
                "--dy-std",
                "--output-dir",
                str(dest),
                "-o",
                "libbridgeprobe.dll",
            ],
            [
                str(sdk / "bin/cjc.exe"),
                str(ROOT / "main.cj"),
                "--dy-std",
                "--import-path",
                str(dest),
                "-L",
                str(dest),
                "-lbridgeprobe",
                "-o",
                str(dest / "app.exe"),
            ],
        ]
        for command in commands:
            proc = subprocess.run(command, capture_output=True, text=True, timeout=90)
            result["builds"].append(
                {
                    "version": version,
                    "command": command,
                    "exit": proc.returncode,
                    "stderr": proc.stderr,
                }
            )
            if proc.returncode:
                save()
                raise RuntimeError(proc.stderr)
        result["builds"][-1]["app"] = pe_info(dest / "app.exe")
        result["builds"][-1]["library"] = pe_info(dest / "libbridgeprobe.dll")
        for name in ["app.exe", "libbridgeprobe.dll"]:
            stripped = dest / ("stripped-" + name)
            subprocess.run(
                ["llvm-strip", "--strip-all", "-o", str(stripped), str(dest / name)],
                check=True,
            )
            result["builds"][-1]["stripped_" + name] = pe_info(stripped)
        print("built", version, flush=True)
        save()
    for app_v in sdks:
        for lib_v in sdks:
            dest = ROOT / "runs" / ("app-" + app_v + "_lib-" + lib_v)
            dest.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / "builds" / app_v / "app.exe", dest / "app.exe")
            shutil.copyfile(
                ROOT / "builds" / lib_v / "libbridgeprobe.dll",
                dest / "libbridgeprobe.dll",
            )
            for runtime_v, sdk in sdks.items():
                runtime = sdk / "runtime/lib/windows_x86_64_cjnative"
                outcome = run(dest / "app.exe", runtime)
                result["runs"].append(
                    {
                        "app_sdk": app_v,
                        "library_sdk": lib_v,
                        "runtime_sdk": runtime_v,
                        **outcome,
                    }
                )
                print(
                    "run",
                    app_v,
                    lib_v,
                    runtime_v,
                    outcome.get("exit_code", "timeout"),
                    flush=True,
                )
                save()
    for version, sdk in sdks.items():
        dest = ROOT / "stripped-runs" / version
        dest.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(
            ROOT / "builds" / version / "stripped-app.exe", dest / "app.exe"
        )
        shutil.copyfile(
            ROOT / "builds" / version / "stripped-libbridgeprobe.dll",
            dest / "libbridgeprobe.dll",
        )
        result["strip_runs"].append(
            {
                "version": version,
                **run(dest / "app.exe", sdk / "runtime/lib/windows_x86_64_cjnative"),
            }
        )
        save()
    print("done", flush=True)


if __name__ == "__main__":
    main()
