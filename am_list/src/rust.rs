mod queries;

use self::queries::{AllFunctionsQuery, AmQuery};
use crate::{FunctionInfo, ListAmFunctions, Result};
use rayon::prelude::*;
use std::{
    collections::{HashSet, VecDeque},
    fs::read_to_string,
    path::{Path, PathBuf},
};
use walkdir::{DirEntry, WalkDir};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct AmStruct {
    module: String,
    strc: String,
}

/// Implementation of the Rust support for listing autometricized functions.
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
                .map(|s| s.ends_with(".rs"))
                .unwrap_or(false)
    }

    fn fully_qualified_module_name(entry: &DirEntry) -> String {
        let mut current_depth = entry.depth();
        let mut mod_name_elements = VecDeque::with_capacity(8);
        let mut path = entry.path();

        // NOTE(magic)
        // This "1" magic constant bears the assumption "am_list" is called
        // from the root of a crate.
        while current_depth > 1 {
            if path.is_dir() {
                if let Some(component) = path.file_name() {
                    mod_name_elements.push_front(component.to_string_lossy().to_string());
                }
            } else if path.is_file() {
                if let Some(stem) = path
                    .file_name()
                    .and_then(|os_str| os_str.to_str())
                    .and_then(|file_name| file_name.strip_suffix(".rs"))
                {
                    if stem != "mod" {
                        mod_name_elements.push_front(stem.to_string());
                    }
                }
            }

            if path.parent().is_some() {
                path = path.parent().unwrap();
                current_depth -= 1;
            } else {
                break;
            }
        }

        itertools::intersperse(mod_name_elements, "::".to_string()).collect()
    }
}

impl ListAmFunctions for Impl {
    fn list_autometrics_functions(&mut self, project_root: &Path) -> Result<Vec<FunctionInfo>> {
        const PREALLOCATED_ELEMS: usize = 100;
        let mut list = HashSet::with_capacity(PREALLOCATED_ELEMS);

        let walker = WalkDir::new(project_root).into_iter();
        let mut source_mod_pairs = Vec::with_capacity(PREALLOCATED_ELEMS);
        let query = AmQuery::try_new()?;
        source_mod_pairs.extend(walker.filter_entry(Self::is_valid).filter_map(|entry| {
            let entry = entry.ok()?;
            let module = Self::fully_qualified_module_name(&entry);
            Some((
                entry
                    .path()
                    .to_str()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
                module,
            ))
        }));

        list.par_extend(
            source_mod_pairs
                .par_iter()
                .filter_map(move |(path, module)| {
                    let source = read_to_string(path).ok()?;
                    let file_name = PathBuf::from(path)
                        .strip_prefix(project_root)
                        .expect("path comes from a project_root WalkDir")
                        .to_str()
                        .expect("file_name is a valid path as it is part of `path`")
                        .to_string();
                    let am_functions = query
                        .list_function_names(&file_name, module.clone(), &source)
                        .unwrap_or_default();
                    Some(am_functions)
                }),
        );

        let mut result = Vec::with_capacity(PREALLOCATED_ELEMS);
        result.extend(list.into_iter().flatten());
        Ok(result)
    }

    fn list_all_function_definitions(&mut self, project_root: &Path) -> Result<Vec<FunctionInfo>> {
        const PREALLOCATED_ELEMS: usize = 400;
        let mut list = HashSet::with_capacity(PREALLOCATED_ELEMS);

        let walker = WalkDir::new(project_root).into_iter();
        let mut source_mod_pairs = Vec::with_capacity(PREALLOCATED_ELEMS);
        let query = AllFunctionsQuery::try_new()?;
        source_mod_pairs.extend(walker.filter_entry(Self::is_valid).filter_map(|entry| {
            let entry = entry.ok()?;
            let module = Self::fully_qualified_module_name(&entry);
            Some((
                entry
                    .path()
                    .to_str()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
                module,
            ))
        }));

        list.par_extend(
            source_mod_pairs
                .par_iter()
                .filter_map(move |(path, module)| {
                    let source = read_to_string(path).ok()?;
                    let file_name = PathBuf::from(path)
                        .strip_prefix(project_root)
                        .expect("path comes from a project_root WalkDir")
                        .to_str()
                        .expect("file_name is a valid path as it is part of `path`")
                        .to_string();
                    let am_functions = query
                        .list_function_names(&file_name, module.clone(), &source)
                        .unwrap_or_default();
                    Some(am_functions)
                }),
        );

        let mut result = Vec::with_capacity(PREALLOCATED_ELEMS);
        result.extend(list.into_iter().flatten());
        Ok(result)
    }
}

#[cfg(test)]
mod tests;
