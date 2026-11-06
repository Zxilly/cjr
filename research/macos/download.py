import concurrent.futures
import hashlib
import json
import pathlib
import time
import urllib.request

root = pathlib.Path("research/macos")
(root / "archives").mkdir(exist_ok=True)
m = json.loads((root / "versions.json").read_bytes())
n = json.loads((root / "nightly.json").read_bytes())
items = [
    (
        "1.2.0",
        "darwin-arm64",
        m["channels"]["sts"]["versions"]["1.2.0"]["darwin-arm64"],
    ),
    (
        "1.0.5",
        "darwin-arm64",
        m["channels"]["lts"]["versions"]["1.0.5"]["darwin-arm64"],
    ),
]
x = [(v, a["darwin-x64"]) for v, a in n["versions"].items() if "darwin-x64" in a]
v, a = x[-1]
items.append((v, "darwin-x64", a))


def run(item):
    v, t, a = item
    p = root / "archives" / a["name"]
    start = time.time()
    h = hashlib.sha256()
    if not p.exists():
        with (
            urllib.request.urlopen(a["url"], timeout=60) as r,
            p.with_suffix(p.suffix + ".part").open("wb") as f,
        ):
            while b := r.read(1024 * 1024):
                f.write(b)
        p.with_suffix(p.suffix + ".part").replace(p)
    with p.open("rb") as f:
        while b := f.read(1024 * 1024):
            h.update(b)
    actual = h.hexdigest()
    expected = a["sha256"].lower()
    assert not expected or actual == expected
    out = {
        "version": v,
        "target": t,
        "name": a["name"],
        "bytes": p.stat().st_size,
        "sha256": actual,
        "expected_sha256": expected,
        "checksum_verified": bool(expected),
        "seconds": round(time.time() - start, 1),
    }
    print(json.dumps(out), flush=True)
    return out


with concurrent.futures.ThreadPoolExecutor(max_workers=3) as ex:
    results = list(ex.map(run, items))
(root / "downloads.json").write_text(json.dumps(results, indent=2))
