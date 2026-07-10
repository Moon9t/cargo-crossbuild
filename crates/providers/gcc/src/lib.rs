use std::path::PathBuf;

use anyhow::Result;
use crossbuild_core::{
    error::CrossBuildError,
    model::{Abi, Architecture, HostInfo, TargetFamily, TargetTriple},
    provider::{LinkerFlavor, LinkerProvider, LinkerResolution},
};

/// System default linker provider.
pub struct SystemLinkerProvider;

impl LinkerProvider for SystemLinkerProvider {
    fn name(&self) -> &'static str {
        "system"
    }

    fn priority(&self) -> i32 {
        0
    }

    fn can_provide(&self, target: &TargetTriple, host: &HostInfo) -> bool {
        target.triple == host.host_triple.triple
    }

    fn resolve(
        &self,
        _target: &TargetTriple,
        _host: &HostInfo,
        _request: &crossbuild_core::model::BuildRequest,
    ) -> Result<LinkerResolution, CrossBuildError> {
        Ok(LinkerResolution::new(
            PathBuf::from("cc"),
            LinkerFlavor::Gnu,
        ))
    }
}

/// LLD linker provider.
pub struct LldLinkerProvider;

impl LinkerProvider for LldLinkerProvider {
    fn name(&self) -> &'static str {
        "lld"
    }

    fn priority(&self) -> i32 {
        100
    }

    fn can_provide(&self, target: &TargetTriple, _host: &HostInfo) -> bool {
        let has_linker = which::which("lld").is_ok() || which::which("ld.lld").is_ok();
        has_linker && target.abi != Abi::Msvc
    }

    fn resolve(
        &self,
        target: &TargetTriple,
        _host: &HostInfo,
        _request: &crossbuild_core::model::BuildRequest,
    ) -> Result<LinkerResolution, CrossBuildError> {
        let linker_path = which::which("ld.lld")
            .or_else(|_| which::which("lld"))
            .map_err(|_| CrossBuildError::ToolNotFound {
                tool: "lld".to_string(),
            })?;

        let flavor = if target.is_wasm() {
            LinkerFlavor::WasmLld
        } else {
            LinkerFlavor::Lld
        };

        Ok(LinkerResolution::new(linker_path, flavor))
    }
}

/// Mold linker provider.
pub struct MoldLinkerProvider;

impl LinkerProvider for MoldLinkerProvider {
    fn name(&self) -> &'static str {
        "mold"
    }

    fn priority(&self) -> i32 {
        90
    }

    fn can_provide(&self, target: &TargetTriple, _host: &HostInfo) -> bool {
        let has_mold = which::which("mold").is_ok();
        has_mold && matches!(target.family(), TargetFamily::Linux) && target.abi != Abi::Msvc
    }

    fn resolve(
        &self,
        _target: &TargetTriple,
        _host: &HostInfo,
        _request: &crossbuild_core::model::BuildRequest,
    ) -> Result<LinkerResolution, CrossBuildError> {
        let linker_path = which::which("mold").map_err(|_| CrossBuildError::ToolNotFound {
            tool: "mold".to_string(),
        })?;

        Ok(LinkerResolution::new(linker_path, LinkerFlavor::Mold))
    }
}

/// MSVC linker provider.
pub struct MsvcLinkerProvider;

impl LinkerProvider for MsvcLinkerProvider {
    fn name(&self) -> &'static str {
        "msvc"
    }

    fn priority(&self) -> i32 {
        200
    }

    fn can_provide(&self, target: &TargetTriple, _host: &HostInfo) -> bool {
        target.family() == TargetFamily::Windows && target.abi == Abi::Msvc
    }

    fn resolve(
        &self,
        target: &TargetTriple,
        _host: &HostInfo,
        _request: &crossbuild_core::model::BuildRequest,
    ) -> Result<LinkerResolution, CrossBuildError> {
        let link_path = find_msvc_link(&target.arch)?;
        Ok(LinkerResolution::new(link_path, LinkerFlavor::Msvc))
    }
}

fn find_msvc_link(arch: &Architecture) -> Result<PathBuf, CrossBuildError> {
    let arch_dir = match arch {
        Architecture::X86_64 => "x64",
        Architecture::X86 => "x86",
        Architecture::AArch64 => "arm64",
        Architecture::Arm => "arm",
        _ => {
            return Err(CrossBuildError::ToolNotFound {
                tool: format!("msvc linker for architecture {:?}", arch),
            })
        }
    };

    if let Some(path) = try_find_via_vs_installation(arch_dir) {
        return Ok(path);
    }

    if let Ok(path) = which::which("link") {
        return Ok(path);
    }

    Err(CrossBuildError::ToolNotFound {
        tool: "MSVC link.exe (install Visual Studio or run from Developer Command Prompt)"
            .to_string(),
    })
}

fn try_find_via_vs_installation(arch_dir: &str) -> Option<PathBuf> {
    let program_files = std::env::var("ProgramFiles")
        .or_else(|_| std::env::var("ProgramFiles(x86)"))
        .ok()?;

    let vs_base = PathBuf::from(&program_files).join("Microsoft Visual Studio");
    let editions = ["Enterprise", "Professional", "Community", "BuildTools"];
    let years = ["2022", "2019"];

    for year in &years {
        for edition in &editions {
            let tools_dir = vs_base
                .join(year)
                .join(edition)
                .join("VC")
                .join("Tools")
                .join("MSVC");

            if let Ok(entries) = std::fs::read_dir(&tools_dir) {
                for entry in entries.flatten() {
                    let path = entry
                        .path()
                        .join("bin")
                        .join(format!("Host{}", arch_dir))
                        .join(arch_dir)
                        .join("link.exe");
                    if path.exists() {
                        return Some(path);
                    }
                }
            }
        }
    }

    None
}

/// Zig linker provider (uses `zig cc` as a linker).
pub struct ZigLinkerProvider;

impl LinkerProvider for ZigLinkerProvider {
    fn name(&self) -> &'static str {
        "zig"
    }

    fn priority(&self) -> i32 {
        50
    }

    fn can_provide(&self, _target: &TargetTriple, _host: &HostInfo) -> bool {
        which::which("zig").is_ok()
    }

    fn resolve(
        &self,
        target: &TargetTriple,
        _host: &HostInfo,
        _request: &crossbuild_core::model::BuildRequest,
    ) -> Result<LinkerResolution, CrossBuildError> {
        let zig_path = which::which("zig").map_err(|_| CrossBuildError::ToolNotFound {
            tool: "zig".to_string(),
        })?;

        let target_arg = format!("-target {}", target.triple);
        Ok(LinkerResolution::new(zig_path.clone(), LinkerFlavor::Lld).with_args(vec![target_arg]))
    }
}
