//! Bounded whole-object downloads. Only transient transport/server failures retry.
use crate::{Error, Result, model::Artifact, util};
use backon::{BlockingRetryable, ExponentialBuilder};
use reqwest::blocking::Client;
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    time::Duration,
};

const MAX_ARCHIVE_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_RETRIES: usize = 2;

struct Failure {
    error: Error,
    retryable: bool,
}
impl From<Error> for Failure {
    fn from(error: Error) -> Self {
        Self {
            error,
            retryable: false,
        }
    }
}
impl From<std::io::Error> for Failure {
    fn from(error: std::io::Error) -> Self {
        Error::from(error).into()
    }
}
fn transport(error: reqwest::Error) -> Failure {
    let retryable = error.is_timeout() || error.is_connect() || error.is_body();
    Failure {
        error: error.into(),
        retryable,
    }
}

pub(super) fn download(a: &Artifact, dest: &Path) -> Result<()> {
    download_with(&crate::http::client()?, a, dest, Duration::from_secs(1))
}
fn download_with(client: &Client, a: &Artifact, dest: &Path, backoff: Duration) -> Result<()> {
    (|| attempt_download(client, a, dest))
        .retry(
            ExponentialBuilder::default()
                .with_min_delay(backoff)
                .with_max_times(MAX_RETRIES)
                .with_jitter(),
        )
        .when(|failure| failure.retryable)
        .call()
        .map_err(|failure| failure.error)
}

fn attempt_download(
    client: &Client,
    a: &Artifact,
    dest: &Path,
) -> std::result::Result<(), Failure> {
    let mut response = client.get(&a.url).send().map_err(transport)?;
    let status = response.status();
    if status != reqwest::StatusCode::OK {
        return Err(Failure {
            error: Error::new(
                "DownloadFailed",
                format!("expected HTTP 200, received {status}"),
            ),
            retryable: status.is_server_error() || matches!(status.as_u16(), 408 | 429),
        });
    }
    if response
        .content_length()
        .is_some_and(|n| n > MAX_ARCHIVE_BYTES)
    {
        return Err(Error::new("ResourceLimit", "SDK archive exceeds 4 GiB").into());
    }
    let mut file = fs::File::create(dest)?;
    let mut total = 0u64;
    let mut buffer = [0u8; 128 * 1024];
    loop {
        let n = response.read(&mut buffer).map_err(|error| Failure {
            error: error.into(),
            retryable: true,
        })?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > MAX_ARCHIVE_BYTES {
            return Err(Error::new("ResourceLimit", "SDK archive exceeds 4 GiB").into());
        }
        // Disk failures are permanent for this invocation and must not redownload gigabytes.
        file.write_all(&buffer[..n])?;
    }
    file.sync_all()?;
    if util::hash_file(dest)? != a.sha256 {
        return Err(Error::new(
            "ChecksumMismatch",
            "download does not match official SHA-256",
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn artifact(url: String) -> Artifact {
        Artifact {
            channel: "lts".into(),
            version: "1.0.5".into(),
            platform: "linux-x64".into(),
            name: "sdk.tar.gz".into(),
            url,
            sha256: util::digest(b"sdk"),
        }
    }
    fn client() -> Client {
        Client::builder()
            .no_proxy()
            .retry(reqwest::retry::never())
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap()
    }
    #[test]
    fn retries_server_error_and_replaces_partial_file() {
        let mut server = mockito::Server::new();
        let unavailable = server
            .mock("GET", "/sdk")
            .with_status(503)
            .expect(1)
            .create();
        let success = server
            .mock("GET", "/sdk")
            .with_status(200)
            .with_body("sdk")
            .expect(1)
            .create();
        let t = tempfile::tempdir().unwrap();
        let dest = t.path().join("download");
        fs::write(&dest, b"old partial download longer than SDK").unwrap();
        download_with(
            &client(),
            &artifact(format!("{}/sdk", server.url())),
            &dest,
            Duration::ZERO,
        )
        .unwrap();
        unavailable.assert();
        success.assert();
        assert_eq!(fs::read(dest).unwrap(), b"sdk");
    }
    #[test]
    fn transient_failures_stop_after_three_total_requests() {
        let mut server = mockito::Server::new();
        let unavailable = server
            .mock("GET", "/sdk")
            .with_status(503)
            .expect(3)
            .create();
        let t = tempfile::tempdir().unwrap();
        assert!(
            download_with(
                &client(),
                &artifact(format!("{}/sdk", server.url())),
                &t.path().join("download"),
                Duration::ZERO
            )
            .is_err()
        );
        unavailable.assert();
    }
    #[test]
    fn interrupted_body_is_downloaded_again_from_the_start() {
        let mut server = mockito::Server::new();
        let interrupted = server
            .mock("GET", "/sdk")
            .with_chunked_body(|writer| {
                writer.write_all(b"partial archive")?;
                Err(std::io::ErrorKind::ConnectionReset.into())
            })
            .expect(1)
            .create();
        let complete = server
            .mock("GET", "/sdk")
            .with_body("sdk")
            .expect(1)
            .create();
        let t = tempfile::tempdir().unwrap();
        let dest = t.path().join("download");
        download_with(
            &client(),
            &artifact(format!("{}/sdk", server.url())),
            &dest,
            Duration::ZERO,
        )
        .unwrap();
        interrupted.assert();
        complete.assert();
        assert_eq!(fs::read(dest).unwrap(), b"sdk");
    }
    #[test]
    fn permanent_http_and_integrity_errors_are_not_retried() {
        for (status, code) in [(404, "DownloadFailed"), (200, "ChecksumMismatch")] {
            let mut server = mockito::Server::new();
            let mock = server
                .mock("GET", "/sdk")
                .with_status(status)
                .with_body("bad")
                .expect(1)
                .create();
            let t = tempfile::tempdir().unwrap();
            let error = download_with(
                &client(),
                &artifact(format!("{}/sdk", server.url())),
                &t.path().join("download"),
                Duration::ZERO,
            )
            .unwrap_err();
            mock.assert();
            assert!(matches!(error, Error::Diagnostic { code: c, .. } if c == code));
        }
    }
}
