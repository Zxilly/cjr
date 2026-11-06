import json
import subprocess

from probe import ROOT, pe_info, run

results = []
for version in ["1.1.3", "1.2.0"]:
    sdk = ROOT / "sdks" / version / "cangjie"
    for label, source in [
        ("hello", 'main() { println("hello-static"); 0 }'),
        ("thread", "main() { let f = spawn { => 42 }; println(f.get()); 0 }"),
    ]:
        d = ROOT / "builds" / "static-isolation" / version / label
        d.mkdir(parents=True, exist_ok=True)
        (d / "main.cj").write_text(source)
        c = subprocess.run(
            [
                str(sdk / "bin/cjc.exe"),
                str(d / "main.cj"),
                "--static",
                "-o",
                str(d / "app.exe"),
            ],
            capture_output=True,
            text=True,
        )
        r = {
            "version": version,
            "case": label,
            "compile_exit": c.returncode,
            "compile_stderr": c.stderr,
        }
        if c.returncode == 0:
            r.update({"pe": pe_info(d / "app.exe"), "run": run(d / "app.exe", None)})
        results.append(r)
        print(version, label, r.get("run", {}).get("exit_code"), flush=True)
(ROOT / "static-isolation.json").write_text(json.dumps(results, indent=2))
