fn main() { println!("dependency graph compiled"); }

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Read, Write};
    use object::Object;
    use serde::Deserialize;
    use sha2::{Digest, Sha256};

    #[test]
    fn sha256_known_vector() {
        let hex: String = Sha256::digest(b"abc").iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(hex,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn sdk_config_toml_and_manifest_json() {
        #[derive(Deserialize)]
        struct Config { home: String }
        let config: Config = toml::from_str("home = 'C:/sdk'").unwrap();
        assert_eq!(config.home, "C:/sdk");
        let manifest: serde_json::Value = serde_json::from_str(r#"{"latest":null,"versions":{"1.2.0":{}}}"#).unwrap();
        assert!(manifest["latest"].is_null());
    }

    #[test]
    fn zip_deflate_roundtrip() {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        writer.start_file("runtime/lib/probe.dll", options).unwrap();
        writer.write_all(b"runtime-payload").unwrap();
        let bytes = writer.finish().unwrap();
        let mut archive = zip::ZipArchive::new(bytes).unwrap();
        let mut file = archive.by_index(0).unwrap();
        assert_eq!(file.enclosed_name().unwrap(), std::path::PathBuf::from("runtime/lib/probe.dll"));
        let mut out = String::new();
        file.read_to_string(&mut out).unwrap();
        assert_eq!(out, "runtime-payload");
    }

    #[test]
    fn tar_gzip_roundtrip() {
        let gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        let mut writer = tar::Builder::new(gzip);
        let data = b"runtime-payload";
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        writer.append_data(&mut header, "runtime/lib/probe.so", &data[..]).unwrap();
        let gzip = writer.into_inner().unwrap();
        let compressed = gzip.finish().unwrap();
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(&compressed[..]));
        let mut files = archive.entries().unwrap();
        let mut file = files.next().unwrap().unwrap();
        assert_eq!(file.path().unwrap().to_string_lossy(), "runtime/lib/probe.so");
        let mut out = Vec::new();
        file.read_to_end(&mut out).unwrap();
        assert_eq!(out, data);
    }

    #[test]
    fn object_reads_native_test_executable() {
        let data = std::fs::read(std::env::current_exe().unwrap()).unwrap();
        let file = object::File::parse(&data[..]).unwrap();
        assert!(file.sections().count() > 0);
        assert_eq!(file.architecture(), object::Architecture::X86_64);
    }

    #[test]
    fn rustls_blocking_client_constructs() {
        let _client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build().unwrap();
    }

    #[test]
    fn tempfile_create_and_cleanup() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("receipt.json");
        std::fs::write(&path, b"{}").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"{}");
    }
}
