//! Installation planning and execution for cross-compilation.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use crossbuild_core::{config::CrossBuildConfig, model::TargetTriple};

/// Describes a planned installation destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallPlan {
    pub destination: PathBuf,
}

impl InstallPlan {
    pub fn new(destination: impl Into<PathBuf>) -> Self {
        Self {
            destination: destination.into(),
        }
    }

    pub fn resolved_destination(&self, workspace_root: &Path) -> PathBuf {
        if self.destination.is_absolute() {
            self.destination.clone()
        } else {
            workspace_root.join(&self.destination)
        }
    }
}

/// Package manager for cross-compilation dependencies.
pub struct PackageManager {
    #[allow(dead_code)]
    config: CrossBuildConfig,
}

impl PackageManager {
    pub fn new(config: CrossBuildConfig) -> Self {
        Self { config }
    }

    /// Adds a target via rustup if it is a rustup-supported target.
    pub fn install_target(&self, target: &TargetTriple) -> Result<()> {
        let available = self.available_targets()?;
        if !available.iter().any(|t| t == &target.triple) {
            anyhow::bail!("target {} is not a rustup-supported target", target.triple);
        }
        self.rustup_target_add(&target.triple)
    }

    /// Installs system packages for cross-compilation.
    /// Detects the host OS and runs the appropriate package manager.
    pub fn install_system_packages(
        &self,
        target: &TargetTriple,
        packages: &[String],
    ) -> Result<()> {
        let os = std::env::consts::OS;
        match os {
            "linux" => {
                let managers: &[(&str, &[&str])] = &[
                    ("apt-get", &["install", "-y"]),
                    ("dnf", &["install", "-y"]),
                    ("pacman", &["-S", "--noconfirm"]),
                ];
                for (cmd, args) in managers {
                    if Command::new("which").arg(cmd).output().is_ok()
                        && Command::new("which")
                            .arg(cmd)
                            .output()
                            .is_ok_and(|o| o.status.success())
                    {
                        Command::new(cmd)
                            .args(*args)
                            .args(packages)
                            .status()
                            .context(format!("failed to run {} install", cmd))?;
                        return Ok(());
                    }
                }
                anyhow::bail!("no supported package manager found (tried apt-get, dnf, pacman)");
            }
            "macos" => {
                Command::new("brew")
                    .arg("install")
                    .args(packages)
                    .status()
                    .context("failed to run brew install")?;
                Ok(())
            }
            "windows" => {
                println!(
                    "Would install packages for {} on Windows: {:?}",
                    target.triple, packages
                );
                Ok(())
            }
            _ => anyhow::bail!("unsupported host OS: {}", os),
        }
    }

    /// Runs `rustup target add <target>`.
    pub fn rustup_target_add(&self, target: &str) -> Result<()> {
        let status = Command::new("rustup")
            .args(["target", "add", target])
            .status()
            .context("failed to execute rustup target add")?;
        if !status.success() {
            anyhow::bail!("rustup target add {} failed", target);
        }
        Ok(())
    }

    /// Checks if a target is installed via `rustup target list --installed`.
    pub fn is_target_installed(&self, target: &str) -> bool {
        Command::new("rustup")
            .args(["target", "list", "--installed"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.lines().any(|line| line.trim() == target))
            .unwrap_or(false)
    }

    /// Lists all available targets from rustup.
    pub fn available_targets(&self) -> Result<Vec<String>> {
        let output = Command::new("rustup")
            .args(["target", "list"])
            .output()
            .context("failed to execute rustup target list")?;
        if !output.status.success() {
            anyhow::bail!("rustup target list failed");
        }
        let stdout = String::from_utf8(output.stdout)
            .context("rustup target list output is not valid UTF-8")?;
        Ok(stdout
            .lines()
            .map(|line| line.split_whitespace().next().unwrap_or("").to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }
}

/// Describes a verified source for an external artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadRequest {
    pub url: String,
    pub destination: PathBuf,
    pub expected_checksum: Option<String>,
    pub checksum_algorithm: ChecksumAlgorithm,
    pub timeout: std::time::Duration,
    pub headers: std::collections::HashMap<String, String>,
}

impl DownloadRequest {
    /// Creates a new download request.
    pub fn new(url: impl Into<String>, destination: impl Into<PathBuf>) -> Self {
        Self {
            url: url.into(),
            destination: destination.into(),
            expected_checksum: None,
            checksum_algorithm: ChecksumAlgorithm::Sha256,
            timeout: std::time::Duration::from_secs(300),
            headers: std::collections::HashMap::new(),
        }
    }

    /// Creates a new download request with checksum verification.
    pub fn with_checksum(
        url: impl Into<String>,
        destination: impl Into<PathBuf>,
        checksum: impl Into<String>,
        algorithm: ChecksumAlgorithm,
    ) -> Self {
        Self {
            url: url.into(),
            destination: destination.into(),
            expected_checksum: Some(checksum.into()),
            checksum_algorithm: algorithm,
            timeout: std::time::Duration::from_secs(300),
            headers: std::collections::HashMap::new(),
        }
    }

    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Returns the provenance label for logging.
    pub fn provenance_label(&self) -> String {
        match &self.expected_checksum {
            Some(checksum) => format!("{} ({}:{})", self.url, self.checksum_algorithm, checksum),
            None => self.url.clone(),
        }
    }

    /// Checks if the request has checksum verification.
    pub fn is_verified(&self) -> bool {
        self.expected_checksum.is_some()
    }
}

/// Supported checksum algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumAlgorithm {
    Sha256,
    Sha512,
    Blake3,
}

impl std::fmt::Display for ChecksumAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChecksumAlgorithm::Sha256 => f.write_str("sha256"),
            ChecksumAlgorithm::Sha512 => f.write_str("sha512"),
            ChecksumAlgorithm::Blake3 => f.write_str("blake3"),
        }
    }
}

/// Describes package-manager level operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageManagerPlan {
    pub command: String,
}

impl PackageManagerPlan {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
        }
    }

    pub fn command_name(&self) -> &str {
        &self.command
    }
}

/// Describes release orchestration metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleasePlan {
    pub version: String,
}

impl ReleasePlan {
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            version: version.into(),
        }
    }

    pub fn is_prerelease(&self) -> bool {
        self.version.contains('-')
    }

    pub fn tag_name(&self) -> String {
        format!("v{}", self.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbuild_core::model::TargetTriple;
    use crossbuild_core::ValidationPlan;
    use crossbuild_core::WrapperPlan;

    #[test]
    fn install_plan_creation() {
        let plan = InstallPlan::new("target/crossbuild");
        assert_eq!(
            plan.destination,
            std::path::PathBuf::from("target/crossbuild")
        );
    }

    #[test]
    fn release_and_install_helpers_are_deterministic() {
        let release = ReleasePlan::new("1.2.3-alpha.1");
        let install = InstallPlan::new("bin");
        let wrapper = WrapperPlan::new("cargo", "build");
        let target = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();
        let validation =
            ValidationPlan::new("release-validation", target.clone()).with_release_mode(true);
        let package_manager = PackageManagerPlan::new("cargo");

        assert!(release.is_prerelease());
        assert_eq!(release.tag_name(), "v1.2.3-alpha.1");
        assert_eq!(
            install.resolved_destination(&PathBuf::from("C:/workspace")),
            PathBuf::from("C:/workspace/bin")
        );
        assert_eq!(wrapper.invocation(), "cargo build");
        assert!(validation.requires_release_mode());
        assert_eq!(package_manager.command_name(), "cargo");
    }
}
