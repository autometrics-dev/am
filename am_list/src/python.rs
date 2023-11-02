mod queries;

use crate::{FunctionInfo, InstrumentFile, ListAmFunctions, Result};
use log::debug;
use queries::{AllFunctionsQuery, AmImportQuery, AmQuery};
use rayon::prelude::*;
use std::{
    collections::HashSet,
    fs::read_to_string,
    path::{Path, PathBuf, MAIN_SEPARATOR},
};
use walkdir::{DirEntry, WalkDir};

/// Implementation of the Python support for listing autometricized functions.
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
                .path()
                .extension()
                .map_or(false, |ext| ext == "py" || ext == "py3")
    }

    fn list_files(
        project_root: &Path,
        exclude_patterns: Option<&ignore::gitignore::Gitignore>,
    ) -> Vec<String> {
        const PREALLOCATED_ELEMS: usize = 100;
        let walker = WalkDir::new(project_root).into_iter();
        let mut project_files = Vec::with_capacity(PREALLOCATED_ELEMS);
        project_files.extend(walker.filter_entry(Self::is_valid).filter_map(|entry| {
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

            Some(
                entry
                    .path()
                    .to_str()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
            )
        }));

        project_files
    }
}

impl ListAmFunctions for Impl {
    fn list_autometrics_functions(&mut self, project_root: &Path) -> Result<Vec<FunctionInfo>> {
        const PREALLOCATED_ELEMS: usize = 100;
        let mut list = HashSet::with_capacity(PREALLOCATED_ELEMS);
        let root_name = project_root
            .file_name()
            .map(|s| s.to_str().unwrap_or_default())
            .unwrap_or("");
        let project_files = Self::list_files(project_root, None);

        list.par_extend(project_files.par_iter().filter_map(move |path| {
            let relative_module_name = Path::new(path)
                .strip_prefix(project_root)
                .ok()?
                .with_extension("")
                .to_str()?
                .replace(MAIN_SEPARATOR, ".");
            let module_name = format!("{}.{}", root_name, relative_module_name);
            let source = read_to_string(path).ok()?;
            let import_query = AmImportQuery::try_new().ok()?;
            let decorator_name = import_query.get_decorator_name(source.as_str()).ok()?;
            let query = AmQuery::try_new(decorator_name.as_str()).ok()?;
            let file_name = PathBuf::from(path)
                .strip_prefix(project_root)
                .expect("path comes from a project_root WalkDir")
                .to_str()
                .expect("file_name is a valid path as it is part of `path`")
                .to_string();
            let names = query
                .list_function_names(&file_name, &source, module_name.as_str())
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
        let root_name = project_root
            .file_name()
            .map(|s| s.to_str().unwrap_or_default())
            .unwrap_or("");

        let project_files = Self::list_files(project_root, None);

        list.par_extend(project_files.par_iter().filter_map(move |path| {
            let relative_module_name = Path::new(path)
                .strip_prefix(project_root)
                .ok()?
                .with_extension("")
                .to_str()?
                .replace(MAIN_SEPARATOR, ".");
            let module_name = format!("{}.{}", root_name, relative_module_name);
            let source = read_to_string(path).ok()?;
            let file_name = PathBuf::from(path)
                .strip_prefix(project_root)
                .expect("path comes from a project_root WalkDir")
                .to_str()
                .expect("file_name is a valid path as it is part of `path`")
                .to_string();
            let query = AllFunctionsQuery::try_new().ok()?;
            let names = query
                .list_function_names(&file_name, &source, module_name.as_str())
                .unwrap_or_default();
            Some(names)
        }));

        let mut result = Vec::with_capacity(PREALLOCATED_ELEMS);
        result.extend(list.into_iter().flatten());
        Ok(result)
    }

    fn list_autometrics_functions_in_single_file(
        &mut self,
        source_code: &str,
    ) -> Result<Vec<FunctionInfo>> {
        let import_query = AmImportQuery::try_new()?;
        let decorator_name = import_query.get_decorator_name(source_code).ok();
        if decorator_name.is_none() {
            return Ok(Vec::new());
        }
        let query = AmQuery::try_new(decorator_name.as_ref().unwrap())?;
        query.list_function_names("<single file>", source_code, "")
    }

    fn list_all_function_definitions_in_single_file(
        &mut self,
        source_code: &str,
    ) -> Result<Vec<FunctionInfo>> {
        let query = AllFunctionsQuery::try_new()?;
        query.list_function_names("<single file>", source_code, "")
    }
}

impl InstrumentFile for Impl {
    fn instrument_source_code(&mut self, source: &str) -> Result<String> {
        const DEF_LEN: usize = "def ".len();

        let mut locations = self.list_all_functions_in_single_file(source)?;
        locations.sort_by_key(|info| {
            info.definition
                .as_ref()
                .map(|def| def.range.start.line)
                .unwrap_or_default()
        });

        let has_am_directive = source
            .lines()
            .any(|line| line.contains("from autometrics import autometrics"));

        let mut new_code = crop::Rope::from(source);
        // Keeping track of inserted lines to update the byte offset to insert code to,
        // only works if the locations list is sorted from top to bottom
        let mut inserted_lines = 0;

        if !has_am_directive {
            new_code.insert(0, "from autometrics import autometrics\n");
            inserted_lines += 1;
        }

        for function_info in locations {
            if function_info.definition.is_none() || function_info.instrumentation.is_some() {
                continue;
            }

            let def_line = function_info.definition.as_ref().unwrap().range.start.line;
            let def_col = function_info
                .definition
                .unwrap()
                .range
                .start
                .column
                .saturating_sub(DEF_LEN);
            let byte_offset = new_code.byte_of_line(inserted_lines + def_line);
            new_code.insert(
                byte_offset,
                format!("{}@autometrics\n", " ".repeat(def_col)),
            );
            inserted_lines += 1;
        }

        Ok(new_code.to_string())
    }

    fn instrument_project(
        &mut self,
        project_root: &Path,
        exclude_patterns: Option<&ignore::gitignore::Gitignore>,
    ) -> Result<()> {
        let sources_modules = Self::list_files(project_root, exclude_patterns);

        for path in sources_modules {
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
