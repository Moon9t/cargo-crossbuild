use crossbuild_core::{
    error::CrossBuildError,
    model::{Abi, BuildRequest, HostInfo, TargetTriple, ToolchainHint},
    provider::{ToolchainProvider, ToolchainResolution},
};

/// Clang-based toolchain provider.
pub struct ClangToolchainProvider;

impl ToolchainProvider for ClangToolchainProvider {
    fn name(&self) -> &'static str {
        "clang"
    }

    fn priority(&self) -> i32 {
        70
    }

    fn can_provide(&self, target: &TargetTriple, _host: &HostInfo) -> bool {
        target.abi != Abi::Msvc
    }

    fn resolve(
        &self,
        target: &TargetTriple,
        _host: &HostInfo,
        _request: &BuildRequest,
    ) -> Result<ToolchainResolution, CrossBuildError> {
        let clang_path = which::which("clang").map_err(|_| CrossBuildError::ToolNotFound {
            tool: "clang".to_string(),
        })?;

        let mut resolution = ToolchainResolution::new()
            .with_note("Using clang toolchain")
            .with_rustc(clang_path.clone())
            .with_cargo(which::which("cargo")?)
            .with_env("CC", clang_path.to_string_lossy())
            .with_env("CXX", "clang++");

        if target.triple != _host.host_triple.triple {
            resolution = resolution.with_target_spec(target.triple.clone());
        }

        Ok(resolution)
    }

    fn hint(&self) -> ToolchainHint {
        ToolchainHint::Rustup
    }
}
