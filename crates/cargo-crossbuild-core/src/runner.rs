//! Build plan execution with timing and diagnostics.

use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use crate::diagnostics::{Diagnostic, DiagnosticSink};
use crate::error::CrossBuildError;
use crate::model::{BuildPlan, ExecutionMode, RunReport};

/// Executes build plans.
pub struct Runner;

impl Runner {
    pub fn run(
        plan: &BuildPlan,
        sink: &mut dyn DiagnosticSink,
    ) -> Result<RunReport, CrossBuildError> {
        let command_repr = plan.command.to_string();
        let start = Instant::now();

        if plan.request.execution_mode == ExecutionMode::DryRun {
            sink.emit(Diagnostic::info(
                "CB1000",
                format!("dry run: {command_repr}"),
            ));
            return Ok(RunReport {
                executed: false,
                command: command_repr,
                working_directory: plan.command.current_dir.clone(),
                exit_code: None,
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        sink.emit(Diagnostic::info(
            "CB1001",
            format!("running {command_repr}"),
        ));

        let mut command = Command::new(&plan.command.program);
        command.current_dir(&plan.command.current_dir);
        command.args(&plan.command.args);
        command.envs(&plan.command.env);

        // Capture output for diagnostics
        let output = command.output().map_err(|source| {
            if source.kind() == std::io::ErrorKind::NotFound {
                CrossBuildError::CargoUnavailable {
                    program: plan.command.program.clone(),
                }
            } else {
                CrossBuildError::Io { path: None, source }
            }
        })?;

        let duration = start.elapsed().as_millis() as u64;

        // Emit stdout/stderr as diagnostics
        if !output.stdout.is_empty() {
            let stdout_str = String::from_utf8_lossy(&output.stdout);
            for line in stdout_str.lines() {
                if !line.trim().is_empty() {
                    sink.emit(Diagnostic::info("CB1010", line.to_string()));
                }
            }
        }

        if !output.stderr.is_empty() {
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            for line in stderr_str.lines() {
                if !line.trim().is_empty() {
                    sink.emit(Diagnostic::warning("CB1011", line.to_string()));
                }
            }
        }

        let exit_code = output.status.code();

        if !output.status.success() {
            sink.emit(Diagnostic::error(
                "CB1002",
                format!("build failed with exit code: {exit_code:?}"),
            ));
            return Err(CrossBuildError::BuildFailed {
                command: command_repr,
                exit_code,
            });
        }

        sink.emit(Diagnostic::info(
            "CB1003",
            format!("build completed successfully in {}ms", duration),
        ));

        Ok(RunReport {
            executed: true,
            command: command_repr,
            working_directory: plan.command.current_dir.clone(),
            exit_code,
            duration_ms: duration,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::StderrDiagnosticSink;
    use crate::model::{BuildPlan, BuildRequest, CommandLine, ExecutionMode, PlanStep, TargetTriple};
    use std::path::PathBuf;

    #[test]
    fn dry_run_works() {
        let plan = BuildPlan {
            request: BuildRequest::new(
                PathBuf::from("Cargo.toml"),
                TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap(),
            ).with_execution_mode(ExecutionMode::DryRun),
            host: crate::platform::detect_host().unwrap(),
            target: crate::platform::assess_target(&TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap(), &crate::platform::detect_host().unwrap()),
            command: CommandLine::new("cargo", PathBuf::from(".")),
            steps: vec![PlanStep::InvokeCargo],
            provider_actions: vec![],
            cargo_config: None,
            cache_key: "test".to_string(),
        };

        let mut sink = StderrDiagnosticSink::new(false);
        let report = Runner::run(&plan, &mut sink).unwrap();
        assert!(!report.executed);
    }
}