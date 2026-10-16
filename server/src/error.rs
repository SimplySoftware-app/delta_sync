use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    NotConfigured(String),
    #[error("{0}")]
    Internal(String),
}

impl ApiError {
    pub fn bad_request(err: impl ToString) -> Self {
        Self::BadRequest(err.to_string())
    }

    pub fn internal(err: impl ToString) -> Self {
        Self::Internal(err.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotConfigured(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        if matches!(self, Self::Internal(_)) {
            tracing::error!(error = %self, "request failed");
        }
        let message = self.to_string();
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}
