use crate::{TelemetryConfig, TelemetryRecord, redact_preview};
use serde_json::{Map, Value};
use std::sync::{Arc, Mutex};

/// A sink that receives telemetry records from the tracing layer.
pub trait TelemetrySink: Send + Sync + 'static {
    fn try_send(&self, record: TelemetryRecord);
}

/// A tracing layer that maps tracing events to telemetry records.
pub struct TelemetryLayer<S> {
    sink: S,
    config: TelemetryConfig,
}

impl<S> TelemetryLayer<S> {
    pub fn new(sink: S, config: TelemetryConfig) -> Self {
        Self { sink, config }
    }
}

impl<S, T> tracing_subscriber::Layer<T> for TelemetryLayer<S>
where
    S: TelemetrySink,
    T: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, T>) {
        let mut visitor = FieldVisitor::new(event.metadata(), &self.config);
        event.record(&mut visitor);

        let mut record = visitor.finish();
        if record.event_name.is_empty() {
            return;
        }
        if record.event_name == "llm_delta" && !self.config.delta_events {
            return;
        }

        if let Some(span) = ctx.event_span(event) {
            let ext = span.extensions();
            if let Some(data) = ext.get::<TelemetrySpanData>() {
                if record.session_id.is_empty() {
                    record.session_id = data.session_id.clone();
                }
                if record.turn_id.is_empty() {
                    record.turn_id = data.turn_id.clone();
                }
                if record.trace_id.is_empty() {
                    record.trace_id = data.trace_id.clone();
                }
                if record.span_id.is_empty() {
                    record.span_id = data.span_id.clone();
                }
                if record.parent_span_id.is_none() {
                    record.parent_span_id = data.parent_span_id.clone();
                }
            }
        }

        self.sink.try_send(record);
    }

    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, T>,
    ) {
        let parent_data = ctx
            .span(id)
            .and_then(|span| span.parent())
            .and_then(|parent| parent.extensions().get::<TelemetrySpanData>().cloned());

        let mut visitor = SpanFieldVisitor {
            session_id: parent_data
                .as_ref()
                .map(|data| data.session_id.clone())
                .unwrap_or_default(),
            turn_id: parent_data
                .as_ref()
                .map(|data| data.turn_id.clone())
                .unwrap_or_default(),
            trace_id: parent_data
                .as_ref()
                .map(|data| data.trace_id.clone())
                .unwrap_or_else(|| format_span_id(id)),
            span_id: format_span_id(id),
            parent_span_id: parent_data.as_ref().map(|data| data.span_id.clone()),
        };
        attrs.record(&mut visitor);

        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(TelemetrySpanData {
                session_id: visitor.session_id,
                turn_id: visitor.turn_id,
                trace_id: visitor.trace_id,
                span_id: visitor.span_id,
                parent_span_id: visitor.parent_span_id,
            });
        }
    }
}

fn format_span_id(id: &tracing::span::Id) -> String {
    format!("{:016x}", id.clone().into_u64())
}

fn level_to_str(level: &tracing::Level) -> &'static str {
    match *level {
        tracing::Level::TRACE => "trace",
        tracing::Level::DEBUG => "debug",
        tracing::Level::INFO => "info",
        tracing::Level::WARN => "warn",
        tracing::Level::ERROR => "error",
    }
}

struct FieldVisitor<'a> {
    record: TelemetryRecord,
    attributes: Map<String, Value>,
    config: &'a TelemetryConfig,
}

impl<'a> FieldVisitor<'a> {
    fn new(metadata: &tracing::Metadata<'_>, config: &'a TelemetryConfig) -> Self {
        Self {
            record: TelemetryRecord::builder("", crate::EventPriority::Low)
                .level(level_to_str(metadata.level()))
                .target(metadata.target())
                .build(),
            attributes: Map::new(),
            config,
        }
    }

    fn insert_attribute(&mut self, field: &str, value: Value) {
        self.attributes.insert(field.to_string(), value);
    }

    fn finish(mut self) -> TelemetryRecord {
        self.record.attributes_json = Value::Object(std::mem::take(&mut self.attributes));
        self.record
    }
}

impl tracing::field::Visit for FieldVisitor<'_> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "event_name" => self.record.event_name = value.to_string(),
            "event_priority" => {
                self.record.event_priority = match value {
                    "high" => crate::EventPriority::High,
                    _ => crate::EventPriority::Low,
                };
            }
            "session_id" => self.record.session_id = value.to_string(),
            "turn_id" => self.record.turn_id = value.to_string(),
            "trace_id" => self.record.trace_id = value.to_string(),
            "span_id" => self.record.span_id = value.to_string(),
            "parent_span_id" => self.record.parent_span_id = Some(value.to_string()),
            "user_id" => self.record.user_id = Some(value.to_string()),
            "model_name" => self.record.model_name = Some(value.to_string()),
            "provider" => self.record.provider = Some(value.to_string()),
            "billing_dedupe_key" => self.record.billing_dedupe_key = Some(value.to_string()),
            "tool_name" => self.record.tool_name = Some(value.to_string()),
            "tool_outcome" => self.record.tool_outcome = Some(value.to_string()),
            "error_code" => self.record.error_code = Some(value.to_string()),
            "error_message" => self.record.error_message = Some(value.to_string()),
            "request_preview" => {
                if self.config.full_logging {
                    self.record.request_preview = Some(redact_preview(value, 256));
                }
            }
            "response_preview" => {
                if self.config.full_logging {
                    self.record.response_preview = Some(redact_preview(value, 256));
                }
            }
            other => self.insert_attribute(other, Value::String(value.to_string())),
        }
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        match field.name() {
            "sequence_no" => self.record.sequence_no = value as u32,
            "step_index" => self.record.step_index = Some(value as u32),
            "input_tokens" => self.record.input_tokens = Some(value),
            "output_tokens" => self.record.output_tokens = Some(value),
            "total_tokens" => self.record.total_tokens = Some(value),
            "tool_duration_ms" => self.record.tool_duration_ms = Some(value),
            other => self.insert_attribute(other, Value::from(value)),
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.insert_attribute(field.name(), Value::from(value));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.insert_attribute(field.name(), Value::from(value));
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.insert_attribute(field.name(), Value::String(format!("{value:?}")));
    }
}

struct SpanFieldVisitor {
    session_id: String,
    turn_id: String,
    trace_id: String,
    span_id: String,
    parent_span_id: Option<String>,
}

impl tracing::field::Visit for SpanFieldVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "session_id" => self.session_id = value.to_string(),
            "turn_id" => self.turn_id = value.to_string(),
            "trace_id" => self.trace_id = value.to_string(),
            "span_id" => self.span_id = value.to_string(),
            _ => {}
        }
    }

    fn record_debug(&mut self, _field: &tracing::field::Field, _value: &dyn std::fmt::Debug) {}
}

/// Custom span data for telemetry correlation.
#[derive(Clone)]
struct TelemetrySpanData {
    session_id: String,
    turn_id: String,
    trace_id: String,
    span_id: String,
    parent_span_id: Option<String>,
}

/// A recording sink for testing that captures all records in memory.
#[derive(Default, Clone)]
pub struct RecordingSink {
    records: Arc<Mutex<Vec<TelemetryRecord>>>,
}

impl TelemetrySink for RecordingSink {
    fn try_send(&self, record: TelemetryRecord) {
        self.records.lock().unwrap().push(record);
    }
}

impl RecordingSink {
    pub fn take(&self) -> Vec<TelemetryRecord> {
        self.records.lock().unwrap().drain(..).collect()
    }

    pub fn records(&self) -> Vec<TelemetryRecord> {
        self.records.lock().unwrap().clone()
    }
}
