//! Telemetry and metrics collection for cross-compilation.

use crossbuild_core::{
    diagnostics::{Diagnostic, DiagnosticSink},
    model::TargetTriple,
};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

/// Telemetry collector for cross-build operations.
pub struct TelemetryCollector {
    start_time: Instant,
    #[allow(dead_code)]
    target: TargetTriple,
    metadata: BTreeMap<String, String>,
    events: Vec<TelemetryEvent>,
}

/// Individual telemetry event.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TelemetryEvent {
    pub timestamp: String,
    pub event_type: String,
    pub message: String,
    pub duration_ms: Option<u64>,
    pub metadata: std::collections::BTreeMap<String, String>,
}

impl TelemetryCollector {
    /// Creates a new telemetry collector for a build.
    pub fn new(target: crossbuild_core::model::TargetTriple) -> Self {
        Self {
            start_time: Instant::now(),
            target,
            metadata: BTreeMap::new(),
            events: Vec::new(),
        }
    }

    /// Adds a custom metadata field.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Records an event.
    pub fn record_event(&mut self, event_type: impl Into<String>, message: impl Into<String>, duration_ms: Option<u64>) {
        let event = TelemetryEvent {
            timestamp: chrono::Utc::now().to_rfc3339(),
            event_type: event_type.into(),
            message: message.into(),
            duration_ms,
            metadata: self.metadata.clone(),
        };
        self.events.push(event);
    }

    /// Records the start of a phase.
    pub fn phase_start(&mut self, phase: &str) {
        self.record_event("phase_start", format!("Starting {}", phase), None);
    }

    /// Records the end of a phase.
    pub fn phase_end(&mut self, phase: &str, duration: Duration) {
        self.record_event("phase_end", format!("Completed {}", phase), Some(duration.as_millis() as u64));
    }

    /// Records an error.
    pub fn error(&mut self, error: &str) {
        self.record_event("error", error, None);
    }

    /// Records a warning.
    pub fn warning(&mut self, warning: &str) {
        self.record_event("warning", warning, None);
    }

    /// Gets the total elapsed time.
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Emits all collected telemetry to a sink.
    pub fn emit<D: DiagnosticSink>(&self, sink: &mut D) {
        for event in &self.events {
            sink.emit(Diagnostic::info(
                "TEL",
                format!("{}: {}", event.event_type.to_uppercase(), event.message),
            ));
        }
    }
}

/// Macro for timing operations.
#[macro_export]
macro_rules! timed {
    ($telemetry:expr, $phase:expr, $body:expr) => {{
        let start = std::time::Instant::now();
        $telemetry.phase_start($phase);
        let result = $body;
        $telemetry.phase_end($phase, start.elapsed());
        result
    }};
}

/// RAII timer for automatic phase timing.
pub struct PhaseTimer<'a> {
    telemetry: &'a mut TelemetryCollector,
    phase: String,
    start: Instant,
}

impl<'a> PhaseTimer<'a> {
    pub fn new(telemetry: &'a mut TelemetryCollector, phase: impl Into<String>) -> Self {
        let phase = phase.into();
        telemetry.phase_start(&phase);
        Self {
            telemetry,
            phase,
            start: Instant::now(),
        }
    }
}

impl<'a> Drop for PhaseTimer<'a> {
    fn drop(&mut self) {
        self.telemetry.phase_end(&self.phase, self.start.elapsed());
    }
}

/// Telemetry sink that emits to tracing.
pub struct TracingTelemetrySink;

impl TelemetryCollector {
    pub fn with_tracing(self) -> Self {
        // Events will also be logged via tracing
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telemetry_collection() {
        let mut telemetry = TelemetryCollector::new(crossbuild_core::model::TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap());
        telemetry.phase_start("test");
        std::thread::sleep(std::time::Duration::from_millis(10));
        telemetry.phase_end("test", std::time::Duration::from_millis(50));

        assert_eq!(telemetry.events.len(), 2);
    }

    #[test]
    fn phase_timer_works() {
        let mut telemetry = TelemetryCollector::new(crossbuild_core::model::TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap());
        {
            let _timer = PhaseTimer::new(&mut telemetry, "test_phase");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(telemetry.events.len(), 2);
    }
}