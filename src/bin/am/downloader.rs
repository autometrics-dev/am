use crate::commands::start::CLIENT;
use anyhow::{anyhow, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressState, ProgressStyle};
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs::File;
use std::io::{BufWriter, Write};

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
    _sha256sum: &str,
    _org: &str,
    _repo: &str,
    _version: &str,
    _package: &str,
) -> Result<()> {
    todo!()
}
