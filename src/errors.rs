use miette::Diagnostic;
use thiserror::Error;

// TODO: Move to achitek_utils crate (needed for VFS operations)
#[derive(Debug, Error, Diagnostic)]
pub enum FileOperation {
    #[error("reading a file")]
    Read,
    #[error("writing a file")]
    Write,
    #[error("creating a directory")]
    Mkdir,
}
#[derive(Debug, Error, Diagnostic)]
#[error("I/O error: {operation} on path '{path}'")]
#[diagnostic(
    code(achitek::io),
    help("Check file permissions, disk space, or that the path is correct.")
)]
pub struct IoError {
    pub operation: FileOperation,
    pub path: std::path::PathBuf,
    #[source]
    pub source: std::io::Error,
}
impl IoError {
    pub fn new(operation: FileOperation, path: std::path::PathBuf, error: std::io::Error) -> Self {
        Self {
            operation,
            path,
            source: error,
        }
    }
}

#[derive(Debug, Error, Diagnostic)]
pub enum FileFormat {
    #[error("achitekfile")]
    Achitekfile,
}
#[derive(Debug, Error, Diagnostic)]
#[error("Parsing error: {file_format} on '{path}'")]
#[diagnostic(code(achitek::parse), help("Review file"))]
pub struct ParseError {
    pub file_format: FileFormat,
    pub path: std::path::PathBuf,
    #[source]
    pub source: Box<dyn std::error::Error + Send + Sync + 'static>,
}
impl ParseError {
    pub fn new<E>(file_format: FileFormat, path: std::path::PathBuf, error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self {
            file_format,
            path,
            source: Box::new(error),
        }
    }
}
