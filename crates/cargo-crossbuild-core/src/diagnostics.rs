use std::io::{self, Write};

/// Diagnostic severity used by the engine and CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// A structured diagnostic event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    pub help: Option<String>,
}

impl Diagnostic {
    pub fn info(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Info,
            code,
            message: message.into(),
            help: None,
        }
    }

    pub fn warning(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            code,
            message: message.into(),
            help: None,
        }
    }

    pub fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code,
            message: message.into(),
            help: None,
        }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

/// Receives diagnostics emitted by the engine.
pub trait DiagnosticSink {
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
        };

        let mut stderr = io::stderr().lock();
        let _ = writeln!(
            stderr,
            "[{severity}] {}: {}",
            diagnostic.code, diagnostic.message
        );
        if self.verbose {
            if let Some(help) = diagnostic.help {
                let _ = writeln!(stderr, "  help: {help}");
            }
        }
    }
}
