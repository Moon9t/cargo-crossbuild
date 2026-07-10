//! Core planning and execution engine for `cargo-crossbuild`.

pub mod cache;
pub mod cargo_config;
pub mod config;
pub mod diagnostics;
pub mod downloader;
pub mod engine;
pub mod error;
pub mod installer;
pub mod linker;
pub mod lockfile;
pub mod model;
pub mod package_manager;
pub mod planner;
pub mod platform;
pub mod provider;
pub mod registry;
pub mod release;
pub mod runner;
pub mod sysroot;
pub mod testing;
pub mod toolchain;
pub mod wrappers;

pub use cache::{CacheManager, CachePolicy};
pub use cargo_config::{create_cross_config, merge_cargo_configs, CargoConfigGenerator};
pub use config::CrossBuildConfig;
pub use diagnostics::{Diagnostic, DiagnosticSink, Severity, StderrDiagnosticSink};
pub use downloader::{ChecksumAlgorithm, DownloadRequest, DownloadResult, Downloader};
pub use engine::CrossBuildEngine;
pub use model::ExecutionReport;
pub use error::CrossBuildError;
pub use linker::LinkerConfig;
pub use lockfile::{compute_config_hash, compute_manifest_hash, Lockfile, LockfileManager};
pub use model::{
    Architecture, Abi, BuildPlan, BuildRequest, CommandLine, Endianness, ExecutionMode,
    HostInfo, LinkerFlavor, LinkerHint, OperatingSystem, PlanStep, Profile, ProviderAction,
    SysrootHint, TargetFamily, TargetInfo, TargetParseError, TargetSupport, TargetTriple,
    ToolchainConfig, ToolchainHint, Vendor, HostDetectError, ValidationPlan, WrapperPlan,
};
pub use package_manager::PackageManagerPlan;
pub use planner::Planner;
pub use platform::{assess_target, detect_host, CapabilityMatrix, KnownTargets, rustup_target_available};
pub use provider::{
    LinkerProvider, LinkerResolution, ProviderRegistry, SysrootProvider,
    SysrootResolution, ToolchainProvider, ToolchainResolution, find_rustup_toolchain,
};
pub use registry::ProviderRegistry as Registry;
pub use release::ReleasePlan;
pub use sysroot::SysrootConfig;
pub use testing::ValidationPlan as TestValidationPlan;
pub use installer::InstallPlan;