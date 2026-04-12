use clap::{Parser, Subcommand};
use env_logger::Builder;
use kopye::api::KopyeError;
use log::LevelFilter;
use miette::Result as MietteResult;

#[derive(Debug, Clone, Parser)]
#[command(about, version)]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
    #[arg(long, short, global = true, help = "Enable verbose output")]
    // .action(ArgAction::SetTrue)
    verbose: bool,
}

#[derive(Debug, Clone, Subcommand)]
enum Commands {
    #[command(about = "Copies a template from a repo reference to a destination")]
    Copy {
        #[arg(
            help = "git repository reference where templates live",
            long_help = r#"
git repository reference where templates live

Examples:
  gh:account/repo              - GitHub repository
  gl:account/repo              - GitLab repository
  git@host:account/repo.git    - SSH git URL
  git+https://example.com/...  - HTTPS git URL
            "#,
            required = true
        )]
        repo: String,
        #[arg(help = "template name", required = true)]
        template: String,
        #[arg(
            help = "The destination directory where the project will be created",
            required = true
        )]
        destination: String,
    },
    #[command(about = "list template")]
    List {
        #[arg(
            help = "git repository reference where templates live",
            long_help = r#"
git repository reference where templates live

Examples:
  gh:account/repo              - GitHub repository
  gl:account/repo              - GitLab repository
  git@host:account/repo.git    - SSH git URL
  git+https://example.com/...  - HTTPS git URL
            "#,
            required = true
        )]
        repo: String,
    },
}

fn main() -> MietteResult<()> {
    let args = Cli::parse();

    init_logger(args.verbose);

    match args.commands {
        Commands::Copy {
            repo,
            template,
            destination,
        } => {
            handle_copy(repo, template, destination).map_err(miette::Report::new)?;

            Ok(())
        }
        Commands::List { repo } => {
            handle_list(repo).map_err(miette::Report::new)?;

            Ok(())
        }
    }
}

fn init_logger(verbose: bool) {
    let mut builder = Builder::from_default_env();

    let level = if verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Warn
    };

    builder.filter(None, level);

    builder.init();
}

fn handle_copy(repo: String, template: String, destination: String) -> Result<(), KopyeError> {
    kopye::api::copy_template(&repo, &template, &destination)
}

fn handle_list(repo: String) -> Result<(), KopyeError> {
    kopye::api::list_templates(&repo)
}
