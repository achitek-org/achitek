use crate::{
    prompt::{apply_changes, get_answers, Answer, PromptError},
    source::Source,
    utils::normalize_path,
};
use indexmap::IndexMap;
use kopye_utils::{
    error::{IoError, VfsError},
    preview,
    transaction::{Active, FinalTransactionState, Transaction},
    vfs::{apply_vfs, build_vfs},
};
use miette::Diagnostic;
use tera::{Context, Tera};
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum TemplateError {
    #[error("I/O error within template domain")]
    #[diagnostic(code(kopye::template::io))]
    Io(#[from] IoError),

    #[error("VFS operation failed")]
    #[diagnostic(code(kopye::template::vfs))]
    Vfs(#[from] VfsError),

    #[error("Project not found with name: {name}")]
    #[diagnostic(
        code(kopye::template::project_not_found),
        help("Make sure project is available -> point to documentation about creating projects")
    )]
    ProjectNotFound { name: String },

    #[error("Error occurred trying to prompt user")]
    #[diagnostic(code(kopye::template::prompt))]
    Prompt(#[from] PromptError),

    #[error("Error occurred trying to convert blueprint directory to string")]
    #[diagnostic(
        code(kopye::template::invalid_project_string_unicode),
        help("Please check the path")
    )]
    InvalidProjectStringUnicode { path: std::path::PathBuf },

    #[error("Error occurred attempting to initialize tera instance")]
    #[diagnostic(code(kopye::template::tera_instance_initialization))]
    TeraInstanceInitialization {
        pattern: String,
        #[source]
        source: tera::Error,
    },

    #[error("Error occurred attempting to generate out file name")]
    #[diagnostic(code(kopye::template::generate_filename))]
    GenerateFileName { path: std::path::PathBuf },

    #[error("Error occurrend attempting to render template")]
    #[diagnostic(code(kopye::template::render))]
    Render {
        context: Context,
        #[source]
        source: tera::Error,
    },

    #[error("unable to strip prefix from directory")]
    #[diagnostic(code(kopye::template::strip_prefix))]
    StripPrefix {
        path: std::path::PathBuf,
        dir: std::path::PathBuf,
        source: std::path::StripPrefixError,
    },
}

/// Makes a [`Tera`] [`Context`] object, hydrated with user prompt answers.
fn make_tera_context(answers: IndexMap<String, Answer>) -> Context {
    let mut base_ctx = Context::new();
    for (key, answer) in answers {
        match answer {
            Answer::String(ans) => base_ctx.insert(&key, &ans),
            Answer::Bool(ans) => base_ctx.insert(&key, &ans),
            Answer::Array(ans) => base_ctx.insert(&key, &ans),
        }
    }

    base_ctx.clone()
}
/// Renders the specified template from the given [`Source`] into `destination`,
pub fn try_render(
    config: Source,
    template: &str,
    destination: &str,
) -> Result<FinalTransactionState, TemplateError> {
    let path_to_blueprint = config
        .projects
        .get(template)
        .ok_or_else(|| TemplateError::ProjectNotFound {
            name: template.to_string(),
        })?
        .path
        .clone();

    let blueprint_directory = config.source_dir.join(normalize_path(&path_to_blueprint));

    let answers = get_answers(&blueprint_directory)?;

    let tera_context = make_tera_context(answers);

    let pattern = format!("{}/**/*.tera", blueprint_directory.display());

    let mut tera = Tera::new(&pattern)
        .map_err(|e| TemplateError::TeraInstanceInitialization { pattern, source: e })?;

    let vfs = build_vfs(&blueprint_directory, &mut tera, &tera_context)?;

    let destination_path = std::path::PathBuf::from(destination);

    preview::as_tree(&vfs, &destination_path);

    let mut trx = Transaction::<Active>::new();

    if apply_changes()? {
        apply_vfs(&vfs, &destination_path, &mut trx)?;

        Ok(FinalTransactionState::Committed(trx.commit()))
    } else {
        Ok(FinalTransactionState::Canceled(trx.cancel()))
    }
}
