#[cfg(feature = "diagnostics")]
use miette::Diagnostic;
use tera::Context;

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
pub enum VfsError {
    #[error("I/O error within VFS operations")]
    #[cfg_attr(feature = "diagnostics", diagnostic(code(kopye_utils::vfs::io)))]
    Io(#[from] IoError),

    #[error("Error occurrend attempting to render template")]
    #[cfg_attr(feature = "diagnostics", diagnostic(code(kopye_utils::vfs::render)))]
    Render {
        context: Context,
        #[source]
        source: tera::Error,
    },

    #[error("unable to strip prefix from directory")]
    #[cfg_attr(feature = "diagnostics", diagnostic(code(kopye_utils::vfs::strip_prefix)))]
    StripPrefix {
        path: std::path::PathBuf,
        dir: std::path::PathBuf,
        source: std::path::StripPrefixError,
    },
}

#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "diagnostics", derive(Diagnostic))]
#[error("I/O error: {operation} on path '{path}'")]
#[cfg_attr(
    feature = "diagnostics",
    diagnostic(
        code(kopye_utils::io),
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
