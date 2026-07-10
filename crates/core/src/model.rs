use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};


use crate::CrossBuildError;

/// Error parsing a target triple component.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TargetParseError {
    #[error("empty target triple")]
    Empty,
    #[error("target triple contains whitespace")]
    Whitespace,
    #[error("invalid target triple format: expected at least {expected_min} components, got {actual} for `{triple}`")]
    InvalidFormat { triple: String, expected_min: usize, actual: usize },
    #[error("unknown architecture: {0}")]
    UnknownArchitecture(String),
    #[error("unknown vendor: {0}")]
    UnknownVendor(String),
    #[error("unknown operating system: {0}")]
    UnknownOs(String),
    #[error("unknown abi: {0}")]
    UnknownAbi(String),
}

/// CPU architecture.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Architecture {
    X86_64,
    AArch64,
    X86,
    Arm,
    Arm64,
    RiscV64,
    PowerPC64,
    S390x,
    Mips64,
    LoongArch64,
    Wasm32,
    Wasm64,
    Other(String),
}

impl Architecture {
    pub fn parse(s: &str) -> Result<Self, TargetParseError> {
        Ok(match s.to_lowercase().as_str() {
            "x86_64" | "amd64" => Architecture::X86_64,
            "aarch64" | "arm64" => Architecture::AArch64,
            "i686" | "i586" | "i386" | "x86" => Architecture::X86,
            "arm" | "armv7" | "armv7a" | "armv7hf" => Architecture::Arm,
            "riscv64" | "riscv64gc" => Architecture::RiscV64,
            "powerpc64le" | "ppc64le" => Architecture::PowerPC64,
            "s390x" => Architecture::S390x,
            "mips64" | "mips64el" => Architecture::Mips64,
            "loongarch64" => Architecture::LoongArch64,
            "wasm32" => Architecture::Wasm32,
            "wasm64" => Architecture::Wasm64,
            other => Architecture::Other(other.to_string()),
        })
    }

    pub fn pointer_width(&self) -> u8 {
        match self {
            Architecture::X86_64
            | Architecture::AArch64
            | Architecture::Arm64
            | Architecture::RiscV64
            | Architecture::PowerPC64
            | Architecture::S390x
            | Architecture::Mips64
            | Architecture::LoongArch64
            | Architecture::Wasm64 => 64,
            Architecture::X86
            | Architecture::Arm
            | Architecture::Wasm32 => 32,
            Architecture::Other(_) => 64,
        }
    }

    pub fn endianness(&self) -> Endianness {
        match self {
            Architecture::X86_64 | Architecture::X86 | Architecture::Wasm32 | Architecture::Wasm64 => Endianness::Little,
            Architecture::AArch64 | Architecture::Arm | Architecture::Arm64 | Architecture::RiscV64 => Endianness::Little,
            Architecture::PowerPC64 => Endianness::Little,
            Architecture::S390x => Endianness::Big,
            Architecture::Mips64 => Endianness::Big,
            Architecture::LoongArch64 => Endianness::Little,
            Architecture::Other(_) => Endianness::Little,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Architecture::X86_64 => "x86_64",
            Architecture::AArch64 => "aarch64",
            Architecture::X86 => "i686",
            Architecture::Arm => "arm",
            Architecture::Arm64 => "aarch64",
            Architecture::RiscV64 => "riscv64",
            Architecture::PowerPC64 => "powerpc64le",
            Architecture::S390x => "s390x",
            Architecture::Mips64 => "mips64",
            Architecture::LoongArch64 => "loongarch64",
            Architecture::Wasm32 => "wasm32",
            Architecture::Wasm64 => "wasm64",
            Architecture::Other(name) => name,
        }
    }
}

impl Display for Architecture {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Byte order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Endianness {
    Little,
    Big,
}

/// Vendor field in target triple.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Vendor {
    Unknown,
    Pc,
    Apple,
    Linux,
    Uwp,
    Fuchsia,
    PlayStation,
    Nintendo,
    Sony,
    Microsoft,
    Other,
}

impl Vendor {
    pub fn parse(s: &str) -> Result<Self, TargetParseError> {
        Ok(match s.to_lowercase().as_str() {
            "unknown" => Vendor::Unknown,
            "pc" => Vendor::Pc,
            "apple" => Vendor::Apple,
            "linux" => Vendor::Linux,
            "uwp" => Vendor::Uwp,
            "fuchsia" => Vendor::Fuchsia,
            "playstation" => Vendor::PlayStation,
            "nintendo" => Vendor::Nintendo,
            "sony" => Vendor::Sony,
            "microsoft" => Vendor::Microsoft,
            _ => Vendor::Other,
        })
    }

    pub fn name(&self) -> &str {
        match self {
            Vendor::Unknown => "unknown",
            Vendor::Pc => "pc",
            Vendor::Apple => "apple",
            Vendor::Linux => "linux",
            Vendor::Uwp => "uwp",
            Vendor::Fuchsia => "fuchsia",
            Vendor::PlayStation => "playstation",
            Vendor::Nintendo => "nintendo",
            Vendor::Sony => "sony",
            Vendor::Microsoft => "microsoft",
            Vendor::Other => "unknown",
        }
    }
}

impl Display for Vendor {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Operating system in target triple.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OperatingSystem {
    None,
    Linux,
    Windows,
    MacOs,
    FreeBSD,
    NetBSD,
    OpenBSD,
    DragonflyBSD,
    Solaris,
    Illumos,
    Android,
    Ios,
    TvOS,
    WatchOS,
    Wasm,
    Wasi,
    Uefi,
    Redox,
    Heron,
    Fuchsia,
    Zos,
    Other(String),
}

impl OperatingSystem {
    pub fn parse(s: &str) -> Result<Self, TargetParseError> {
        Ok(match s.to_lowercase().as_str() {
            "none" => OperatingSystem::None,
            "linux" => OperatingSystem::Linux,
            "windows" => OperatingSystem::Windows,
            "darwin" | "macos" => OperatingSystem::MacOs,
            "freebsd" => OperatingSystem::FreeBSD,
            "openbsd" => OperatingSystem::OpenBSD,
            "netbsd" => OperatingSystem::NetBSD,
            "dragonflybsd" | "dragonfly" => OperatingSystem::DragonflyBSD,
            "solaris" => OperatingSystem::Solaris,
            "illumos" => OperatingSystem::Illumos,
            "android" => OperatingSystem::Android,
            "ios" => OperatingSystem::Ios,
            "tvos" => OperatingSystem::TvOS,
            "watchos" => OperatingSystem::WatchOS,
            "wasm" => OperatingSystem::Wasm,
            "wasi" => OperatingSystem::Wasi,
            "fuchsia" => OperatingSystem::Fuchsia,
            "redox" => OperatingSystem::Redox,
            "heron" => OperatingSystem::Heron,
            "zos" => OperatingSystem::Zos,
            other => OperatingSystem::Other(other.to_string()),
        })
    }

    pub fn name(&self) -> &str {
        match self {
            OperatingSystem::None => "none",
            OperatingSystem::Linux => "linux",
            OperatingSystem::Windows => "windows",
            OperatingSystem::MacOs => "darwin",
            OperatingSystem::FreeBSD => "freebsd",
            OperatingSystem::OpenBSD => "openbsd",
            OperatingSystem::NetBSD => "netbsd",
            OperatingSystem::DragonflyBSD => "dragonflybsd",
            OperatingSystem::Solaris => "solaris",
            OperatingSystem::Illumos => "illumos",
            OperatingSystem::Android => "android",
            OperatingSystem::Ios => "ios",
            OperatingSystem::TvOS => "tvos",
            OperatingSystem::WatchOS => "watchos",
            OperatingSystem::Wasm => "wasi",
            OperatingSystem::Wasi => "wasi",
            OperatingSystem::Uefi => "uefi",
            OperatingSystem::Redox => "redox",
            OperatingSystem::Heron => "heron",
            OperatingSystem::Fuchsia => "fuchsia",
            OperatingSystem::Zos => "zos",
            OperatingSystem::Other(s) => s,
        }
    }

    pub fn is_unix_like(&self) -> bool {
        matches!(
            self,
            OperatingSystem::Linux
                | OperatingSystem::MacOs
                | OperatingSystem::FreeBSD
                | OperatingSystem::OpenBSD
                | OperatingSystem::NetBSD
                | OperatingSystem::DragonflyBSD
                | OperatingSystem::Solaris
                | OperatingSystem::Illumos
                | OperatingSystem::Android
                | OperatingSystem::Redox
                | OperatingSystem::Fuchsia
        )
    }
}

impl Display for OperatingSystem {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// ABI (Application Binary Interface) specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Abi {
    None,
    Gnu,
    Musl,
    Msvc,
    Uwp,
    Wasm32,
    Wasm64,
    Eabi,
    Eabihf,
    Android,
    Simulator,
}

impl Abi {
    pub fn parse(s: &str) -> Result<Self, TargetParseError> {
        Ok(match s.to_lowercase().as_str() {
            "none" => Abi::None,
            "gnu" => Abi::Gnu,
            "musl" => Abi::Musl,
            "msvc" => Abi::Msvc,
            "uwp" => Abi::Uwp,
            "wasm32" => Abi::Wasm32,
            "wasm64" => Abi::Wasm64,
            "eabi" => Abi::Eabi,
            "eabihf" => Abi::Eabihf,
            "android" => Abi::Android,
            "simulator" => Abi::Simulator,
            other => return Err(TargetParseError::UnknownAbi(other.to_string())),
        })
    }

    pub fn default_for_os(os: &OperatingSystem) -> Self {
        match os {
            OperatingSystem::Linux => Abi::Gnu,
            OperatingSystem::Windows => Abi::Msvc,
            OperatingSystem::MacOs => Abi::None,
            OperatingSystem::Wasm => Abi::Wasm32,
            OperatingSystem::Wasi => Abi::None,
            OperatingSystem::Android => Abi::Android,
            OperatingSystem::FreeBSD
            | OperatingSystem::OpenBSD
            | OperatingSystem::NetBSD
            | OperatingSystem::DragonflyBSD => Abi::Gnu,
            _ => Abi::None,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Abi::None => "none",
            Abi::Gnu => "gnu",
            Abi::Musl => "musl",
            Abi::Msvc => "msvc",
            Abi::Uwp => "uwp",
            Abi::Wasm32 => "wasm32",
            Abi::Wasm64 => "wasm64",
            Abi::Eabi => "eabi",
            Abi::Eabihf => "eabihf",
            Abi::Android => "android",
            Abi::Simulator => "simulator",
        }
    }
}

impl Display for Abi {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl Default for Abi {
    fn default() -> Self {
        Abi::None
    }
}

/// Target family classification for provider routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetFamily {
    Windows,
    Linux,
    MacOs,
    Wasm,
    BareMetal,
    Other,
}

impl TargetFamily {
    fn from_os_abi(os: &OperatingSystem, abi: &Abi) -> Self {
        match (os, abi) {
            (OperatingSystem::Windows, _) => TargetFamily::Windows,
            (OperatingSystem::Linux, _) => TargetFamily::Linux,
            (OperatingSystem::MacOs, _) => TargetFamily::MacOs,
            (OperatingSystem::Wasm, _) | (OperatingSystem::Wasi, _) => TargetFamily::Wasm,
            (OperatingSystem::None, _) => TargetFamily::BareMetal,
            _ => TargetFamily::Other,
        }
    }
}

/// A fully qualified target triple with parsed components.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TargetTriple {
    pub triple: String,
    pub arch: Architecture,
    pub vendor: Vendor,
    pub os: OperatingSystem,
    pub abi: Abi,
    pub family: TargetFamily,
}

impl TargetTriple {
    pub fn parse(triple: &str) -> Result<Self, TargetParseError> {
        let trimmed = triple.trim();
        if trimmed.is_empty() {
            return Err(TargetParseError::Empty);
        }

        if trimmed.chars().any(char::is_whitespace) {
            return Err(TargetParseError::Whitespace);
        }

        let parts: Vec<&str> = trimmed.split('-').collect();
        if parts.len() < 2 {
            return Err(TargetParseError::InvalidFormat {
                triple: trimmed.to_string(),
                expected_min: 2,
                actual: parts.len(),
            });
        }

        let arch = Architecture::parse(parts[0])?;

        let (vendor, os, abi, family) = if parts.len() == 2 {
            // 2-component target: arch-os (e.g. wasm32-wasi, wasm32-unknown)
            let os = OperatingSystem::parse(parts[1])?;
            let abi = Abi::default_for_os(&os);
            let family = TargetFamily::from_os_abi(&os, &abi);
            (Vendor::Unknown, os, abi, family)
        } else if parts.len() == 3 {
            // 3-component target: arch-vendor-os
            let vendor = Vendor::parse(parts[1])?;
            let os = OperatingSystem::parse(parts[2])?;
            let abi = Abi::default_for_os(&os);
            let family = TargetFamily::from_os_abi(&os, &abi);
            (vendor, os, abi, family)
        } else {
            let vendor = Vendor::parse(parts[1])?;
            let os = OperatingSystem::parse(parts[2])?;
            let abi = Abi::parse(parts[3])?;
            let family = TargetFamily::from_os_abi(&os, &abi);
            (vendor, os, abi, family)
        };

        Ok(Self {
            triple: trimmed.to_string(),
            arch,
            vendor,
            os,
            abi,
            family,
        })
    }

    pub fn as_str(&self) -> &str {
        &self.triple
    }

    pub fn is_windows(&self) -> bool {
        matches!(self.family, TargetFamily::Windows)
    }

    pub fn is_linux(&self) -> bool {
        matches!(self.family, TargetFamily::Linux)
    }

    pub fn is_macos(&self) -> bool {
        matches!(self.family, TargetFamily::MacOs)
    }

    pub fn is_wasm(&self) -> bool {
        matches!(self.family, TargetFamily::Wasm)
    }

    pub fn is_bare_metal(&self) -> bool {
        matches!(self.family, TargetFamily::BareMetal)
    }

    pub fn pointer_width(&self) -> u8 {
        self.arch.pointer_width()
    }

    pub fn endianness(&self) -> Endianness {
        self.arch.endianness()
    }
}

impl FromStr for TargetTriple {
    type Err = TargetParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl Display for TargetTriple {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.triple)
    }
}

impl PartialOrd for TargetTriple {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TargetTriple {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.triple.cmp(&other.triple)
    }
}

/// Target family trait for TargetTriple.
impl TargetTriple {
    pub fn family(&self) -> TargetFamily {
        self.family
    }
}

/// Trait for target family classification.
pub trait TargetFamilyExt {
    fn family(&self) -> TargetFamily;
}

impl TargetFamilyExt for TargetTriple {
    fn family(&self) -> TargetFamily {
        self.family
    }
}

/// Known target triples organized by tier.
pub struct KnownTargets;

impl KnownTargets {
    /// Returns all tier 1 target triples.
    pub fn tier1() -> &'static [&'static str] {
        &[
            "x86_64-unknown-linux-gnu",
            "x86_64-pc-windows-msvc",
            "aarch64-unknown-linux-gnu",
            "aarch64-apple-darwin",
            "x86_64-apple-darwin",
        ]
    }

    /// Returns all tier 2 target triples.
    pub fn tier2() -> &'static [&'static str] {
        &[
            "x86_64-unknown-linux-musl",
            "aarch64-unknown-linux-musl",
            "x86_64-unknown-freebsd",
            "aarch64-unknown-freebsd",
            "x86_64-unknown-netbsd",
            "aarch64-unknown-netbsd",
            "x86_64-unknown-openbsd",
            "x86_64-unknown-illumos",
            "powerpc64le-unknown-linux-gnu",
            "s390x-unknown-linux-gnu",
            "riscv64gc-unknown-linux-gnu",
            "x86_64-pc-windows-gnu",
            "i686-pc-windows-msvc",
            "i686-pc-windows-gnu",
            "aarch64-pc-windows-msvc",
            "wasm32-wasi",
            "wasm32-unknown-unknown",
            "wasm32-unknown-emscripten",
        ]
    }

    /// Checks if a target is a known tier 1 target.
    pub fn is_tier1(target: &str) -> bool {
        Self::tier1().contains(&target)
    }

    /// Checks if a target is a known tier 2 target.
    pub fn is_tier2(target: &str) -> bool {
        Self::tier2().contains(&target)
    }

    /// Returns all known targets (tier 1 + tier 2).
    pub fn all_known() -> Vec<&'static str> {
        let mut targets = Vec::new();
        targets.extend_from_slice(Self::tier1());
        targets.extend_from_slice(Self::tier2());
        targets
    }
}

/// Execution mode for a build request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    DryRun,
    Execute,
}

/// Build profile configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Dev,
    Release,
    Custom(&'static str),
}

impl Default for Profile {
    fn default() -> Self {
        Profile::Dev
    }
}

impl Display for Profile {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Profile::Dev => f.write_str("dev"),
            Profile::Release => f.write_str("release"),
            Profile::Custom(name) => f.write_str(name),
        }
    }
}

/// A user request submitted through the CLI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildRequest {
    pub manifest_path: PathBuf,
    pub target_triple: TargetTriple,
    pub cargo_args: Vec<String>,
    pub execution_mode: ExecutionMode,
    pub verbose: bool,
    pub profile: Profile,
    pub features: Vec<String>,
    pub no_default_features: bool,
    pub workspace: bool,
    pub exclude: Vec<String>,
}

impl BuildRequest {
    pub fn new(manifest_path: PathBuf, target_triple: TargetTriple) -> Self {
        Self {
            manifest_path,
            target_triple,
            cargo_args: Vec::new(),
            execution_mode: ExecutionMode::Execute,
            verbose: false,
            profile: Profile::default(),
            features: Vec::new(),
            no_default_features: false,
            workspace: false,
            exclude: Vec::new(),
        }
    }

    pub fn with_cargo_args(mut self, args: Vec<String>) -> Self {
        self.cargo_args = args;
        self
    }

    pub fn with_execution_mode(mut self, mode: ExecutionMode) -> Self {
        self.execution_mode = mode;
        self
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    pub fn with_profile(mut self, profile: Profile) -> Self {
        self.profile = profile;
        self
    }

    pub fn with_features(mut self, features: Vec<String>) -> Self {
        self.features = features;
        self
    }

    pub fn with_no_default_features(mut self, no_default_features: bool) -> Self {
        self.no_default_features = no_default_features;
        self
    }

    pub fn with_workspace(mut self, workspace: bool) -> Self {
        self.workspace = workspace;
        self
    }

    pub fn with_exclude(mut self, exclude: Vec<String>) -> Self {
        self.exclude = exclude;
        self
    }
}

/// Host information derived from the running process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostInfo {
    pub host_triple: TargetTriple,
    pub os: OperatingSystem,
    pub arch: Architecture,
    pub rustc_version: Option<String>,
    pub cargo_version: Option<String>,
    pub target_dir: PathBuf,
}

impl HostInfo {
    pub fn detect() -> Result<Self, HostDetectError> {
        let host_triple_str = Self::detect_host_triple_from_rustc()?;
        let host_triple = TargetTriple::parse(&host_triple_str)
            .map_err(|e| HostDetectError::ParseError(e.to_string()))?;

        let rustc_version = Self::detect_rustc_version();
        let cargo_version = Self::detect_cargo_version();
        let target_dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("target")
            .join("crossbuild");

        Ok(Self {
            os: host_triple.os.clone(),
            arch: host_triple.arch.clone(),
            host_triple,
            rustc_version,
            cargo_version,
            target_dir,
        })
    }

    fn detect_host_triple_from_rustc() -> Result<String, HostDetectError> {
        let output = std::process::Command::new("rustc")
            .arg("-vV")
            .output()
            .map_err(|_| HostDetectError::RustcNotFound)?;

        let stdout = String::from_utf8(output.stdout)
            .map_err(|_| HostDetectError::ParseError("invalid UTF-8".to_string()))?;

        for line in stdout.lines() {
            if line.starts_with("host: ") {
                return Ok(line.strip_prefix("host: ").unwrap().to_string());
            }
        }

        Err(HostDetectError::ParseError("host triple not found in rustc -vV output".to_string()))
    }

    fn detect_rustc_version() -> Option<String> {
        std::process::Command::new("rustc")
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
    }

    fn detect_cargo_version() -> Option<String> {
        std::process::Command::new("cargo")
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
    }
}

/// Errors during host detection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostDetectError {
    ParseError(String),
    RustcNotFound,
    CargoNotFound,
}

impl Display for HostDetectError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            HostDetectError::ParseError(e) => write!(f, "failed to parse host triple: {e}"),
            HostDetectError::RustcNotFound => f.write_str("rustc not found in PATH"),
            HostDetectError::CargoNotFound => f.write_str("cargo not found in PATH"),
        }
    }
}

impl std::error::Error for HostDetectError {}

/// Target information with capability assessment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetInfo {
    pub triple: TargetTriple,
    pub is_native: bool,
    pub requires_cross: bool,
    pub supported: TargetSupport,
    pub toolchain_hint: ToolchainHint,
    pub sysroot_hint: SysrootHint,
    pub linker_hint: LinkerHint,
}

/// Target support level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetSupport {
    /// Tier 1: Guaranteed to build and pass tests
    Tier1,
    /// Tier 2: Guaranteed to build, tests may not run
    Tier2,
    /// Tier 3: No guarantees, community maintained
    Tier3,
    /// Not supported by rustup
    Unsupported,
}

/// Suggested toolchain provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolchainHint {
    Rustup,
    Zig,
    CrossDocker,
    Custom,
}

/// Suggested sysroot provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SysrootHint {
    Rustup,
    Zig,
    Custom,
    None,
}

/// Suggested linker provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkerHint {
    SystemDefault,
    Lld,
    Mold,
    Zig,
    MSVC,
    Custom,
}

/// A resolved command line with explicit environment overrides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLine {
    pub program: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub current_dir: PathBuf,
}

impl CommandLine {
    pub fn new(program: impl Into<String>, current_dir: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            env: BTreeMap::new(),
            current_dir: current_dir.into(),
        }
    }

    pub fn push_arg(&mut self, arg: impl Into<String>) {
        self.args.push(arg.into());
    }

    pub fn extend_args(&mut self, args: impl IntoIterator<Item = impl Into<String>>) {
        self.args.extend(args.into_iter().map(Into::into));
    }

    pub fn set_env(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.env.insert(key.into(), value.into());
    }

    pub fn extend_env(&mut self, env: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>) {
        self.env.extend(env.into_iter().map(|(k, v)| (k.into(), v.into())));
    }
}

impl Display for CommandLine {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.program)?;
        for arg in &self.args {
            write!(f, " ")?;
            if arg.contains(' ') || arg.contains('"') {
                write!(f, "{:?}", arg)?;
            } else {
                write!(f, "{arg}")?;
            }
        }
        Ok(())
    }
}

/// A provider contribution to the final build plan.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderAction {
    pub provider_name: String,
    pub notes: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cargo_config: Option<toml::Table>,
}

/// A snippet of Cargo configuration to merge.
#[derive(Debug, Clone, PartialEq)]
pub struct CargoConfigSnippet {
    pub target_section: Option<String>,
    pub config: toml::Table,
}

/// A single step in the build plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanStep {
    ValidateManifest { path: PathBuf },
    ValidateTarget { target: TargetTriple },
    DetectHost,
    ResolveProviders,
    PrepareEnvironment,
    GenerateCargoConfig,
    ResolveLinker,
    PrepareCache,
    InvokeCargo,
    CaptureDiagnostics,
    VerifyArtifacts,
}

/// A fully resolved build plan.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildPlan {
    pub request: BuildRequest,
    pub host: HostInfo,
    pub target: TargetInfo,
    pub command: CommandLine,
    pub steps: Vec<PlanStep>,
    pub provider_actions: Vec<ProviderAction>,
    pub cargo_config: Option<toml::Table>,
    pub cache_key: String,
}

impl BuildPlan {
    pub fn manifest_directory(&self) -> &Path {
        self.request
            .manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
    }

    pub fn target_triple(&self) -> &TargetTriple {
        &self.request.target_triple
    }

    pub fn is_cross_compilation(&self) -> bool {
        self.target.requires_cross
    }
}

/// Execution report returned by the engine.
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionReport {
    pub plan: BuildPlan,
    pub run: RunReport,
}

/// Report from the runner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunReport {
    pub executed: bool,
    pub command: String,
    pub working_directory: PathBuf,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
}

/// Validation plan for cross-compilation testing.
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

/// Wrapper command and its target invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrapperPlan {
    pub wrapper: String,
    pub target: String,
}

impl WrapperPlan {
    pub fn new(wrapper: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            wrapper: wrapper.into(),
            target: target.into(),
        }
    }

    pub fn invocation(&self) -> String {
        format!("{} {}", self.wrapper, self.target)
    }
}

/// Cache policy for cross-build artifacts and downloaded assets.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CachePolicy {
    pub root: PathBuf,
    pub max_size_bytes: Option<u64>,
    pub max_age: Option<Duration>,
    pub compress: bool,
}

impl Default for CachePolicy {
    fn default() -> Self {
        Self {
            root: PathBuf::from("target").join("crossbuild-cache"),
            max_size_bytes: Some(10 * 1024 * 1024 * 1024), // 10 GB
            max_age: Some(Duration::from_secs(30 * 24 * 60 * 60)), // 30 days
            compress: true,
        }
    }
}

impl CachePolicy {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            ..Default::default()
        }
    }

    pub fn with_max_size(mut self, bytes: u64) -> Self {
        self.max_size_bytes = Some(bytes);
        self
    }

    pub fn with_max_age(mut self, age: Duration) -> Self {
        self.max_age = Some(age);
        self
    }

    pub fn with_compression(mut self, compress: bool) -> Self {
        self.compress = compress;
        self
    }

    pub fn absolute_root(&self, workspace_root: &Path) -> PathBuf {
        if self.root.is_absolute() {
            self.root.clone()
        } else {
            workspace_root.join(&self.root)
        }
    }

    pub fn cache_key(&self, workspace_root: &Path, target: &TargetTriple) -> String {
        let workspace_label = workspace_root
            .to_string_lossy()
            .replace(['\\', '/', ':'], "_");
        format!("{}::{}", workspace_label, target.triple)
    }

    pub fn download_dir(&self, workspace_root: &Path) -> PathBuf {
        self.absolute_root(workspace_root).join("downloads")
    }

    pub fn sysroot_dir(&self, workspace_root: &Path) -> PathBuf {
        self.absolute_root(workspace_root).join("sysroots")
    }

    pub fn toolchain_dir(&self, workspace_root: &Path) -> PathBuf {
        self.absolute_root(workspace_root).join("toolchains")
    }

    pub fn build_dir(&self, workspace_root: &Path, target: &TargetTriple) -> PathBuf {
        self.absolute_root(workspace_root)
            .join("builds")
            .join(&target.triple)
    }

    pub fn metadata_path(&self, workspace_root: &Path) -> PathBuf {
        self.absolute_root(workspace_root).join("metadata.json")
    }
}

/// Cache metadata for tracking entries.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheMetadata {
    pub entries: BTreeMap<String, CacheEntry>,
    pub total_size_bytes: u64,
    pub last_cleanup: u64,
}

impl Default for CacheMetadata {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
            total_size_bytes: 0,
            last_cleanup: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

/// Individual cache entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheEntry {
    pub key: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub created: u64,
    pub last_accessed: u64,
    pub access_count: u64,
    pub entry_type: CacheEntryType,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum CacheEntryType {
    Download,
    Sysroot,
    Toolchain,
    BuildArtifact,
    Other,
}

/// Cache manager for handling all cache operations.
pub struct CacheManager {
    policy: CachePolicy,
    workspace_root: PathBuf,
    metadata: CacheMetadata,
}

impl CacheManager {
    /// Creates a new cache manager.
    pub fn new(policy: CachePolicy, workspace_root: impl AsRef<Path>) -> Result<Self, CrossBuildError> {
        let workspace_root = workspace_root.as_ref().to_path_buf();
        let root = policy.absolute_root(&workspace_root);
        fs::create_dir_all(&root).map_err(|source| CrossBuildError::Io {
            path: Some(root),
            source,
        })?;

        fs::create_dir_all(policy.download_dir(&workspace_root)).map_err(|source| CrossBuildError::Io {
            path: Some(policy.download_dir(&workspace_root)),
            source,
        })?;
        fs::create_dir_all(policy.sysroot_dir(&workspace_root)).map_err(|source| CrossBuildError::Io {
            path: Some(policy.sysroot_dir(&workspace_root)),
            source,
        })?;
        fs::create_dir_all(policy.toolchain_dir(&workspace_root)).map_err(|source| CrossBuildError::Io {
            path: Some(policy.toolchain_dir(&workspace_root)),
            source,
        })?;

        let metadata_path = policy.metadata_path(&workspace_root);
        let metadata = if metadata_path.exists() {
            let content = fs::read_to_string(&metadata_path).map_err(|source| CrossBuildError::Io {
                path: Some(metadata_path),
                source,
            })?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            CacheMetadata::default()
        };

        Ok(Self {
            policy,
            workspace_root,
            metadata,
        })
    }

    /// Gets the path for a cached download.
    pub fn get_download(&self, url: &str, checksum: Option<&str>) -> Option<PathBuf> {
        let key = self.download_key(url, checksum);
        self.metadata.entries.get(&key).and_then(|entry| {
            if entry.path.exists() {
                Some(entry.path.clone())
            } else {
                None
            }
        })
    }

    /// Stores a downloaded file in the cache.
    pub fn store_download(
        &mut self,
        url: &str,
        checksum: Option<&str>,
        source_path: &Path,
    ) -> Result<PathBuf, CrossBuildError> {
        let key = self.download_key(url, checksum);
        let dest = self.policy.download_dir(&self.workspace_root).join(&key);

        fs::copy(source_path, &dest).map_err(|source| CrossBuildError::Io {
            path: Some(dest.clone()),
            source,
        })?;

        let size = fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
        let now = current_timestamp();

        self.metadata.entries.insert(
            key.clone(),
            CacheEntry {
                key: key.clone(),
                path: dest.clone(),
                size_bytes: size,
                created: now,
                last_accessed: now,
                access_count: 1,
                entry_type: CacheEntryType::Download,
                metadata: {
                    let mut m = BTreeMap::new();
                    m.insert("url".to_string(), url.to_string());
                    if let Some(cs) = checksum {
                        m.insert("checksum".to_string(), cs.to_string());
                    }
                    m
                },
            },
        );
        self.metadata.total_size_bytes += size;
        self.save_metadata()?;

        Ok(dest)
    }

    /// Gets or creates a sysroot cache entry.
    pub fn get_sysroot(&self, target: &TargetTriple, provider: &str) -> Option<PathBuf> {
        let key = self.sysroot_key(target, provider);
        self.metadata.entries.get(&key).and_then(|entry| {
            if entry.path.exists() {
                Some(entry.path.clone())
            } else {
                None
            }
        })
    }

    /// Stores a sysroot in the cache.
    pub fn store_sysroot(
        &mut self,
        target: &TargetTriple,
        provider: &str,
        source_path: &Path,
    ) -> Result<PathBuf, CrossBuildError> {
        let key = self.sysroot_key(target, provider);
        let dest = self.policy.sysroot_dir(&self.workspace_root).join(&key);

        if source_path.is_dir() {
            copy_dir(source_path, &dest)?;
        } else {
            fs::copy(source_path, &dest).map_err(|source| CrossBuildError::Io {
                path: Some(dest.clone()),
                source,
            })?;
        }

        let size = dir_size(&dest).unwrap_or(0);
        let now = current_timestamp();

        self.metadata.entries.insert(
            key.clone(),
            CacheEntry {
                key: key.clone(),
                path: dest.clone(),
                size_bytes: size,
                created: now,
                last_accessed: now,
                access_count: 1,
                entry_type: CacheEntryType::Sysroot,
                metadata: {
                    let mut m = BTreeMap::new();
                    m.insert("target".to_string(), target.triple.clone());
                    m.insert("provider".to_string(), provider.to_string());
                    m
                },
            },
        );
        self.metadata.total_size_bytes += size;
        self.save_metadata()?;

        Ok(dest)
    }

    /// Gets the build directory for a target.
    pub fn build_dir(&self, target: &TargetTriple) -> PathBuf {
        self.policy.build_dir(&self.workspace_root, target)
    }

    /// Cleans up old or excess cache entries.
    pub fn cleanup(&mut self) -> Result<CleanupReport, CrossBuildError> {
        let mut report = CleanupReport::default();
        let now = current_timestamp();

        // Remove expired entries
        if let Some(max_age) = self.policy.max_age {
            let cutoff = now - max_age.as_secs();
            let expired: Vec<_> = self.metadata.entries
                .iter()
                .filter(|(_, entry)| entry.last_accessed < cutoff)
                .map(|(k, _)| k.clone())
                .collect();

            for key in expired {
                if let Some(entry) = self.metadata.entries.remove(&key) {
                    if entry.path.exists() {
                        remove_entry(&entry.path)?;
                    }
                    report.removed_entries += 1;
                    report.freed_bytes += entry.size_bytes;
                    self.metadata.total_size_bytes = self.metadata.total_size_bytes.saturating_sub(entry.size_bytes);
                }
            }
        }

        // Enforce size limit
        if let Some(max_size) = self.policy.max_size_bytes {
            if self.metadata.total_size_bytes > max_size {
                // Sort by last accessed (LRU)
                let mut entries: Vec<_> = self.metadata.entries
                    .iter()
                    .map(|(k, v)| (k.clone(), v.last_accessed, v.size_bytes, v.path.clone()))
                    .collect();
                entries.sort_by_key(|(_, last_accessed, _, _)| *last_accessed);

                for (key, _, _size, path) in entries {
                    if self.metadata.total_size_bytes <= max_size {
                        break;
                    }
                    if let Some(entry) = self.metadata.entries.remove(&key) {
                        if path.exists() {
                            remove_entry(&path)?;
                        }
                        report.removed_entries += 1;
                        report.freed_bytes += entry.size_bytes;
                        self.metadata.total_size_bytes = self.metadata.total_size_bytes.saturating_sub(entry.size_bytes);
                    }
                }
            }
        }

        self.metadata.last_cleanup = now;
        self.save_metadata()?;

        Ok(report)
    }

    fn download_key(&self, url: &str, checksum: Option<&str>) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        if let Some(cs) = checksum {
            hasher.update(cs.as_bytes());
        }
        hex::encode(hasher.finalize())[..16].to_string()
    }

    fn sysroot_key(&self, target: &TargetTriple, provider: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(target.triple.as_bytes());
        hasher.update(provider.as_bytes());
        format!("sysroot-{}", &hex::encode(hasher.finalize())[..16])
    }

    /// Returns a reference to the cache policy.
    pub fn policy(&self) -> &CachePolicy {
        &self.policy
    }

    /// Returns current cache statistics.
    pub fn stats(&self) -> CacheStats {
        let mut by_type = BTreeMap::new();
        for entry in self.metadata.entries.values() {
            *by_type.entry(entry.entry_type).or_insert(0) += 1;
        }
        CacheStats {
            total_entries: self.metadata.entries.len(),
            total_size_bytes: self.metadata.total_size_bytes,
            by_type,
            root: self.policy.absolute_root(&self.workspace_root),
        }
    }

    fn save_metadata(&self) -> Result<(), CrossBuildError> {
        let path = self.policy.metadata_path(&self.workspace_root);
        let content = serde_json::to_string_pretty(&self.metadata)
            .map_err(|e| CrossBuildError::configuration(e.to_string()))?;
        fs::write(&path, content).map_err(|source| CrossBuildError::Io {
            path: Some(path),
            source,
        })
    }
}

/// Cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_size_bytes: u64,
    pub by_type: BTreeMap<CacheEntryType, usize>,
    pub root: PathBuf,
}

/// Cleanup report.
#[derive(Debug, Default, Clone)]
pub struct CleanupReport {
    pub removed_entries: usize,
    pub freed_bytes: u64,
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn copy_dir(src: &Path, dest: &Path) -> Result<(), CrossBuildError> {
    fs::create_dir_all(dest).map_err(|source| CrossBuildError::Io {
        path: Some(dest.to_path_buf()),
        source,
    })?;

    for entry in fs::read_dir(src).map_err(|source| CrossBuildError::Io {
        path: Some(src.to_path_buf()),
        source,
    })? {
        let entry = entry.map_err(|source| CrossBuildError::Io {
            path: Some(src.to_path_buf()),
            source,
        })?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir(&src_path, &dest_path)?;
        } else {
            fs::copy(&src_path, &dest_path).map_err(|source| CrossBuildError::Io {
                path: Some(dest_path),
                source,
            })?;
        }
    }
    Ok(())
}

fn dir_size(path: &Path) -> Result<u64, CrossBuildError> {
    let mut size = 0;
    for entry in fs::read_dir(path).map_err(|source| CrossBuildError::Io {
        path: Some(path.to_path_buf()),
        source,
    })? {
        let entry = entry.map_err(|source| CrossBuildError::Io {
            path: Some(path.to_path_buf()),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() {
            size += dir_size(&path)?;
        } else {
            size += fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        }
    }
    Ok(size)
}

fn remove_entry(path: &Path) -> Result<(), CrossBuildError> {
    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|source| CrossBuildError::Io {
            path: Some(path.to_path_buf()),
            source,
        })
    } else {
        fs::remove_file(path).map_err(|source| CrossBuildError::Io {
            path: Some(path.to_path_buf()),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TargetTriple;
    use tempfile::tempdir;

    #[test]
    fn cache_policy_default() {
        let policy = CachePolicy::default();
        assert_eq!(policy.root, PathBuf::from("target").join("crossbuild-cache"));
        assert_eq!(policy.max_size_bytes, Some(10 * 1024 * 1024 * 1024));
    }

    #[test]
    fn cache_key_generation() {
        let policy = CachePolicy::default();
        let workspace = PathBuf::from("/home/user/project");
        let target = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();

        let key = policy.cache_key(&workspace, &target);
        assert!(key.contains("home_user_project"));
        assert!(key.contains("x86_64-unknown-linux-gnu"));
    }

    #[test]
    fn cache_manager_creation() {
        let dir = tempdir().unwrap();
        let policy = CachePolicy::new(dir.path().join("cache"));
        let manager = CacheManager::new(policy, dir.path()).unwrap();

        let stats = manager.stats();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.total_size_bytes, 0);
    }

    #[test]
    fn download_caching() {
        let dir = tempdir().unwrap();
        let policy = CachePolicy::new(dir.path().join("cache"));
        let mut manager = CacheManager::new(policy, dir.path()).unwrap();

        // Create a test file
        let source = dir.path().join("test-download");
        fs::write(&source, b"test content").unwrap();

        // Store in cache
        let cached = manager.store_download(
            "https://example.com/file",
            Some("sha256:abc123"),
            &source,
        ).unwrap();

        assert!(cached.exists());

        // Retrieve from cache
        let retrieved = manager.get_download("https://example.com/file", Some("sha256:abc123"));
        assert_eq!(retrieved, Some(cached));

        // Different checksum should not match
        let retrieved2 = manager.get_download("https://example.com/file", Some("sha256:different"));
        assert_eq!(retrieved2, None);
    }

    #[test]
    fn sysroot_caching() {
        let dir = tempdir().unwrap();
        let policy = CachePolicy::new(dir.path().join("cache"));
        let mut manager = CacheManager::new(policy, dir.path()).unwrap();

        let target = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();

        // Create a test sysroot
        let sysroot = dir.path().join("sysroot");
        fs::create_dir_all(sysroot.join("lib")).unwrap();
        fs::write(sysroot.join("lib").join("libc.so"), b"fake").unwrap();

        // Store in cache
        let cached = manager.store_sysroot(&target, "rustup", &sysroot).unwrap();

        assert!(cached.exists());
        assert!(cached.join("lib").join("libc.so").exists());

        // Retrieve from cache
        let retrieved = manager.get_sysroot(&target, "rustup");
        assert_eq!(retrieved, Some(cached));
    }

    #[test]
    fn cleanup_removes_old_entries() {
        let dir = tempdir().unwrap();
        let policy = CachePolicy::new(dir.path().join("cache"))
            .with_max_age(Duration::from_secs(60)); // 1 minute
        let mut manager = CacheManager::new(policy, dir.path()).unwrap();

        let target = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();

        // Create entries with old timestamps
        let sysroot = dir.path().join("sysroot");
        fs::create_dir_all(sysroot.join("lib")).unwrap();
        fs::write(sysroot.join("lib").join("libc.so"), b"fake").unwrap();

        let cached = manager.store_sysroot(&target, "rustup", &sysroot).unwrap();

        // Manually set old timestamp
        if let Some(entry) = manager.metadata.entries.get_mut(&manager.sysroot_key(&target, "rustup")) {
            entry.last_accessed = current_timestamp() - 120; // 2 minutes ago
        }
        manager.save_metadata().unwrap();

        // Run cleanup
        let report = manager.cleanup().unwrap();
        assert_eq!(report.removed_entries, 1);
        assert!(!cached.exists());
    }
}