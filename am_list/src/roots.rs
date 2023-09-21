use log::debug;
use walkdir::{DirEntry, WalkDir};

use crate::{AmlError, Language};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

/// Use file heuristics to detect valid project roots under the given directory.
pub fn find_project_roots(repo: &Path) -> Result<Vec<(PathBuf, Language)>, AmlError> {
    let abs_repo = repo.canonicalize().map_err(|_| AmlError::InvalidPath)?;
    debug!("Looking for roots in {}", abs_repo.display());
    let rust_roots = find_rust_roots(&abs_repo)
        .into_iter()
        .map(|project_root| (project_root, Language::Rust));
    let ts_roots = find_typescript_roots(&abs_repo)
        .into_iter()
        .map(|project_root| (project_root, Language::Typescript));
    let go_roots = find_go_roots(&abs_repo)
        .into_iter()
        .map(|project_root| (project_root, Language::Go));
    let py_roots = find_py_roots(&abs_repo)
        .into_iter()
        .map(|project_root| (project_root, Language::Python));

    Ok(rust_roots
        .chain(ts_roots)
        .chain(go_roots)
        .chain(py_roots)
        .collect())
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry.depth() != 0
        && entry
            .file_name()
            .to_str()
            .map(|s| s.starts_with('.'))
            .unwrap_or(false)
}

fn find_rust_roots(repo: &Path) -> Vec<PathBuf> {
    fn is_in_target(entry: &DirEntry) -> bool {
        let mut depth = entry.depth();
        let mut pointer = entry.path();
        while depth > 0 {
            if pointer
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| s == "target")
                .unwrap_or(false)
                && pointer.is_dir()
            {
                return true;
            }

            depth -= 1;
            pointer = match pointer.parent() {
                Some(new_pointer) => new_pointer,
                None => {
                    return false;
                }
            };
        }

        false
    }

    let walker = WalkDir::new(repo).into_iter();
    walker
        .filter_entry(|e| !is_hidden(e) && !is_in_target(e))
        .filter_map(|e| -> Option<PathBuf> {
            match e {
                Ok(path) => {
                    if path.file_type().is_file() && path.file_name().to_str() == Some("Cargo.toml")
                    {
                        path.path().parent().map(Path::to_path_buf)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
        .collect()
}

fn find_typescript_roots(repo: &Path) -> Vec<PathBuf> {
    fn is_in_node_modules(entry: &DirEntry) -> bool {
        let mut depth = entry.depth();
        let mut pointer = entry.path();
        while depth > 0 {
            if pointer
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| s == "node_modules")
                .unwrap_or(false)
                && pointer.is_dir()
            {
                return true;
            }

            depth -= 1;
            pointer = match pointer.parent() {
                Some(new_pointer) => new_pointer,
                None => {
                    return false;
                }
            };
        }

        false
    }

    let walker = WalkDir::new(repo).into_iter();
    walker
        .filter_entry(|e| !is_hidden(e) && !is_in_node_modules(e))
        .filter_map(|e| -> Option<PathBuf> {
            match e {
                Ok(path) => {
                    if path.file_type().is_file()
                        && path.file_name().to_str() == Some("package.json")
                    {
                        path.path().parent().map(Path::to_path_buf)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
        .collect()
}

fn find_go_roots(repo: &Path) -> Vec<PathBuf> {
    fn is_in_vendor(entry: &DirEntry) -> bool {
        let mut depth = entry.depth();
        let mut pointer = entry.path();
        while depth > 0 {
            if pointer
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| s == "vendor")
                .unwrap_or(false)
                && pointer.is_dir()
            {
                return true;
            }

            depth -= 1;
            pointer = match pointer.parent() {
                Some(new_pointer) => new_pointer,
                None => {
                    return false;
                }
            };
        }

        false
    }

    let walker = WalkDir::new(repo).into_iter();
    walker
        .filter_entry(|e| !is_hidden(e) && !is_in_vendor(e))
        .filter_map(|e| -> Option<PathBuf> {
            match e {
                Ok(path) => {
                    if path.file_type().is_file() && path.file_name().to_str() == Some("go.mod") {
                        path.path().parent().map(Path::to_path_buf)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
        .collect()
}

fn find_py_roots(repo: &Path) -> HashSet<PathBuf> {
    let walker = WalkDir::new(repo).into_iter();
    walker
        .filter_entry(|e| !is_hidden(e))
        .filter_map(|e| -> Option<PathBuf> {
            match e {
                Ok(path) => {
                    if path.file_type().is_file() {
                        match path.file_name().to_str() {
                            Some("setup.py")
                            | Some("requirements.txt")
                            | Some("pyproject.toml") => path.path().parent().map(Path::to_path_buf),
                            _ => None,
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
        .collect()
}
