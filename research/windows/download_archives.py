"""Portable, resumable official Windows SDK research download helper."""

import concurrent.futures
import hashlib
import json
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent
MANIFEST = json.loads((ROOT / "versions.json").read_text())


def digest(path):
    h = hashlib.sha256()
    with path.open("rb") as f:
        for b in iter(lambda: f.read(4 * 1024 * 1024), b""):
            h.update(b)
    return h.hexdigest()


def fetch(pair):
    channel, version = pair
    a = MANIFEST["channels"][channel]["versions"][version]["win32-x64"]
    expected = a["sha256"].lower()
    dest = ROOT / "archives" / a["name"]
    partial = dest.with_suffix(".partial")
    if dest.exists() and digest(dest) == expected:
        print("already complete", version, flush=True)
        return
    if partial.exists() and digest(partial) == expected:
        partial.replace(dest)
        print("promoted complete partial", version, flush=True)
        return
    start = partial.stat().st_size if partial.exists() else 0
    headers = {"User-Agent": "cjr-feasibility-research/0.1"}
    if start:
        headers["Range"] = f"bytes={start}-"
    req = urllib.request.Request(a["url"], headers=headers)
    with urllib.request.urlopen(req, timeout=90) as response:
        append = response.status == 206
        if append:
            assert response.headers["Content-Range"].startswith(f"bytes {start}-")
        elif response.status != 200:
            raise ValueError(f"unexpected HTTP {response.status}")
        print(
            "download",
            version,
            "resume" if append else "full",
            "offset",
            start,
            flush=True,
        )
        with partial.open("ab" if append else "wb") as f:
            while b := response.read(1024 * 1024):
                f.write(b)
    actual = digest(partial)
    assert actual == expected, (version, actual, expected)
    partial.replace(dest)
    print("verified", version, dest.stat().st_size, flush=True)


with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
    list(pool.map(fetch, [("lts", "1.0.5"), ("sts", "1.1.3")]))
