//! Core planning and execution engine for `cargo-crossbuild`.

pub mod cache;
pub mod cargo_config;
pub mod config;
pub mod diagnostics;
pub mod downloader;
pub mod error;
pub mod model;
pub mod planner;
pub mod platform;
pub mod provider;
pub mod registry;

pub use cache::{CacheManager, CachePolicy};
pub use cargo_config::{create_cross_config, merge_cargo_configs, CargoConfigGenerator};
pub use config::CrossBuildConfig;
pub use diagnostics::{Diagnostic, DiagnosticSink, Severity, StderrDiagnosticSink};
pub use downloader::{ChecksumAlgorithm, DownloadRequest, DownloadResult, Downloader};
pub use error::CrossBuildError;
pub use model::{
    Abi, Architecture, BuildPlan, BuildRequest, CommandLine, Endianness, ExecutionMode,
    ExecutionReport, HostDetectError, HostInfo, LinkerHint, OperatingSystem, PlanStep, Profile,
    ProviderAction, RunReport, StandardValidations, SysrootHint, TargetFamily, TargetInfo,
    TargetParseError, TargetSupport, TargetTriple, ToolchainHint, ValidationPlan, Vendor,
    WrapperPlan,
};
pub use planner::Planner;
pub use platform::{
    assess_target, detect_host, rustup_target_available, CapabilityMatrix, KnownTargets,
};
pub use provider::{
    BuiltinToolchainProvider, LinkerFlavor, LinkerProvider, LinkerResolution, NoSysrootProvider,
    RustupSysrootProvider, RustupToolchainProvider, SysrootProvider, SysrootResolution,
    ToolchainProvider, ToolchainResolution, ZigSysrootProvider, ZigToolchainProvider,
};
