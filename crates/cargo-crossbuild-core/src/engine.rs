//! Cross-build engine - orchestrates planning and execution.

use std::time::Instant;

use crate::config::CrossBuildConfig;
use crate::diagnostics::{Diagnostic, DiagnosticSink};
use crate::error::CrossBuildError;
use crate::model::{BuildPlan, BuildRequest, ExecutionMode, ExecutionReport, RunReport};
use crate::planner::Planner;
use crate::runner::Runner;

/// Top-level engine that orchestrates planning and execution.
pub struct CrossBuildEngine {
    planner: Planner,
}

impl CrossBuildEngine {
    pub fn new() -> Self {
        Self {
            planner: Planner::new(),
        }
    }

    /// Executes a cross-build request.
    pub fn execute(
        &self,
        request: BuildRequest,
        config: &CrossBuildConfig,
        sink: &mut dyn DiagnosticSink,
    ) -> Result<ExecutionReport, CrossBuildError> {
        let start = Instant::now();

        sink.emit(Diagnostic::info("CB0001", "planning cross-build request"));
        let plan = self.planner.plan(request, config)?;

        sink.emit(Diagnostic::info("CB0002", format!("resolved target: {}", plan.target.triple)));
        sink.emit(Diagnostic::info("CB0003", format!("host: {}", plan.host.host_triple)));
        sink.emit(Diagnostic::info("CB0004", format!("cross-compilation: {}", plan.is_cross_compilation())));

        // Emit provider notes
        for action in &plan.provider_actions {
            for note in &action.notes {
                sink.emit(Diagnostic::info("CB0100", format!("{}: {}", action.provider_name, note)));
            }
        }

        // Emit plan steps
        for step in &plan.steps {
            sink.emit(Diagnostic::info("CB0200", format!("step: {:?}", step)));
        }

        // Write cargo config if present
        if let Some(config) = &plan.cargo_config {
            let config_path = plan.manifest_directory().join(".cargo").join("config.toml");
            std::fs::create_dir_all(config_path.parent().unwrap()).map_err(|e| CrossBuildError::Io {
                path: Some(config_path.clone()),
                source: e,
            })?;
            let content = toml::to_string_pretty(config).map_err(|e| CrossBuildError::configuration(e.to_string()))?;
            std::fs::write(&config_path, content).map_err(|e| CrossBuildError::Io {
                path: Some(config_path.clone()),
                source: e,
            })?;
            sink.emit(Diagnostic::info("CB0300", format!("wrote cargo config to {}", config_path.display())));
        }

        // Execute the plan
        let run = Runner::run(&plan, sink)?;
        let duration = start.elapsed();

        sink.emit(Diagnostic::info("CB0400", format!("build completed in {:.2}s", duration.as_secs_f64())));

        Ok(ExecutionReport { plan, run })
    }

    /// Performs a dry run - plans but doesn't execute.
    pub fn dry_run(
        &self,
        request: BuildRequest,
        config: &CrossBuildConfig,
        sink: &mut dyn DiagnosticSink,
    ) -> Result<BuildPlan, CrossBuildError> {
        let mut dry_request = request;
        dry_request.execution_mode = crate::model::ExecutionMode::DryRun;
        self.planner.plan(dry_request, config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::StderrDiagnosticSink;
    use crate::model::{BuildRequest, TargetTriple, ExecutionMode};
    use tempfile::tempdir;

    #[test]
    fn engine_creation() {
        let engine = CrossBuildEngine::new();
        // Engine doesn't implement Debug, just verify it can be created
        let _ = engine;
    }

    #[test]
    fn dry_run_works() {
        let engine = CrossBuildEngine::new();
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("Cargo.toml");
        std::fs::write(&manifest, r#"
[package]
name = "test"
version = "0.1.0"
edition = "2021"
"#).unwrap();

        let host = crate::platform::detect_host().unwrap();
        let target = TargetTriple::parse(&host.host_triple.triple).unwrap();
        let request = BuildRequest::new(manifest, target)
            .with_execution_mode(ExecutionMode::DryRun);
        let config = CrossBuildConfig::default();
        let mut sink = StderrDiagnosticSink::new(false);

        let plan = engine.dry_run(request, &config, &mut sink).unwrap();
        assert_eq!(plan.request.execution_mode, ExecutionMode::DryRun);
    }
}