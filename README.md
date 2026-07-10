# cargo-crossbuild

`cargo-crossbuild` is a Rust cross-compilation platform for building crates across Windows, Linux, and macOS without Docker or virtual machines.

The project is organized as a Rust workspace with a narrow CLI crate and a core library that owns planning, validation, provider selection, diagnostics, and execution.

The primary architecture decision is documented in [docs/adr-0001-workspace-and-engine.md](docs/adr-0001-workspace-and-engine.md).

## Current Scope

The initial implementation focuses on a production-grade architecture and a working build pipeline:

- manifest discovery,
- host and target validation,
- plan construction,
- cargo invocation,
- deterministic target directory selection,
- and structured diagnostics.

The architecture is intentionally open for provider expansion, native dependency handling, cache policy, installer support, and release workflows.

## Usage

```bash
cargo run -p cargo-crossbuild-cli -- --target x86_64-unknown-linux-gnu
```

Dry-run mode prints the resolved plan without executing cargo:

```bash
cargo run -p cargo-crossbuild-cli -- --target x86_64-unknown-linux-gnu --dry-run
```
