//! Diagnostic system for crossbuild.

use std::collections::BTreeMap;
use std::fmt;
use std::io::{self, Write};

use anyhow::Result;

/// Diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Error,
    Debug,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Info => f.write_str("info"),
            Severity::Warning => f.write_str("warning"),
            Severity::Error => f.write_str("error"),
            Severity::Debug => f.write_str("debug"),
        }
    }
}

/// A structured diagnostic event.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    pub help: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl Diagnostic {
    pub fn info(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Info,
            code,
            message: message.into(),
            help: None,
            metadata: BTreeMap::new(),
        }
    }

    pub fn warning(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            code,
            message: message.into(),
            help: None,
            metadata: BTreeMap::new(),
        }
    }

    pub fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code,
            message: message.into(),
            help: None,
            metadata: BTreeMap::new(),
        }
    }

    pub fn debug(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Debug,
            code,
            message: message.into(),
            help: None,
            metadata: BTreeMap::new(),
        }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Receives diagnostics emitted by the engine.
pub trait DiagnosticSink: Send + Sync {
    fn emit(&mut self, diagnostic: Diagnostic);
}

/// Writes diagnostics to stderr in a stable, human-readable format.
pub struct StderrDiagnosticSink {
    verbose: bool,
}

impl StderrDiagnosticSink {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }
}

impl DiagnosticSink for StderrDiagnosticSink {
    fn emit(&mut self, diagnostic: Diagnostic) {
        let severity = match diagnostic.severity {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
            Severity::Debug => "debug",
        };

        let mut stderr = io::stderr().lock();
        let _ = writeln!(
            stderr,
            "[{}] {}: {}",
            severity, diagnostic.code, diagnostic.message
        );
        if self.verbose {
            if let Some(help) = diagnostic.help {
                let _ = writeln!(stderr, "  help: {help}");
            }
            for (k, v) in &diagnostic.metadata {
                let _ = writeln!(stderr, "  {}: {}", k, v);
            }
        }
    }
}

/// Progress indicator for long-running operations.
pub struct Progress {
    message: String,
    current: usize,
    total: usize,
    start_time: std::time::Instant,
}

impl Progress {
    pub fn new(message: impl Into<String>, total: usize) -> Self {
        Self {
            message: message.into(),
            current: 0,
            total,
            start_time: std::time::Instant::now(),
        }
    }

    pub fn increment(&mut self) {
        self.current += 1;
        self.render();
    }

    pub fn finish(self) {
        let elapsed = self.start_time.elapsed();
        eprintln!(
            "{} completed in {:.2}s",
            self.message,
            elapsed.as_secs_f64()
        );
    }

    fn render(&self) {
        if self.total > 0 {
            let pct = (self.current as f64 / self.total as f64) * 100.0;
            eprint!(
                "\r{} [{}/{}] {:.1}%",
                self.message, self.current, self.total, pct
            );
        } else {
            eprint!("\r{} [{}]", self.message, self.current);
        }
        let _ = std::io::stderr().flush();
    }
}

/// Spinner for indeterminate progress.
pub struct Spinner {
    message: String,
    frames: Vec<&'static str>,
    index: usize,
}

impl Spinner {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            frames: vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            index: 0,
        }
    }

    pub fn tick(&mut self) {
        eprint!("\r{} {}", self.frames[self.index], self.message);
        let _ = std::io::stderr().flush();
        self.index = (self.index + 1) % self.frames.len();
    }

    pub fn finish(self, message: impl Into<String>) {
        eprintln!("\r✓ {}", message.into());
    }
}

/// Structured logging for telemetry event logger for machine-readable output.
pub struct EventLogger {
    writer: Box<dyn Write + Send>,
}

impl EventLogger {
    pub fn new(writer: Box<dyn Write + Send>) -> Self {
        Self { writer }
    }

    pub fn log(&mut self, event: &serde_json::Value) -> Result<()> {
        let json = serde_json::to_string(event)?;
        writeln!(self.writer, "{}", json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_creation() {
        let d = Diagnostic::info("CB0001", "test message");
        assert_eq!(d.severity, Severity::Info);
        assert_eq!(d.code, "CB0001");
        assert_eq!(d.message, "test message");
    }

    #[test]
    fn diagnostic_with_help() {
        let d = Diagnostic::error("CB0001", "error").with_help("try this");
        assert_eq!(d.help, Some("try this".to_string()));
    }
}