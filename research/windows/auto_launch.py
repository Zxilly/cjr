"""Research-only metadata -> exact official SDK -> native execution proof.

This uses verified research archives, not a finished cjr cache implementation.
"""

import json
import shutil
import sys
from pathlib import Path

from probe import ROOT, pe_info, run


def launch(path):
    report = pe_info(path)
    versions = set(report["versions"])
    if len(versions) != 1:
        raise ValueError(f"No unique SDK version: {sorted(versions)}")
    required = next(iter(versions))
    manifest = json.loads((ROOT / "versions.json").read_text())
    official = [
        channel["versions"][required]["win32-x64"]
        for channel in manifest["channels"].values()
        if required in channel["versions"]
        and "win32-x64" in channel["versions"][required]
    ]
    if len({a["sha256"].lower() for a in official}) != 1:
        raise ValueError("Official asset missing or ambiguous")
    receipt = json.loads((ROOT / "sdks" / required / "complete.json").read_text())
    if receipt["sha256"] != official[0]["sha256"].lower():
        raise ValueError("Research archive identity mismatch")
    source = ROOT / "sdks" / required / "cangjie/runtime/lib/windows_x86_64_cjnative"
    runtime = ROOT / "runtimes" / required
    runtime.mkdir(parents=True, exist_ok=True)
    for dll in source.glob("*.dll"):
        shutil.copy2(dll, runtime / dll.name)
    license_path = ROOT / "sdks" / required / "cangjie/LICENSE"
    if license_path.exists():
        shutil.copy2(license_path, runtime / "LICENSE")
    outcome = run(Path(path).resolve(), runtime)
    return {
        "binary": str(path),
        "detected_sdk": required,
        "selected_runtime": str(runtime),
        "outcome": outcome,
    }


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("usage: python auto_launch.py <PE executable>")
    result = launch(Path(sys.argv[1]))
    print(json.dumps(result, indent=2))
    raise SystemExit(result["outcome"].get("exit_code", 1))
