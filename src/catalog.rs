use crate::{Error, Result, config::Config, model::Artifact, util};
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path, time::Duration};

pub const STABLE: &str = "https://raw.githubusercontent.com/Zxilly/cangjie-version-manifest/refs/heads/master/versions.json";
pub const NIGHTLY: &str = "https://raw.githubusercontent.com/Zxilly/cangjie-version-manifest/refs/heads/master/nightly.json";
const MIRROR: &str = "https://raw.gitcode.com/Zxilly/cangjie-version-manifest/raw/master/";
#[derive(Default, Deserialize)]
struct Manifest {
    #[serde(default)]
    channels: BTreeMap<String, Channel>,
}
#[derive(Default, Deserialize)]
struct Channel {
    #[serde(default)]
    versions: BTreeMap<String, BTreeMap<String, WireArtifact>>,
}
#[derive(Deserialize)]
struct WireArtifact {
    name: String,
    url: String,
    #[serde(default)]
    sha256: String,
}

pub fn decode(bytes: &[u8], nightly: bool) -> Result<Vec<Artifact>> {
    let channels = if nightly {
        BTreeMap::from([(
            "nightly".to_owned(),
            serde_json::from_slice::<Channel>(bytes)?,
        )])
    } else {
        serde_json::from_slice::<Manifest>(bytes)?.channels
    };
    let mut out = vec![];
    for (channel, c) in channels {
        if !matches!(channel.as_str(), "lts" | "sts" | "nightly") {
            continue;
        }
        for (version, platforms) in c.versions {
            for (platform, a) in platforms {
                out.push(Artifact {
                    channel: channel.clone(),
                    version: version.clone(),
                    platform,
                    name: a.name,
                    url: a.url,
                    sha256: a.sha256.to_ascii_lowercase(),
                });
            }
        }
    }
    Ok(out)
}
pub fn official_metadata(url: &str) -> Option<bool> {
    let url = url.replace("/refs/heads/master/", "/master/");
    for (base, _) in [
        (
            "https://raw.githubusercontent.com/Zxilly/cangjie-version-manifest/master/",
            (),
        ),
        (MIRROR, ()),
    ] {
        if url == format!("{base}versions.json") {
            return Some(false);
        }
        if url == format!("{base}nightly.json") {
            return Some(true);
        }
    }
    None
}
pub fn local(config: &Config, warnings: &mut Vec<String>) -> Vec<Artifact> {
    let mut out = vec![];
    for (name, nightly) in [("versions.json", false), ("nightly.json", true)] {
        let p = config.home.join("metadata").join(name);
        if p.exists() {
            match util::read_limited(&p, 32 * 1024 * 1024).and_then(|b| decode(&b, nightly)) {
                Ok(v) => out.extend(v),
                Err(e) => warnings.push(format!("{}: {e}", p.display())),
            }
        }
    }
    if let Some(home) = &config.cjv_home {
        match crate::cjv::cache_entries(home) {
            Ok(entries) => {
                for (e, p) in entries {
                    if e.kind != "metadata" {
                        continue;
                    }
                    if let Some(nightly) = official_metadata(&e.url) {
                        match util::read_limited(&p, 32 * 1024 * 1024)
                            .and_then(|b| decode(&b, nightly))
                        {
                            Ok(v) => out.extend(v),
                            Err(e) => warnings.push(format!("cjv metadata: {e}")),
                        }
                    }
                }
            }
            Err(e) => warnings.push(format!("cjv cache: {e}")),
        }
    }
    out
}
pub fn exact(all: &[Artifact], version: &str, platform: &str) -> Result<Option<Artifact>> {
    let mut matching = all
        .iter()
        .filter(|a| a.version == version && a.platform == platform);
    let Some(a) = matching.next() else {
        return Ok(None);
    };
    if matching.any(|b| b.sha256 != a.sha256 || b.channel != a.channel) {
        return Err(Error::new(
            "AmbiguousRelease",
            format!("conflicting official records for {version}/{platform}"),
        ));
    }
    Ok(Some(a.clone()))
}
pub fn resolve(
    config: &Config,
    store: &crate::store::StoreTransaction<'_>,
    version: &str,
    platform: &str,
    offline: bool,
    verbose: bool,
) -> Result<Artifact> {
    let mut warnings = vec![];
    let local = local(config, &mut warnings);
    if verbose {
        for w in warnings {
            crate::diagnostics::emit(format_args!("cjr: {w}"));
        }
    }
    if let Some(a) = exact(&local, version, platform)? {
        a.validate()?;
        return Ok(a);
    }
    if offline {
        return Err(Error::new(
            "OfflineCacheMiss",
            format!("no official metadata for {version}/{platform}"),
        ));
    }
    let client = crate::http::client()?;
    let mut all = vec![];
    for (name, url, nightly) in [
        ("versions.json", STABLE, false),
        ("nightly.json", NIGHTLY, true),
    ] {
        let path = store.root().join("metadata").join(name);
        let mut result = None;
        for endpoint in [url.to_owned(), format!("{MIRROR}{name}")] {
            match fetch_metadata(&client, &endpoint, &path, nightly) {
                Ok(v) => {
                    result = Some(v);
                    break;
                }
                Err(e) => {
                    if verbose {
                        crate::diagnostics::emit(format_args!("cjr: metadata source failed: {e}"));
                    }
                }
            }
        }
        all.extend(result.ok_or_else(|| {
            Error::new(
                "CatalogUnavailable",
                format!("could not fetch official {name}"),
            )
        })?);
        if let Some(a) = exact(&all, version, platform)? {
            a.validate()?;
            return Ok(a);
        }
    }
    Err(Error::new(
        "RuntimeUnavailable",
        format!("official catalog has no {version}/{platform}"),
    ))
}
fn fetch_metadata(
    client: &reqwest::blocking::Client,
    url: &str,
    path: &Path,
    nightly: bool,
) -> Result<Vec<Artifact>> {
    use std::io::{Read, Write};
    let response = client
        .get(url)
        .timeout(Duration::from_secs(45))
        .send()?
        .error_for_status()?;
    let mut bytes = vec![];
    response
        .take(32 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 32 * 1024 * 1024 {
        return Err(Error::new("ResourceLimit", "catalog too large"));
    }
    let all = decode(&bytes, nightly)?;
    std::fs::create_dir_all(util::parent(path)?)?;
    let mut temp = tempfile::NamedTempFile::new_in(util::parent(path)?)?;
    temp.write_all(&bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|e| Error::at(path, e.error))?;
    Ok(all)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manifest_exact() {
        let b=br#"{"channels":{"sts":{"latest":null,"versions":{"1.2.0":{"linux-x64":{"name":"x","url":"https://example.com/x","sha256":"ABC"}}}}}}"#;
        let a = decode(b, false).unwrap();
        assert_eq!(a[0].sha256, "abc");
        assert!(exact(&a, "1.2", "linux-x64").unwrap().is_none());
        assert!(a[0].validate().is_err());
    }
    #[test]
    fn sources() {
        assert_eq!(official_metadata(STABLE), Some(false));
        assert_eq!(
            official_metadata("https://evil.example/versions.json"),
            None
        );
    }
}
