import hashlib
import json
import pathlib
import subprocess

root = pathlib.Path("/mnt/e/Project/CS_Project/2026/cjr/research/linux")
store = pathlib.Path((root / "sdk-store.txt").read_text())
for v in ["1.0.5", "1.1.3", "1.2.0"]:
    p = store / v / "cangjie/lib/linux_x86_64_cjnative/cjld.lds"
    lines = p.read_text().splitlines()
    i = next(
        i for i, l in enumerate(lines) if "/* Cangjie Metadata sections start */" in l
    )
    (root / f"{v}-linker-metadata.txt").write_text(
        "Source: "
        + str(p)
        + "\nSHA256: "
        + hashlib.sha256(p.read_bytes()).hexdigest()
        + "\n"
        + "\n".join(f"{n + 1}: {lines[n]}" for n in range(i, min(i + 23, len(lines))))
        + "\n"
    )
report = []
for v in ["1.0.5", "1.1.3", "1.2.0"]:
    p = store / "runtime-only" / v
    report.append(
        {
            "version": v,
            "runtime_only_bytes": sum(
                x.stat().st_size for x in p.rglob("*") if x.is_file()
            ),
            "archive_bytes": (
                root / "sdk-archives" / f"cangjie-sdk-linux-x64-{v}.tar.gz"
            )
            .stat()
            .st_size,
        }
    )
(root / "sizes.json").write_text(json.dumps(report, indent=2))
print(report)
