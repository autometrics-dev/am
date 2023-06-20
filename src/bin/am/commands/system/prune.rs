use crate::interactive;
use anyhow::{bail, Context, Result};
use clap::Parser;
use directories::ProjectDirs;
use std::io;
use tracing::{debug, info};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Arguments {
    /// Force the cleanup without asking for confirmation.
    #[clap(short, long, default_value = "false")]
    force: bool,
}

pub async fn handle_command(args: Arguments) -> Result<()> {
    // If the users hasn't specified the `force` argument, then ask the user if
    // they want to continue.
    if !args.force && !interactive::confirm("Prune all am program files?")? {
        bail!("Pruning cancelled");
    }

    // Get local directory
    let project_dirs =
        ProjectDirs::from("", "autometrics", "am").context("Unable to determine home directory")?;
    let local_data = project_dirs.data_local_dir().to_owned();

    debug!("Deleting all content from {:?}", local_data);

    // For now just greedily delete everything in the local data directory for am
    if let Err(err) = remove_dir_all::remove_dir_contents(&local_data) {
        // If the root directory does not exist, we can ignore the error (NOTE:
        // I don't know if it is possible to get this error in any other
        // situations)
        if err.kind() != io::ErrorKind::NotFound {
            return Err(err.into());
        }
    }

    info!("Pruning complete");
    Ok(())
}
