use std::path::PathBuf;

use crossbuild_core::{
    error::CrossBuildError,
    model::{Abi, Architecture, BuildRequest, HostInfo, OperatingSystem, TargetFamily, TargetTriple, ToolchainHint},
    provider::{ToolchainProvider, ToolchainResolution},
};

/// Rustup-based toolchain provider.
pub struct RustupToolchainProvider;

impl ToolchainProvider for RustupToolchainProvider {
    fn name(&self) -> &'static str {
        "rustup"
    }

    fn priority(&self) -> i32 {
        100
    }

    fn can_provide(&self, target: &TargetTriple, host: &HostInfo) -> bool {
        if target.triple == host.host_triple.triple {
            return true;
        }
        rustup_target_available(target)
    }

    fn resolve(
        &self,
        target: &TargetTriple,
        host: &HostInfo,
        _request: &BuildRequest,
    ) -> Result<ToolchainResolution, CrossBuildError> {
        let mut resolution = ToolchainResolution::new();

        if target.triple == host.host_triple.triple {
            resolution = resolution
                .with_note("Using host toolchain for native build")
                .with_rustc(which::which("rustc")?)
                .with_cargo(which::which("cargo")?);
        } else {
            let rustup_home = std::env::var("RUSTUP_HOME")
                .map(PathBuf::from)
                .or_else(|_| {
                    std::env::var("HOME")
                        .or_else(|_| std::env::var("USERPROFILE"))
                        .map(|h| PathBuf::from(h).join(".rustup"))
                })
                .unwrap_or_else(|_| PathBuf::from("/rustup"));

            let toolchain = find_rustup_toolchain(&rustup_home.to_string_lossy())?;
            let toolchain_path = rustup_home.join("toolchains").join(&toolchain);

            let rustc_path = toolchain_path.join("bin").join("rustc");
            let cargo_path = toolchain_path.join("bin").join("cargo");

            resolution = resolution
                .with_note(format!("Using rustup toolchain: {toolchain}"))
                .with_rustc(rustc_path)
                .with_cargo(cargo_path)
                .with_env("RUSTUP_TOOLCHAIN", toolchain);
        }

        if target.triple != host.host_triple.triple {
            resolution = resolution
                .with_target_spec(target.triple.clone())
                .with_env("CARGO_BUILD_TARGET", target.triple.clone());
        }

        Ok(resolution)
    }

    fn hint(&self) -> ToolchainHint {
        ToolchainHint::Rustup
    }
}

/// Zig-based toolchain provider.
pub struct ZigToolchainProvider;

impl ToolchainProvider for ZigToolchainProvider {
    fn name(&self) -> &'static str {
        "zig"
    }

    fn priority(&self) -> i32 {
        50
    }

    fn can_provide(&self, target: &TargetTriple, _host: &HostInfo) -> bool {
        !matches!(target.family(), TargetFamily::Other | TargetFamily::BareMetal)
            || target.is_wasm()
    }

    fn resolve(
        &self,
        target: &TargetTriple,
        _host: &HostInfo,
        _request: &BuildRequest,
    ) -> Result<ToolchainResolution, CrossBuildError> {
        let zig_path = which::which("zig").map_err(|_| CrossBuildError::ToolNotFound {
            tool: "zig".to_string(),
        })?;

        let target_arg = to_zig_target(target);

        let mut resolution = ToolchainResolution::new()
            .with_note(format!("Using zig cc for target: {target_arg}"))
            .with_rustc(zig_path.clone())
            .with_cargo(which::which("cargo")?)
            .with_env("CC", format!("zig cc -target {}", target_arg))
            .with_env("CXX", format!("zig c++ -target {}", target_arg))
            .with_env("AR", "zig ar")
            .with_env("CARGO_TARGET_RUNNER", format!("zig cc -target {}", target_arg));

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

impl ToolchainProvider for BuiltinToolchainProvider {
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
        _host: &HostInfo,
        _request: &BuildRequest,
    ) -> Result<ToolchainResolution, CrossBuildError> {
        Ok(ToolchainResolution::new()
            .with_note("Using host toolchain")
            .with_rustc(which::which("rustc")?)
            .with_cargo(which::which("cargo")?))
    }

    fn hint(&self) -> ToolchainHint {
        ToolchainHint::Rustup
    }
}

fn to_zig_target(target: &TargetTriple) -> String {
    let arch = match target.arch {
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
        Architecture::Other(ref s) => s,
    };

    let os = match target.os {
        OperatingSystem::Linux => "linux",
        OperatingSystem::Windows => "windows",
        OperatingSystem::MacOs => "macos",
        OperatingSystem::FreeBSD => "freebsd",
        OperatingSystem::NetBSD => "netbsd",
        OperatingSystem::OpenBSD => "openbsd",
        OperatingSystem::DragonflyBSD => "dragonflybsd",
        OperatingSystem::Solaris => "solaris",
        OperatingSystem::Illumos => "illumos",
        OperatingSystem::Android => "android",
        OperatingSystem::Wasm => "wasi",
        OperatingSystem::Wasi => "wasi",
        OperatingSystem::None => "freestanding",
        OperatingSystem::Uefi => "uefi",
        OperatingSystem::Ios => "ios",
        OperatingSystem::TvOS => "tvos",
        OperatingSystem::WatchOS => "watchos",
        OperatingSystem::Heron => "heron",
        OperatingSystem::Zos => "zos",
        OperatingSystem::Fuchsia => "fuchsia",
        OperatingSystem::Redox => "redox",
        OperatingSystem::Other(ref s) => s,
    };

    let abi = match target.abi {
        Abi::Gnu => "gnu",
        Abi::Musl => "musl",
        Abi::Msvc => "msvc",
        Abi::Android => "android",
        Abi::Wasm32 => "wasi",
        Abi::None => "",
        Abi::Eabi => "eabi",
        Abi::Eabihf => "eabihf",
        Abi::Simulator => "simulator",
        Abi::Uwp => "uwp",
        Abi::Wasm64 => "wasi",
    };

    if abi.is_empty() {
        format!("{}-{}", arch, os)
    } else {
        format!("{}-{}-{}", arch, os, abi)
    }
}

/// Checks if a target is available via rustup.
fn rustup_target_available(target: &TargetTriple) -> bool {
    const RUSTUP_TARGETS: &[&str] = &[
        "x86_64-unknown-linux-gnu",
        "x86_64-unknown-linux-musl",
        "aarch64-unknown-linux-gnu",
        "aarch64-unknown-linux-musl",
        "x86_64-pc-windows-msvc",
        "x86_64-pc-windows-gnu",
        "aarch64-pc-windows-msvc",
        "i686-pc-windows-msvc",
        "i686-pc-windows-gnu",
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
        "wasm32-wasi",
        "wasm32-unknown-unknown",
        "wasm32-unknown-emscripten",
        "x86_64-unknown-freebsd",
        "aarch64-unknown-freebsd",
        "powerpc64le-unknown-linux-gnu",
        "s390x-unknown-linux-gnu",
        "riscv64gc-unknown-linux-gnu",
    ];

    RUSTUP_TARGETS.contains(&target.triple.as_str())
}

/// Finds the default rustup toolchain.
fn find_rustup_toolchain(rustup_home: &str) -> Result<String, CrossBuildError> {
    let toolchains_dir = PathBuf::from(rustup_home).join("toolchains");
    if !toolchains_dir.exists() {
        return Err(CrossBuildError::SysrootNotFound {
            target: "rustup".to_string(),
        });
    }

    let default_file = PathBuf::from(rustup_home).join("settings").join("default-toolchain");
    if default_file.exists() {
        let content = std::fs::read_to_string(&default_file)
            .map_err(|_| CrossBuildError::SysrootNotFound {
                target: "rustup".to_string(),
            })?;
        let toolchain = content.trim().to_string();
        if toolchains_dir.join(&toolchain).exists() {
            return Ok(toolchain);
        }
    }

    for entry in std::fs::read_dir(&toolchains_dir).map_err(|_| CrossBuildError::SysrootNotFound {
        target: "rustup".to_string(),
    })? {
        let entry = entry.map_err(|_| CrossBuildError::SysrootNotFound {
            target: "rustup".to_string(),
        })?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.contains("stable") || name.contains("1.") {
            return Ok(name);
        }
    }

    Err(CrossBuildError::SysrootNotFound {
        target: "rustup".to_string(),
    })
}
