use am_list::Language;
use clap::{Args, Subcommand};
use std::path::PathBuf;
use tracing::info;

#[derive(Args)]
pub struct Arguments {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List functions in a single project, giving the language implementation
    Single(SingleProject),
    /// List functions in all projects under the given directory, detecting languages on a best-effort basis.
    All(AllProjects),
}

#[derive(Args)]
struct SingleProject {
    /// Language to detect autometrics functions for. Valid values are:
    /// - 'rust' or 'rs' for Rust,
    /// - 'go' for Golang,
    /// - 'typescript', 'ts', 'javascript', or 'js' for Typescript/Javascript,
    /// - 'python' or 'py' for Python.
    #[arg(short, long, value_name = "LANGUAGE", verbatim_doc_comment)]
    language: Language,
    /// Root of the project to start the search on:
    /// - For Rust projects it must be where the Cargo.toml lie,
    /// - For Go projects it must be the root of the repository,
    /// - For Python projects it must be the root of the library,
    /// - For Typescript projects it must be where the package.json lie.
    #[arg(value_name = "ROOT", verbatim_doc_comment)]
    root: PathBuf,
    /// List all functions instead of only the autometricized ones (defaults to false)
    #[arg(short, long, default_value = "false")]
    all_functions: bool,
    /// Pretty print the resulting JSON (defaults to false)
    #[arg(short, long, default_value = "false")]
    pretty: bool,
}

#[derive(Args)]
struct AllProjects {
    /// Main directory to start the subprojects search on. am currently detects
    /// Rust (Cargo.toml), Typescript (package.json), and Golang (go.mod)
    /// projects.
    #[arg(value_name = "ROOT")]
    root: PathBuf,
    /// Pretty print the resulting JSON (defaults to false)
    #[arg(short, long, default_value = "false")]
    pretty: bool,
}

pub fn handle_command(args: Arguments) -> anyhow::Result<()> {
    match args.command {
        Command::Single(args) => handle_single_project(args),
        Command::All(args) => handle_all_projects(args),
    }
}

fn handle_all_projects(args: AllProjects) -> Result<(), anyhow::Error> {
    let root = args.root;
    info!("Listing functions in {}:", root.display());
    let res = am_list::list_all_project_functions(&root)?;

    if args.pretty {
        println!("{}", serde_json::to_string_pretty(&res)?);
    } else {
        println!("{}", serde_json::to_string(&res)?);
    }
    info!(
        "Total: {} functions",
        res.values().map(|list| list.1.len()).sum::<usize>()
    );

    Ok(())
}

fn handle_single_project(args: SingleProject) -> Result<(), anyhow::Error> {
    let root = args.root;
    info!("Autometrics functions in {}:", root.display());

    let res = am_list::list_single_project_functions(&root, args.language, args.all_functions)?;

    if args.pretty {
        println!("{}", serde_json::to_string_pretty(&res)?);
    } else {
        println!("{}", serde_json::to_string(&res)?);
    }
    info!("Total: {} functions", res.len());

    Ok(())
}
