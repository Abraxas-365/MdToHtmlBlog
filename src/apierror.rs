use actix_web::{HttpResponse, ResponseError};
use serde::Serialize;
use thiserror::Error;

use crate::renderer::error::RendererError;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Internal Server Error: {0}")]
    InternalError(String),

    #[error("Not Found: {0}")]
    NotFound(String),

    #[error("Bad Request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Validation Error: {0}")]
    ValidationError(String),

    #[error(transparent)]
    RendererError(#[from] RendererError),
}

#[derive(Serialize)]
struct ErrorResponse {
    code: u16,
    message: String,
    error: String,
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        let error_response = ErrorResponse {
            code: status_code.as_u16(),
            message: self.to_string(),
            error: self.error_type(),
        };

        HttpResponse::build(status_code).json(error_response)
    }

    fn status_code(&self) -> actix_web::http::StatusCode {
        use actix_web::http::StatusCode;

        match self {
            ApiError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            ApiError::ValidationError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::RendererError(err) => match err {
                RendererError::FileReadError { .. } => StatusCode::NOT_FOUND,
                RendererError::FileWriteError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
                RendererError::MarkdownParseError(_) => StatusCode::BAD_REQUEST,
                RendererError::LanguageError(_) => StatusCode::INTERNAL_SERVER_ERROR,
                RendererError::TemplateError(_) => StatusCode::INTERNAL_SERVER_ERROR,
                RendererError::InvalidPathError(_) => StatusCode::NOT_FOUND,
                RendererError::MissingMetadataError(_) => StatusCode::BAD_REQUEST,
            },
        }
    }
}

impl ApiError {
    fn error_type(&self) -> String {
        match self {
            ApiError::InternalError(_) => "INTERNAL_ERROR",
            ApiError::NotFound(_) => "NOT_FOUND",
            ApiError::BadRequest(_) => "BAD_REQUEST",
            ApiError::Unauthorized(_) => "UNAUTHORIZED",
            ApiError::Forbidden(_) => "FORBIDDEN",
            ApiError::ValidationError(_) => "VALIDATION_ERROR",
            ApiError::RendererError(err) => match err {
                RendererError::FileReadError { .. } => "FILE_READ_ERROR",
                RendererError::FileWriteError { .. } => "FILE_WRITE_ERROR",
                RendererError::MarkdownParseError(_) => "MARKDOWN_PARSE_ERROR",
                RendererError::LanguageError(_) => "LANGUAGE_ERROR",
                RendererError::TemplateError(_) => "TEMPLATE_ERROR",
                RendererError::InvalidPathError(_) => "INVALID_PATH_ERROR",
                RendererError::MissingMetadataError(_) => "MISSING_METADATA_ERROR",
            },
        }
        .to_string()
    }
}

// Convenience methods for creating errors
impl ApiError {
    pub fn internal_error<T: ToString>(message: T) -> Self {
        ApiError::InternalError(message.to_string())
    }

    pub fn not_found<T: ToString>(message: T) -> Self {
        ApiError::NotFound(message.to_string())
    }

    pub fn bad_request<T: ToString>(message: T) -> Self {
        ApiError::BadRequest(message.to_string())
    }

    pub fn unauthorized<T: ToString>(message: T) -> Self {
        ApiError::Unauthorized(message.to_string())
    }

    pub fn forbidden<T: ToString>(message: T) -> Self {
        ApiError::Forbidden(message.to_string())
    }

    pub fn validation_error<T: ToString>(message: T) -> Self {
        ApiError::ValidationError(message.to_string())
    }
}
