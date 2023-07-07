use anyhow::Result;
use std::env;
use std::fs::remove_dir_all;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use tracing::warn;

#[repr(transparent)]
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

        Ok(AutoCleanupDir {
            path: start_dir.join(".autometrics").join(process),
            ephemeral
        })
    }
}

impl Drop for AutoCleanupDir {
    fn drop(&mut self) {
        if self.ephemeral {
            if let Err(err) = remove_dir_all(&self.0) {
                warn!(?err, "failed to remove data directory despite --ephemeral being passed");
            }
        }
    }
}

impl Deref for AutoCleanupDir {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<Path> for AutoCleanupDir {
    fn as_ref(&self) -> &Path {
        self.0.as_path()
    }
}
