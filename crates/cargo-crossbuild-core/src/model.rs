//! Core data models for `cargo-crossbuild`.

use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Error during target triple parsing.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TargetParseError {
    #[error("target triple cannot be empty")]
    Empty,
    #[error("target triple must not contain whitespace")]
    Whitespace,
    #[error("target triple '{triple}' has invalid format: expected at least {expected_min} components, got {actual}")]
    InvalidFormat {
        triple: String,
        expected_min: usize,
        actual: usize,
    },
    #[error("unknown architecture: {0}")]
    UnknownArchitecture(String),
    #[error("unknown vendor: {0}")]
    UnknownVendor(String),
    #[error("unknown operating system: {0}")]
    UnknownOperatingSystem(String),
    #[error("unknown ABI: {0}")]
    UnknownAbi(String),
}

/// CPU architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    Other,
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
            _ => Architecture::Other,
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
            Architecture::Other => 64,
        }
    }

    pub fn endianness(&self) -> Endianness {
        match self {
            Architecture::Mips64 => Endianness::Big,
            Architecture::S390x => Endianness::Big,
            Architecture::PowerPC64 => Endianness::Big,
            _ => Endianness::Little,
        }
    }

    pub fn canonical_name(&self) -> &str {
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
            Architecture::Other => "unknown",
        }
    }
}

impl Display for Architecture {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.canonical_name())
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
    Fuschia,
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
            "fuschia" | "fuchsia" => Vendor::Fuschia,
            "playstation" => Vendor::PlayStation,
            "nintendo" => Vendor::Nintendo,
            "sony" => Vendor::Sony,
            "microsoft" => Vendor::Microsoft,
            _ => Vendor::Other,
        })
    }

    pub fn canonical_name(&self) -> &str {
        match self {
            Vendor::Unknown => "unknown",
            Vendor::Pc => "pc",
            Vendor::Apple => "apple",
            Vendor::Linux => "linux",
            Vendor::Uwp => "uwp",
            Vendor::Fuschia => "fuchsia",
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
        f.write_str(self.canonical_name())
    }
}

/// Operating system in target triple.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperatingSystem {
    None,
    Linux,
    Windows,
    MacOs,
    FreeBsd,
    NetBsd,
    OpenBsd,
    DragonflyBsd,
    Solaris,
    Illumos,
    Android,
    Ios,
    TvOs,
    WatchOs,
    Wasm,
    Wasi,
    Uefi,
    Redox,
    Heron,
    Fuchsia,
    Zos,
    Other,
}

impl OperatingSystem {
    pub fn parse(s: &str) -> Result<Self, TargetParseError> {
        Ok(match s.to_lowercase().as_str() {
            "none" => OperatingSystem::None,
            "linux" => OperatingSystem::Linux,
            "windows" => OperatingSystem::Windows,
            "darwin" | "macos" => OperatingSystem::MacOs,
            "freebsd" => OperatingSystem::FreeBsd,
            "netbsd" => OperatingSystem::NetBsd,
            "openbsd" => OperatingSystem::OpenBsd,
            "dragonflybsd" | "dragonfly" => OperatingSystem::DragonflyBsd,
            "solaris" => OperatingSystem::Solaris,
            "illumos" => OperatingSystem::Illumos,
            "android" => OperatingSystem::Android,
            "ios" => OperatingSystem::Ios,
            "tvos" => OperatingSystem::TvOs,
            "watchos" => OperatingSystem::WatchOs,
            "wasm" | "wasi" => OperatingSystem::Wasm,
            "fuchsia" => OperatingSystem::Fuchsia,
            "redox" => OperatingSystem::Redox,
            "uefi" => OperatingSystem::Uefi,
            _ => OperatingSystem::Other,
        })
    }

    pub fn canonical_name(&self) -> &str {
        match self {
            OperatingSystem::None => "none",
            OperatingSystem::Linux => "linux",
            OperatingSystem::Windows => "windows",
            OperatingSystem::MacOs => "darwin",
            OperatingSystem::FreeBsd => "freebsd",
            OperatingSystem::NetBsd => "netbsd",
            OperatingSystem::OpenBsd => "openbsd",
            OperatingSystem::DragonflyBsd => "dragonflybsd",
            OperatingSystem::Solaris => "solaris",
            OperatingSystem::Illumos => "illumos",
            OperatingSystem::Android => "android",
            OperatingSystem::Ios => "ios",
            OperatingSystem::TvOs => "tvos",
            OperatingSystem::WatchOs => "watchos",
            OperatingSystem::Wasm => "wasi",
            OperatingSystem::Wasi => "wasi",
            OperatingSystem::Uefi => "uefi",
            OperatingSystem::Redox => "redox",
            OperatingSystem::Heron => "heron",
            OperatingSystem::Fuchsia => "fuchsia",
            OperatingSystem::Zos => "zos",
            OperatingSystem::Other => "unknown",
        }
    }

    pub fn is_unix_like(&self) -> bool {
        matches!(
            self,
            OperatingSystem::Linux
                | OperatingSystem::MacOs
                | OperatingSystem::FreeBsd
                | OperatingSystem::OpenBsd
                | OperatingSystem::NetBsd
                | OperatingSystem::DragonflyBsd
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
        f.write_str(self.canonical_name())
    }
}

/// ABI (Application Binary Interface) specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Abi {
    None,
    Gnu,
    Musl,
    Msvc,
    Android,
    Eabi,
    Eabihf,
    Wasm32,
    Wasm64,
}

impl Abi {
    pub fn parse(s: &str) -> Result<Self, TargetParseError> {
        Ok(match s.to_lowercase().as_str() {
            "none" => Abi::None,
            "gnu" => Abi::Gnu,
            "musl" => Abi::Musl,
            "msvc" => Abi::Msvc,
            "android" => Abi::Android,
            "eabi" => Abi::Eabi,
            "eabihf" => Abi::Eabihf,
            "wasm32" => Abi::Wasm32,
            "wasm64" => Abi::Wasm64,
            other => return Err(TargetParseError::UnknownAbi(other.to_string())),
        })
    }

    pub fn default_for_os(os: &OperatingSystem) -> Self {
        match os {
            OperatingSystem::Linux => Abi::Gnu,
            OperatingSystem::Windows => Abi::Msvc,
            OperatingSystem::MacOs => Abi::None,
            OperatingSystem::FreeBsd
            | OperatingSystem::NetBsd
            | OperatingSystem::OpenBsd
            | OperatingSystem::DragonflyBsd => Abi::Gnu,
            OperatingSystem::Android => Abi::Android,
            OperatingSystem::Ios
            | OperatingSystem::TvOs
            | OperatingSystem::WatchOs => Abi::None,
            OperatingSystem::Wasm | OperatingSystem::Wasi => Abi::Wasm32,
            OperatingSystem::Fuchsia => Abi::None,
            OperatingSystem::Redox => Abi::None,
            OperatingSystem::Uefi => Abi::None,
            OperatingSystem::None => Abi::None,
            OperatingSystem::Other => Abi::None,
            OperatingSystem::Illumos => Abi::Gnu,
            OperatingSystem::Solaris => Abi::Gnu,
            OperatingSystem::Heron => Abi::None,
            OperatingSystem::Zos => Abi::None,
        }
    }

    pub fn canonical_name(&self) -> &str {
        match self {
            Abi::None => "none",
            Abi::Gnu => "gnu",
            Abi::Musl => "musl",
            Abi::Msvc => "msvc",
            Abi::Android => "android",
            Abi::Eabi => "eabi",
            Abi::Eabihf => "eabihf",
            Abi::Wasm32 => "wasm32",
            Abi::Wasm64 => "wasm64",
        }
    }
}

impl Display for Abi {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.canonical_name())
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
    pub fn from_os_abi(os: &OperatingSystem, abi: &Abi) -> Self {
        match (os, abi) {
            (OperatingSystem::Windows, _) => TargetFamily::Windows,
            (OperatingSystem::Linux, _) => TargetFamily::Linux,
            (OperatingSystem::MacOs, _) => TargetFamily::MacOs,
            (OperatingSystem::Wasm, _) => TargetFamily::Wasm,
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
        
        // Handle special case: wasm32-wasi (arch-os, 2 parts)
        let is_wasm_wasi = parts.len() == 2 && parts[0].starts_with("wasm") && parts[1] == "wasi";
        
        if parts.len() < 3 && !is_wasm_wasi {
            return Err(TargetParseError::InvalidFormat {
                triple: trimmed.to_string(),
                expected_min: 3,
                actual: parts.len(),
            });
        }

        let arch = Architecture::parse(parts[0])?;
        let vendor = if is_wasm_wasi {
            Vendor::Unknown // wasm32-wasi has no vendor
        } else {
            Vendor::parse(parts[1])?
        };
        let os = if is_wasm_wasi {
            OperatingSystem::parse("wasi")?
        } else {
            OperatingSystem::parse(parts[2])?
        };

        let (abi, family) = if parts.len() >= 4 && !is_wasm_wasi {
            let abi = Abi::parse(parts[3])?;
            let family = TargetFamily::from_os_abi(&os, &abi);
            (abi, family)
        } else {
            let abi = Abi::default_for_os(&os);
            let family = TargetFamily::from_os_abi(&os, &abi);
            (abi, family)
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

    pub fn family(&self) -> TargetFamily {
        self.family
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
        // Get host triple from rustc -vV
        let host_triple_str = Self::detect_host_triple_from_rustc()?;
        let host_triple = TargetTriple::parse(&host_triple_str)
            .map_err(|e| HostDetectError::ParseError(e.to_string()))?;

        let rustc_version = Self::detect_rustc_version();
        let cargo_version = Self::detect_cargo_version();
        let target_dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("target")
            .join("crossbuild");

        // Clone the fields we need from host_triple before moving it
        let os = host_triple.os;
        let arch = host_triple.arch;

        Ok(Self {
            host_triple,
            os,
            arch,
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
    Msvc,
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
    pub cargo_config: Option<CargoConfigSnippet>,
}

/// A snippet of Cargo configuration to merge.
#[derive(Debug, Clone, PartialEq)]
pub struct CargoConfigSnippet {
    pub target_section: Option<String>,
    pub config: toml::Table,
}

/// A single step in the build plan.
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
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

/// Release orchestration metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleasePlan {
    pub version: String,
}

impl ReleasePlan {
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            version: version.into(),
        }
    }

    pub fn is_prerelease(&self) -> bool {
        self.version.contains('-')
    }

    pub fn tag_name(&self) -> String {
        format!("v{}", self.version)
    }
}

/// Package manager operation plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageManagerPlan {
    pub command: String,
}

impl PackageManagerPlan {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
        }
    }

    pub fn command_name(&self) -> &str {
        &self.command
    }
}

/// Lockfile snapshot for reproducible builds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockfileSnapshot {
    pub target_triple: String,
    pub manifest_path: String,
}

impl LockfileSnapshot {
    pub fn new(target_triple: impl Into<String>, manifest_path: impl Into<String>) -> Self {
        Self {
            target_triple: target_triple.into(),
            manifest_path: manifest_path.into(),
        }
    }

    pub fn cache_key(&self) -> String {
        format!("{}::{}", self.target_triple, self.manifest_path)
    }
}

/// Sysroot configuration for a target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SysrootConfig {
    pub target: TargetTriple,
    pub path: PathBuf,
    pub lib_dir: PathBuf,
    pub include_dir: Option<PathBuf>,
    pub is_builtin: bool,
}

impl SysrootConfig {
    pub fn new(target: TargetTriple, path: PathBuf) -> Self {
        let lib_dir = path.join("lib");
        let include_dir = path.join("include");

        Self {
            target,
            path,
            lib_dir: lib_dir.clone(),
            include_dir: if include_dir.exists() { Some(include_dir) } else { None },
            is_builtin: false,
        }
    }

    pub fn with_builtin(mut self, builtin: bool) -> Self {
        self.is_builtin = builtin;
        self
    }

    pub fn linker_search_paths(&self) -> Vec<PathBuf> {
        vec![self.lib_dir.clone()]
    }

    pub fn include_paths(&self) -> Vec<PathBuf> {
        self.include_dir.iter().cloned().collect()
    }
}

/// Linker configuration for a target.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkerConfig {
    pub target: TargetTriple,
    pub linker_path: PathBuf,
    pub flavor: LinkerFlavor,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cargo_config: Option<toml::Table>,
}

impl LinkerConfig {
    pub fn new(target: TargetTriple, linker_path: PathBuf, flavor: LinkerFlavor) -> Self {
        Self {
            target,
            linker_path,
            flavor,
            args: Vec::new(),
            env: BTreeMap::new(),
            cargo_config: None,
        }
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
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

    /// Generates the cargo config snippet for this linker.
    pub fn cargo_config_snippet(&self) -> toml::Table {
        let mut table = toml::Table::new();
        let target_key = format!("target.{}", self.target.as_str());
        let mut target_table = toml::Table::new();
        target_table.insert("linker".to_string(), toml::Value::String(self.linker_path.to_string_lossy().into_owned()));
        
        if !self.args.is_empty() {
            target_table.insert("linker-flavor".to_string(), toml::Value::String(self.flavor.cargo_name().to_string()));
            target_table.insert("linker-args".to_string(), toml::Value::Array(
                self.args.iter().map(|a| toml::Value::String(a.clone())).collect()
            ));
        } else {
            target_table.insert("linker-flavor".to_string(), toml::Value::String(self.flavor.cargo_name().to_string()));
        }

        table.insert(target_key, toml::Value::Table(target_table));
        table
    }
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

/// Toolchain configuration for a target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolchainConfig {
    pub target: TargetTriple,
    pub rustc_path: Option<PathBuf>,
    pub cargo_path: Option<PathBuf>,
    pub target_spec: Option<String>,
    pub rustflags: Vec<String>,
    pub env: BTreeMap<String, String>,
}

impl ToolchainConfig {
    pub fn new(target: TargetTriple) -> Self {
        Self {
            target,
            rustc_path: None,
            cargo_path: None,
            target_spec: None,
            rustflags: Vec::new(),
            env: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_triples() {
        let targets = [
            "x86_64-unknown-linux-gnu",
            "aarch64-unknown-linux-musl",
            "x86_64-pc-windows-msvc",
            "aarch64-apple-darwin",
            "wasm32-wasi",
            "riscv64gc-unknown-linux-gnu",
        ];
        for t in targets {
            TargetTriple::parse(t).unwrap_or_else(|e| panic!("failed to parse {t}: {e}"));
        }
    }

    #[test]
    fn parses_architecture_correctly() {
        let t = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();
        assert_eq!(t.arch, Architecture::X86_64);
        assert_eq!(t.pointer_width(), 64);
        assert_eq!(t.endianness(), Endianness::Little);

        let t = TargetTriple::parse("aarch64-unknown-linux-gnu").unwrap();
        assert_eq!(t.arch, Architecture::AArch64);
        assert_eq!(t.pointer_width(), 64);

        let t = TargetTriple::parse("i686-unknown-linux-gnu").unwrap();
        assert_eq!(t.arch, Architecture::X86);
        assert_eq!(t.pointer_width(), 32);
    }

    #[test]
    fn identifies_target_families() {
        assert!(TargetTriple::parse("x86_64-pc-windows-msvc").unwrap().is_windows());
        assert!(TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap().is_linux());
        assert!(TargetTriple::parse("aarch64-apple-darwin").unwrap().is_macos());
        assert!(TargetTriple::parse("wasm32-wasi").unwrap().is_wasm());
    }

    #[test]
    fn rejects_invalid_triples() {
        assert!(TargetTriple::parse("").is_err());
        assert!(TargetTriple::parse("invalid").is_err());
        assert!(TargetTriple::parse("x86_64").is_err());
        assert!(TargetTriple::parse("x86_64-unknown").is_err());
    }

    #[test]
    fn target_triple_display() {
        let t = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();
        assert_eq!(t.to_string(), "x86_64-unknown-linux-gnu");
    }

    #[test]
    fn abi_defaults() {
        assert_eq!(Abi::default_for_os(&OperatingSystem::Linux), Abi::Gnu);
        assert_eq!(Abi::default_for_os(&OperatingSystem::Windows), Abi::Msvc);
        assert_eq!(Abi::default_for_os(&OperatingSystem::MacOs), Abi::None);
        assert_eq!(Abi::default_for_os(&OperatingSystem::Wasm), Abi::Wasm32);
    }
}