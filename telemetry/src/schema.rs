use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPriority {
    High,
    Low,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TelemetryRecord {
    pub ingest_id: Option<Uuid>,
    pub schema_version: u16,
    pub occurred_at: DateTime<Utc>,
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub session_id: String,
    pub turn_id: String,
    pub step_index: Option<u32>,
    pub sequence_no: u32,
    pub level: String,
    pub target: String,
    pub event_name: String,
    pub event_priority: EventPriority,
    pub user_id: Option<String>,
    pub model_name: Option<String>,
    pub provider: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub billing_dedupe_key: Option<String>,
    pub tool_name: Option<String>,
    pub tool_outcome: Option<String>,
    pub tool_duration_ms: Option<u64>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub request_preview: Option<String>,
    pub response_preview: Option<String>,
    pub attributes_json: Value,
}

impl TelemetryRecord {
    pub fn builder(event_name: &str, priority: EventPriority) -> TelemetryRecordBuilder {
        TelemetryRecordBuilder {
            record: TelemetryRecord {
                ingest_id: None,
                schema_version: 1,
                occurred_at: Utc::now(),
                trace_id: String::new(),
                span_id: String::new(),
                parent_span_id: None,
                session_id: String::new(),
                turn_id: String::new(),
                step_index: None,
                sequence_no: 0,
                level: "info".to_string(),
                target: String::new(),
                event_name: event_name.to_string(),
                event_priority: priority,
                user_id: None,
                model_name: None,
                provider: None,
                input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                billing_dedupe_key: None,
                tool_name: None,
                tool_outcome: None,
                tool_duration_ms: None,
                error_code: None,
                error_message: None,
                request_preview: None,
                response_preview: None,
                attributes_json: Value::Object(Map::new()),
            },
        }
    }

    pub fn validate(&self) -> Result<(), crate::TelemetryError> {
        if self.event_name == "llm_response_completed" && self.billing_dedupe_key.is_none() {
            return Err(crate::TelemetryError::Validation(
                "billing_dedupe_key is required for llm_response_completed".into(),
            ));
        }
        if !self.attributes_json.is_object() {
            return Err(crate::TelemetryError::Validation(
                "attributes_json must be a JSON object".into(),
            ));
        }
        Ok(())
    }
}

pub struct TelemetryRecordBuilder {
    record: TelemetryRecord,
}

impl TelemetryRecordBuilder {
    pub fn session_id(mut self, v: &str) -> Self {
        self.record.session_id = v.to_string();
        self
    }

    pub fn turn_id(mut self, v: &str) -> Self {
        self.record.turn_id = v.to_string();
        self
    }

    pub fn trace_id(mut self, v: &str) -> Self {
        self.record.trace_id = v.to_string();
        self
    }

    pub fn span_id(mut self, v: &str) -> Self {
        self.record.span_id = v.to_string();
        self
    }

    pub fn parent_span_id(mut self, v: &str) -> Self {
        self.record.parent_span_id = Some(v.to_string());
        self
    }

    pub fn step_index(mut self, v: u32) -> Self {
        self.record.step_index = Some(v);
        self
    }

    pub fn sequence_no(mut self, v: u32) -> Self {
        self.record.sequence_no = v;
        self
    }

    pub fn level(mut self, v: &str) -> Self {
        self.record.level = v.to_string();
        self
    }

    pub fn target(mut self, v: &str) -> Self {
        self.record.target = v.to_string();
        self
    }

    pub fn user_id(mut self, v: &str) -> Self {
        self.record.user_id = Some(v.to_string());
        self
    }

    pub fn model_name(mut self, v: &str) -> Self {
        self.record.model_name = Some(v.to_string());
        self
    }

    pub fn provider(mut self, v: &str) -> Self {
        self.record.provider = Some(v.to_string());
        self
    }

    pub fn input_tokens(mut self, v: u64) -> Self {
        self.record.input_tokens = Some(v);
        self
    }

    pub fn output_tokens(mut self, v: u64) -> Self {
        self.record.output_tokens = Some(v);
        self
    }

    pub fn total_tokens(mut self, v: u64) -> Self {
        self.record.total_tokens = Some(v);
        self
    }

    pub fn billing_dedupe_key(mut self, v: &str) -> Self {
        self.record.billing_dedupe_key = Some(v.to_string());
        self
    }

    pub fn tool_name(mut self, v: &str) -> Self {
        self.record.tool_name = Some(v.to_string());
        self
    }

    pub fn tool_outcome(mut self, v: &str) -> Self {
        self.record.tool_outcome = Some(v.to_string());
        self
    }

    pub fn tool_duration_ms(mut self, v: u64) -> Self {
        self.record.tool_duration_ms = Some(v);
        self
    }

    pub fn error_code(mut self, v: &str) -> Self {
        self.record.error_code = Some(v.to_string());
        self
    }

    pub fn error_message(mut self, v: &str) -> Self {
        self.record.error_message = Some(v.to_string());
        self
    }

    pub fn request_preview(mut self, v: &str) -> Self {
        self.record.request_preview = Some(v.to_string());
        self
    }

    pub fn response_preview(mut self, v: &str) -> Self {
        self.record.response_preview = Some(v.to_string());
        self
    }

    pub fn attributes_json(mut self, v: Value) -> Self {
        self.record.attributes_json = v;
        self
    }

    pub fn build(self) -> TelemetryRecord {
        self.record
    }
}
