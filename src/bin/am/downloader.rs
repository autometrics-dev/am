use crate::commands::start::CLIENT;
use anyhow::{anyhow, bail, Result};
use flate2::read::GzDecoder;
use indicatif::{MultiProgress, ProgressBar, ProgressState, ProgressStyle};
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, error};

/// downloads `package` into `destination`, returning the sha256sum hex-digest of the downloaded file
pub async fn download_github_release(
    destination: &File,
    org: &str,
    repo: &str,
    version: &str,
    package: &str,
    multi_progress: &MultiProgress,
) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut response = CLIENT
        .get(format!(
            "https://github.com/{org}/{repo}/releases/download/v{version}/{package}"
        ))
        .send()
        .await?
        .error_for_status()?;

    let total_size = response
        .content_length()
        .ok_or_else(|| anyhow!("didn't receive content length"))?;
    let mut downloaded = 0;

    let pb = multi_progress.add(ProgressBar::new(total_size));

    // https://github.com/console-rs/indicatif/blob/HEAD/examples/download.rs#L12
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] {msg} [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
            .with_key("eta", |state: &ProgressState, w: &mut dyn fmt::Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
            .progress_chars("=> ")
    );

    pb.set_message(format!(
        "Downloading {package} from github.com/{org}/{repo}"
    ));

    let mut buffer = BufWriter::new(destination);

    while let Some(ref chunk) = response.chunk().await? {
        buffer.write_all(chunk)?;
        hasher.update(chunk);

        let new_size = (downloaded + chunk.len() as u64).min(total_size);
        downloaded = new_size;

        pb.set_position(downloaded);
    }

    pb.finish_and_clear();
    multi_progress.remove(&pb);

    let checksum = hex::encode(hasher.finalize());
    Ok(checksum)
}

pub async fn verify_checksum(
    sha256sum: &str,
    org: &str,
    repo: &str,
    version: &str,
    package: &str,
) -> Result<()> {
    let checksums = CLIENT
        .get(format!(
            "https://github.com/{org}/{repo}/releases/download/v{version}/sha256sums.txt"
        ))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    // Go through all the lines in the checksum file and look for the one that
    // we need for our current service/version/os/arch.
    let expected_checksum = checksums
        .lines()
        .find_map(|line| match line.split_once("  ") {
            Some((checksum, filename)) if package == filename => Some(checksum),
            _ => None,
        })
        .ok_or_else(|| anyhow!("unable to find checksum for {package} in checksum list"))?;

    if expected_checksum != sha256sum {
        error!(
            ?expected_checksum,
            calculated_checksum = ?sha256sum,
            "Calculated checksum for downloaded archive did not match expected checksum",
        );
        bail!("checksum did not match");
    }

    Ok(())
}

pub async fn unpack(
    archive: &File,
    package: &str,
    destination_path: &PathBuf,
    prefix: &str,
    multi_progress: &MultiProgress,
) -> Result<()> {
    let tar_file = GzDecoder::new(archive);
    let mut ar = tar::Archive::new(tar_file);

    let pb = multi_progress.add(ProgressBar::new_spinner());
    pb.set_style(ProgressStyle::default_spinner());
    pb.enable_steady_tick(Duration::from_millis(120));
    pb.set_message(format!("Unpacking {package}..."));

    for entry in ar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;

        debug!("Unpacking {}", path.display());

        // Remove the prefix and join it with the base directory.
        let path = path.strip_prefix(&prefix)?.to_owned();
        let path = destination_path.join(path);

        entry.unpack(&path)?;
    }

    pb.finish_and_clear();
    multi_progress.remove(&pb);
    Ok(())
}
