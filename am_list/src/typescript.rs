mod imports;
mod queries;

use crate::{FunctionInfo, InstrumentFile, ListAmFunctions, Result};
use log::{debug, trace};
use rayon::prelude::*;
use std::{
    collections::{HashSet, VecDeque},
    fs::read_to_string,
    path::{Path, PathBuf},
};
use walkdir::{DirEntry, WalkDir};

use self::queries::{AllFunctionsQuery, AmQuery, TypescriptFunctionInfo};

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

    fn ts_function_definitions_in_single_file(
        &mut self,
        source_code: &str,
    ) -> Result<Vec<TypescriptFunctionInfo>> {
        let query = AllFunctionsQuery::try_new()?;
        query.list_function_names("<single file>", "", source_code)
    }

    fn list_files_and_modules(
        project_root: &Path,
        exclude_patterns: Option<&ignore::gitignore::Gitignore>,
    ) -> Vec<(PathBuf, String)> {
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

            let module = Self::qualified_module_name(&entry);
            Some((entry.path().to_path_buf(), module))
        }));

        source_mod_pairs
    }
}

impl ListAmFunctions for Impl {
    fn list_autometrics_functions(&mut self, project_root: &Path) -> Result<Vec<FunctionInfo>> {
        const PREALLOCATED_ELEMS: usize = 100;
        let mut list = HashSet::with_capacity(PREALLOCATED_ELEMS);
        let source_mod_pairs = Self::list_files_and_modules(project_root, None);
        let query = AmQuery::try_new()?;

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
                    let names = query
                        .list_function_names(&file_name, module, &source)
                        .ok()?;
                    Some(
                        names
                            .into_iter()
                            .map(|info| info.inner_info)
                            .collect::<Vec<_>>(),
                    )
                }),
        );

        let mut result = Vec::with_capacity(PREALLOCATED_ELEMS);
        result.extend(list.into_iter().flatten());
        trace!("Item list: {result:?}");
        Ok(result)
    }

    fn list_autometrics_functions_in_single_file(
        &mut self,
        source_code: &str,
    ) -> Result<Vec<FunctionInfo>> {
        let query = AmQuery::try_new()?;
        query.list_function_names("<single file>", "", source_code, None)
    }

    fn list_all_function_definitions_in_single_file(
        &mut self,
        source_code: &str,
    ) -> Result<Vec<FunctionInfo>> {
        Ok(self
            .ts_function_definitions_in_single_file(source_code)?
            .into_iter()
            .map(Into::into)
            .collect())
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

        let mut ts_specific_locations = self.ts_function_definitions_in_single_file(source)?;
        ts_specific_locations.sort_by_key(|info| {
            info.inner_info
                .definition
                .as_ref()
                .map(|def| def.range.start.line)
                .unwrap_or_default()
        });

        let has_am_directive = source.lines().any(|line| {
            line.contains("import { autometrics } from")
                || line.contains("import { Autometrics } from")
                || line.contains("import { autometrics, Autometrics } from")
        });
        let mut placeholder_offset_range = None;
        let mut needs_decorator_import = false;
        let mut needs_wrapper_import = false;

        let mut new_code = crop::Rope::from(source);
        // Keeping track of inserted lines to update the byte offset to insert code to,
        // only works if the locations list is sorted from top to bottom
        let mut inserted_lines = 0;

        if !has_am_directive {
            new_code.insert(
                0,
                "import { placeholder } from '@autometrics/autometrics';\n",
            );
            inserted_lines += 1;
            placeholder_offset_range = Some("import { ".len().."import { placeholder".len());
        }

        for function_info in locations {
            if function_info.definition.is_none() || function_info.instrumentation.is_some() {
                continue;
            }

            let ts_loc = ts_specific_locations
                .iter()
                .find_map(|info| {
                    if info.inner_info.id == function_info.id {
                        Some(info.function_rvalue_range.clone())
                    } else {
                        None
                    }
                })
                .flatten();

            match ts_loc {
                Some(rvalue_range) => {
                    let start_byte_offset = new_code
                        .byte_of_line(inserted_lines + rvalue_range.start.line)
                        + rvalue_range.start.column;
                    new_code.insert(start_byte_offset, "autometrics(");
                    let end_byte_offset = new_code
                        .byte_of_line(inserted_lines + rvalue_range.end.line)
                        + rvalue_range.end.column;
                    new_code.insert(end_byte_offset, ")");
                    needs_wrapper_import = true;
                }
                None => {
                    let def_line = function_info.definition.as_ref().unwrap().range.start.line;
                    let byte_offset = new_code.byte_of_line(inserted_lines + def_line);
                    new_code.insert(byte_offset, "@Autometrics()\n");
                    inserted_lines += 1;
                    needs_decorator_import = true;
                }
            }
        }

        if let Some(range) = placeholder_offset_range {
            let imports = match (needs_wrapper_import, needs_decorator_import) {
                (true, true) => "autometrics, Autometrics",
                (true, false) => "autometrics",
                (false, true) => "Autometrics",
                (false, false) => "",
            };
            new_code.replace(range, imports);
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
            debug!("Instrumenting {}", path.display());
            let old_source = read_to_string(&path)?;
            let new_source = self.instrument_source_code(&old_source)?;
            std::fs::write(path, new_source)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests;
