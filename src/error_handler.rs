use actix_web::{
    error::ResponseError,
    http::StatusCode,
    HttpResponse,
};
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    InternalError(String),
    DatabaseError(String),
    AuthenticationError(String),
    #[allow(dead_code)]  // Keep for potential future use
    ValidationError(String),
    #[allow(dead_code)]  // Keep for potential future use
    NotFound(String),
    RateLimitExceeded,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::InternalError(msg) => write!(f, "Internal server error: {}", msg),
            AppError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            AppError::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
            AppError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::RateLimitExceeded => write!(f, "Rate limit exceeded"),
        }
    }
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        let (status_code, error_message) = match self {
            AppError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AppError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error occurred".to_string()),
            AppError::AuthenticationError(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::RateLimitExceeded => (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded. Please try again later.".to_string()),
        };
        
        HttpResponse::build(status_code)
            .body(format!("Error {}: {}", status_code.as_u16(), error_message))
    }
    
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::InternalError(_) | AppError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::AuthenticationError(_) => StatusCode::UNAUTHORIZED,
            AppError::ValidationError(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::RateLimitExceeded => StatusCode::TOO_MANY_REQUESTS,
        }
    }
}

// Conversion helpers
impl From<async_sqlite::Error> for AppError {
    fn from(err: async_sqlite::Error) -> Self {
        AppError::DatabaseError(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::InternalError(err.to_string())
    }
}

// Helper function to wrap results - removed as unused
// If needed in the future, use: result.map_err(|e| ErrorInternalServerError(e))

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_display() {
        let err = AppError::ValidationError("Invalid input".to_string());
        assert_eq!(err.to_string(), "Validation error: Invalid input");
        
        let err = AppError::RateLimitExceeded;
        assert_eq!(err.to_string(), "Rate limit exceeded");
    }
    
    #[test]
    fn test_error_status_codes() {
        assert_eq!(AppError::InternalError("test".to_string()).status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(AppError::ValidationError("test".to_string()).status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(AppError::AuthenticationError("test".to_string()).status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(AppError::NotFound("test".to_string()).status_code(), StatusCode::NOT_FOUND);
        assert_eq!(AppError::RateLimitExceeded.status_code(), StatusCode::TOO_MANY_REQUESTS);
    }
}