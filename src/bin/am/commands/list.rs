use am_list::{FunctionInfo, Language, ListAmFunctions};
use clap::{Args, Subcommand};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use tracing::info;

// TODO(gagbo): add an additional subcommand that makes use of am_list::find_roots to
// list all the functions under a given folder, by detecting the languages and all the
// subprojects included.

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
    let projects = am_list::find_project_roots(&root)?;
    let mut res: BTreeMap<String, Vec<FunctionInfo>> = BTreeMap::new();

    // TODO: try to parallelize this loop if possible
    for (path, language) in projects.iter() {
        info!(
            "Listing functions in {} (Language: {})",
            path.display(),
            language
        );
        let project_fns = list_single_project_functions(path, *language, true)?;

        res.entry(path.to_string_lossy().to_string())
            .or_default()
            .extend(project_fns);
    }

    if args.pretty {
        println!("{}", serde_json::to_string_pretty(&res)?);
    } else {
        println!("{}", serde_json::to_string(&res)?);
    }
    info!(
        "Total: {} functions",
        res.values().map(|list| list.len()).sum::<usize>()
    );

    Ok(())
}

fn handle_single_project(args: SingleProject) -> Result<(), anyhow::Error> {
    let root = args.root;
    info!("Autometrics functions in {}:", root.display());

    let res = list_single_project_functions(&root, args.language, args.all_functions)?;

    if args.pretty {
        println!("{}", serde_json::to_string_pretty(&res)?);
    } else {
        println!("{}", serde_json::to_string(&res)?);
    }
    info!("Total: {} functions", res.len());

    Ok(())
}

fn list_single_project_functions(
    root: &Path,
    language: Language,
    all_functions: bool,
) -> Result<Vec<FunctionInfo>, anyhow::Error> {
    let mut implementor: Box<dyn ListAmFunctions> = match language {
        Language::Rust => Box::new(am_list::rust::Impl {}),
        Language::Go => Box::new(am_list::go::Impl {}),
        Language::Typescript => Box::new(am_list::typescript::Impl {}),
        Language::Python => Box::new(am_list::python::Impl {}),
    };
    let mut res = if all_functions {
        implementor.list_all_functions(root)?
    } else {
        implementor.list_autometrics_functions(root)?
    };
    res.sort();
    Ok(res)
}
