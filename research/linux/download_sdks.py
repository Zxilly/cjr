import concurrent.futures
import hashlib
import json
import pathlib
import time
import urllib.request

root = pathlib.Path("/mnt/e/Project/CS_Project/2026/cjr/research/linux")
root.mkdir(parents=True, exist_ok=True)
url = "https://raw.githubusercontent.com/Zxilly/cangjie-version-manifest/master/versions.json"
try:
    data = urllib.request.urlopen(url, timeout=30).read()
    (root / "manifest.json").write_bytes(data)
    print("manifest: online refreshed", flush=True)
except Exception as e:
    data = pathlib.Path(
        "/mnt/e/Project/CS_Project/2026/cangjie-version-manifest/versions.json"
    ).read_bytes()
    (root / "manifest.json").write_bytes(data)
    print("manifest: local fallback", type(e).__name__, flush=True)
m = json.loads(data)
items = []
for channel, version in [("lts", "1.0.5"), ("sts", "1.1.3"), ("sts", "1.2.0")]:
    item = dict(m["channels"][channel]["versions"][version]["linux-x64"])
    item.update(version=version, channel=channel)
    item["sha256"] = item["sha256"].lower()
    items.append(item)
(root / "selected-assets.json").write_text(json.dumps(items, indent=2))
archives = root / "sdk-archives"
archives.mkdir(exist_ok=True)


def digest(p):
    h = hashlib.sha256()
    with p.open("rb") as f:
        for b in iter(lambda: f.read(4 * 1024 * 1024), b""):
            h.update(b)
    return h.hexdigest()


def get(item):
    p = archives / item["name"]
    start = time.time()
    if p.exists() and digest(p) == item["sha256"]:
        return {
            "version": item["version"],
            "bytes": p.stat().st_size,
            "verified": True,
            "cached": True,
        }
    tmp = p.with_suffix(p.suffix + ".partial")
    if tmp.exists() and digest(tmp) == item["sha256"]:
        tmp.replace(p)
        return {
            "version": item["version"],
            "bytes": p.stat().st_size,
            "verified": True,
            "cached": True,
        }
    req = urllib.request.Request(
        item["url"], headers={"User-Agent": "cjr-feasibility-research/0.1"}
    )
    with urllib.request.urlopen(req, timeout=120) as response, tmp.open("wb") as out:
        while True:
            block = response.read(1024 * 1024)
            if not block:
                break
            out.write(block)
    actual = digest(tmp)
    if actual != item["sha256"]:
        raise ValueError("checksum mismatch " + item["version"])
    tmp.replace(p)
    result = {
        "version": item["version"],
        "bytes": p.stat().st_size,
        "verified": True,
        "seconds": round(time.time() - start, 2),
    }
    print(result, flush=True)
    return result


results = []
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
    futures = {pool.submit(get, item): item for item in items}
    for f in concurrent.futures.as_completed(futures):
        try:
            results.append(f.result())
        except Exception as e:
            result = {
                "version": futures[f]["version"],
                "error": type(e).__name__,
                "message": str(e),
            }
            print(result, flush=True)
            results.append(result)
(root / "download-results.json").write_text(json.dumps(results, indent=2))
