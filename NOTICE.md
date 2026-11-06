# References and third-party materials

The cjv discovery and cache adapters implement the on-disk protocol observed in
[Zxilly/cjv](https://github.com/Zxilly/cjv), including cache record keys, settings,
managed installation receipts and lifecycle locks. cjv is licensed under Apache-2.0.
The adapter is implemented independently in Rust and does not link or execute cjv.

Official distribution metadata comes from
[Zxilly/cangjie-version-manifest](https://github.com/Zxilly/cangjie-version-manifest)
(ISC license). That license does not replace the Cangjie SDK or runtime licenses.
cjr retains the SDK's top-level license/notice files with the extracted payload.

Rust dependency licenses are supplied by their respective crates, with dependency
identities recorded in Cargo.lock.

The website uses Patua One, JetBrains Mono and Noto Sans SC through Fontsource,
under the SIL Open Font License 1.1. Website builds include their license files
in the generated `licenses/` directory.
