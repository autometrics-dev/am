mod imports;
mod queries;

use crate::{FunctionInfo, ListAmFunctions, Result};
use rayon::prelude::*;
use std::{
    collections::{HashSet, VecDeque},
    fs::read_to_string,
    path::{Path, PathBuf},
};
use walkdir::{DirEntry, WalkDir};

use self::queries::{AllFunctionsQuery, AmQuery};

/// Implementation of the Typescript support for listing autometricized functions.
#[derive(Clone, Copy, Debug, Default)]
pub struct Impl {}

impl Impl {
    fn is_hidden(entry: &DirEntry) -> bool {
        entry
            .file_name()
            .to_str()
            .map(|s| s.starts_with('.') || s == "node_modules")
            .unwrap_or(false)
    }

    fn is_valid(entry: &DirEntry) -> bool {
        if Impl::is_hidden(entry) {
            return false;
        }
        entry.file_type().is_dir()
            || entry
                .path()
                .extension()
                .map(|ext| {
                    let ext = ext.to_str().unwrap_or("");
                    ["js", "jsx", "ts", "tsx", "mjs"].contains(&ext)
                })
                .unwrap_or(false)
    }

    fn qualified_module_name(entry: &DirEntry) -> String {
        let mut current_depth = entry.depth();
        let mut mod_name_elements = VecDeque::with_capacity(8);
        let mut path = entry.path();

        // NOTE(magic)
        // This "1" magic constant bears the assumption "am_list" is called
        // from the root of a typescript repository.
        while current_depth > 1 {
            if path.is_dir() {
                if let Some(component) = path.file_name() {
                    mod_name_elements.push_front(component.to_string_lossy().to_string());
                }
            } else if path.is_file() {
                if let Some(stem) = path.file_name().and_then(|os_str| os_str.to_str()) {
                    mod_name_elements.push_front(stem.to_string());
                }
            }

            if path.parent().is_some() {
                path = path.parent().unwrap();
                current_depth -= 1;
            } else {
                break;
            }
        }
        itertools::intersperse(mod_name_elements, "/".to_string()).collect()
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
            let module = Self::qualified_module_name(&entry);
            Some((entry.path().to_path_buf(), module))
        }));

        list.par_extend(
            source_mod_pairs
                .par_iter()
                .filter_map(move |(path, module)| {
                    let query = AmQuery::try_new().ok()?;
                    let source = read_to_string(path).ok()?;
                    let file_name = PathBuf::from(path)
                        .strip_prefix(project_root)
                        .expect("path comes from a project_root WalkDir")
                        .to_str()
                        .expect("file_name is a valid path as it is part of `path`")
                        .to_string();
                    let names = query
                        .list_function_names(&file_name, module, &source, Some(path))
                        .ok()?;
                    Some(names.into_iter().collect::<Vec<_>>())
                }),
        );

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
            let module = Self::qualified_module_name(&entry);
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
                    let query = AllFunctionsQuery::try_new().ok()?;
                    let file_name = PathBuf::from(path)
                        .strip_prefix(project_root)
                        .expect("path comes from a project_root WalkDir")
                        .to_str()
                        .expect("file_name is a valid path as it is part of `path`")
                        .to_string();
                    let names = query
                        .list_function_names(&file_name, module, &source)
                        .ok()?;
                    Some(names.into_iter().collect::<Vec<_>>())
                }),
        );

        let mut result = Vec::with_capacity(PREALLOCATED_ELEMS);
        result.extend(list.into_iter().flatten());
        Ok(result)
    }
}

#[cfg(test)]
mod tests;
