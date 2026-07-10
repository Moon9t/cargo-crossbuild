use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::{
    cache::{CacheManager, CachePolicy},
    cargo_config::CargoConfigGenerator,
    config::CrossBuildConfig,
    error::CrossBuildError,
    model::{
        BuildPlan, BuildRequest, CommandLine,
        PlanStep, Profile, ProviderAction, TargetTriple,
    },
    platform::{assess_target, detect_host},
    registry::ProviderRegistry,
};

/// Resolves requests into executable build plans.
#[derive(Debug)]
pub struct Planner {
    pub(crate) registry: ProviderRegistry,
    cache_policy: CachePolicy,
}

impl Planner {
    pub fn new() -> Self {
        Self {
            registry: ProviderRegistry::new(),
            cache_policy: CachePolicy::default(),
        }
    }

    pub fn with_cache_policy(mut self, policy: CachePolicy) -> Self {
        self.cache_policy = policy;
        self
    }

    pub fn plan(
        &self,
        request: BuildRequest,
        config: &CrossBuildConfig,
    ) -> Result<BuildPlan, CrossBuildError> {
        let target_triple = request.target_triple.clone();
        let manifest_path = normalize_manifest_path(&request.manifest_path)?;
        let host = detect_host()?;
        let target = assess_target(&target_triple, &host);

        let workspace_root = manifest_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        // Resolve all providers
        let complete_resolution = self.registry.resolve_all(&target_triple, &host, &request)?;

        // Create cache manager
        let cache_manager = CacheManager::new(self.cache_policy.clone(), &workspace_root)?;

        // Generate cargo config
        let cargo_config = CargoConfigGenerator::new(target_triple.clone(), workspace_root.clone())
            .with_linker(
                complete_resolution.linker.linker_path.clone(),
                complete_resolution.linker.flavor.cargo_name(),
                complete_resolution.linker.linker_args.clone(),
            )
            .with_build_target(&target_triple);

        let cargo_config = if let Some(sysroot) = &complete_resolution.sysroot {
            if !sysroot.sysroot_path.as_os_str().is_empty() {
                cargo_config.with_sysroot(sysroot.sysroot_path.clone())
            } else {
                cargo_config
            }
        } else {
            cargo_config
        };

        let cargo_config = if !complete_resolution.toolchain.rustflags.is_empty() {
            cargo_config.with_rustflags(complete_resolution.toolchain.rustflags.clone())
        } else {
            cargo_config
        };

        let cargo_config_table = cargo_config.build();

        // Merge environment from all providers
        let mut env = BTreeMap::new();
        env.insert(
            "CARGO_TARGET_DIR".to_string(),
            config
                .target_dir_for(&workspace_root)
                .to_string_lossy()
                .into_owned(),
        );
        env.extend(config.extra_env.clone());
        env.extend(complete_resolution.merged_env());

        // Build command
        let mut command = CommandLine::new(config.cargo_program_str(), &workspace_root);
        command.env = env;
        command.push_arg("build");
        command.push_arg("--manifest-path");
        command.push_arg(manifest_path.to_string_lossy().into_owned());
        command.push_arg("--target");
        command.push_arg(target_triple.as_str());

        // Add profile flag
        match request.profile {
            Profile::Release => {
                command.push_arg("--release");
            }
            Profile::Dev => {}
            Profile::Custom(name) => {
                command.push_arg("--profile");
                command.push_arg(name);
            }
        }

        // Add features
        if !request.features.is_empty() {
            command.push_arg("--features");
            command.push_arg(request.features.join(","));
        }

        if request.no_default_features {
            command.push_arg("--no-default-features");
        }

        if request.workspace {
            command.push_arg("--workspace");
        }

        for exclude in &request.exclude {
            command.push_arg("--exclude");
            command.push_arg(exclude);
        }

        // Add cargo args
        for arg in &request.cargo_args {
            command.push_arg(arg.clone());
        }

        // Build steps
        let steps = vec![
            PlanStep::ValidateManifest { path: manifest_path.clone() },
            PlanStep::ValidateTarget { target: target_triple.clone() },
            PlanStep::DetectHost,
            PlanStep::ResolveProviders,
            PlanStep::PrepareEnvironment,
            PlanStep::GenerateCargoConfig,
            PlanStep::ResolveLinker,
            PlanStep::PrepareCache,
            PlanStep::InvokeCargo,
            PlanStep::CaptureDiagnostics,
            PlanStep::VerifyArtifacts,
        ];

        // Generate cache key
        let cache_key = cache_manager.policy().cache_key(&workspace_root, &target_triple);

        // Build provider actions from the complete resolution
        let provider_actions = build_provider_actions(&complete_resolution, &target_triple);

        Ok(BuildPlan {
            request: BuildRequest {
                manifest_path,
                ..request
            },
            host,
            target,
            command,
            steps,
            provider_actions,
            cargo_config: Some(cargo_config_table),
            cache_key,
        })
    }
}

fn normalize_manifest_path(path: &Path) -> Result<PathBuf, CrossBuildError> {
    let metadata = std::fs::metadata(path).map_err(|source| CrossBuildError::Io {
        path: Some(path.to_path_buf()),
        source,
    })?;

    if metadata.is_dir() {
        let candidate = path.join("Cargo.toml");
        if candidate.is_file() {
            return Ok(candidate);
        }
        return Err(CrossBuildError::ManifestNotFound {
            searched_from: path.to_path_buf(),
        });
    }

    if path.file_name().and_then(|name| name.to_str()) != Some("Cargo.toml") {
        return Err(CrossBuildError::ManifestNotCargoToml {
            path: path.to_path_buf(),
        });
    }

    Ok(path.to_path_buf())
}

/// Builds provider action entries from the complete resolution.
fn build_provider_actions(
    resolution: &crate::registry::CompleteResolution,
    _target: &TargetTriple,
) -> Vec<ProviderAction> {
    let mut actions = Vec::new();

    // Toolchain action
    actions.push(ProviderAction {
        provider_name: "toolchain".to_string(),
        notes: resolution.toolchain.notes.clone(),
        env: resolution.toolchain.env.clone(),
        cargo_config: resolution.toolchain.cargo_config.clone(),
    });

    // Sysroot action (if present)
    if let Some(ref sysroot) = resolution.sysroot {
        actions.push(ProviderAction {
            provider_name: "sysroot".to_string(),
            notes: sysroot.notes.clone(),
            env: sysroot.env.clone(),
            cargo_config: sysroot.cargo_config.clone(),
        });
    }

    // Linker action
    actions.push(ProviderAction {
        provider_name: "linker".to_string(),
        notes: resolution.linker.notes.clone(),
        env: resolution.linker.env.clone(),
        cargo_config: resolution.linker.cargo_config.clone(),
    });

    actions
}

impl Default for Planner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{BuildRequest, TargetTriple};
    use crate::provider::{
        BuiltinToolchainProvider, RustupToolchainProvider, ZigToolchainProvider,
        NoSysrootProvider,
    };
    use tempfile::tempdir;

    /// Test-only linker provider that always resolves.
    struct TestLinkerProvider;

    impl crate::provider::LinkerProvider for TestLinkerProvider {
        fn name(&self) -> &'static str { "test-linker" }
        fn priority(&self) -> i32 { 0 }
        fn can_provide(&self, _target: &TargetTriple, _host: &crate::model::HostInfo) -> bool { true }
        fn resolve(
            &self,
            target: &TargetTriple,
            _host: &crate::model::HostInfo,
            _request: &BuildRequest,
        ) -> Result<crate::provider::LinkerResolution, CrossBuildError> {
            Ok(crate::provider::LinkerResolution::new(
                std::path::PathBuf::from("cc"),
                crate::provider::LinkerFlavor::Gnu,
            ))
        }
    }

    #[test]
    fn planner_creation() {
        let planner = Planner::new();
        assert!(format!("{:?}", planner).contains("Planner"));
    }

    #[test]
    fn planner_resolves_native_target() {
        let mut planner = Planner::new();
        planner.registry.register_toolchain(Box::new(BuiltinToolchainProvider));
        planner.registry.register_toolchain(Box::new(RustupToolchainProvider));
        planner.registry.register_toolchain(Box::new(ZigToolchainProvider));
        planner.registry.register_sysroot(Box::new(NoSysrootProvider));
        planner.registry.register_linker(Box::new(TestLinkerProvider));

        let dir = tempdir().unwrap();
        let manifest = dir.path().join("Cargo.toml");
        std::fs::write(&manifest, r#"
[package]
name = "test"
version = "0.1.0"
edition = "2021"
"#).unwrap();

        let host = crate::platform::detect_host().unwrap();
        let target = TargetTriple::parse(&host.host_triple.triple).unwrap();
        let request = BuildRequest::new(manifest, target);
        let config = CrossBuildConfig::default();

        let plan = planner.plan(request, &config).unwrap();
        assert!(!plan.is_cross_compilation());
    }
}