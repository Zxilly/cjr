use object::{Object, ObjectSection};
use std::error::Error;

fn inspect(path: &std::path::Path) -> Result<serde_json::Value, Box<dyn Error>> {
    let bytes = std::fs::read(path)?;
    let file = object::File::parse(&bytes[..])?;
    if file.format() != object::BinaryFormat::Pe || !file.is_64() || !file.is_little_endian() {
        return Err("expected PE32+ little endian".into());
    }
    let version_section = file.section_by_name(".cjsdkv").ok_or("missing .cjsdkv")?;
    let pointers = version_section.data()?;
    if pointers.len() % 8 != 0 { return Err("unaligned version pointer table".into()); }
    let mut versions = Vec::new();
    for pointer in pointers.chunks_exact(8) {
        let address = u64::from_le_bytes(pointer.try_into()?);
        let section = file.sections().find(|s| {
            s.address().checked_add(s.size()).is_some_and(|end| address >= s.address() && address < end)
        }).ok_or("unmapped VA")?;
        let offset = usize::try_from(address.checked_sub(section.address()).ok_or("bad VA")?)?;
        let section_bytes = section.data()?;
        let rest = section_bytes.get(offset..).ok_or("VA beyond file bytes")?;
        let length = rest.iter().take(256).position(|b| *b == 0).ok_or("unterminated version")?;
        versions.push(std::str::from_utf8(&rest[..length])?.to_owned());
    }
    let mut libraries = Vec::new();
    for entry in file.imports()? {
        let import = entry?;
        libraries.push(String::from_utf8_lossy(import.library()).into_owned());
    }
    libraries.sort(); libraries.dedup();
    Ok(serde_json::json!({"path":path,"versions":versions,"libraries":libraries}))
}

fn main() -> Result<(), Box<dyn Error>> {
    let results: Result<Vec<_>,_> = std::env::args_os().skip(1)
        .map(|p| inspect(std::path::Path::new(&p))).collect();
    println!("{}", serde_json::to_string_pretty(&results?)?);
    Ok(())
}
