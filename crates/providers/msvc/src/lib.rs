use std::path::PathBuf;

use crossbuild_core::{
    error::CrossBuildError,
    model::{Abi, Architecture, BuildRequest, HostInfo, TargetFamily, TargetTriple},
    provider::{LinkerFlavor, LinkerProvider, LinkerResolution},
};

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
        _request: &BuildRequest,
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
