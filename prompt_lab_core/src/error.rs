use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PromptLabErrorCode {
    InvalidInput,
    NotFound,
    Conflict,
    DbError,
    ParseError,
}

impl PromptLabErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_INPUT",
            Self::NotFound => "NOT_FOUND",
            Self::Conflict => "CONFLICT",
            Self::DbError => "DB_ERROR",
            Self::ParseError => "PARSE_ERROR",
        }
    }
}

#[derive(Debug, Error)]
pub enum PromptLabError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("json serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid value for {field}: {value}")]
    InvalidEnum { field: &'static str, value: String },

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("{entity} with id={id} was not found")]
    NotFound { entity: &'static str, id: i64 },

    #[error("conflict: {0}")]
    Conflict(String),
}

impl PromptLabError {
    pub fn code(&self) -> PromptLabErrorCode {
        match self {
            Self::InvalidEnum { .. } | Self::InvalidInput(_) => PromptLabErrorCode::InvalidInput,
            Self::NotFound { .. } => PromptLabErrorCode::NotFound,
            Self::Conflict(_) => PromptLabErrorCode::Conflict,
            Self::Json(_) => PromptLabErrorCode::ParseError,
            Self::Database(_) | Self::Migration(_) | Self::Io(_) => PromptLabErrorCode::DbError,
        }
    }
}

pub type Result<T> = std::result::Result<T, PromptLabError>;
