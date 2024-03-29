use crate::commands::start::CLIENT;
use crate::downloader::download_github_release;
use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use directories::ProjectDirs;
use indicatif::MultiProgress;
use itertools::Itertools;
use octocrab::models::repos::{Asset, Release};
use self_replace::self_replace;
use semver_rs::Version;
use std::fs::{File, OpenOptions};
use std::time::{Duration, SystemTime};
use std::{env, fs};
use tracing::{debug, error, info, trace, warn};

const AUTOMETRICS_GITHUB_ORG: &str = "autometrics-dev";
const AUTOMETRICS_AM_REPO: &str = "am";

#[derive(Parser)]
pub struct Arguments {
    /// Whenever to ignore Homebrew checks and forcefully update
    #[clap(long, short)]
    force: bool,
}

pub(crate) async fn handle_command(args: Arguments, mp: MultiProgress) -> Result<()> {
    let release = latest_release().await?;

    if !update_needed(&release)? {
        info!("Already on the latest version");
        return Ok(());
    }

    let new_tag = release.tag_name;

    if is_homebrew() && !args.force {
        info!("A new version of `am` is available: {new_tag}");
        info!("You can update by running `brew upgrade am` (or use `am update --force`)");
        return Ok(());
    }

    info!("Updating to {new_tag}");

    let asset_needed = asset_needed()?;

    let assets: Option<(&Asset, &Asset)> = release
        .assets
        .iter()
        .filter(|a| a.name.starts_with(asset_needed))
        .sorted_by(|a, b| a.name.cmp(&b.name))
        .collect_tuple();

    if assets.is_none() {
        error!("Could not find release for your target platform.");
        return Ok(());
    }

    // .unwrap is safe because we checked above if its none
    // because of .sorted_by above (which sorts by name), the .sha256 file will be the second one *guaranteed*
    let (binary_asset, sha256_asset) = assets.unwrap();

    let executable = env::current_exe()?;
    let temp_exe = executable
        .parent()
        .ok_or_else(|| anyhow!("Parent directory not found"))?
        .join("am_update.part");

    let file = File::create(&temp_exe)?;

    let calculated_checksum = download_github_release(
        &file,
        AUTOMETRICS_GITHUB_ORG,
        AUTOMETRICS_AM_REPO,
        new_tag.strip_prefix('v').unwrap_or(&new_tag),
        &binary_asset.name,
        &mp,
    )
    .await?;

    let checksum_line = CLIENT
        .get(sha256_asset.browser_download_url.clone())
        .send()
        .await?
        .text()
        .await?;

    let remote_checksum = checksum_line
        .split_once(' ')
        .map(|(checksum, _)| checksum)
        .unwrap_or(&checksum_line);

    if calculated_checksum != remote_checksum {
        debug!(
            %remote_checksum,
            %calculated_checksum, "Calculated sha256 hash does not match the remote sha256 hash"
        );

        fs::remove_file(&temp_exe).context("Failed to delete file that failed checksum match")?;
        drop(temp_exe);

        bail!("Calculated sha256 hash does not match the remote sha256 hash");
    }

    self_replace(&temp_exe).context("failed to replace self")?;
    fs::remove_file(&temp_exe).context("failed to delete updater file")?;

    info!("Successfully updated to {new_tag}");
    Ok(())
}

pub(crate) async fn update_check() {
    let Some(project_dirs) = ProjectDirs::from("", "autometrics", "am") else {
        warn!("failed to run update checker: home directory does not exist");
        return;
    };

    let config_dir = project_dirs.config_dir();

    if let Err(err) = fs::create_dir_all(config_dir) {
        error!(?err, "failed to create config directory");
        return;
    }

    let check_file = config_dir.join("version_check");

    let should_check = match fs::metadata(&check_file) {
        Ok(metadata) => {
            if let Ok(date) = metadata.modified() {
                date < (SystemTime::now() - Duration::from_secs(60 * 60 * 24))
            } else {
                false
            }
        }
        Err(err) => {
            // This will most likely be caused by the file not existing, so we
            // will just trace it and go ahead with the version check.
            trace!(%err, "checking the update file check resulted in a error");
            true
        }
    };

    // We've checked the version recently, so just return early indicating that
    // no update should be done.
    if !should_check {
        return;
    }

    let Ok(release) = latest_release().await else {
        return;
    };
    let Ok(needs_update) = update_needed(&release) else {
        return;
    };

    if let Err(err) = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&check_file)
    {
        trace!(?err, "failed to create `version_check` file");
    }

    if !needs_update {
        return;
    }

    info!("New update is available: {}", release.tag_name);
}

fn update_needed(release: &Release) -> Result<bool> {
    let current_tag = Version::new(env!("CARGO_PKG_VERSION")).parse()?;
    let new_tag = Version::new(
        release
            .tag_name
            .strip_prefix('v')
            .unwrap_or(&release.tag_name),
    )
    .parse()?;

    Ok(new_tag > current_tag)
}

async fn latest_release() -> Result<Release> {
    octocrab::instance()
        .repos(AUTOMETRICS_GITHUB_ORG, AUTOMETRICS_AM_REPO)
        .releases()
        .get_latest()
        .await
        .context("failed to check latest release from GitHub")
}

fn asset_needed() -> Result<&'static str> {
    Ok(match env!("TARGET") {
        "x86_64-unknown-linux-gnu" => "am-linux-x86_64",
        "aarch64-unknown-linux-gnu" => "am-linux-aarch64",
        "x86_64-apple-darwin" => "am-macos-aarch64",
        "aarch64-apple-darwin" => "am-macos-x86_64",
        target => bail!("unsupported target: {target}"),
    })
}

#[inline]
fn is_homebrew() -> bool {
    #[cfg(target_os = "linux")]
    return env::current_exe()
        .map(|path| path.starts_with("/home/linuxbrew/.linuxbrew"))
        .unwrap_or_default();

    #[cfg(target_os = "macos")]
    return env::current_exe()
        .map(|path| path.starts_with("/usr/local") || path.starts_with("/opt/homebrew"))
        .unwrap_or_default();

    #[cfg(all(not(target_os = "linux"), not(target_os = "macos")))]
    return false;
}
