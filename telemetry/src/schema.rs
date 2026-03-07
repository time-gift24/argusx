#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPriority {
    High,
    Low,
}

#[derive(Debug, Clone)]
pub struct TelemetryRecord {
    pub schema_version: u16,
    pub event_name: String,
    pub event_priority: EventPriority,
    pub session_id: String,
    pub turn_id: String,
    pub trace_id: String,
    pub span_id: String,
    pub sequence_no: u32,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub billing_dedupe_key: Option<String>,
    pub attributes_json: serde_json::Value,
}

impl TelemetryRecord {
    pub fn builder(event_name: &str, priority: EventPriority) -> TelemetryRecordBuilder {
        TelemetryRecordBuilder {
            event_name: event_name.to_string(),
            event_priority: priority,
            session_id: String::new(),
            turn_id: String::new(),
            trace_id: String::new(),
            span_id: String::new(),
            sequence_no: 0,
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            billing_dedupe_key: None,
        }
    }

    pub fn validate(&self) -> Result<(), crate::TelemetryError> {
        if self.event_name == "llm_response_completed" && self.billing_dedupe_key.is_none() {
            return Err(crate::TelemetryError::Validation(
                "billing_dedupe_key is required for llm_response_completed".into(),
            ));
        }
        Ok(())
    }
}

pub struct TelemetryRecordBuilder {
    event_name: String,
    event_priority: EventPriority,
    session_id: String,
    turn_id: String,
    trace_id: String,
    span_id: String,
    sequence_no: u32,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    total_tokens: Option<u64>,
    billing_dedupe_key: Option<String>,
}

impl TelemetryRecordBuilder {
    pub fn session_id(mut self, v: &str) -> Self {
        self.session_id = v.to_string();
        self
    }

    pub fn turn_id(mut self, v: &str) -> Self {
        self.turn_id = v.to_string();
        self
    }

    pub fn trace_id(mut self, v: &str) -> Self {
        self.trace_id = v.to_string();
        self
    }

    pub fn span_id(mut self, v: &str) -> Self {
        self.span_id = v.to_string();
        self
    }

    pub fn sequence_no(mut self, v: u32) -> Self {
        self.sequence_no = v;
        self
    }

    pub fn input_tokens(mut self, v: u64) -> Self {
        self.input_tokens = Some(v);
        self
    }

    pub fn output_tokens(mut self, v: u64) -> Self {
        self.output_tokens = Some(v);
        self
    }

    pub fn total_tokens(mut self, v: u64) -> Self {
        self.total_tokens = Some(v);
        self
    }

    pub fn billing_dedupe_key(mut self, v: &str) -> Self {
        self.billing_dedupe_key = Some(v.to_string());
        self
    }

    pub fn build(self) -> TelemetryRecord {
        TelemetryRecord {
            schema_version: 1,
            event_name: self.event_name,
            event_priority: self.event_priority,
            session_id: self.session_id,
            turn_id: self.turn_id,
            trace_id: self.trace_id,
            span_id: self.span_id,
            sequence_no: self.sequence_no,
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
            total_tokens: self.total_tokens,
            billing_dedupe_key: self.billing_dedupe_key,
            attributes_json: serde_json::Value::Null,
        }
    }
}
