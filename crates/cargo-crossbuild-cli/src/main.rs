use std::env;
use std::path::PathBuf;
use std::process;
use std::time::Duration;

use cargo_crossbuild_core::{
    BuildRequest, CrossBuildConfig, CrossBuildEngine, CrossBuildError, ExecutionMode,
    Profile, StderrDiagnosticSink, TargetTriple,
};
use clap::{Arg, ArgAction, Command};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), CrossBuildError> {
    let matches = build_cli().get_matches();

    // Get the subcommand matches for "crossbuild"
    let crossbuild_matches = matches.subcommand_matches("crossbuild").expect("crossbuild subcommand required");

    let target_str = crossbuild_matches.get_one::<String>("target").expect("target is required");
    let target = TargetTriple::parse(target_str).map_err(|e| CrossBuildError::InvalidTarget {
        target: target_str.clone(),
        reason: e.to_string(),
    })?;

    let manifest_path = match crossbuild_matches.get_one::<String>("manifest-path") {
        Some(path) => PathBuf::from(path),
        None => discover_manifest_path()?,
    };

    let dry_run = crossbuild_matches.get_flag("dry-run");
    let verbose = crossbuild_matches.get_flag("verbose");
    let profile = parse_profile(crossbuild_matches.get_one::<String>("profile"));
    let features: Vec<String> = crossbuild_matches
        .get_many::<String>("features")
        .map(|v| v.cloned().collect())
        .unwrap_or_default();
    let no_default_features = crossbuild_matches.get_flag("no-default-features");
    let workspace = crossbuild_matches.get_flag("workspace");
    let exclude: Vec<String> = crossbuild_matches
        .get_many::<String>("exclude")
        .map(|v| v.cloned().collect())
        .unwrap_or_default();

    let cargo_args: Vec<String> = crossbuild_matches
        .get_many::<String>("cargo-args")
        .map(|v| v.cloned().collect())
        .unwrap_or_default();

    let mut request = BuildRequest::new(manifest_path, target)
        .with_cargo_args(cargo_args)
        .with_execution_mode(if dry_run { ExecutionMode::DryRun } else { ExecutionMode::Execute })
        .with_verbose(verbose)
        .with_profile(profile)
        .with_features(features)
        .with_no_default_features(no_default_features)
        .with_workspace(workspace)
        .with_exclude(exclude);

    let config = CrossBuildConfig::from_environment();
    let engine = CrossBuildEngine::new();
    let mut sink = StderrDiagnosticSink::new(verbose);

    let report = engine.execute(request, &config, &mut sink)?;

    if verbose {
        eprintln!("resolved command: {}", report.run.command);
        eprintln!("working directory: {}", report.run.working_directory.display());
        if let Some(code) = report.run.exit_code {
            eprintln!("exit code: {code}");
        }
        eprintln!("duration: {}ms", report.run.duration_ms);
    }

    Ok(())
}

fn build_cli() -> Command {
    Command::new("cargo-crossbuild")
        .bin_name("cargo")
        .name("cargo-crossbuild")
        .subcommand_required(true)
        .subcommand(
            Command::new("crossbuild")
                .about("Cross-compile Rust projects")
                .arg(
                    Arg::new("target")
                        .long("target")
                        .value_name("TRIPLE")
                        .required(true)
                        .help("Target triple (e.g., x86_64-unknown-linux-gnu)"),
                )
                .arg(
                    Arg::new("manifest-path")
                        .long("manifest-path")
                        .value_name("PATH")
                        .help("Path to Cargo.toml or workspace directory"),
                )
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .action(ArgAction::SetTrue)
                        .help("Print the resolved plan without executing cargo"),
                )
                .arg(
                    Arg::new("execute")
                        .long("execute")
                        .action(ArgAction::SetTrue)
                        .help("Execute the build (default)"),
                )
                .arg(
                    Arg::new("profile")
                        .long("profile")
                        .value_name("PROFILE")
                        .help("Build profile (dev, release, or custom)"),
                )
                .arg(
                    Arg::new("features")
                        .long("features")
                        .value_name("FEATURES")
                        .action(ArgAction::Append)
                        .help("Space-separated list of features to enable"),
                )
                .arg(
                    Arg::new("no-default-features")
                        .long("no-default-features")
                        .action(ArgAction::SetTrue)
                        .help("Do not enable default features"),
                )
                .arg(
                    Arg::new("workspace")
                        .long("workspace")
                        .action(ArgAction::SetTrue)
                        .help("Build all packages in the workspace"),
                )
                .arg(
                    Arg::new("exclude")
                        .long("exclude")
                        .value_name("PACKAGE")
                        .action(ArgAction::Append)
                        .help("Exclude packages from the build"),
                )
                .arg(
                    Arg::new("verbose")
                        .short('v')
                        .long("verbose")
                        .action(ArgAction::SetTrue)
                        .help("Emit detailed diagnostics"),
                )
                .arg(
                    Arg::new("cargo-args")
                        .last(true)
                        .allow_hyphen_values(true)
                        .help("Arguments to pass to cargo build"),
                ),
        )
}

fn parse_profile(s: Option<&String>) -> Profile {
    match s.map(|s| s.as_str()) {
        Some("dev") => Profile::Dev,
        Some("release") => Profile::Release,
        Some(custom) => Profile::Custom(Box::leak(custom.to_string().into_boxed_str())),
        None => Profile::Dev,
    }
}

fn discover_manifest_path() -> Result<PathBuf, CrossBuildError> {
    let mut current = env::current_dir().map_err(|source| CrossBuildError::Io { path: None, source })?;

    loop {
        let candidate = current.join("Cargo.toml");
        if candidate.is_file() {
            return Ok(candidate);
        }

        if !current.pop() {
            return Err(CrossBuildError::ManifestNotFound {
                searched_from: env::current_dir()
                    .map_err(|source| CrossBuildError::Io { path: None, source })?,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_args() {
        let matches = build_cli().try_get_matches_from(vec!["cargo", "crossbuild", "--target", "x86_64-unknown-linux-gnu"]).unwrap();
        let crossbuild_matches = matches.subcommand_matches("crossbuild").unwrap();
        assert_eq!(crossbuild_matches.get_one::<String>("target").unwrap(), "x86_64-unknown-linux-gnu");
    }

    #[test]
    fn parses_cargo_passthrough() {
        let matches = build_cli().try_get_matches_from(vec!["cargo", "crossbuild", "--target", "x86_64-unknown-linux-gnu", "--", "--release"]).unwrap();
        let crossbuild_matches = matches.subcommand_matches("crossbuild").unwrap();
        assert_eq!(crossbuild_matches.get_many::<String>("cargo-args").unwrap().collect::<Vec<_>>(), vec!["--release"]);
    }

    #[test]
    fn parses_features() {
        let matches = build_cli().try_get_matches_from(vec!["cargo", "crossbuild", "--target", "x86_64-unknown-linux-gnu", "--features", "feat1", "--features", "feat2"]).unwrap();
        let crossbuild_matches = matches.subcommand_matches("crossbuild").unwrap();
        let features: Vec<_> = crossbuild_matches.get_many::<String>("features").unwrap().cloned().collect();
        assert_eq!(features, vec!["feat1", "feat2"]);
    }
}