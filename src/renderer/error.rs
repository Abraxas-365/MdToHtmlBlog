use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RendererError {
    #[error("Failed to read file: {path}")]
    FileReadError {
        path: String,
        #[source]
        source: io::Error,
    },

    #[error("Failed to write file: {path}")]
    FileWriteError {
        path: String,
        #[source]
        source: io::Error,
    },

    #[error("Failed to parse markdown content")]
    MarkdownParseError(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("Failed to set markdown language")]
    LanguageError(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("Template error: {0}")]
    TemplateError(String),

    #[error("Invalid path: {0}")]
    InvalidPathError(String),

    #[error("Missing required metadata: {0}")]
    MissingMetadataError(String),
}
