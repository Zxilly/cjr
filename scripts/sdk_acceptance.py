"""Native SDK acceptance for CI or a local host. Downloads only verified official assets.

python scripts/sdk_acceptance.py --cjr target/release/cjr --versions 1.0.5 1.2.0
Artifacts and complete transcripts are retained under --root (default: target/sdk-tests).
"""

import argparse
import hashlib
import json
import os
import pathlib
import platform
import shutil
import subprocess
import tarfile
import urllib.request
import zipfile

parser = argparse.ArgumentParser()
parser.add_argument("--cjr", required=True, type=pathlib.Path)
parser.add_argument(
    "--root", type=pathlib.Path, default=pathlib.Path("target/sdk-tests")
)
parser.add_argument("--versions", nargs="+", default=["1.0.5", "1.2.0"])
args = parser.parse_args()
root = args.root.resolve()
root.mkdir(parents=True, exist_ok=True)
cjr = args.cjr.resolve()
repo = pathlib.Path(__file__).resolve().parent.parent
host = platform.system()
arch = platform.machine().lower()
key = {
    "Windows": "win32-x64",
    "Linux": "linux-x64",
    "Darwin": "darwin-arm64" if arch in ("arm64", "aarch64") else "darwin-x64",
}[host]
manifest_url = "https://raw.githubusercontent.com/Zxilly/cangjie-version-manifest/refs/heads/master/versions.json"
manifest_bytes = urllib.request.urlopen(manifest_url, timeout=60).read()
manifest = json.loads(manifest_bytes)
results = []


def run(name, command, env, expected=0):
    p = subprocess.run(
        list(map(str, command)), check=False, env=env, capture_output=True, timeout=300
    )
    results.append(
        {
            "name": name,
            "command": list(map(str, command)),
            "exit": p.returncode,
            "expected": expected,
            "stdout": p.stdout.decode(errors="replace"),
            "stderr": p.stderr.decode(errors="replace"),
        }
    )
    (root / "results.json").write_text(
        json.dumps(results, indent=2, ensure_ascii=False), encoding="utf-8"
    )
    if p.returncode != expected:
        raise RuntimeError(
            f"{name}: expected {expected}, got {p.returncode}\n{results[-1]['stderr']}"
        )
    return p


for version in args.versions:
    a = next(
        v["versions"][version][key]
        for v in manifest["channels"].values()
        if version in v["versions"] and key in v["versions"][version]
    )
    digest = a["sha256"].lower()
    if len(digest) != 64:
        raise RuntimeError(
            f"{version}/{key}: no official checksum; cannot certify this asset"
        )
    archive = root / a["name"]
    if not archive.exists():
        print("Downloading", a["name"], flush=True)
        with (
            urllib.request.urlopen(a["url"], timeout=120) as response,
            archive.open("wb") as output,
        ):
            shutil.copyfileobj(response, output)
    with archive.open("rb") as f:
        actual = hashlib.file_digest(f, "sha256").hexdigest()
    if actual != digest:
        raise RuntimeError("archive checksum mismatch")
    extracted = root / version / "sdk"
    if not extracted.exists():
        extracted.mkdir(parents=True)
        if archive.suffix == ".zip":
            with zipfile.ZipFile(archive) as z:
                z.extractall(extracted)
        else:
            with tarfile.open(archive, "r:gz") as t:
                t.extractall(extracted, filter="data")
    sdk = extracted / "cangjie"
    compiler = sdk / "bin" / ("cjc.exe" if host == "Windows" else "cjc")
    compile_env = dict(os.environ, CANGJIE_HOME=str(sdk))
    platform_flags = []
    if host == "Darwin":
        # Match the official envsetup.sh: cjc needs Apple's SDK for libSystem.
        compile_env["SDKROOT"] = subprocess.check_output(
            ["xcrun", "--sdk", "macosx", "--show-sdk-path"], text=True, timeout=30
        ).strip()
        # Old bundled LLD cannot read current Xcode TBD files. Use Apple's
        # matching linker in this disposable compiler tree only. The verified
        # archive reused by cjr, and all Cangjie libraries, remain untouched.
        apple_ld = (
            run(f"linker/{version}", ["xcrun", "--find", "ld"], compile_env)
            .stdout.decode()
            .strip()
        )
        bundled_ld = sdk / "third_party/llvm/bin/ld64.lld"
        saved_ld = bundled_ld.with_name("ld64.lld.bundled")
        if not saved_ld.exists():
            bundled_ld.rename(saved_ld)
            bundled_ld.symlink_to(apple_ld)
        # Match bundled LLD's ordinary pointer layout; chained metadata fixups
        # are outside the currently supported binary formats.
        platform_flags = ["--link-options", "-no_fixup_chains"]
    home = root / version / "cjr"
    cjv = root / version / "cjv"
    (home / "metadata").mkdir(parents=True, exist_ok=True)
    (home / "metadata/versions.json").write_bytes(manifest_bytes)
    (home / ".cjr-store.json").write_text(
        json.dumps({"schema_version": 1, "kind": "cjr-runtime-store"})
    )
    (cjv / "cache").mkdir(parents=True, exist_ok=True)
    cachekey = hashlib.sha256(
        ("artifact\0" + a["url"] + "\0" + digest).encode()
    ).hexdigest()
    cached = cjv / "cache" / f"{cachekey}.data"
    if not cached.exists():
        os.link(archive, cached)
    (cjv / "cache" / f"{cachekey}.json").write_text(
        json.dumps(
            {
                "key": cachekey,
                "kind": "artifact",
                "url": a["url"],
                "sha256": digest,
                "size": archive.stat().st_size,
            }
        )
    )
    env = dict(os.environ, CJR_HOME=str(home), CJV_HOME=str(cjv))
    for k in list(env):
        if k.startswith(("LD_", "DYLD_", "CANGJIE")):
            env.pop(k)
    env["PATH"] = (
        os.environ["SystemRoot"] + r"\System32"
        if host == "Windows"
        else "/usr/bin:/bin"
    )
    for mode, flags in [
        ("default", []),
        ("dynamic", ["--dy-std"]),
        ("stripped", ["--dy-std", "-s"]),
    ]:
        binary = root / version / (mode + (".exe" if host == "Windows" else ""))
        run(
            f"compile/{version}/{mode}",
            [
                compiler,
                repo / "tests/fixtures/workload.cj",
                *platform_flags,
                *flags,
                "-o",
                binary,
            ],
            compile_env,
        )
        inspected = run(
            f"inspect/{version}/{mode}", [cjr, "inspect", binary, "--json"], env
        )
        assert json.loads(inspected.stdout)["binary"]["sdk_version"] == version
        checked_env = dict(env)
        if host == "Linux":
            checked_env["LD_DEBUG"] = "libs"
        if host == "Darwin":
            checked_env["DYLD_PRINT_LIBRARIES"] = "1"
        app_args = ["--help", "--", "a b"] + ([] if host == "Windows" else ["中文"])
        result = run(
            f"run/{version}/{mode}",
            [cjr, "--offline", binary, *app_args],
            checked_env,
            37,
        )
        assert b"PASS collections exceptions threads allocations" in result.stdout
        assert b"ARG=[--]" in result.stdout and b"ARG=[a b]" in result.stdout
        if host in ("Linux", "Darwin"):
            images = [
                line
                for line in result.stderr.decode(errors="replace").splitlines()
                if ("calling init:" in line if host == "Linux" else True)
                and "libcangjie" in line
            ]
            assert images and all(str(home / "runtimes") in line for line in images), (
                images
            )
    run("list", [cjr, "list", "--json"], env)
    run("clean default", [cjr, "cache", "clean"], env)
    # Existing receipts must suffice when both metadata and cjv are unavailable.
    env["CJV_HOME"] = str(root / "absent-cjv")
    run("receipt offline", [cjr, "--offline", binary], env, 37)
    run("clean preview", [cjr, "cache", "clean", "--all", "--dry-run"], env)
print(f"{len(results)} native acceptance steps passed on {host}/{arch}")
