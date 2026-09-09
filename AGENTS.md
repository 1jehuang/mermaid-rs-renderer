# AGENTS.md

## Cursor Cloud specific instructions

This is **mmdr** — a pure-Rust Mermaid diagram renderer CLI and library. No databases, Docker, or external services needed.

### Build & test

Standard commands per CI (see `.github/workflows/ci.yml`):

- `cargo fmt -- --check` — formatting
- `cargo clippy --all-targets --all-features -- -D warnings` — linting
- `cargo test --all-targets --all-features` — full test suite (unit + CLI + invariant)
- `cargo test --no-default-features --lib` — library-only tests without optional features
- `cargo test --doc --all-features` — doc tests

### Run the CLI

```
cargo run --all-features -- -i input.mmd -o output.svg
```

For PNG: add `-e png`. Stdin: use `-i -`. The `--timing` flag emits per-stage microsecond timings to stderr.

### Gotchas

- Rust edition 2024 requires **Rust 1.85+**. The update script runs `rustup update stable` to ensure this.
- The `clippy.toml` sets `too-many-arguments-threshold = 10` and `type-complexity-threshold = 500`.
- One invariant test (`all_repository_fixtures_satisfy_layout_invariants`) is known to fail on `master` — this is pre-existing and not caused by environment setup.
- The `package.json` Node.js dependencies (`@mermaid-js/mermaid-cli`, `vega-cli`, `vega-lite`) are only for benchmarking scripts in `scripts/`; they are **not** needed for core development.
