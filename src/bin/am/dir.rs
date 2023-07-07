use anyhow::Result;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::{env, fs};
use tracing::warn;

pub struct AutoCleanupDir {
    path: PathBuf,
    ephemeral: bool,
}

impl AutoCleanupDir {
    pub(crate) fn new(process: &str, ephemeral: bool) -> Result<AutoCleanupDir> {
        let start_dir = if ephemeral {
            env::temp_dir()
        } else {
            env::current_dir()?
        };

        let path = start_dir.join(".autometrics").join(process);
        fs::create_dir_all(&path)?;

        Ok(AutoCleanupDir { path, ephemeral })
    }
}

impl Drop for AutoCleanupDir {
    fn drop(&mut self) {
        if self.ephemeral {
            if let Err(err) = fs::remove_dir_all(&self) {
                warn!(
                    ?err,
                    "failed to remove data directory despite --ephemeral being passed"
                );
            }
        }
    }
}

impl Deref for AutoCleanupDir {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl AsRef<Path> for AutoCleanupDir {
    fn as_ref(&self) -> &Path {
        self.path.as_path()
    }
}
