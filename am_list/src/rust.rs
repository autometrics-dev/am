mod queries;

use self::queries::{AllFunctionsQuery, AmQuery};
use crate::{FunctionInfo, InstrumentFile, ListAmFunctions, Result};
use log::debug;
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
            .map(|s| {
                s.starts_with('.') ||
                 // We only ignore folders/files named "target" if they are at
                 // the root of the project, for the unlikely case where there
                 // is a "target" module deeper in the project.
                 (entry.depth() == 1 && s == "target")
            })
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
        // This "0" magic constant bears the assumption "am_list" is called
        // from the root of a crate _or workspace_.
        //
        // HACK: Using the name of the directory all the time for module will
        // only work in workspaces if the sub-crate is always imported as the
        // name of its folder.
        while current_depth > 0 {
            if path.is_dir() {
                if let Some(component) = path.file_name() {
                    let component = component.to_string_lossy();
                    if component != "src" {
                        mod_name_elements.push_front(component.replace('-', "_"));
                    }
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

    fn list_files_and_modules(
        project_root: &Path,
        exclude_patterns: Option<&ignore::gitignore::Gitignore>,
    ) -> Vec<(String, String)> {
        const PREALLOCATED_ELEMS: usize = 100;

        let walker = WalkDir::new(project_root).into_iter();
        let mut source_mod_pairs = Vec::with_capacity(PREALLOCATED_ELEMS);
        source_mod_pairs.extend(walker.filter_entry(Self::is_valid).filter_map(|entry| {
            let entry = entry.ok()?;

            if let Some(pattern) = exclude_patterns {
                let ignore_match =
                    pattern.matched_path_or_any_parents(entry.path(), entry.file_type().is_dir());
                if matches!(ignore_match, ignore::Match::Ignore(_)) {
                    debug!(
                        "The exclusion pattern got a match on {}: {:?}",
                        entry.path().display(),
                        ignore_match
                    );
                    return None;
                }
            }

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

        source_mod_pairs
    }
}

impl ListAmFunctions for Impl {
    fn list_autometrics_functions(&mut self, project_root: &Path) -> Result<Vec<FunctionInfo>> {
        const PREALLOCATED_ELEMS: usize = 100;
        let mut list = HashSet::with_capacity(PREALLOCATED_ELEMS);
        let query = AmQuery::try_new()?;
        let source_mod_pairs = Self::list_files_and_modules(project_root, None);

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
        let source_mod_pairs = Self::list_files_and_modules(project_root, None);
        let query = AllFunctionsQuery::try_new()?;

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

    fn list_autometrics_functions_in_single_file(
        &mut self,
        source_code: &str,
    ) -> Result<Vec<FunctionInfo>> {
        let query = AmQuery::try_new()?;
        query.list_function_names("<single file>", String::new(), source_code)
    }

    fn list_all_function_definitions_in_single_file(
        &mut self,
        source_code: &str,
    ) -> Result<Vec<FunctionInfo>> {
        let query = AllFunctionsQuery::try_new()?;
        query.list_function_names("<single file>", String::new(), source_code)
    }
}

impl InstrumentFile for Impl {
    fn instrument_source_code(&mut self, source: &str) -> Result<String> {
        let mut locations = self.list_all_functions_in_single_file(source)?;
        locations.sort_by_key(|info| {
            info.definition
                .as_ref()
                .map(|def| def.range.start.line)
                .unwrap_or_default()
        });

        let mut new_code = crop::Rope::from(source);
        // Keeping track of inserted lines to update the byte offset to insert code to,
        // only works if the locations list is sorted from top to bottom
        let mut inserted_lines = 0;

        for function_info in locations {
            if function_info.definition.is_none() || function_info.instrumentation.is_some() {
                continue;
            }

            let def_line = function_info.definition.as_ref().unwrap().range.start.line;
            let byte_offset = new_code.byte_of_line(inserted_lines + def_line);
            new_code.insert(byte_offset, "#[autometrics::autometrics]\n");
            inserted_lines += 1;
        }

        Ok(new_code.to_string())
    }

    fn instrument_project(
        &mut self,
        project_root: &Path,
        exclude_patterns: Option<&ignore::gitignore::Gitignore>,
    ) -> Result<()> {
        let sources_modules = Self::list_files_and_modules(project_root, exclude_patterns);

        for (path, _module) in sources_modules {
            if std::fs::metadata(&path)?.is_dir() {
                continue;
            }
            debug!("Instrumenting {path}");
            let old_source = read_to_string(&path)?;
            let new_source = self.instrument_source_code(&old_source)?;
            std::fs::write(path, new_source)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests;
