mod queries;

use crate::{FunctionInfo, ListAmFunctions, Result};
use queries::{AllFunctionsQuery, AmQuery};
use rayon::prelude::*;
use std::{
    collections::HashSet,
    fs::read_to_string,
    path::{Path, PathBuf},
};
use walkdir::{DirEntry, WalkDir};

/// Implementation of the Go support for listing autometricized functions.
#[derive(Clone, Copy, Debug, Default)]
pub struct Impl {}

impl Impl {
    fn is_hidden(entry: &DirEntry) -> bool {
        entry
            .file_name()
            .to_str()
            .map(|s| s.starts_with('.'))
            .unwrap_or(false)
    }

    fn is_valid(entry: &DirEntry) -> bool {
        if Impl::is_hidden(entry) {
            return false;
        }
        entry.file_type().is_dir()
            || entry
                .file_name()
                .to_str()
                .map(|s| s.ends_with(".go"))
                .unwrap_or(false)
    }
}

impl ListAmFunctions for Impl {
    fn list_autometrics_functions(&mut self, project_root: &Path) -> Result<Vec<FunctionInfo>> {
        const PREALLOCATED_ELEMS: usize = 100;
        let mut list = HashSet::with_capacity(PREALLOCATED_ELEMS);

        let walker = WalkDir::new(project_root).into_iter();
        let mut source_mod_pairs = Vec::with_capacity(PREALLOCATED_ELEMS);
        source_mod_pairs.extend(walker.filter_entry(Self::is_valid).filter_map(|entry| {
            let entry = entry.ok()?;
            Some(
                entry
                    .path()
                    .to_str()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
            )
        }));

        list.par_extend(source_mod_pairs.par_iter().filter_map(move |path| {
            let source = read_to_string(path).ok()?;
            let file_name = PathBuf::from(path)
                .strip_prefix(project_root)
                .expect("path comes from a project_root WalkDir")
                .to_str()
                .expect("file_name is a valid path as it is part of `path`")
                .to_string();
            let query = AmQuery::try_new().ok()?;
            let names = query
                .list_function_names(&file_name, &source)
                .unwrap_or_default();
            Some(names)
        }));

        let mut result = Vec::with_capacity(PREALLOCATED_ELEMS);
        result.extend(list.into_iter().flatten());
        Ok(result)
    }

    fn list_all_function_definitions(&mut self, project_root: &Path) -> Result<Vec<FunctionInfo>> {
        const PREALLOCATED_ELEMS: usize = 100;
        let mut list = HashSet::with_capacity(PREALLOCATED_ELEMS);

        let walker = WalkDir::new(project_root).into_iter();
        let mut source_mod_pairs = Vec::with_capacity(PREALLOCATED_ELEMS);
        source_mod_pairs.extend(walker.filter_entry(Self::is_valid).filter_map(|entry| {
            let entry = entry.ok()?;
            Some(
                entry
                    .path()
                    .to_str()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
            )
        }));

        list.par_extend(source_mod_pairs.par_iter().filter_map(move |path| {
            let source = read_to_string(path).ok()?;
            let file_name = PathBuf::from(path)
                .strip_prefix(project_root)
                .expect("path comes from a project_root WalkDir")
                .to_str()
                .expect("file_name is a valid path as it is part of `path`")
                .to_string();
            let query = AllFunctionsQuery::try_new().ok()?;
            let names = query
                .list_function_names(&file_name, &source)
                .unwrap_or_default();
            Some(names)
        }));

        let mut result = Vec::with_capacity(PREALLOCATED_ELEMS);
        result.extend(list.into_iter().flatten());
        Ok(result)
    }
}

#[cfg(test)]
mod tests;
