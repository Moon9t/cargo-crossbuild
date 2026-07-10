# ADR 0001: Architecture of cargo-crossbuild

## Status
Accepted

## Context
cargo-crossbuild is a production-grade Rust cross-compilation platform that enables `cargo crossbuild --target <target>` with a consistent experience across Windows, Linux, and macOS without Docker, VMs, or manual configuration. It must support Rust crates from simple libraries to complex compiler toolchains with native dependencies.

## Decision
We adopt a modular, provider-based architecture with clear separation of concerns:

### Core Crates
1. **cargo-crossbuild-core** - Core engine, planning, execution, providers, models
2. **cargo-crossbuild-cli** - CLI entry point, argument parsing, user interaction

### Core Modules (in cargo-crossbuild-core)
- **model** - Core domain types (BuildRequest, BuildPlan, TargetTriple, HostInfo, etc.)
- **platform** - Host detection, target triple parsing/validation, platform capability matrix
- **config** - Configuration loading from environment, Cargo.toml, config files
- **planner** - BuildPlanner resolves requests into executable BuildPlans
- **engine** - CrossBuildEngine orchestrates planning and execution
- **runner** - Executes build plans via cargo
- **provider** - Provider traits (ToolchainProvider, SysrootProvider, LinkerProvider)
- **registry** - ProviderRegistry for provider discovery and resolution
- **cache** - CacheManager for downloads, sysroots, build artifacts
- **downloader** - Secure downloader with checksum verification
- **installer** - Toolchain/sysroot installer
- **linker** - LinkerResolver for target-appropriate linker detection
- **cargo_config** - CargoConfigGenerator for .cargo/config.toml generation
- **lockfile** - LockfileManager for reproducible builds
- **diagnostics** - Structured diagnostics with DiagnosticSink
- **error** - Error types with rich context
- **testing** - Testing framework for cross-compilation validation

### Provider Architecture
Providers are the extension points for toolchain/sysroot/linker resolution:
- **ToolchainProvider** - Provides compiler toolchain (rustc, cargo, linker) for target
  - BuiltinProvider: Uses host rustup toolchains
  - ZigProvider: Uses zig cc as universal linker/compiler
  - CrossProvider: Uses cross Docker images (optional, for compatibility)
- **SysrootProvider** - Provides target sysroot (libc, libstd, crt objects)
  - BuiltinSysrootProvider: Uses rustup target stdlib
  - ZigSysrootProvider: Uses zig's built-in sysroots
  - CustomSysrootProvider: User-provided sysroot paths
- **LinkerProvider** - Provides target-appropriate linker
  - BuiltinLinkerProvider: Platform default linkers
  - LldProvider: Uses lld (cross-platform)
  - MoldProvider: Uses mold (fast linker)
  - ZigLinkerProvider: Uses zig cc as linker

### Execution Pipeline
```
CLI Request
    → BuildRequest (validated)
    → Planner.plan() → BuildPlan
        → HostTargetDetector.detect() → HostInfo + TargetInfo
        → ProviderRegistry.resolve() → ProviderActions[]
        → CargoConfigGenerator.generate() → .cargo/config.toml
        → LinkerResolver.resolve() → LinkerConfig
        → CacheManager.prepare() → Cached artifacts ready
    → Engine.execute() → ExecutionReport
        → Runner.run() → cargo build with generated config/env
        → DiagnosticSink emits structured diagnostics
```

### Cross-Platform Strategy
- No Docker/VM requirement
- Host-native toolchains via rustup (when host==target)
- Zig cc as universal C compiler/linker for cross-targets
- rustup target stdlib for pure-Rust targets
- User-provided sysroots for complex native deps
- Cargo config generation for seamless cargo integration

### Cache Strategy
- `target/crossbuild-cache/` per workspace
- Keyed by: workspace root + target triple + config hash
- Stores: downloaded toolchains, sysroots, build artifacts
- Content-addressed with SHA256 verification

### Lockfile Strategy
- `crossbuild.lock` per workspace
- Records: target triple, toolchain versions, sysroot hashes, config hash
- Enables reproducible builds in CI

### Configuration Precedence
1. CLI flags (highest)
2. Environment variables (CROSSBUILD_*)
3. Workspace `.cargo-crossbuild.toml`
4. User `~/.config/cargo-crossbuild/config.toml`
5. Built-in defaults (lowest)

## Consequences
### Positive
- Clear separation of concerns
- Extensible provider architecture
- No Docker/VM dependency
- Cross-platform by design
- Reproducible builds via lockfile
- Testable modules with clear boundaries

### Negative
- More upfront architecture complexity
- Provider implementation effort
- Cache invalidation complexity

### Risks
- Zig availability on all platforms
- Sysroot completeness for complex targets
- Linker compatibility edge cases

## Implementation Order
1. Target triple parsing/validation (model/platform)
2. Host/target detection and capability matrix (platform)
3. Provider traits and registry (provider/registry)
4. Builtin providers (toolchain, sysroot, linker)
5. Zig provider (toolchain, sysroot, linker)
6. Cache manager and downloader (cache/downloader)
7. Linker resolver (linker)
8. Cargo config generator (cargo_config)
9. Lockfile manager (lockfile)
10. Planner and engine integration (planner/engine)
11. CLI implementation (CLI crate)
12. Testing framework and integration tests
13. Documentation and ADRs