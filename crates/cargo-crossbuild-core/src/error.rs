use std::fmt::{self, Display, Formatter};
use std::io;
use std::path::PathBuf;
use which;

/// Errors produced by `cargo-crossbuild`.
#[derive(Debug, thiserror::Error)]
pub enum CrossBuildError {
    #[error("invalid target triple `{target}`: {reason}")]
    InvalidTarget { target: String, reason: String },

    #[error("could not find Cargo.toml starting from {}", searched_from.display())]
    ManifestNotFound { searched_from: PathBuf },

    #[error("manifest path must point to Cargo.toml, got {}", path.display())]
    ManifestNotCargoToml { path: PathBuf },

    #[error("unable to execute cargo program `{program}`")]
    CargoUnavailable { program: String },

    #[error("host `{host}` does not support target `{target}` in the current configuration")]
    HostUnsupported { host: String, target: String },

    #[error("provider `{provider}` failed: {reason}")]
    ProviderFailed { provider: String, reason: String },

    #[error("no suitable {provider_type} provider found for target `{target}`")]
    ProviderNotFound { provider_type: String, target: String },

    #[error("build command `{command}` exited with status {exit_code:?}")]
    BuildFailed { command: String, exit_code: Option<i32> },

    #[error("{message}")]
    Configuration { message: String },

    #[error("I/O error at {}: {source}", path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "unknown".to_string()))]
    Io { path: Option<PathBuf>, source: io::Error },

    #[error("required tool not found: {tool}")]
    ToolNotFound { tool: String },

    #[error("sysroot not needed for native build")]
    SysrootNotNeeded,

    #[error("sysroot not found for target `{target}`")]
    SysrootNotFound { target: String },

    #[error("checksum verification failed for {url}: expected {expected}, got {actual}")]
    ChecksumMismatch { url: String, expected: String, actual: String },

    #[error("download failed for {url}: {reason}")]
    DownloadFailed { url: String, reason: String },

    #[error("lockfile corrupted: {reason}")]
    LockfileCorrupted { reason: String },

    #[error("cache operation failed: {reason}")]
    CacheError { reason: String },

    #[error("which error: {source}")]
    WhichError { source: which::Error },
}

impl CrossBuildError {
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }
}

impl From<io::Error> for CrossBuildError {
    fn from(source: io::Error) -> Self {
        CrossBuildError::Io { path: None, source }
    }
}

impl From<which::Error> for CrossBuildError {
    fn from(source: which::Error) -> Self {
        CrossBuildError::WhichError { source }
    }
}

impl From<crate::model::HostDetectError> for CrossBuildError {
    fn from(err: crate::model::HostDetectError) -> Self {
        match err {
            crate::model::HostDetectError::ParseError(e) => CrossBuildError::configuration(e),
            crate::model::HostDetectError::RustcNotFound => CrossBuildError::ToolNotFound {
                tool: "rustc".to_string(),
            },
            crate::model::HostDetectError::CargoNotFound => CrossBuildError::ToolNotFound {
                tool: "cargo".to_string(),
            },
        }
    }
}