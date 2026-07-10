use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use crossbuild_core::{
    cache::CachePolicy,
    config::CrossBuildConfig,
    diagnostics::StderrDiagnosticSink,
    model::{BuildRequest, ExecutionMode, Profile, TargetTriple},
    platform::{detect_host, KnownTargets},
};
use crossbuild_engine::CrossBuildEngine;

#[derive(Parser)]
#[command(
    name = "cargo-crossbuild",
    about = "Cross-compilation tool for Rust projects",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser)]
enum Command {
    /// Build a Rust project for a specific target
    Build(BuildArgs),
    /// Run diagnostics to check system setup
    Doctor,
    /// Clean build artifacts and cache
    Clean,
    /// List known target triples
    ListTargets,
}

#[derive(Parser)]
struct BuildArgs {
    /// Target triple to build for
    #[arg(short = 't', long = "target", required = true)]
    target: String,

    /// Path to Cargo.toml
    #[arg(short = 'm', long = "manifest-path", default_value = "Cargo.toml")]
    manifest_path: String,

    /// Plan only, don't execute
    #[arg(long = "dry-run")]
    dry_run: bool,

    /// Build in release mode
    #[arg(long = "release")]
    release: bool,

    /// Comma-separated list of features to activate
    #[arg(long = "features")]
    features: Option<String>,

    /// Do not activate the default features
    #[arg(long = "no-default-features")]
    no_default_features: bool,

    /// Build all packages in the workspace
    #[arg(long = "workspace")]
    workspace: bool,

    /// Enable verbose output
    #[arg(short = 'v', long = "verbose")]
    verbose: bool,

    /// Build with the given profile
    #[arg(long = "profile")]
    profile: Option<String>,

    /// Packages to exclude from the build (for workspace builds)
    #[arg(long = "exclude")]
    exclude: Vec<String>,

    /// Extra arguments passed through to cargo
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    cargo_args: Vec<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Build(args) => cmd_build(args),
        Command::Doctor => cmd_doctor(),
        Command::Clean => cmd_clean(),
        Command::ListTargets => cmd_list_targets(),
    }
}

fn cmd_build(args: BuildArgs) -> Result<()> {
    let target = TargetTriple::parse(&args.target)
        .with_context(|| format!("invalid target triple: '{}'", args.target))?;

    let config = CrossBuildConfig::from_environment()
        .context("failed to load crossbuild configuration from environment")?;

    let profile = if args.release {
        Profile::Release
    } else if let Some(ref name) = args.profile {
        Profile::Custom(Box::leak(name.clone().into_boxed_str()))
    } else {
        Profile::Dev
    };

    let features: Vec<String> = args
        .features
        .as_ref()
        .map(|f| f.split(',').map(String::from).collect())
        .unwrap_or_default();

    let execution_mode = if args.dry_run {
        ExecutionMode::DryRun
    } else {
        ExecutionMode::Execute
    };

    let request = BuildRequest::new(PathBuf::from(&args.manifest_path), target)
        .with_execution_mode(execution_mode)
        .with_verbose(args.verbose)
        .with_profile(profile)
        .with_features(features)
        .with_no_default_features(args.no_default_features)
        .with_workspace(args.workspace)
        .with_exclude(args.exclude)
        .with_cargo_args(args.cargo_args);

    let engine = CrossBuildEngine::new();
    let mut sink = StderrDiagnosticSink::new(args.verbose);

    if args.dry_run {
        let plan = engine
            .dry_run(request, &config, &mut sink)
            .context("dry run failed")?;

        println!("Dry-run plan for target '{}':", plan.target.triple);
        println!("  Command: {}", plan.command);
        println!("  Steps:");
        for step in &plan.steps {
            println!("    - {step:?}");
        }
        println!("  Cache key: {}", plan.cache_key);
        if let Some(ref config_toml) = plan.cargo_config {
            println!("  Cargo config:");
            println!(
                "{}",
                toml::to_string_pretty(config_toml)
                    .expect("config serialization should always succeed")
            );
        }
    } else {
        let report = engine
            .execute(request, &config, &mut sink)
            .context("build failed")?;

        println!("Build completed successfully");
        if let Some(code) = report.run.exit_code {
            println!("Exit code: {code}");
        }
    }

    Ok(())
}

fn cmd_doctor() -> Result<()> {
    println!("=== cargo-crossbuild Doctor ===");
    println!();

    match detect_host() {
        Ok(host) => {
            println!("Host:");
            println!("  Triple:  {}", host.host_triple);
            println!("  OS:      {}", host.os);
            println!("  Arch:    {}", host.arch);
            if let Some(ref v) = host.rustc_version {
                println!("  rustc:   {v}");
            }
            if let Some(ref v) = host.cargo_version {
                println!("  cargo:   {v}");
            }
        }
        Err(e) => {
            println!("Host detection failed: {e}");
        }
    }
    println!();

    println!("Known targets:");
    println!("  Tier 1:");
    for t in KnownTargets::tier1() {
        println!("    - {t}");
    }
    println!("  Tier 2:");
    for t in KnownTargets::tier2() {
        println!("    - {t}");
    }
    println!();

    println!("Environment:");
    println!(
        "  CARGO:                 {}",
        std::env::var("CARGO").unwrap_or_default()
    );
    println!(
        "  CROSSBUILD_TARGET_DIR: {}",
        std::env::var("CROSSBUILD_TARGET_DIR").unwrap_or_default()
    );

    Ok(())
}

fn cmd_clean() -> Result<()> {
    let policy = CachePolicy::default();
    let workspace_root = std::env::current_dir().context("failed to get current directory")?;
    let root = policy.absolute_root(&workspace_root);

    if root.exists() {
        std::fs::remove_dir_all(&root)
            .with_context(|| format!("failed to remove cache directory '{}'", root.display()))?;
        println!("Removed crossbuild cache: {}", root.display());
    } else {
        println!(
            "Crossbuild cache directory does not exist: {}",
            root.display()
        );
    }

    Ok(())
}

fn cmd_list_targets() -> Result<()> {
    println!("Tier 1 targets:");
    for t in KnownTargets::tier1() {
        println!("  {t}");
    }
    println!();
    println!("Tier 2 targets:");
    for t in KnownTargets::tier2() {
        println!("  {t}");
    }
    Ok(())
}
