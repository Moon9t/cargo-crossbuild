use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use crossbuild_core::config::CrossBuildConfig;
use crossbuild_core::model::{BuildPlan, HostInfo, TargetTriple};

/// Environment configuration extracted from a resolved build plan.
#[derive(Debug, Clone)]
pub struct Environment {
    pub target: TargetTriple,
    pub host: HostInfo,
    pub env_vars: BTreeMap<String, String>,
    pub cargo_target_dir: Option<PathBuf>,
    pub cargo_config: Option<toml::Table>,
}

impl Environment {
    /// Creates an `Environment` from a fully resolved `BuildPlan`.
    ///
    /// Extracts target triple, host info, environment variables from
    /// the command line and provider actions, and the cargo config.
    pub fn from_plan(plan: &BuildPlan) -> Self {
        let mut env_vars = plan.command.env.clone();

        for action in &plan.provider_actions {
            env_vars.extend(action.env.clone());
        }

        let cargo_target_dir = env_vars
            .get("CARGO_TARGET_DIR")
            .map(PathBuf::from);

        Self {
            target: plan.target_triple().clone(),
            host: plan.host.clone(),
            env_vars,
            cargo_target_dir,
            cargo_config: plan.cargo_config.clone(),
        }
    }

    /// Applies the environment variables to the current process.
    ///
    /// Sets CC, CXX, AR, RUSTFLAGS, CARGO_TARGET_DIR, PATH additions,
    /// and all other env vars extracted from the plan.
    pub fn apply(&self) {
        for (key, value) in &self.env_vars {
            std::env::set_var(key, value);
        }

        if let Some(ref dir) = self.cargo_target_dir {
            std::env::set_var("CARGO_TARGET_DIR", dir);
        }
    }

    /// Writes the cargo config.toml to `output_path` from the plan's cargo_config.
    pub fn generate_cargo_config(&self, output_path: &Path) -> Result<()> {
        if let Some(ref config) = self.cargo_config {
            let toml_str = toml::to_string(config)?;
            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(output_path, toml_str)?;
        }
        Ok(())
    }
}

/// Convenience function to build an `Environment` from a plan and config.
pub fn setup_environment(plan: &BuildPlan, _config: &CrossBuildConfig) -> Result<Environment> {
    Ok(Environment::from_plan(plan))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbuild_core::model::{BuildRequest, CommandLine, TargetTriple};
    use std::collections::BTreeMap;

    fn dummy_plan() -> BuildPlan {
        let target = TargetTriple::parse("aarch64-unknown-linux-gnu").unwrap();
        let host_triple = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();
        let host = HostInfo {
            host_triple: host_triple,
            os: crossbuild_core::model::OperatingSystem::Linux,
            arch: crossbuild_core::model::Architecture::X86_64,
            rustc_version: None,
            cargo_version: None,
            target_dir: PathBuf::from("target/crossbuild"),
        };
        let mut env = BTreeMap::new();
        env.insert("CARGO_TARGET_DIR".to_string(), "/tmp/cross".to_string());
        env.insert("CC_aarch64-unknown-linux-gnu".to_string(), "aarch64-linux-gnu-gcc".to_string());

        BuildPlan {
            request: BuildRequest::new(
                PathBuf::from("."),
                TargetTriple::parse("aarch64-unknown-linux-gnu").unwrap(),
            ),
            host,
            target: crossbuild_core::model::TargetInfo {
                triple: target.clone(),
                is_native: false,
                requires_cross: true,
                supported: crossbuild_core::model::TargetSupport::Tier2,
                toolchain_hint: crossbuild_core::model::ToolchainHint::Rustup,
                sysroot_hint: crossbuild_core::model::SysrootHint::Rustup,
                linker_hint: crossbuild_core::model::LinkerHint::Lld,
            },
            command: CommandLine {
                program: "cargo".to_string(),
                args: vec!["build".to_string()],
                env,
                current_dir: PathBuf::from("."),
            },
            steps: vec![],
            provider_actions: vec![],
            cargo_config: None,
            cache_key: "test".to_string(),
        }
    }

    #[test]
    fn from_plan_creates_environment() {
        let plan = dummy_plan();
        let env = Environment::from_plan(&plan);
        assert_eq!(env.target.triple, "aarch64-unknown-linux-gnu");
        assert!(env.env_vars.contains_key("CARGO_TARGET_DIR"));
        assert_eq!(
            env.env_vars.get("CARGO_TARGET_DIR").unwrap(),
            "/tmp/cross"
        );
    }

    #[test]
    fn apply_sets_env_vars() {
        let plan = dummy_plan();
        let env = Environment::from_plan(&plan);
        env.apply();
        assert_eq!(
            std::env::var("CC_aarch64-unknown-linux-gnu").ok(),
            Some("aarch64-linux-gnu-gcc".to_string())
        );
        std::env::remove_var("CC_aarch64-unknown-linux-gnu");
    }

    #[test]
    fn generate_cargo_config_does_not_write_when_none() {
        let plan = dummy_plan();
        let env = Environment::from_plan(&plan);
        let tmp = std::env::temp_dir().join("cargo-crossbuild-test-config.toml");
        let _ = std::fs::remove_file(&tmp);
        env.generate_cargo_config(&tmp).unwrap();
        assert!(!tmp.exists());
    }
}
