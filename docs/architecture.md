# cargo-crossbuild Architecture

## Purpose

The system is split into a CLI entrypoint and a core planning engine. The CLI owns user interaction and argument parsing. The core library owns all build semantics.

## Boundary Model

- `cargo-crossbuild-cli` parses input and forwards a `BuildRequest`.
- `cargo-crossbuild-core` validates the request, chooses providers, assembles a `BuildPlan`, and optionally executes it.

## Core Responsibilities

- manifest discovery,
- host and target modeling,
- provider resolution,
- command construction,
- diagnostics,
- and execution.

## Extension Points

The provider registry is intentionally open-ended. Future work can add platform providers, native dependency resolvers, cache-aware preflight steps, installers, and release orchestration without disturbing the CLI contract.
