//! The carpenter error hierarchy + envelope `code` mapping.
//!
//! Every variant maps 1:1 to an envelope `code` string ([`CarpenterError::code`]).
//! Commands never raise to the caller — failures become an error envelope.

use serde_json::{json, Value};

/// Every failure mode carpenter surfaces to the caller.
#[derive(Debug, thiserror::Error)]
pub enum CarpenterError {
    /// Requested id/slug does not exist.
    #[error("not found: {0}")]
    NotFound(String),
    /// Create would collide (duplicate slug, `.venv` present, …).
    #[error("already exists: {0}")]
    AlreadyExists(String),
    /// Bad `--spec` JSON, unknown enum value, or failed cross-field validation.
    #[error("validation error: {0}")]
    ValidationError(String),
    /// SQLite failure or missing file (no course venv, `uv` not on PATH, …).
    #[error("store error: {0}")]
    StoreError(String),
    /// A managed/scaffolding cell errored during execution.
    #[error("execute error: {message}")]
    ExecuteError {
        /// human-readable summary.
        message: String,
        /// structured payload — `{index,ename,evalue}` or `{errors:[…]}`.
        details: serde_json::Value,
    },
    /// Destructive op not confirmed (`--force` gate, declined or unavailable
    /// confirmation prompt), or unresolvable sync conflict.
    #[error("conflict: {0}")]
    Conflict(String),
}

impl CarpenterError {
    /// The envelope `code` string (the variant name).
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "NotFound",
            Self::AlreadyExists(_) => "AlreadyExists",
            Self::ValidationError(_) => "ValidationError",
            Self::StoreError(_) => "StoreError",
            Self::ExecuteError { .. } => "ExecuteError",
            Self::Conflict(_) => "Conflict",
        }
    }

    /// The envelope `details` payload. Populated as variants gain structured data.
    pub fn details(&self) -> Value {
        match self {
            Self::ExecuteError { details, .. } => details.clone(),
            _ => json!({}),
        }
    }
}

#[cfg(test)]
#[test]
fn codes_match_variant_names() {
    use CarpenterError::*;
    assert_eq!(NotFound("x".into()).code(), "NotFound");
    assert_eq!(AlreadyExists("x".into()).code(), "AlreadyExists");
    assert_eq!(ValidationError("x".into()).code(), "ValidationError");
    assert_eq!(StoreError("x".into()).code(), "StoreError");
    assert_eq!(
        ExecuteError {
            message: "x".into(),
            details: json!({}),
        }
        .code(),
        "ExecuteError"
    );
    assert_eq!(Conflict("x".into()).code(), "Conflict");
}
