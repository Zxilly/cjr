import json

from auto_launch import ROOT, launch

results = []
for version in ["1.0.5", "1.1.3", "1.2.0"]:
    for mode in ["default", "dynamic-std", "stripped"]:
        result = launch(ROOT / "builds" / version / mode / "workload.exe")
        results.append(result)
        print(
            version,
            mode,
            result["detected_sdk"],
            result["outcome"]["exit_code"],
            flush=True,
        )
(ROOT / "auto-launch-results.json").write_text(json.dumps(results, indent=2))
assert all(r["outcome"]["exit_code"] == 0 for r in results)
for result in results:
    assert result["outcome"]["modules"]
    assert all(
        str(ROOT / "runtimes").lower() in path.lower()
        for path in result["outcome"]["modules"]
    )
