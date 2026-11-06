import json
import pathlib
import subprocess

from run_matrix import OUT, ROOT, STORE, env, run, runtime

out = []
for tags in ["old", "new"]:
    p = OUT / "1.0.5" / ("probe-" + tags + "-rpath")
    flags = [
        "--link-options",
        ("--disable-new-dtags" if tags == "old" else "--enable-new-dtags")
        + " -rpath "
        + str(runtime("1.1.3")),
    ]
    r = run(
        [
            STORE / "1.0.5/cangjie/bin/cjc",
            ROOT / "probe.cj",
            "--dy-std",
            *flags,
            "-o",
            p,
        ],
        cwd=p.parent,
    )
    print("build", tags, r["exit"], r["stderr"], flush=True)
    entry = {"kind": tags, "build": r}
    if r["exit"] == 0:
        entry["dynamic"] = subprocess.getoutput("readelf -dW " + str(p))
        result = run([p], env=env("1.0.5", debug=True), cwd=p.parent, timeout=35)
        full = result.pop("stderr")
        (ROOT / f"rpath-{tags}-loader.txt").write_text(full)
        result["stderr_excerpt"] = full[-1000:]
        entry["run"] = result
        print("run", tags, result["exit"], result["stdout"], flush=True)
    out.append(entry)
(ROOT / "rpath-results.json").write_text(json.dumps(out, ensure_ascii=False, indent=2))
