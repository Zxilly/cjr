import json
import pathlib
import runpy

mod = runpy.run_path("research/macos/inspect_macho.py")
root = pathlib.Path("research/macos").resolve()
result = []
for sdk in sorted((root / "extracted").iterdir()):
    items = [mod["inspect"](p) for p in (sdk / "cangjie/runtime/lib").glob("*/*.dylib")]
    byname = {pathlib.Path(i["path"]).name for i in items}
    missing = []
    extraction = json.loads((root / "extraction.json").read_text())
    links = [
        x
        for entry in extraction
        if entry["archive"] == sdk.name + ".tar.gz"
        for x in entry["selected"]
        if isinstance(x, dict)
    ]
    linkmap = {pathlib.PurePosixPath(x["link"]).name: x["target"] for x in links}
    for name, target in linkmap.items():
        seen = set()
        while target in linkmap:
            assert target not in seen, "link cycle"
            seen.add(target)
            target = linkmap[target]
        assert target in byname, "missing symlink target"
    byname.update(linkmap)
    for i in items:
        for dep in i.get("dependencies", []):
            if dep["command"] == "0xd":
                continue
            name = dep["path"]
            if name.startswith("@rpath/") and name[7:] not in byname:
                missing.append({"file": i["path"], "dependency": name})
    cjpm = mod["inspect"](sdk / "cangjie/tools/bin/cjpm")
    summary = {
        "sdk": sdk.name,
        "regular_libraries": len(items),
        "library_symlinks": links,
        "metadata_libraries": sum(bool(i["metadata_sections"]) for i in items),
        "version_values": sorted(set(v for i in items for v in i["versions"])),
        "metadata_decode_errors": sum(len(i["decode_errors"]) for i in items),
        "unresolved_rpath_basenames": missing,
        "cjpm": cjpm,
    }
    result.append(summary)
    print(json.dumps(summary, ensure_ascii=False), flush=True)
(root / "library-summary.json").write_text(
    json.dumps(result, ensure_ascii=False, indent=2)
)
