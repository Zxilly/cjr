//! Shared HTTP transport configuration; download policy belongs to provision.
use crate::Result;
use std::time::Duration;

pub(crate) fn client() -> Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder()
        .retry(reqwest::retry::never())
        .user_agent(concat!("cjr/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(1800))
        .https_only(true)
        .build()?)
}
