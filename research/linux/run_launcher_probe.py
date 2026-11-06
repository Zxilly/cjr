import json
import pathlib

from launcher_probe import ROOT, STORE, VERSIONS, execute, requirement

out = []
for version in VERSIONS:
    for mode in ["default", "dy-std", "static"]:
        binary = STORE / "artifacts" / version / f"probe-{mode}-stripped"
        r = execute(binary, debug=True)
        full = r.pop("stderr")
        (ROOT / f"{version}-{mode}-runtime-only-loader.txt").write_text(full)
        r["stderr_excerpt"] = full[-700:]
        out.append(r)
        print(
            version,
            mode,
            r["detected_version"],
            r["exit"],
            r["stdout"].strip(),
            flush=True,
        )
for appv, libv in [("1.2.0", "1.1.3"), ("1.1.3", "1.0.5")]:
    p = STORE / "artifacts" / appv / f"mixed-static-with-{libv}"
    try:
        result = {"binary": str(p), "detected": requirement(p)}
    except ValueError as e:
        result = {"binary": str(p), "expected_rejection": str(e)}
    out.append(result)
    print(result, flush=True)
(ROOT / "launcher-results.json").write_text(
    json.dumps(out, ensure_ascii=False, indent=2)
)
