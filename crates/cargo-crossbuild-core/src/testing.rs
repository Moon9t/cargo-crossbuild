//! Testing framework for cross-compilation validation.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

use crate::model::{BuildRequest, TargetTriple, ExecutionMode, Profile};
use crate::error::CrossBuildError;

/// Validation plan for cross-build testing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationPlan {
    pub label: String,
    pub target: TargetTriple,
    pub test_command: Vec<String>,
    pub expected_artifacts: Vec<PathBuf>,
    pub requires_release: bool,
    pub env: BTreeMap<String, String>,
    pub timeout_secs: u64,
}

impl ValidationPlan {
    pub fn new(label: impl Into<String>, target: TargetTriple) -> Self {
        Self {
            label: label.into(),
            target,
            test_command: vec!["cargo".into(), "test".into()],
            expected_artifacts: Vec::new(),
            requires_release: false,
            env: BTreeMap::new(),
            timeout_secs: 300,
        }
    }

    pub fn with_test_command(mut self, cmd: Vec<String>) -> Self {
        self.test_command = cmd;
        self
    }

    pub fn with_artifacts(mut self, artifacts: Vec<PathBuf>) -> Self {
        self.expected_artifacts = artifacts;
        self
    }

    pub fn with_release_mode(mut self, requires_release: bool) -> Self {
        self.requires_release = requires_release;
        self
    }

    pub fn with_env(mut self, env: BTreeMap<String, String>) -> Self {
        self.env = env;
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    pub fn requires_release_mode(&self) -> bool {
        self.requires_release
    }
}

/// Test runner for cross-compilation validation.
pub struct CrossTestRunner {
    workspace_root: PathBuf,
}

impl CrossTestRunner {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
        }
    }

    /// Runs a validation plan.
    pub fn run(&self, plan: &ValidationPlan, build_artifacts: &PathBuf) -> Result<ValidationResult, CrossBuildError> {
        let mut cmd = Command::new(&plan.test_command[0]);
        cmd.current_dir(&self.workspace_root);
        cmd.args(&plan.test_command[1..]);

        for (k, v) in &plan.env {
            cmd.env(k, v);
        }

        // Set target directory
        cmd.env("CARGO_TARGET_DIR", build_artifacts);

        let output = cmd.output().map_err(|source| CrossBuildError::Io {
            path: None,
            source,
        })?;

        let mut result = ValidationResult {
            label: plan.label.clone(),
            target: plan.target.clone(),
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
            artifacts_found: Vec::new(),
            artifacts_missing: Vec::new(),
        };

        // Check for expected artifacts
        for artifact in &plan.expected_artifacts {
            let path = build_artifacts.join(artifact);
            if path.exists() {
                result.artifacts_found.push(path);
            } else {
                result.artifacts_missing.push(path);
            }
        }

        Ok(result)
    }

    /// Runs all validations for a build.
    pub fn run_all(
        &self,
        plans: &[ValidationPlan],
        build_artifacts: &PathBuf,
    ) -> Vec<Result<ValidationResult, CrossBuildError>> {
        plans.iter()
            .map(|plan| self.run(plan, build_artifacts))
            .collect()
    }
}

/// Result of a validation run.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub label: String,
    pub target: TargetTriple,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub artifacts_found: Vec<PathBuf>,
    pub artifacts_missing: Vec<PathBuf>,
}

impl ValidationResult {
    pub fn summary(&self) -> String {
        if self.success && self.artifacts_missing.is_empty() {
            format!("✓ {} ({})", self.label, self.target)
        } else if !self.success {
            format!("✗ {} ({}) - exit code: {:?}", self.label, self.target, self.exit_code)
        } else {
            format!("⚠ {} ({}) - missing {} artifacts",
                self.label, self.target, self.artifacts_missing.len())
        }
    }
}

/// Pre-defined validation plans for common scenarios.
pub struct StandardValidations;

impl StandardValidations {
    /// Validation for a basic library crate.
    pub fn library(target: TargetTriple) -> ValidationPlan {
        ValidationPlan::new("library", target)
            .with_test_command(vec!["cargo".into(), "test".into(), "--lib".into()])
            .with_artifacts(vec![
                PathBuf::from("libtest.rlib"),
                PathBuf::from("deps"),
            ])
    }

    /// Validation for a binary crate.
    pub fn binary(target: TargetTriple) -> ValidationPlan {
        ValidationPlan::new("binary", target)
            .with_test_command(vec!["cargo".into(), "test".into(), "--bin".into(), "main".into()])
            .with_artifacts(vec![PathBuf::from("main")])
    }

    /// Validation for a crate with both library and binary.
    pub fn mixed(target: TargetTriple) -> ValidationPlan {
        ValidationPlan::new("mixed", target)
            .with_test_command(vec!["cargo".into(), "test".into()])
            .with_artifacts(vec![
                PathBuf::from("libtest.rlib"),
                PathBuf::from("main"),
            ])
    }

    /// Validation for no_std crate.
    pub fn no_std(target: TargetTriple) -> ValidationPlan {
        ValidationPlan::new("no_std", target)
            .with_test_command(vec!["cargo".into(), "build".into()])
            .with_artifacts(vec![PathBuf::from("libnostd.rlib")])
            .with_env({
                let mut env = BTreeMap::new();
                env.insert("RUSTFLAGS".to_string(), "--cfg=no_std".to_string());
                env
            })
    }

    /// All standard validations for a target.
    pub fn all(target: TargetTriple) -> Vec<ValidationPlan> {
        vec![
            Self::library(target.clone()),
            Self::binary(target.clone()),
            Self::mixed(target.clone()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TargetTriple;

    #[test]
    fn validation_plan_creation() {
        let target = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();
        let plan = ValidationPlan::new("test", target.clone())
            .with_test_command(vec!["cargo".into(), "test".into()])
            .with_artifacts(vec![PathBuf::from("libtest.rlib")]);

        assert_eq!(plan.label, "test");
        assert_eq!(plan.target, target);
        assert!(!plan.requires_release_mode());
    }

    #[test]
    fn standard_validations() {
        let target = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();
        let plans = StandardValidations::all(target.clone());
        assert_eq!(plans.len(), 3);
        assert!(plans.iter().any(|p| p.label == "library"));
        assert!(plans.iter().any(|p| p.label == "binary"));
        assert!(plans.iter().any(|p| p.label == "mixed"));
    }

    #[test]
    fn validation_result_summary() {
        let target = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();
        let result = ValidationResult {
            label: "test".to_string(),
            target: target.clone(),
            success: true,
            stdout: "ok".to_string(),
            stderr: "".to_string(),
            exit_code: Some(0),
            artifacts_found: vec![],
            artifacts_missing: vec![],
        };
        assert!(result.summary().contains("✓"));
    }
}