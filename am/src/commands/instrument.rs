use am_list::Language;
use anyhow::Context;
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
    /// Instrument functions in a single project, giving the language implementation
    ///
    /// IMPORTANT: This will add code in your files! If you want to easily
    /// undo the effects of this command, stage your work in progress (using `git add` or similar)
    /// So that a command like `git restore .` can undo all unstaged changes, leaving your work
    /// in progress alone.
    Single(SingleProject),
    /// Instrument functions in all projects under the given directory, detecting languages on a best-effort basis.
    ///
    /// IMPORTANT: This will add code in your files! If you want to easily
    /// undo the effects of this command, stage your work in progress (using `git add` or similar)
    /// So that a command like `git restore .` can undo all unstaged changes, leaving your work
    /// in progress alone.
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
    /// A list of patterns to exclude from instrumentation. The patterns follow .gitignore rules, so
    /// `--exclude "/vendor/"` will exclude all the vendor subdirectory only at the root, and adding
    /// a pattern that starts with `!` will unignore a file or directory
    #[arg(short, long, value_name = "PATTERNS")]
    exclude: Vec<String>,
}

#[derive(Args)]
struct AllProjects {
    /// Main directory to start the subprojects search on. am currently detects
    /// Rust (Cargo.toml), Typescript (package.json), and Golang (go.mod)
    /// projects.
    #[arg(value_name = "ROOT")]
    root: PathBuf,
    /// A list of patterns to exclude from instrumentation. The patterns follow .gitignore rules, so
    /// `--exclude "/vendor/"` will exclude all the vendor subdirectory only at the root, and adding
    /// a pattern that starts with `!` will unignore a file or directory
    #[arg(short, long, value_name = "PATTERNS")]
    exclude: Vec<String>,
}

pub fn handle_command(args: Arguments) -> anyhow::Result<()> {
    match args.command {
        Command::Single(args) => handle_single_project(args),
        Command::All(args) => handle_all_projects(args),
    }
}

fn handle_all_projects(args: AllProjects) -> Result<(), anyhow::Error> {
    let root = args
        .root
        .canonicalize()
        .context("The path must be resolvable to an absolute path")?;
    info!("Instrumenting functions in {}:", root.display());

    let mut exclude_patterns_builder = ignore::gitignore::GitignoreBuilder::new(&root);
    for pattern in args.exclude {
        exclude_patterns_builder.add_line(None, &pattern)?;
    }
    let exclude_patterns = exclude_patterns_builder.build()?;

    am_list::instrument_all_project_files(&root, &exclude_patterns)?;

    println!("If your project has Golang files, you need to run `go generate` now.");

    Ok(())
}

fn handle_single_project(args: SingleProject) -> Result<(), anyhow::Error> {
    let root = args
        .root
        .canonicalize()
        .context("The path must be resolvable to an absolute path")?;
    info!("Instrumenting functions in {}:", root.display());

    let mut exclude_patterns_builder = ignore::gitignore::GitignoreBuilder::new(&root);
    for pattern in args.exclude {
        exclude_patterns_builder.add_line(None, &pattern)?;
    }
    let exclude_patterns = exclude_patterns_builder.build()?;

    am_list::instrument_single_project_files(&root, args.language, &exclude_patterns)?;

    if args.language == Language::Go {
        println!("You need to run `go generate` now.");
    }

    Ok(())
}
