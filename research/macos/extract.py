import json
import pathlib
import tarfile

root = pathlib.Path("research/macos")
result = []
for a in (root / "archives").glob("*.tar.gz"):
    out = root / "extracted" / a.name.removesuffix(".tar.gz")
    out.mkdir(parents=True, exist_ok=True)
    with tarfile.open(a) as t:
        selected = []
        for e in t:
            pp = pathlib.PurePosixPath(e.name)
            if pp.is_absolute() or ".." in pp.parts:
                raise ValueError("invalid member")
            wanted = (
                "/runtime/lib/" in e.name
                or e.name.endswith("/cjpm")
                or e.name.endswith("/cjc")
                or e.name.endswith("/envsetup.sh")
            )
            if wanted and e.isfile():
                dest = out.joinpath(*pp.parts)
                dest.parent.mkdir(parents=True, exist_ok=True)
                with t.extractfile(e) as f:
                    dest.write_bytes(f.read())
                selected.append(e.name)
            elif wanted and e.issym():
                selected.append({"link": e.name, "target": e.linkname})
    result.append({"archive": a.name, "selected": selected})
    print(a.name, len(selected), flush=True)
(root / "extraction.json").write_text(json.dumps(result, indent=2))
