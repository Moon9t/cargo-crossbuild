//! Provider implementations for toolchain, sysroot, and linker resolution.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::model::{
    Abi, Architecture, BuildRequest, HostInfo, LinkerFlavor, LinkerHint, OperatingSystem,
    SysrootHint, TargetFamily, TargetTriple, ToolchainHint,
};
use crate::error::CrossBuildError;

/// Rustup-based toolchain provider.
pub struct RustupToolchainProvider;

impl super::ToolchainProvider for RustupToolchainProvider {
    fn name(&self) -> &'static str {
        "rustup"
    }

    fn priority(&self) -> i32 {
        100
    }

    fn can_provide(&self, target: &TargetTriple, host: &HostInfo) -> bool {
        // Can provide if target is available via rustup
        if target.triple == host.host_triple.triple {
            return true;
        }
        crate::rustup_target_available(target)
    }

    fn resolve(
        &self,
        target: &TargetTriple,
        host: &HostInfo,
        _request: &BuildRequest,
) -> Result<crate::provider::ToolchainResolution, CrossBuildError> {
        let mut resolution = crate::provider::ToolchainResolution::new();

        if target.triple == host.host_triple.triple {
            resolution = resolution
                .with_note("Using host toolchain for native build")
                .with_rustc(which::which("rustc")?)
                .with_cargo(which::which("cargo")?);
        } else {
            // Cross-compilation with rustup target
            let rustup_home = std::env::var("RUSTUP_HOME")
                .or_else(|_| std::env::var("HOME"))
                .map(|h| PathBuf::from(h).join(".rustup"))
                .unwrap_or_else(|_| PathBuf::from("/rustup"));

            let toolchain = crate::find_rustup_toolchain(&rustup_home.to_string_lossy())?;
            let toolchain_path = rustup_home.join("toolchains").join(&toolchain);

            let rustc_path = toolchain_path.join("bin").join("rustc");
            let cargo_path = toolchain_path.join("bin").join("cargo");

            resolution = resolution
                .with_note(format!("Using rustup toolchain: {toolchain}"))
                .with_rustc(rustc_path)
                .with_cargo(cargo_path)
                .with_env("RUSTUP_TOOLCHAIN", toolchain);
        }

        // Add target specification if not native
        if target.triple != host.host_triple.triple {
            resolution = resolution.with_target_spec(target.triple.clone());
            resolution = resolution.with_env("CARGO_BUILD_TARGET", target.triple.clone());
        }

        Ok(resolution)
    }

    fn hint(&self) -> ToolchainHint {
        ToolchainHint::Rustup
    }
}

/// Zig-based toolchain provider.
pub struct ZigToolchainProvider;

impl crate::provider::ToolchainProvider for ZigToolchainProvider {
    fn name(&self) -> &'static str {
        "zig"
    }

    fn priority(&self) -> i32 {
        50
    }

    fn can_provide(&self, target: &TargetTriple, _host: &HostInfo) -> bool {
        // Zig can target most platforms
        !matches!(target.family(), TargetFamily::Other | TargetFamily::BareMetal)
            || target.is_wasm()
    }

    fn resolve(
        &self,
        target: &TargetTriple,
        _host: &HostInfo,
        _request: &BuildRequest,
    ) -> Result<crate::provider::ToolchainResolution, CrossBuildError> {
        let zig_path = which::which("zig").map_err(|_| CrossBuildError::ToolNotFound {
            tool: "zig".to_string(),
        })?;

        let target_arg = target.to_zig_target();

        let mut resolution = crate::provider::ToolchainResolution::new()
            .with_note(format!("Using zig cc for target: {target_arg}"))
            .with_rustc(zig_path.clone())
            .with_cargo(which::which("cargo")?)
            .with_env("CC", format!("zig cc -target {}", target_arg))
            .with_env("CXX", format!("zig c++ -target {}", target_arg))
            .with_env("AR", "zig ar")
            .with_env("CARGO_TARGET_RUNNER", format!("zig cc -target {}", target_arg));

        // Add linker flags for zig
        resolution = resolution.with_env(
            "CARGO_TARGET_RUSTFLAGS",
            format!("-C linker=zig cc -target {}", target_arg),
        );

        Ok(resolution)
    }

    fn hint(&self) -> ToolchainHint {
        ToolchainHint::Zig
    }
}

/// Built-in toolchain provider (host native only).
pub struct BuiltinToolchainProvider;

impl crate::provider::ToolchainProvider for BuiltinToolchainProvider {
    fn name(&self) -> &'static str {
        "builtin"
    }

    fn priority(&self) -> i32 {
        200
    }

    fn can_provide(&self, target: &TargetTriple, host: &HostInfo) -> bool {
        target.triple == host.host_triple.triple
    }

    fn resolve(
        &self,
        _target: &TargetTriple,
        host: &HostInfo,
        _request: &BuildRequest,
    ) -> Result<crate::provider::ToolchainResolution, CrossBuildError> {
        Ok(crate::provider::ToolchainResolution::new()
            .with_note("Using host toolchain")
            .with_rustc(which::which("rustc")?)
            .with_cargo(which::which("cargo")?))
    }

    fn hint(&self) -> ToolchainHint {
        ToolchainHint::Rustup
    }
}

/// Trait for targets that can convert to zig target string.
trait ZigTarget {
    fn to_zig_target(&self) -> String;
}

impl ZigTarget for TargetTriple {
    fn to_zig_target(&self) -> String {
        let arch = match self.arch {
            Architecture::X86_64 => "x86_64",
            Architecture::AArch64 => "aarch64",
            Architecture::X86 => "x86",
            Architecture::Arm => "arm",
            Architecture::Arm64 => "aarch64",
            Architecture::RiscV64 => "riscv64",
            Architecture::PowerPC64 => "powerpc64le",
            Architecture::S390x => "s390x",
            Architecture::Mips64 => "mips64",
            Architecture::LoongArch64 => "loongarch64",
            Architecture::Wasm32 => "wasm32",
            Architecture::Wasm64 => "wasm64",
            Architecture::Other => "unknown",
        };

        let os = match self.os {
            OperatingSystem::Linux => "linux",
            OperatingSystem::Windows => "windows",
            OperatingSystem::MacOs => "macos",
            OperatingSystem::FreeBsd => "freebsd",
            OperatingSystem::NetBsd => "netbsd",
            OperatingSystem::OpenBsd => "openbsd",
            OperatingSystem::DragonflyBsd => "dragonflybsd",
            OperatingSystem::Android => "android",
            OperatingSystem::Wasm => "wasi",
            OperatingSystem::Wasi => "wasi",
            OperatingSystem::None => "freestanding",
            OperatingSystem::Uefi => "uefi",
            OperatingSystem::Ios => "ios",
            OperatingSystem::TvOs => "tvos",
            OperatingSystem::WatchOs => "watchos",
            OperatingSystem::Fuchsia => "fuchsia",
            OperatingSystem::Redox => "redox",
            OperatingSystem::Solaris => "solaris",
            OperatingSystem::Illumos => "illumos",
            OperatingSystem::Heron => "heron",
            OperatingSystem::Zos => "zos",
            OperatingSystem::Other => "unknown",
        };

        let abi = match self.abi {
            Abi::Gnu => "gnu",
            Abi::Musl => "musl",
            Abi::Msvc => "msvc",
            Abi::Android => "android",
            Abi::Wasm32 => "wasi",
            Abi::None => "",
            Abi::Eabi => "eabi",
            Abi::Eabihf => "eabihf",
            Abi::Wasm64 => "wasi",
        };

        // Special case: wasm32-wasi has OS=wasi and ABI=wasi, avoid duplication
        if os == "wasi" && abi == "wasi" {
            format!("{}-{}", arch, os)
        } else if abi.is_empty() {
            format!("{}-{}", arch, os)
        } else {
            format!("{}-{}-{}", arch, os, abi)
        }
    }
}

/// Rustup-based sysroot provider.
pub struct RustupSysrootProvider;

impl crate::provider::SysrootProvider for RustupSysrootProvider {
    fn name(&self) -> &'static str {
        "rustup"
    }

    fn priority(&self) -> i32 {
        100
    }

    fn can_provide(&self, target: &TargetTriple, host: &HostInfo) -> bool {
        if target.triple == host.host_triple.triple {
            return false; // Native builds don't need sysroot
        }
        crate::rustup_target_available(target)
    }

    fn resolve(
        &self,
        target: &TargetTriple,
        host: &HostInfo,
        _request: &BuildRequest,
    ) -> Result<crate::provider::SysrootResolution, CrossBuildError> {
        if target.triple == host.host_triple.triple {
            return Err(CrossBuildError::SysrootNotNeeded);
        }

        let rustup_home = std::env::var("RUSTUP_HOME")
            .or_else(|_| std::env::var("HOME"))
            .map(|h| PathBuf::from(h).join(".rustup"))
            .unwrap_or_else(|_| PathBuf::from("/rustup"));

        let toolchain = crate::find_rustup_toolchain(&rustup_home.to_string_lossy())?;
        let sysroot = rustup_home
            .join("toolchains")
            .join(&toolchain)
            .join("lib")
            .join("rustlib")
            .join(&target.triple);

        if !sysroot.exists() {
            return Err(CrossBuildError::SysrootNotFound {
                target: target.triple.clone(),
            });
        }

        let mut resolution = crate::provider::SysrootResolution::new(sysroot.clone())
            .with_note(format!("Using rustup sysroot from toolchain: {toolchain}"))
            .with_env("CARGO_SYSROOT", sysroot.to_string_lossy())
            .with_builtin(true);

        // Add linker search paths
        let lib_dir = sysroot.join("lib");
        if lib_dir.exists() {
            resolution = resolution.with_env("LIBRARY_PATH", lib_dir.to_string_lossy());
        }

        Ok(resolution)
    }
}

/// Zig-based sysroot provider.
pub struct ZigSysrootProvider;

impl crate::provider::SysrootProvider for ZigSysrootProvider {
    fn name(&self) -> &'static str {
        "zig"
    }

    fn priority(&self) -> i32 {
        50
    }

    fn can_provide(&self, target: &TargetTriple, _host: &HostInfo) -> bool {
        // Zig provides sysroots for many targets
        !matches!(target.os, OperatingSystem::None)
    }

    fn resolve(
        &self,
        target: &TargetTriple,
        _host: &HostInfo,
        _request: &BuildRequest,
    ) -> Result<crate::provider::SysrootResolution, CrossBuildError> {
        let zig_path = which::which("zig").map_err(|_| CrossBuildError::ToolNotFound {
            tool: "zig".to_string(),
        })?;

        // Zig doesn't have a separate sysroot - it uses its internal libc
        let sysroot = std::env::temp_dir().join("zig-sysroot").join(&target.triple);

        let mut resolution = crate::provider::SysrootResolution::new(sysroot)
            .with_note("Using zig's built-in libc/sysroot")
            .with_env("ZIG_SYSROOT", "1");

        Ok(resolution)
    }
}

/// No sysroot needed (wasm, bare metal).
pub struct NoSysrootProvider;

impl crate::provider::SysrootProvider for NoSysrootProvider {
    fn name(&self) -> &'static str {
        "none"
    }

    fn priority(&self) -> i32 {
        200
    }

    fn can_provide(&self, target: &TargetTriple, host: &HostInfo) -> bool {
        target.triple == host.host_triple.triple
            || target.is_wasm()
            || target.is_bare_metal()
    }

    fn resolve(
        &self,
        target: &TargetTriple,
        host: &HostInfo,
        _request: &BuildRequest,
    ) -> Result<crate::provider::SysrootResolution, CrossBuildError> {
        if target.triple == host.host_triple.triple {
            return Err(CrossBuildError::SysrootNotNeeded);
        }

        Ok(crate::provider::SysrootResolution::new(PathBuf::new())
            .with_note("No sysroot required for this target"))
    }
}

/// System default linker provider.
pub struct SystemLinkerProvider;

impl crate::provider::LinkerProvider for SystemLinkerProvider {
    fn name(&self) -> &'static str {
        "system"
    }

    fn priority(&self) -> i32 {
        100
    }

    fn can_provide(&self, target: &TargetTriple, host: &HostInfo) -> bool {
        target.triple == host.host_triple.triple
    }

    fn resolve(
        &self,
        _target: &TargetTriple,
        _host: &HostInfo,
        _request: &BuildRequest,
    ) -> Result<super::LinkerResolution, CrossBuildError> {
        Ok(super::LinkerResolution::new(
            which::which("ld").unwrap_or_else(|_| PathBuf::from("ld")),
            LinkerFlavor::Gnu,
        )
        .with_note("Using system default linker"))
    }
}

/// LLD linker provider.
pub struct LldLinkerProvider;

impl crate::provider::LinkerProvider for LldLinkerProvider {
    fn name(&self) -> &'static str {
        "lld"
    }

    fn priority(&self) -> i32 {
        150
    }

    fn can_provide(&self, target: &TargetTriple, _host: &HostInfo) -> bool {
        // LLD supports most ELF targets and WASM
        matches!(
            target.family(),
            TargetFamily::Linux
                | TargetFamily::Wasm
                | TargetFamily::MacOs
                | TargetFamily::BareMetal
                | TargetFamily::Other
        ) && target.abi != Abi::Msvc
    }

    fn resolve(
        &self,
        target: &TargetTriple,
        _host: &HostInfo,
        _request: &BuildRequest,
    ) -> Result<crate::provider::LinkerResolution, CrossBuildError> {
        let (linker_name, flavor) = match target.family() {
            TargetFamily::Wasm => ("wasm-ld", LinkerFlavor::WasmLld),
            TargetFamily::MacOs => ("ld64.lld", LinkerFlavor::Darwin),
            _ => ("ld.lld", LinkerFlavor::Lld),
        };

        let linker_path = which::which(linker_name).map_err(|_| CrossBuildError::ToolNotFound {
            tool: linker_name.to_string(),
        })?;

        Ok(crate::provider::LinkerResolution::new(linker_path, flavor)
            .with_note(format!("Using LLD linker: {linker_name}")))
    }
}

/// Mold linker provider.
pub struct MoldLinkerProvider;

impl crate::provider::LinkerProvider for MoldLinkerProvider {
    fn name(&self) -> &'static str {
        "mold"
    }

    fn priority(&self) -> i32 {
        180
    }

    fn can_provide(&self, target: &TargetTriple, _host: &HostInfo) -> bool {
        // Mold supports ELF targets
        matches!(target.family(), TargetFamily::Linux)
            && target.abi != Abi::Msvc
            && target.arch.pointer_width() == 64
    }

    fn resolve(
        &self,
        target: &TargetTriple,
        _host: &HostInfo,
        _request: &BuildRequest,
    ) -> Result<crate::provider::LinkerResolution, CrossBuildError> {
        let linker_path = which::which("mold").map_err(|_| CrossBuildError::ToolNotFound {
            tool: "mold".to_string(),
        })?;

        Ok(crate::provider::LinkerResolution::new(linker_path, LinkerFlavor::Mold)
            .with_note("Using mold linker (fast)"))
    }
}

/// MSVC linker provider.
pub struct MsvcLinkerProvider;

impl crate::provider::LinkerProvider for MsvcLinkerProvider {
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
    ) -> Result<crate::provider::LinkerResolution, CrossBuildError> {
        // Find link.exe
        let linker_path = find_msvc_link(&target.arch)?;

        Ok(crate::provider::LinkerResolution::new(linker_path, LinkerFlavor::Msvc)
            .with_note("Using MSVC link.exe"))
    }
}

/// Zig linker provider.
pub struct ZigLinkerProvider;

impl crate::provider::LinkerProvider for ZigLinkerProvider {
    fn name(&self) -> &'static str {
        "zig"
    }

    fn priority(&self) -> i32 {
        120
    }

    fn can_provide(&self, target: &TargetTriple, _host: &HostInfo) -> bool {
        // Zig can link most targets
        !matches!(target.family(), TargetFamily::Other)
    }

    fn resolve(
        &self,
        target: &TargetTriple,
        _host: &HostInfo,
        _request: &BuildRequest,
    ) -> Result<crate::provider::LinkerResolution, CrossBuildError> {
        let zig_path = which::which("zig").map_err(|_| CrossBuildError::ToolNotFound {
            tool: "zig".to_string(),
        })?;

        let target_arg = target.to_zig_target();

        Ok(crate::provider::LinkerResolution::new(zig_path, LinkerFlavor::Lld)
            .with_args(vec!["cc".to_string(), "-target".to_string(), target_arg.clone()])
            .with_note(format!("Using zig cc as linker for {target_arg}")))
    }
}

/// Finds MSVC link.exe for the target architecture.
fn find_msvc_link(arch: &Architecture) -> Result<PathBuf, CrossBuildError> {
    // Try to find via vswhere or known paths
    let program_files = std::env::var("ProgramFiles")
        .or_else(|_| std::env::var("ProgramFiles(x86)"))
        .map_err(|_| CrossBuildError::ToolNotFound {
            tool: "MSVC".to_string(),
        })?;

    let arch_str = match arch {
        Architecture::X86_64 => "x64",
        Architecture::X86 => "x86",
        Architecture::AArch64 => "arm64",
        Architecture::Arm => "arm",
        _ => return Err(CrossBuildError::ToolNotFound {
            tool: format!("MSVC for {:?}", arch),
        }),
    };

    // Search for link.exe in VS installation
    let vs_paths = [
        format!("{}\\Microsoft Visual Studio\\2022\\Community\\VC\\Tools\\MSVC", program_files),
        format!("{}\\Microsoft Visual Studio\\2022\\Professional\\VC\\Tools\\MSVC", program_files),
        format!("{}\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC", program_files),
        format!("{}\\Microsoft Visual Studio\\2019\\Community\\VC\\Tools\\MSVC", program_files),
    ];

    for base in &vs_paths {
        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten() {
                let link_path = entry.path().join("bin").join("Hostx64").join(arch_str).join("link.exe");
                if link_path.exists() {
                    return Ok(link_path);
                }
            }
        }
    }

    // Fallback: try PATH
    which::which("link.exe").map_err(|_| CrossBuildError::ToolNotFound {
        tool: "link.exe".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TargetTriple;
    use crate::provider::ToolchainProvider;

    #[test]
    fn zig_target_conversion() {
        let targets = [
            ("x86_64-unknown-linux-gnu", "x86_64-linux-gnu"),
            ("aarch64-unknown-linux-musl", "aarch64-linux-musl"),
            ("x86_64-pc-windows-msvc", "x86_64-windows-msvc"),
            ("wasm32-wasi", "wasm32-wasi"),
        ];

        for (input, expected) in targets {
            let target = TargetTriple::parse(input).unwrap();
            assert_eq!(target.to_zig_target(), expected);
        }
    }

    #[test]
    fn rustup_provider_native() {
        let provider = RustupToolchainProvider;
        let host = crate::model::HostInfo::detect().unwrap();
        let target = TargetTriple::parse(&host.host_triple.triple).unwrap();
        assert!(provider.can_provide(&target, &host));
    }
}