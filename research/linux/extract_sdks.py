import json
import pathlib
import subprocess
import tarfile

root = pathlib.Path("/mnt/e/Project/CS_Project/2026/cjr/research/linux")
store = pathlib.Path.home() / ".cache/cjr-research-linux"
store.mkdir(parents=True, exist_ok=True)
print("sdk_store", store, flush=True)
(root / "sdk-store.txt").write_text(str(store))
for v in ["1.2.0", "1.1.3", "1.0.5"]:
    arc = root / "sdk-archives" / f"cangjie-sdk-linux-x64-{v}.tar.gz"
    if not arc.exists():
        continue
    dest = store / v
    if not (dest / "cangjie/bin/cjc").exists():
        dest.mkdir(exist_ok=True)
        subprocess.run(["tar", "-xzf", str(arc), "-C", str(dest)], check=True)
    sdk = dest / "cangjie"
    print(
        v,
        subprocess.run(
            [str(sdk / "bin/cjc"), "--version"], capture_output=True, text=True
        ).stdout,
        flush=True,
    )
