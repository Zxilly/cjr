#![deny(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]

pub mod app;
pub mod catalog;
pub mod cjv;
pub mod cli;
pub mod config;
pub mod error;
pub mod inspect;
pub mod loader;
pub mod lock;
pub mod model;
pub mod process;
pub mod provision;
pub mod store;
pub mod util;

pub use error::{Error, Result};
// All unavoidable OS FFI is audited behind safe interfaces in one private module.
mod diagnostics;
#[allow(unsafe_code)]
mod platform;

mod http;
