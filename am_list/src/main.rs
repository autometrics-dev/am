use am_list::ListAmFunctions;
use clap::{Args, Parser, Subcommand};
use flexi_logger::{AdaptiveFormat, Logger};
use log::info;
use std::{path::PathBuf, str::FromStr};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List all the autometrics functions in the project, with their matching
    /// modules
    List(ListArgs),
}

#[derive(Args)]
struct ListArgs {
    /// Language to detect autometrics functions for.
    #[arg(short, long, value_name = "LANGUAGE")]
    language: Language,
    /// Root of the project to start the search on.
    /// - For Rust projects it must be where the Cargo.toml lie,
    /// - For Go projects it must be the root of the repository.
    #[arg(value_name = "ROOT")]
    root: PathBuf,
    /// List all functions instead of only the autometricized ones (defaults to false)
    #[arg(short, long, default_value = "false")]
    all_functions: bool,
    /// Pretty print the resulting JSON (defaults to false)
    #[arg(short, long, default_value = "false")]
    pretty: bool,
}

#[derive(Clone, Copy)]
enum Language {
    Rust,
    Go,
    Typescript,
    Python,
}

impl FromStr for Language {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let discriminant = s.to_lowercase();
        if ["rust", "rs"].contains(&discriminant.as_str()) {
            return Ok(Self::Rust);
        }

        if discriminant == "go" {
            return Ok(Self::Go);
        }

        if ["typescript", "ts", "javascript", "js"].contains(&discriminant.as_str()) {
            return Ok(Self::Typescript);
        }

        if ["python", "py"].contains(&discriminant.as_str()) {
            return Ok(Self::Python);
        }

        Err(format!("Unknown language: {s}"))
    }
}

fn main() -> anyhow::Result<()> {
    Logger::try_with_env()?
        .adaptive_format_for_stderr(AdaptiveFormat::Detailed)
        .start()?;
    let args = Cli::try_parse()?;

    match args.command {
        Command::List(args) => {
            let root = args.root;
            info!("Autometrics functions in {}:", root.display());

            let mut implementor: Box<dyn ListAmFunctions> = match args.language {
                Language::Rust => Box::new(am_list::rust::Impl {}),
                Language::Go => Box::new(am_list::go::Impl {}),
                Language::Typescript => Box::new(am_list::typescript::Impl {}),
                Language::Python => Box::new(am_list::python::Impl {}),
            };

            let mut res = if args.all_functions {
                implementor.list_all_functions(&root)?
            } else {
                implementor.list_autometrics_functions(&root)?
            };

            res.sort();
            if args.pretty {
                println!("{}", serde_json::to_string_pretty(&res)?);
            } else {
                println!("{}", serde_json::to_string(&res)?);
            }
            info!("Total: {} functions", res.len());

            Ok(())
        }
    }
}
