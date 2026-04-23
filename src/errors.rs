use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "diagnostics", derive(Diagnostic))]
pub enum FileOperation {
    #[error("reading a file")]
    Read,
    #[error("writing a file")]
    Write,
    #[error("creating a directory")]
    Mkdir,
}

#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "diagnostics", derive(Diagnostic))]
#[error("I/O error: {operation} on path '{path}'")]
#[cfg_attr(
    feature = "diagnostics",
    diagnostic(
        code(achitek::io),
        help("Check file permissions, disk space, or that the path is correct.")
    )
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
