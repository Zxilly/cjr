import json
import pathlib
import shutil
import subprocess

from run_matrix import OUT, ROOT, STORE, VERSIONS, env, inspect, run, runtime

r = json.loads((ROOT / "results.json").read_text())
r["metadata"] = [inspect(pathlib.Path(x["path"])) for x in r["metadata"]]
for v in VERSIONS:
    for mode in ["default", "dy-std", "static"]:
        p = OUT / v / f"probe-{mode}-stripped"
        x = run([p], env=env(v, debug=True), cwd=p.parent)
        (ROOT / f"{v}-{mode}-loader.txt").write_text(x["stderr"])
    d = OUT / v / "staticlib"
    d.mkdir(exist_ok=True)
    x = run(
        [
            STORE / v / "cangjie/bin/cjc",
            ROOT / "probelib.cj",
            "--output-type=staticlib",
            "-o",
            d / "libprobelib.a",
        ],
        cwd=d,
    )
    x.update(version=v, mode="static-library")
    r["builds"].append(x)
for appv, libv in [("1.2.0", "1.1.3"), ("1.1.3", "1.0.5")]:
    d = OUT / appv
    lib = OUT / libv / "staticlib"
    p = d / f"mixed-static-with-{libv}"
    x = run(
        [
            STORE / appv / "cangjie/bin/cjc",
            ROOT / "dynamic.cj",
            "--import-path",
            lib,
            "--static-libs",
            "-L",
            lib,
            "-lprobelib",
            "-o",
            p,
        ],
        cwd=d,
    )
    x.update(version=appv, library_version=libv, mode="mixed-static-app")
    r["builds"].append(x)
    print("mixed-static-build", appv, libv, x["exit"], x["stderr"], flush=True)
    if x["exit"] == 0:
        md = inspect(p)
        r["metadata"].append(md)
        print("metadata", md, flush=True)
        x = run([p], env=env(appv), cwd=d)
        x.update(
            binary_version=appv,
            library_version=libv,
            mode="mixed-static-app",
            runtime_version=appv,
        )
        r["runs"].append(x)
        print("run", x, flush=True)
# Preserve small sample artifacts in the repository research directory for the parent to inspect.
samples = ROOT / "artifacts"
samples.mkdir(exist_ok=True)
for v in VERSIONS:
    d = samples / v
    d.mkdir(exist_ok=True)
    for name in [
        "probe-default-stripped",
        "probe-dy-std-stripped",
        "probe-static-stripped",
        "probe-dynamic",
        "libprobelib.so",
    ]:
        shutil.copy2(OUT / v / name, d / name)
(ROOT / "results.json").write_text(json.dumps(r, ensure_ascii=False, indent=2))
