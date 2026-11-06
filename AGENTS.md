# Project instructions

## Engineering

- Prefer standard-library facilities and maintained dependencies for generic infrastructure. Do not handwrite retry/backoff engines, directory walkers, signal dispatch frameworks, or HTTP test servers when an existing dependency provides the required behavior.
- Keep Cangjie metadata, official asset selection, cjv compatibility, loader checks, and cache lifecycle policy in separate modules. Do not put platform FFI in business modules.
- Keep necessary unsafe inside the private `platform` module, expose safe interfaces, and document every unsafe block's preconditions. Preserve the crate's safety and panic lints.
- Cache mutations and execution lease registration must hold `StoreTransaction`. Never mutate or clean cjv-owned files. Never consume an external SDK directory during publication.
- Use typed states for lifecycle decisions. Preserve child exit status even if best-effort cleanup fails. When descendant liveness is uncertain, retain the runtime lease.
- Keep the CLI limited to default execution, `inspect`, `list`, and `cache clean`. Official releases only; no SDK import command or ABI-version guessing.
- Run formatting, strict Clippy, and relevant tests. Changes to supervision need the Unix PTY regression; changes to loading/provisioning need real official SDK acceptance when fixtures are available. Report unavailable platform validation accurately.
- Research fixtures are not production implementations. Do not replace regression assertions with demonstration output or bypass validation to make a fixture pass.

## Git Commit Style

- Before every `git commit`, inspect the repository's recent commit messages
  (for example, `git log --oneline -20`) and match the dominant existing style.
- Prefer the repository's local convention over a generic default. If multiple
  styles are present, use the most recent consistent style or ask before
  committing when the intended style is unclear.
