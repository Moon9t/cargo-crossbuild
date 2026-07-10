# ADR 0001: Workspace and Engine Split

## Status

Accepted.

## Context

`cargo-crossbuild` is intended to become a long-lived cross-compilation platform rather than a one-off wrapper around `cargo build`. The implementation must support platform growth, provider expansion, diagnostics, and native dependency handling without collapsing into a single procedural binary.

The CLI must remain small and stable. The build semantics must remain testable and reusable. Future work will need to add platform providers, registry policies, cache coordination, and release workflows without changing the external command shape.

## Decision

The project is split into a workspace with two crates:

- `cargo-crossbuild-cli` owns command-line parsing, user interaction, and process exit behavior.
- `cargo-crossbuild-core` owns request validation, host and target modeling, provider resolution, planning, diagnostics, and execution.

The core library is modeled around explicit data types rather than hidden global state. The provider registry is extensible so target-specific behavior can be introduced without rewriting the planner or the CLI.

The initial implementation uses only the standard library. External dependencies will be introduced only when they provide a clear engineering advantage that justifies the maintenance and supply-chain cost.

## Consequences

- The CLI remains small, reviewable, and replaceable.
- The planning engine can be unit-tested and integration-tested independently.
- Provider behavior can be expanded without changing the command contract.
- Cross-platform support can evolve through explicit architecture rather than ad hoc branching.
- The initial implementation is conservative: it favors explicit build planning and deterministic configuration over broad automation.

## Notes

The initial build path resolves a manifest, validates the target triple, computes a target directory, applies provider contributions, and invokes Cargo. This is the minimum viable architecture for a production-grade cross-compilation platform. Future ADRs should capture provider-specific linker behavior, registry selection, cache invalidation, downloader policy, and installer/release workflows.
