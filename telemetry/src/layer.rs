use crate::{TelemetryConfig, TelemetryRecord};
use std::sync::{Arc, Mutex};

/// A sink that receives telemetry records from the tracing layer.
pub trait TelemetrySink: Send + Sync + 'static {
    fn try_send(&self, record: TelemetryRecord);
}

/// A tracing layer that maps tracing events to telemetry records.
pub struct TelemetryLayer<S> {
    sink: S,
    #[allow(dead_code)]
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
        use tracing::field::Visit;

        struct FieldVisitor {
            event_name: Option<String>,
            event_priority: crate::EventPriority,
            sequence_no: u32,
            input_tokens: Option<u64>,
            output_tokens: Option<u64>,
            total_tokens: Option<u64>,
            billing_dedupe_key: Option<String>,
        }

        impl Visit for FieldVisitor {
            fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
                match field.name() {
                    "event_name" => self.event_name = Some(value.to_string()),
                    "event_priority" => {
                        self.event_priority = match value {
                            "high" => crate::EventPriority::High,
                            _ => crate::EventPriority::Low,
                        };
                    }
                    "billing_dedupe_key" => self.billing_dedupe_key = Some(value.to_string()),
                    _ => {}
                }
            }

            fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
                match field.name() {
                    "sequence_no" => self.sequence_no = value as u32,
                    "input_tokens" => self.input_tokens = Some(value),
                    "output_tokens" => self.output_tokens = Some(value),
                    "total_tokens" => self.total_tokens = Some(value),
                    _ => {}
                }
            }

            fn record_debug(&mut self, _field: &tracing::field::Field, _value: &dyn std::fmt::Debug) {}
        }

        let mut visitor = FieldVisitor {
            event_name: None,
            event_priority: crate::EventPriority::Low,
            sequence_no: 0,
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            billing_dedupe_key: None,
        };
        event.record(&mut visitor);

        let event_name = match visitor.event_name {
            Some(name) => name,
            None => return, // Skip events without event_name
        };

        // Extract span context for correlation
        let (session_id, turn_id, trace_id, span_id) = ctx.event_span(event)
            .map(|span| {
                // Try to get from the span's extensions
                let ext = span.extensions();
                if let Some(data) = ext.get::<TelemetrySpanData>() {
                    (data.session_id.clone(), data.turn_id.clone(), data.trace_id.clone(), data.span_id.clone())
                } else {
                    (String::new(), String::new(), String::new(), String::new())
                }
            })
            .unwrap_or_default();

        let record = TelemetryRecord {
            schema_version: 1,
            event_name,
            event_priority: visitor.event_priority,
            session_id,
            turn_id,
            trace_id,
            span_id,
            sequence_no: visitor.sequence_no,
            input_tokens: visitor.input_tokens,
            output_tokens: visitor.output_tokens,
            total_tokens: visitor.total_tokens,
            billing_dedupe_key: visitor.billing_dedupe_key,
            attributes_json: serde_json::Value::Null,
        };

        self.sink.try_send(record);
    }

    fn on_new_span(&self, attrs: &tracing::span::Attributes<'_>, id: &tracing::span::Id, ctx: tracing_subscriber::layer::Context<'_, T>) {
        use tracing::field::Visit;

        struct SpanFieldVisitor {
            session_id: String,
            turn_id: String,
            trace_id: String,
            span_id: String,
        }

        impl Visit for SpanFieldVisitor {
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

        let mut visitor = SpanFieldVisitor {
            session_id: String::new(),
            turn_id: String::new(),
            trace_id: String::new(),
            span_id: String::new(),
        };
        attrs.record(&mut visitor);

        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(TelemetrySpanData {
                session_id: visitor.session_id,
                turn_id: visitor.turn_id,
                trace_id: visitor.trace_id,
                span_id: visitor.span_id,
            });
        }
    }
}

/// Custom span data for telemetry correlation
#[derive(Clone)]
struct TelemetrySpanData {
    session_id: String,
    turn_id: String,
    trace_id: String,
    span_id: String,
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
