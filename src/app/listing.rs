use crate::{
    Result, catalog,
    config::Config,
    model::{Artifact, LocalEntry, LocalReport, LocalSource},
    store::Store,
    util,
};
use std::collections::BTreeMap;
pub fn local_report(config: &Config) -> Result<LocalReport> {
    let mut warnings = config.warnings.clone();
    let store = Store::new(config);
    let receipts = store.receipts(&mut warnings);
    let mut artifacts = catalog::local(config, &mut warnings);
    artifacts.extend(receipts.iter().map(|(_, r)| r.artifact.clone()));
    let mut groups: BTreeMap<(String, String, String), LocalEntry> = BTreeMap::new();
    fn add(
        groups: &mut BTreeMap<(String, String, String), LocalEntry>,
        a: &Artifact,
        s: LocalSource,
    ) {
        let key = (a.version.clone(), a.platform.clone(), a.sha256.clone());
        let e = groups.entry(key).or_insert_with(|| LocalEntry {
            version: a.version.clone(),
            platform: a.platform.clone(),
            sha256: a.sha256.clone(),
            sources: vec![],
        });
        if !e
            .sources
            .iter()
            .any(|old| old.path == s.path && old.kind == s.kind)
        {
            e.sources.push(s);
        }
    }
    for (p, r) in receipts {
        let size = r
            .files
            .iter()
            .fold(0u64, |sum, f| sum.saturating_add(f.size));
        add(
            &mut groups,
            &r.artifact,
            LocalSource {
                kind: "cjr-runtime".into(),
                path: p,
                status: format!("receipt: {}; integrity checked on execution", r.provenance),
                bytes: Some(size),
                usage: Some(store.usage(&r.bundle_id).to_string()),
            },
        );
    }
    for a in &artifacts {
        let p = config.home.join("downloads").join(&a.sha256);
        if util::is_digest(&a.sha256) && p.is_file() {
            add(
                &mut groups,
                a,
                LocalSource {
                    kind: "cjr-archive".into(),
                    bytes: std::fs::metadata(&p).ok().map(|m| m.len()),
                    path: p,
                    status: "indexed; checksum checked on execution".into(),
                    usage: None,
                },
            );
        }
    }
    if let Some(home) = &config.cjv_home {
        match crate::cjv::cache_entries(home) {
            Ok(entries) => {
                for (e, p) in entries {
                    if e.kind != "artifact" {
                        continue;
                    }
                    let matches = artifacts
                        .iter()
                        .filter(|a| util::is_digest(&e.sha256) && a.sha256 == e.sha256)
                        .collect::<Vec<_>>();
                    if matches.is_empty() {
                        let a = Artifact {
                            channel: "unknown".into(),
                            version: "unknown".into(),
                            platform: "unknown".into(),
                            sha256: e.sha256,
                            name: String::new(),
                            url: e.url,
                        };
                        add(
                            &mut groups,
                            &a,
                            LocalSource {
                                kind: "cjv-archive (cjv owned)".into(),
                                path: p,
                                status: "official identity unconfirmed".into(),
                                bytes: Some(e.size),
                                usage: None,
                            },
                        );
                    } else {
                        for a in matches {
                            add(
                                &mut groups,
                                a,
                                LocalSource {
                                    kind: "cjv-archive (cjv owned)".into(),
                                    path: p.clone(),
                                    status: "indexed; checksum checked on execution".into(),
                                    bytes: Some(e.size),
                                    usage: None,
                                },
                            );
                        }
                    }
                }
            }
            Err(e) => warnings.push(e.to_string()),
        }
        for a in &artifacts {
            let p = home.join("downloads").join(&a.sha256);
            if util::is_digest(&a.sha256) && p.is_file() {
                add(
                    &mut groups,
                    a,
                    LocalSource {
                        kind: "cjv-download (cjv owned)".into(),
                        bytes: std::fs::metadata(&p).ok().map(|m| m.len()),
                        path: p,
                        status: "indexed; checksum checked on execution".into(),
                        usage: None,
                    },
                );
            }
        }
        match crate::cjv::installations(home) {
            Ok(entries) => {
                for (i, p) in entries {
                    let matches = artifacts
                        .iter()
                        .filter(|a| i.matches(a))
                        .collect::<Vec<_>>();
                    if matches.is_empty() {
                        let a = Artifact {
                            channel: "unknown".into(),
                            version: i.release,
                            platform: i.tuple,
                            sha256: i.sha256,
                            name: String::new(),
                            url: String::new(),
                        };
                        add(
                            &mut groups,
                            &a,
                            LocalSource {
                                kind: "cjv-installation (cjv owned)".into(),
                                path: p,
                                status: "official identity unconfirmed".into(),
                                bytes: None,
                                usage: None,
                            },
                        );
                    } else {
                        for a in matches {
                            add(
                                &mut groups,
                                a,
                                LocalSource {
                                    kind: "cjv-installation (cjv owned)".into(),
                                    path: p.clone(),
                                    status: "managed; snapshot validation required".into(),
                                    bytes: None,
                                    usage: None,
                                },
                            );
                        }
                    }
                }
            }
            Err(e) => warnings.push(e.to_string()),
        }
    }
    Ok(LocalReport {
        schema_version: 1,
        entries: groups.into_values().collect(),
        warnings,
    })
}
