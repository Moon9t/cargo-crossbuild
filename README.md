# cargo-crossbuild

`cargo-crossbuild` is a production-grade Rust cross-compilation platform for building crates across Windows, Linux, and macOS without Docker or virtual machines. It automatically discovers installed toolchains, linkers, and sysroots to produce correct cross-compilation configurations.

## Features

- **Multi-platform targets** — Build for any Rust target triple from any host
- **Automatic provider resolution** — Discovers rustup, Zig, MSVC, GCC, Clang, LLD, and Mold toolchains
- **Native MSVC detection** — Finds Visual Studio's `link.exe` on Windows automatically
- **Zig cross-compilation** — Uses Zig's C frontend as a drop-in cross-compiler
- **Cargo config generation** — Writes `.cargo/config.toml` with correct linker and target settings
- **Dry-run planning** — Preview the build plan without executing it
- **Cache management** — LRU eviction, expiry, and compression for downloaded toolchains and sysroots
- **Lockfile support** — Reproducible builds with configuration hashing and verification
- **Provider registry** — Pluggable provider architecture for custom toolchain/sysroot/linker backends

## Architecture

The workspace is organized into 24 crates with a narrow CLI frontend and a shared core library:

| Crate | Purpose |
|-------|---------|
| `crossbuild-core` | Domain models, traits, platform detection, planner, provider interfaces |
| `crossbuild-engine` | Top-level orchestration: plan + execute pipeline |
| `crossbuild-cli` | Binary frontend with `build`, `doctor`, `clean`, `list-targets` commands |
| `crossbuild-planner` | Resolves build requests into executable plans using provider registry |
| `crossbuild-registry` | Provider registry with default toolchain/sysroot/linker providers |
| `crossbuild-provider-*` | Concrete provider implementations (zig, gcc, clang, msvc, sysroot) |
| `crossbuild-resolver` | DAG-based task scheduler with topological sort and cycle detection |
| `crossbuild-environment` | Environment setup and cargo config generation from build plans |
| `crossbuild-runner` | Build plan executor with output capture and diagnostics |
| `crossbuild-wrappers` | Cross-compilation wrapper script generation (CC, CXX, AR, LD, etc.) |
| `crossbuild-cache` | LRU cache manager with metadata persistence and expiry |
| `crossbuild-downloader` | Secure downloader with checksum verification |
| `crossbuild-installer` | Package manager integration for cross-compilation dependencies |
| `crossbuild-lockfile` | Lockfile creation, verification, and configuration hashing |
| `crossbuild-telemetry` | Event collection with RAII timers and tracing integration |
| `crossbuild-diagnostics` | Diagnostic sink interface with structured diagnostics |
| `crossbuild-sdk` | Package/metadata management for SDK-style installations |
| `crossbuild-toolchain` | Toolchain configuration and resolution helpers |

## Usage

```bash
# List available subcommands
cargo crossbuild --help

# Build for a specific target
cargo crossbuild build --target aarch64-unknown-linux-gnu

# Dry-run to preview the build plan without executing
cargo crossbuild build --dry-run --target x86_64-pc-windows-msvc

# Run system diagnostics
cargo crossbuild doctor

# List known target triples
cargo crossbuild list-targets

# Clean build artifacts and cache
cargo crossbuild clean
```

### Build Options

```
Usage: cargo-crossbuild build [OPTIONS] --target <TARGET> [CARGO_ARGS]...

Arguments:
  [CARGO_ARGS]...  Extra arguments passed through to cargo

Options:
  -t, --target <TARGET>          Target triple (e.g. x86_64-unknown-linux-gnu)
  -m, --manifest-path <PATH>     Path to Cargo.toml [default: ./Cargo.toml]
      --dry-run                  Plan the build without executing it
      --release                  Build in release mode
      --features <FEATURES>      Comma-separated list of features
      --no-default-features      Do not include default features
      --workspace                Build the entire workspace
  -v, --verbose                  Verbose output
      --profile <PROFILE>        Build profile name
      --exclude <PACKAGE>...     Packages to exclude (for workspace builds)
```

## How It Works

1. **Request** — CLI parses the target triple and build options into a `BuildRequest`
2. **Discovery** — Platform detection identifies the host triple, architecture, and OS
3. **Provider Resolution** — The provider registry iterates through toolchain, sysroot, and linker providers by priority, selecting the first that `can_provide()` for the target
4. **Planning** — The planner constructs a `BuildPlan` with command line, cargo config, environment variables, and execution steps
5. **Execution** — The runner executes cargo with the configured environment and linker, capturing output and reporting diagnostics

## Project Status

Production-ready architecture with a working build pipeline. All 24 crates compile and pass tests.
