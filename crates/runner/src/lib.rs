use std::process::Command;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use crossbuild_core::{
    diagnostics::{Diagnostic, DiagnosticSink},
    error::CrossBuildError,
    model::{BuildPlan, ExecutionMode, RunReport},
};

/// Executes build plans.
pub struct Runner;

impl Runner {
    pub fn run(
        plan: &BuildPlan,
        sink: &mut dyn DiagnosticSink,
    ) -> Result<RunReport, CrossBuildError> {
        let command_repr = plan.command.to_string();

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
                duration_ms: 0,
            });
        }

        sink.emit(Diagnostic::info(
            "CB1001",
            format!("running {command_repr}"),
        ));

        let start = std::time::Instant::now();

        let mut command = Command::new(&plan.command.program);
        command.current_dir(&plan.command.current_dir);
        command.args(&plan.command.args);
        command.envs(&plan.command.env);

        // Set up output capture
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());

        let mut child = command.spawn().map_err(|source| {
            if source.kind() == std::io::ErrorKind::NotFound {
                CrossBuildError::CargoUnavailable {
                    program: plan.command.program.clone(),
                }
            } else {
                CrossBuildError::Io { path: None, source }
            }
        })?;

        // Capture and forward output
        let stdout = child.stdout.take().expect("stdout was piped at spawn");
        let stderr = child.stderr.take().expect("stderr was piped at spawn");

        let collected: Arc<Mutex<Vec<Diagnostic>>> = Arc::new(Mutex::new(Vec::new()));

        let out_diags = collected.clone();
        let stdout_handle = std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                if !line.trim().is_empty() {
                    out_diags
                        .lock()
                        .expect("diagnostic lock not poisoned")
                        .push(Diagnostic::info("CB1010", line));
                }
            }
        });

        let err_diags = collected.clone();
        let stderr_handle = std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                if !line.trim().is_empty() {
                    err_diags
                        .lock()
                        .expect("diagnostic lock not poisoned")
                        .push(Diagnostic::warning("CB1011", line));
                }
            }
        });

        let status = child
            .wait()
            .map_err(|source| CrossBuildError::Io { path: None, source })?;

        let _ = stdout_handle.join();
        let _ = stderr_handle.join();

        for diag in collected
            .lock()
            .expect("diagnostic lock not poisoned")
            .drain(..)
        {
            sink.emit(diag);
        }

        let exit_code = status.code();
        let duration_ms = start.elapsed().as_millis() as u64;

        if !status.success() {
            return Err(CrossBuildError::BuildFailed {
                command: command_repr,
                exit_code,
            });
        }

        Ok(RunReport {
            executed: true,
            command: command_repr,
            working_directory: plan.command.current_dir.clone(),
            exit_code,
            duration_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbuild_core::model::{
        BuildPlan, BuildRequest, CommandLine, ExecutionMode, PlanStep, TargetTriple,
    };
    use std::path::PathBuf;

    #[test]
    fn dry_run_works() {
        let plan = BuildPlan {
            request: BuildRequest::new(
                PathBuf::from("Cargo.toml"),
                TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap(),
            )
            .with_execution_mode(ExecutionMode::DryRun),
            host: crossbuild_core::platform::detect_host().unwrap(),
            target: crossbuild_core::platform::assess_target(
                &TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap(),
                &crossbuild_core::platform::detect_host().unwrap(),
            ),
            command: CommandLine::new("cargo", PathBuf::from(".")),
            steps: vec![PlanStep::InvokeCargo],
            provider_actions: vec![],
            cargo_config: None,
            cache_key: "test".to_string(),
        };

        let mut sink = crossbuild_core::diagnostics::StderrDiagnosticSink::new(false);
        let report = Runner::run(&plan, &mut sink).unwrap();
        assert!(!report.executed);
    }
}
