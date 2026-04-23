use crate::{
    prompt,
    source::{self, Source},
    template,
};
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum AchitekError {
    #[error("Template not found with name: {name}")]
    #[diagnostic(
        code(achitek::template_not_found),
        help("Check the template name or run the list command to select an available template.")
    )]
    TemplateNotFound { name: String },

    #[error("Destination already exists: {path}")]
    #[diagnostic(
        code(achitek::destination_already_exists),
        help("Choose a different destination or remove the existing path.")
    )]
    DestinationAlreadyExists { path: PathBuf },

    #[error("operation failed")]
    #[diagnostic(code(achitek::operation_failed))]
    OperationFailed {
        #[source]
        source: anyhow::Error,
    },
}

impl From<source::SourceError> for AchitekError {
    fn from(source: source::SourceError) -> Self {
        Self::OperationFailed {
            source: anyhow::Error::new(source),
        }
    }
}

impl From<template::TemplateError> for AchitekError {
    fn from(source: template::TemplateError) -> Self {
        match source {
            template::TemplateError::ProjectNotFound { name } => Self::TemplateNotFound { name },
            source => Self::OperationFailed {
                source: anyhow::Error::new(source),
            },
        }
    }
}

impl From<prompt::PromptError> for AchitekError {
    fn from(source: prompt::PromptError) -> Self {
        Self::OperationFailed {
            source: anyhow::Error::new(source),
        }
    }
}

fn ensure_destination_available(destination: &str) -> Result<(), AchitekError> {
    let path = Path::new(destination);
    if path.exists() {
        return Err(AchitekError::DestinationAlreadyExists {
            path: path.to_path_buf(),
        });
    }

    Ok(())
}

/// Copies a template from the specified source directory to the provided destination path.
///
/// # Errors
///
/// Returns a [`AchitekError`] if:
///
/// - The configuration could not be built from the `source`.
/// - The template or its files cannot be located or read.
/// - A directory or file cannot be created or written to.
/// - Tera fails to initialize or render a template.
pub fn copy_template(src: &str, template: &str, destination: &str) -> Result<(), AchitekError> {
    ensure_destination_available(destination)?;

    let source = Source::build_from(src)?;

    log::debug!(
        "Attempting to build source from: {}",
        source.source_dir.display()
    );

    template::try_render(source, template, destination)?;

    Ok(())
}

/// Interactively lists and selects a template from the specified source directory, then copies it
/// to a user-provided destination path.
///
/// This function also builds a [`Source`] from the given `source`, then prompts the user to
/// select a template and a destination directory.  files.
///
/// # Errors
///
/// Returns a [`AchitekError`] if:
///
/// - The configuration could not be built from the `source`.
/// - User prompts fail or the user cancels the input.
/// - The template or its files cannot be located or read.
/// - A directory or file cannot be created or written to.
/// - Tera fails to initialize or render a template.
pub fn list_templates(src: &str) -> Result<(), AchitekError> {
    let source = Source::build_from(src)?;

    let template = prompt::get_project(source.clone())?;

    let destination = prompt::get_destination()?;

    ensure_destination_available(&destination)?;

    template::try_render(source, &template, &destination)?;

    Ok(())
}
