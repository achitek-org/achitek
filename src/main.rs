use achitek::api::AchitekError;
use clap::{Parser, Subcommand};
use env_logger::Builder;
use log::LevelFilter;
use miette::Result as MietteResult;

#[derive(Debug, Clone, Parser)]
#[command(about, version)]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
    #[arg(long, short, global = true, help = "Show detailed output")]
    // .action(ArgAction::SetTrue)
    verbose: bool,
}

#[derive(Debug, Clone, Subcommand)]
enum Commands {
    #[command(about = "Copy a template from a repository to a directory")]
    Copy {
        #[arg(
            help = "Reference to the Git repository that contains the templates. Use --help for examples.",
            long_help = r#"
Reference to the Git repository that contains the templates

Examples:
  gh:account/repo              - GitHub repository
  gl:account/repo              - GitLab repository
  git@host:account/repo.git    - SSH git URL
  git+https://example.com/...  - HTTPS git URL
            "#,
            required = true
        )]
        repo: String,
        #[arg(help = "Name of the template", required = true)]
        template: String,
        #[arg(help = "Directory in which to create the project", required = true)]
        destination: String,
    },
    #[command(about = "Select a template and create a project")]
    New {
        #[arg(
            help = "Reference to the Git repository that contains the templates. Use --help for examples.",
            long_help = r#"
Reference to the Git repository that contains the templates

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
        Commands::New { repo } => {
            handle_new(repo).map_err(miette::Report::new)?;

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

fn handle_copy(repo: String, template: String, destination: String) -> Result<(), AchitekError> {
    achitek::api::copy_template(&repo, &template, &destination)
}

fn handle_new(repo: String) -> Result<(), AchitekError> {
    achitek::api::create_template_interactively(&repo)
}
