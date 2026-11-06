"""Reproducible Windows SDK/download/PE/runtime feasibility experiment.
Run from the repository with Python; no global SDK or environment changes.
"""

import ctypes
import hashlib
import json
import os
import shutil
import struct
import subprocess
import time
import urllib.request
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parent.parent
VERSIONS = ["1.0.5", "1.1.3", "1.2.0"]
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
            if "cangjie" in e.szModule.lower() or "boundscheck" in e.szModule.lower():
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
    (ROOT / "archives").mkdir(exist_ok=True)
    (ROOT / "sdks").mkdir(exist_ok=True)
    (ROOT / "builds").mkdir(exist_ok=True)
    url = "https://raw.githubusercontent.com/Zxilly/cangjie-version-manifest/refs/heads/master/versions.json"
    data = urllib.request.urlopen(url, timeout=45).read()
    (ROOT / "versions.json").write_bytes(data)
    manifest = json.loads(data)
    result = {
        "host": "Windows x64",
        "manifest_sha256": hashlib.sha256(data).hexdigest(),
        "sdks": [],
        "builds": [],
        "runs": [],
    }
    roots = {}
    for version in VERSIONS:
        channel = "lts" if version == "1.0.5" else "sts"
        asset = manifest["channels"][channel]["versions"][version]["win32-x64"]
        asset["sha256"] = asset["sha256"].lower()
        archive = ROOT / "archives" / asset["name"]
        source = "existing-research-archive"
        if not archive.exists() or sha(archive) != asset["sha256"]:
            cached = None
            for record in (Path.home() / ".cjv/cache").glob("*.json"):
                try:
                    entry = json.loads(record.read_text())
                    if (
                        entry.get("kind") == "artifact"
                        and entry.get("sha256") == asset["sha256"]
                    ):
                        candidate = record.with_suffix(".data")
                        if candidate.exists() and sha(candidate) == asset["sha256"]:
                            cached = candidate
                            break
                except (OSError, ValueError):
                    pass
            if cached:
                shutil.copyfile(cached, archive)
                source = "cjv-verified-archive-cache"
                print("reused cjv archive", version, flush=True)
            else:
                print("downloading official Windows SDK", version, flush=True)
                partial = archive.with_suffix(".partial")
                for attempt in range(3):
                    try:
                        request = urllib.request.Request(
                            asset["url"],
                            headers={"User-Agent": "cjr-feasibility-research/0.1"},
                        )
                        with (
                            urllib.request.urlopen(request, timeout=60) as r,
                            partial.open("wb") as f,
                        ):
                            shutil.copyfileobj(r, f, 4 * 1024 * 1024)
                        if sha(partial) != asset["sha256"]:
                            raise ValueError("official archive hash mismatch")
                        partial.replace(archive)
                        break
                    except Exception as error:
                        print(
                            "download retry",
                            version,
                            attempt + 1,
                            type(error).__name__,
                            str(error),
                            flush=True,
                        )
                        if attempt == 2:
                            raise
                        time.sleep(2)
                source = "official-download"
        digest = sha(archive)
        assert digest == asset["sha256"]
        dst = ROOT / "sdks" / version
        dst.mkdir(exist_ok=True)
        if not (dst / "complete.json").exists():
            print("extracting", version, flush=True)
            with zipfile.ZipFile(archive) as z:
                for info in z.infolist():
                    dest = (dst / info.filename).resolve()
                    if not dest.is_relative_to(dst.resolve()):
                        raise ValueError("escaping archive path")
                z.extractall(dst)
            (dst / "complete.json").write_text(json.dumps({"sha256": digest}))
        compiler = next(dst.rglob("cjc.exe"))
        sdk = compiler.parent.parent
        runtime = sdk / "runtime/lib/windows_x86_64_cjnative"
        if not runtime.exists():
            runtime = sdk / "runtime/lib/windows_x86_64_llvm"
        roots[version] = (sdk, runtime)
        result["sdks"].append(
            {
                "version": version,
                "source": source,
                "archive_sha256": digest,
                "archive_bytes": archive.stat().st_size,
                "sdk_root": str(sdk),
            }
        )
        (ROOT / "results.json").write_text(json.dumps(result, indent=2))

    for version, (sdk, _) in roots.items():
        for mode, flags in [
            ("default", []),
            ("dynamic-std", ["--dy-std"]),
            ("stripped", ["--dy-std", "-s"]),
            ("static", ["--static"]),
        ]:
            dest = ROOT / "builds" / version / mode
            dest.mkdir(parents=True, exist_ok=True)
            exe = dest / "workload.exe"
            cmd = [
                str(sdk / "bin/cjc.exe"),
                str(ROOT / "workload.cj"),
                *flags,
                "-o",
                str(exe),
            ]
            c = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
            build = {
                "sdk": version,
                "mode": mode,
                "command": cmd,
                "exit_code": c.returncode,
                "stdout": c.stdout,
                "stderr": c.stderr,
            }
            if c.returncode == 0:
                build["pe"] = pe_info(exe)
            result["builds"].append(build)
            print("built", version, mode, c.returncode, flush=True)
            if c.returncode == 0:
                for target in [None, *VERSIONS]:
                    outcome = run(exe, roots[target][1] if target else None)
                    result["runs"].append(
                        {
                            "build_sdk": version,
                            "mode": mode,
                            "runtime_sdk": target,
                            **outcome,
                        }
                    )
                    print(
                        "run",
                        version,
                        mode,
                        target,
                        outcome.get("exit_code", "timeout"),
                        flush=True,
                    )
            (ROOT / "results.json").write_text(json.dumps(result, indent=2))
    # Controlled app-directory shadowing: mismatched runtime beats PATH.
    dest = ROOT / "builds" / "shadowing"
    dest.mkdir(exist_ok=True)
    shutil.copyfile(
        ROOT / "builds" / "1.2.0" / "dynamic-std" / "workload.exe",
        dest / "workload.exe",
    )
    shutil.copyfile(
        roots["1.0.5"][1] / "libcangjie-runtime.dll", dest / "libcangjie-runtime.dll"
    )
    result["shadowing"] = run(dest / "workload.exe", roots["1.2.0"][1])
    (ROOT / "results.json").write_text(json.dumps(result, indent=2))
    print("done", flush=True)


if __name__ == "__main__":
    main()
