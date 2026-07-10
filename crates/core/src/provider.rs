//! Provider implementations for toolchain, sysroot, and linker resolution.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::{
    model::{Abi, Architecture, BuildRequest, HostInfo, OperatingSystem, TargetFamily, TargetTriple, ToolchainHint},
    error::CrossBuildError,
};

/// A toolchain provider supplies the compiler toolchain for a target.
pub trait ToolchainProvider: Send + Sync {
    /// Returns the unique name of this provider.
    fn name(&self) -> &'static str;

    /// Returns the priority of this provider (higher = preferred).
    fn priority(&self) -> i32 {
        0
    }

    /// Checks if this provider can handle the given target on this host.
    fn can_provide(&self, target: &TargetTriple, host: &HostInfo) -> bool;

    /// Resolves the toolchain for the target, returning environment variables
    /// and configuration needed to use it.
    fn resolve(
        &self,
        target: &TargetTriple,
        host: &HostInfo,
        request: &BuildRequest,
    ) -> Result<ToolchainResolution, CrossBuildError>;

    /// Returns the toolchain hint this provider satisfies.
    fn hint(&self) -> ToolchainHint;
}

/// Resolution result from a toolchain provider.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolchainResolution {
    pub env: BTreeMap<String, String>,
    pub cargo_config: Option<toml::Table>,
    pub notes: Vec<String>,
    pub rustc_path: Option<PathBuf>,
    pub cargo_path: Option<PathBuf>,
    pub target_spec: Option<String>,
    pub rustflags: Vec<String>,
}

impl ToolchainResolution {
    pub fn new() -> Self {
        Self {
            env: BTreeMap::new(),
            cargo_config: None,
            notes: Vec::new(),
            rustc_path: None,
            cargo_path: None,
            target_spec: None,
            rustflags: Vec::new(),
        }
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn with_cargo_config(mut self, config: toml::Table) -> Self {
        self.cargo_config = Some(config);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn with_rustc(mut self, path: PathBuf) -> Self {
        self.rustc_path = Some(path);
        self
    }

    pub fn with_cargo(mut self, path: PathBuf) -> Self {
        self.cargo_path = Some(path);
        self
    }

    pub fn with_target_spec(mut self, spec: String) -> Self {
        self.target_spec = Some(spec);
        self
    }

    pub fn with_rustflags(mut self, flags: Vec<String>) -> Self {
        self.rustflags = flags;
        self
    }
}

impl Default for ToolchainResolution {
    fn default() -> Self {
        Self::new()
    }
}

/// A sysroot provider supplies the target sysroot (libc, libstd, crt objects).
pub trait SysrootProvider: Send + Sync {
    /// Returns the unique name of this provider.
    fn name(&self) -> &'static str;

    /// Returns the priority of this provider (higher = preferred).
    fn priority(&self) -> i32 {
        0
    }

    /// Checks if this provider can handle the given target on this host.
    fn can_provide(&self, target: &TargetTriple, host: &HostInfo) -> bool;

    /// Resolves the sysroot for the target.
    fn resolve(
        &self,
        target: &TargetTriple,
        host: &HostInfo,
        request: &BuildRequest,
    ) -> Result<SysrootResolution, CrossBuildError>;
}

/// Resolution result from a sysroot provider.
#[derive(Debug, Clone, PartialEq)]
pub struct SysrootResolution {
    pub sysroot_path: PathBuf,
    pub env: BTreeMap<String, String>,
    pub cargo_config: Option<toml::Table>,
    pub notes: Vec<String>,
    pub is_builtin: bool,
}

impl SysrootResolution {
    pub fn new(sysroot_path: PathBuf) -> Self {
        Self {
            sysroot_path,
            env: BTreeMap::new(),
            cargo_config: None,
            notes: Vec::new(),
            is_builtin: false,
        }
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn with_cargo_config(mut self, config: toml::Table) -> Self {
        self.cargo_config = Some(config);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn with_builtin(mut self, builtin: bool) -> Self {
        self.is_builtin = builtin;
        self
    }
}

/// A linker provider supplies the appropriate linker for a target.
pub trait LinkerProvider: Send + Sync {
    /// Returns the unique name of this provider.
    fn name(&self) -> &'static str;

    /// Returns the priority of this provider (higher = preferred).
    fn priority(&self) -> i32 {
        0
    }

    /// Checks if this provider can handle the given target on this host.
    fn can_provide(&self, target: &TargetTriple, host: &HostInfo) -> bool;

    /// Resolves the linker for the target.
    fn resolve(
        &self,
        target: &TargetTriple,
        host: &HostInfo,
        request: &BuildRequest,
    ) -> Result<LinkerResolution, CrossBuildError>;
}

/// Resolution result from a linker provider.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkerResolution {
    pub linker_path: PathBuf,
    pub linker_args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cargo_config: Option<toml::Table>,
    pub notes: Vec<String>,
    pub flavor: LinkerFlavor,
}

impl LinkerResolution {
    pub fn new(linker_path: PathBuf, flavor: LinkerFlavor) -> Self {
        Self {
            linker_path,
            linker_args: Vec::new(),
            env: BTreeMap::new(),
            cargo_config: None,
            notes: Vec::new(),
            flavor,
        }
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.linker_args = args;
        self
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn with_cargo_config(mut self, config: toml::Table) -> Self {
        self.cargo_config = Some(config);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}

/// Linker flavor for cargo configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkerFlavor {
    Gnu,
    Msvc,
    Lld,
    Mold,
    WasmLld,
    Darwin,
}

impl LinkerFlavor {
    pub fn cargo_name(&self) -> &str {
        match self {
            LinkerFlavor::Gnu => "gcc",
            LinkerFlavor::Msvc => "msvc",
            LinkerFlavor::Lld => "ld.lld",
            LinkerFlavor::Mold => "mold",
            LinkerFlavor::WasmLld => "wasm-ld",
            LinkerFlavor::Darwin => "ld64",
        }
    }
}

/// Provider action returned to the build plan.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderAction {
    pub provider_name: String,
    pub notes: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cargo_config: Option<toml::Table>,
}

/// Trait for targets that can convert to zig target string.
trait ZigTarget {
    fn to_zig_target(&self) -> String;
}

impl ZigTarget for crate::model::TargetTriple {
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
            Architecture::Other(ref s) => s,
        };

        let os = match self.os {
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

        let abi = match self.abi {
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
}

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
        // Can provide if target is available via rustup
        if target.triple == host.host_triple.triple {
            return true;
        }
        crate::platform::rustup_target_available(target)
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
            // Cross-compilation with rustup target
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

        // Add target specification if not native
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
        // Zig can target most platforms
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

        let target_arg = target.to_zig_target();

        let mut resolution = ToolchainResolution::new()
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
        host: &HostInfo,
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

/// Rustup-based sysroot provider.
pub struct RustupSysrootProvider;

impl SysrootProvider for RustupSysrootProvider {
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
        crate::platform::rustup_target_available(target)
    }

    fn resolve(
        &self,
        target: &TargetTriple,
        host: &HostInfo,
        _request: &BuildRequest,
    ) -> Result<SysrootResolution, CrossBuildError> {
        if target.triple == host.host_triple.triple {
            return Err(CrossBuildError::SysrootNotNeeded);
        }

        let rustup_home = std::env::var("RUSTUP_HOME")
            .map(PathBuf::from)
            .or_else(|_| {
                std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .map(|h| PathBuf::from(h).join(".rustup"))
            })
            .unwrap_or_else(|_| PathBuf::from("/rustup"));

        let toolchain = find_rustup_toolchain(&rustup_home.to_string_lossy())?;
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

        let mut resolution = SysrootResolution::new(sysroot.clone())
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

impl SysrootProvider for ZigSysrootProvider {
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
    ) -> Result<SysrootResolution, CrossBuildError> {
        let zig_path = which::which("zig").map_err(|_| CrossBuildError::ToolNotFound {
            tool: "zig".to_string(),
        })?;

        // Zig doesn't have a separate sysroot - it uses its internal libc
        let sysroot = std::env::temp_dir().join("zig-sysroot").join(&target.triple);

        let resolution = SysrootResolution::new(sysroot)
            .with_note("Using zig's built-in libc/sysroot")
            .with_env("ZIG_SYSROOT", "1");

        Ok(resolution)
    }
}

/// No sysroot needed (wasm, bare metal).
pub struct NoSysrootProvider;

impl SysrootProvider for NoSysrootProvider {
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
    ) -> Result<SysrootResolution, CrossBuildError> {
        if target.triple == host.host_triple.triple {
            return Err(CrossBuildError::SysrootNotNeeded);
        }

        Ok(SysrootResolution::new(PathBuf::new())
            .with_note("No sysroot required for this target"))
    }
}

/// Checks if a target is available via rustup.
fn is_rustup_target_available(target: &TargetTriple) -> bool {
    // Check common rustup targets
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

    // Read the default toolchain
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

    // Fallback: find first stable toolchain
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TargetTriple;

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