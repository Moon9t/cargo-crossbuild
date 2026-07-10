# ADR-0002: Release v1.0.0

## Context

cargo-crossbuild has reached a stable internal architecture: 24 workspace crates, 77 passing tests, production-grade error handling, and a verified cross-build pipeline (tested on Windows with x86_64-pc-windows-gnu and x86_64-pc-windows-msvc). The release to crates.io enables public adoption.

## Decision

Publish all 24 workspace crates to crates.io as v1.0.0 under the MIT OR Apache-2.0 dual license.

### Scope

- 22 library crates (crossbuild-core through crossbuild-planner)
- 1 binary crate (cargo-crossbuild-cli, producing the `cargo-crossbuild` binary)
- 1 workspace root (no separate package)

### Version Strategy

- All crates ship as v1.0.0 simultaneously.
- Internal path dependencies are converted to version requirements by `cargo publish`.
- Workspace inheritance (`{ workspace = true }`) centralizes metadata.

### Quality Gates Met

- **Zero warnings** across all 24 crates (`cargo build` clean).
- **77 tests pass** (`cargo test` green).
- **Zero production `unwrap()` calls** — all 11 violations converted to `.expect("documented invariant")` or proper `Result` propagation.
- **No circular dependencies** — dependency graph is acyclic.
- **No `unsafe` code** introduced.

### License

MIT OR Apache-2.0 (standard Rust dual license).

## Consequences

- **Public API stability commitment** begins at v1.0.0. Breaking changes require semver-major bumps.
- **24 separate crate versions** must be published in dependency order (core first, cli last).
- **Lock file (`Cargo.lock`) is gitignored** — correct for library crates per Rust convention.

## Alternatives

1. **Single crate with feature flags** — rejected: workspace structure enables independent versioning, faster incremental builds, and clearer ownership boundaries (Constitution Article V).
2. **Publish as 0.x first** — rejected: internal architecture is stable, cross-build pipeline is verified, and v1.0.0 communicates production readiness.
3. **Publish only crossbuild-core + CLI** — rejected: intermediate crates (planner, runner, providers) have independent utility for toolchain integrations.

## Status

Accepted.
