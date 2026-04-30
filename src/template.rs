use crate::{
    errors::IoError,
    prompt::{Answer, PromptError, apply_changes, get_answers},
    source::Source,
    utils::{
        path, preview,
        transaction::{Active, FinalTransactionState, Transaction},
        vfs::{VfsError, apply_vfs, build_vfs},
    },
};
use indexmap::IndexMap;
use miette::Diagnostic;
use tera::{Context, Tera};
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum TemplateError {
    #[error("template file operation failed")]
    #[diagnostic(code(achitek::template::io))]
    Io(#[from] IoError),

    #[error("failed to build or apply generated files")]
    #[diagnostic(code(achitek::template::vfs))]
    Vfs(#[from] VfsError),

    #[error("template not found: {name}")]
    #[diagnostic(
        code(achitek::template::project_not_found),
        help("Check the template name or run the list command to see available templates.")
    )]
    ProjectNotFound { name: String },

    #[error("failed to collect template prompt answers")]
    #[diagnostic(code(achitek::template::prompt))]
    Prompt(#[from] PromptError),

    #[error("blueprint path is not valid UTF-8: {path}")]
    #[diagnostic(
        code(achitek::template::invalid_project_string_unicode),
        help("Move or rename the blueprint directory to a path that can be represented as UTF-8.")
    )]
    InvalidProjectStringUnicode { path: std::path::PathBuf },

    #[error("failed to load template files with pattern: {pattern}")]
    #[diagnostic(code(achitek::template::tera_instance_initialization))]
    TeraInstanceInitialization {
        pattern: String,
        #[source]
        source: tera::Error,
    },

    #[error("failed to generate output file name for path: {path}")]
    #[diagnostic(code(achitek::template::generate_filename))]
    GenerateFileName { path: std::path::PathBuf },

    #[error("failed to render template with collected answers")]
    #[diagnostic(code(achitek::template::render))]
    Render {
        context: Context,
        #[source]
        source: tera::Error,
    },

    #[error("failed to make path relative to blueprint directory: {path}")]
    #[diagnostic(code(achitek::template::strip_prefix))]
    StripPrefix {
        path: std::path::PathBuf,
        dir: std::path::PathBuf,
        source: std::path::StripPrefixError,
    },
}

/// Renders the specified template from the given [`Source`] into `destination`,
pub fn try_render(
    config: Source,
    template: &str,
    destination: &str,
) -> Result<FinalTransactionState, TemplateError> {
    let path_to_blueprint = config
        .blueprints
        .get(template)
        .ok_or_else(|| TemplateError::ProjectNotFound {
            name: template.to_string(),
        })?
        .path
        .clone();

    let blueprint_directory = config.source_dir.join(path::normalize(&path_to_blueprint));

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
