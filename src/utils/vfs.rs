use crate::{
    errors::{FileOperation, IoError},
    utils::transaction::{Active, RollbackOperation, Transaction},
};
use colored::Colorize;
#[cfg(feature = "diagnostics")]
use miette::Diagnostic;
use std::path::{Path, PathBuf};
use tera::{Context, Tera};
use walkdir::WalkDir;

const TERA_FILE_EXTENSION: &str = "tera";
const CONFIG_FILE_NAME: &str = "Achitekfile";

#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "diagnostics", derive(Diagnostic))]
pub enum VfsError {
    #[error("I/O error within VFS operations")]
    #[cfg_attr(feature = "diagnostics", diagnostic(code(achitek_utils::vfs::io)))]
    Io(#[from] IoError),

    #[error("Error occurrend attempting to render template")]
    #[cfg_attr(feature = "diagnostics", diagnostic(code(achitek_utils::vfs::render)))]
    Render {
        context: Context,
        #[source]
        source: tera::Error,
    },

    #[error("unable to strip prefix from directory")]
    #[cfg_attr(
        feature = "diagnostics",
        diagnostic(code(achitek_utils::vfs::strip_prefix))
    )]
    StripPrefix {
        path: std::path::PathBuf,
        dir: std::path::PathBuf,
        source: std::path::StripPrefixError,
    },
}

/// Represents a virtual file or directory entry to be created in memory before writing to disk.
///
/// This struct can be used to stage content for a file system operation such as rendering
/// templates into a virtual environment.
#[derive(Debug, Clone)]
pub struct VirtualEntry {
    /// The target path where the file or directory should be written. If `None`,
    /// the entry may be skipped or dynamically resolved.
    pub destination: Option<std::path::PathBuf>,
    /// Optional contents to be written if the entry represents a file.
    pub content: Option<String>,
    /// Indicates whether this entry is a file (`true`) or a directory (`false`).
    pub is_file: bool,
}
/// Represents a virtual file system composed of multiple [`VirtualEntry`] values.
///
/// This structure can be used to queue up a collection of file or directory creations
/// before committing them to disk.
#[derive(Debug, Clone, Default)]
pub struct VirtualFS {
    pub entries: Vec<VirtualEntry>,
}
impl VirtualFS {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

/// Recursively walks the `blueprint_directory`, renders each path segment as a tera template
/// and builds up a [`VirtualFS`] of all directories and files that should be created.
pub fn build_vfs(
    source_directory: &Path,
    tera: &mut Tera,
    ctx: &Context,
) -> Result<VirtualFS, VfsError> {
    let mut vfs = VirtualFS::new();

    for entry in WalkDir::new(source_directory) {
        let entry = match entry {
            Ok(e) => e,
            Err(error) => {
                let path = error.path().unwrap_or_else(|| Path::new(""));

                Err(IoError::new(
                    FileOperation::Read,
                    path.to_path_buf(),
                    error.into(),
                ))?
            }
        };

        // skip template configuration file
        let file_name = entry.file_name().to_string_lossy();
        if file_name == CONFIG_FILE_NAME {
            continue;
        }

        let full_path = entry.path();
        let relative = match full_path.strip_prefix(source_directory) {
            Ok(r) => r,
            Err(error) => Err(VfsError::StripPrefix {
                path: full_path.to_path_buf(),
                dir: source_directory.to_path_buf(),
                source: error,
            })?,
        };

        // render the relative path segments/components as tera templates
        let rendered_rel_path = render_path_segments(relative, tera, ctx)?;

        // If `None`, at least one segment rendered to empty, therefore skip
        let Some(rendered_path) = rendered_rel_path else {
            // Skip this file or directory and it's children
            continue;
        };

        if entry.file_type().is_dir() {
            vfs.entries.push(VirtualEntry {
                destination: Some(rendered_path),
                content: None,
                is_file: false,
            });
        } else {
            let mut file_contents = std::fs::read_to_string(full_path).map_err(|error| {
                IoError::new(FileOperation::Read, full_path.to_path_buf(), error)
            })?;

            let mut final_dest = rendered_path.clone();

            let is_tera = rendered_path
                .extension()
                .map(|ext| ext == TERA_FILE_EXTENSION)
                .unwrap_or(false);

            // remove file extension and render file content if .tera extension detected
            if is_tera {
                let file_stem = final_dest.file_stem().unwrap_or_default().to_owned();
                final_dest.set_file_name(file_stem);

                let rendered =
                    tera.render_str(&file_contents, ctx)
                        .map_err(|error| VfsError::Render {
                            context: ctx.clone(),
                            source: error,
                        })?;

                file_contents = rendered;
            }

            vfs.entries.push(VirtualEntry {
                destination: Some(final_dest),
                content: Some(file_contents),
                is_file: true,
            });
        }
    }

    Ok(vfs)
}

/// Applies directory and file creation operations from a [`VirtualFS`].
pub fn apply_vfs(
    vfs: &VirtualFS,
    destination_root: &Path,
    trx: &mut Transaction<Active>,
) -> Result<(), VfsError> {
    // First create all directories
    for entry in vfs.entries.iter().filter(|e| !e.is_file) {
        let Some(rel_dest) = &entry.destination else {
            continue;
        };
        let final_path = destination_root.join(rel_dest);

        create_directory(trx, &final_path)?;
    }

    // Then create all files
    for entry in vfs.entries.iter().filter(|e| e.is_file) {
        let Some(rel_dest) = &entry.destination else {
            continue;
        };
        let final_path = destination_root.join(rel_dest);
        // create parent if necessary
        let parent = final_path.parent();
        if let Some(parent) = parent {
            create_directory(trx, parent)?;
        }

        let contents = entry.content.clone().unwrap_or_default();

        write_file(trx, &final_path, contents)?;
    }

    Ok(())
}

/// Creates all directories in the specified path if they do not exist.
///
/// This function uses [`std::fs::create_dir_all`] to ensure the entire directory path
/// is created. It then registers a [`RollbackOperation::RemoveDir`] on the provided
/// [`Transaction`] to support undoing the creation if needed.
///
/// # Errors
///
/// Returns a [`AchitekError`] if any directory creation fails due to I/O issues.
fn create_directory(trx: &mut Transaction<Active>, path: &std::path::Path) -> Result<(), VfsError> {
    std::fs::create_dir_all(path)
        .map_err(|error| IoError::new(FileOperation::Mkdir, path.into(), error))?;

    trx.add_operation(RollbackOperation::RemoveDir(path.to_path_buf()));

    Ok(())
}
/// Writes a file with the provided contents to the specified path.
///
/// After the file is created or overwritten, a [`RollbackOperation::RemoveFile`] operation
/// is registered in the [`Transaction`] for potential cleanup. Additionally, this
/// function prints a message to the console indicating that the file has been created.
///
/// # Errors
///
/// Returns a [`AchitekError`] if writing to the file fails due to I/O issues.
fn write_file(
    trx: &mut Transaction<Active>,
    path: &std::path::Path,
    contents: String,
) -> Result<(), VfsError> {
    std::fs::write(path, contents.clone())
        .map_err(|error| IoError::new(FileOperation::Write, path.into(), error))?;

    let msg = format!("{} {}", "create".green(), path.display());

    println!("{}", &msg);

    trx.add_operation(RollbackOperation::RemoveFile(path.to_path_buf()));

    Ok(())
}

/// Loops over path segments/components and renders them as tera templates and returns `Some(PathBuf)`
/// It returns `None` if ANY segment is empty (I.E parent directory is conditionally rendered).
///
/// For example, if your path segments are:
///   `["{% if integrations_tests %}tests{% endif %}", "{% if mocks %}mocks{% endif %}", "{{project}}.rs"]`
/// and `integrations_tests=false`, the first segment becomes `""`, so this returns `None`.
fn render_path_segments(
    path: &Path,
    tera: &mut Tera,
    ctx: &Context,
) -> Result<Option<PathBuf>, VfsError> {
    let mut result = PathBuf::new();

    for component in path.components() {
        let segment_str = component.as_os_str().to_string_lossy();

        let rendered = tera
            .render_str(&segment_str, ctx)
            .map_err(|error| VfsError::Render {
                context: ctx.clone(),
                source: error,
            })?;

        if rendered.trim().is_empty() {
            return Ok(None);
        }

        result.push(rendered.trim());
    }

    Ok(Some(result))
}
