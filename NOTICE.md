# References and third-party materials

The cjv discovery and cache adapters implement the on-disk protocol observed in
[Zxilly/cjv](https://github.com/Zxilly/cjv), including cache record keys, settings,
managed installation receipts and lifecycle locks. cjv is licensed under Apache-2.0.
The adapter is implemented independently in Rust and does not link or execute cjv.

Official distribution metadata comes from
[Zxilly/cangjie-version-manifest](https://github.com/Zxilly/cangjie-version-manifest)
(ISC license). That license does not replace the Cangjie SDK or runtime licenses.
cjr retains the SDK's top-level license/notice files with the extracted payload.

Downloaded SDKs, generated research binaries, build caches and private fixture
stores are excluded from source control. Rust dependency licenses are supplied
by their respective crates and recorded dependency identities in Cargo.lock.
