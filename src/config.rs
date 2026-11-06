use crate::{Error, Result};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub home: PathBuf,
    pub cjv_home: Option<PathBuf>,
    pub warnings: Vec<String>,
}
pub fn user_home() -> Result<PathBuf> {
    let key = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(key)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| Error::new("InvalidConfig", format!("{key} is not set")))
}
impl Config {
    pub fn discover() -> Result<Self> {
        let home = match std::env::var_os("CJR_HOME")
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
        {
            Some(p) => p,
            None => user_home()?.join(".cjr"),
        };
        let home = std::path::absolute(home)?;
        if home.parent().is_none() {
            return Err(Error::new(
                "InvalidConfig",
                "CJR_HOME must not be a filesystem root",
            ));
        }
        let mut warnings = vec![];
        let cjv_home = match crate::cjv::discover_home() {
            Ok(p) => Some(p),
            Err(e) => {
                warnings.push(format!("cjv unavailable: {e}"));
                None
            }
        };
        let real_home = home.canonicalize().unwrap_or(home.clone());
        if cjv_home.as_ref().is_some_and(|p| {
            let p = p.canonicalize().unwrap_or(p.clone());
            p == real_home || real_home.starts_with(&p) || p.starts_with(&real_home)
        }) {
            return Err(Error::new(
                "InvalidConfig",
                "CJR_HOME and CJV_HOME must be separate, non-nested directories",
            ));
        }
        Ok(Self {
            home,
            cjv_home,
            warnings,
        })
    }
}
