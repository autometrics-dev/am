pub mod go;
pub mod python;
mod roots;
pub mod rust;
pub mod typescript;

use log::info;
pub use roots::find_project_roots;

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fmt::Display,
    path::{Path, PathBuf},
    str::FromStr,
};
use thiserror::Error;
use tree_sitter::{LanguageError, QueryError};

const FUNC_NAME_CAPTURE: &str = "func.name";

/// The identifier of a function in the form of an "expected" autometrics label.
///
/// This label is given as a best effort most of the time, as some languages
/// cannot provide statically the exact information that is going to be produced
/// by Autometrics.
///
/// ## Function relevant locations
/// The location of the detected definition or instrumentation can be included here.
///
/// For "decoration-based" implementations of Autometrics (like `Rust` or `Python`), the
/// definition and instrumentation locations will be mostly the same (maybe a few lines apart,
/// just because the function decoration is a few lines above)
///
/// For "wrapper-based" implementation of Autometrics (like `Typescript`), the instrumentation
/// will be targetting where the wrapper is called, while the definition location might be missing
/// entirely if a function external to the project is being instrumented.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub id: FunctionId,
    /// The location of the definition of the function
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub definition: Option<Location>,
    /// The location of the instrumentation of the function (e.g. where the Autometrics wrapper is called.)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub instrumentation: Option<Location>,
}

/// A valid key to find a specific function in a codebase.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FunctionId {
    /// The name of the module.
    pub module: String,
    /// The name of the function.
    pub function: String,
}

impl<M, F> From<(M, F)> for FunctionId
where
    M: ToString,
    F: ToString,
{
    fn from((module, function): (M, F)) -> Self {
        Self {
            module: module.to_string(),
            function: function.to_string(),
        }
    }
}

/// A position in a file.
///
/// Lines and columns are 0-based, to mimic the choices made by
/// [Language Server Protocol](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#position)
/// and [tree-sitter](tree_sitter::Point)
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl From<tree_sitter::Point> for Position {
    fn from(point: tree_sitter::Point) -> Self {
        Self {
            line: point.row,
            column: point.column,
        }
    }
}

/// A range in a file.
///
/// The start location is inclusive, the end location is exclusive, meaning a range of 1 character is
/// going to follow `end == start + 1`
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Range {
    /// Inclusive start location of the range.
    pub start: Position,
    /// Exclusive end location of the range.
    pub end: Position,
}

impl From<(tree_sitter::Point, tree_sitter::Point)> for Range {
    fn from((start, end): (tree_sitter::Point, tree_sitter::Point)) -> Self {
        Self {
            start: start.into(),
            end: end.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Location {
    pub file: String,
    pub range: Range,
}

impl<F> From<(F, tree_sitter::Point, tree_sitter::Point)> for Location
where
    F: ToString,
{
    fn from((file_name, start, end): (F, tree_sitter::Point, tree_sitter::Point)) -> Self {
        Self {
            file: file_name.to_string(),
            range: (start, end).into(),
        }
    }
}

impl Display for FunctionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "module: {}, function: {}",
            self.id.module, self.id.function
        )
    }
}

/// Trait to implement to claim "Language support" for am_list.
///
/// This means we can both list all autometricized functions in a project, and
/// all functions defined without distinction in a project.
pub trait ListAmFunctions {
    /// List all the autometricized functions under the given project.
    fn list_autometrics_functions(&mut self, project_root: &Path) -> Result<Vec<FunctionInfo>>;
    /// List all the functions defined in the given project.
    fn list_all_function_definitions(&mut self, project_root: &Path) -> Result<Vec<FunctionInfo>>;
    /// List all the functions in the project, instrumented or just defined.
    ///
    /// This is guaranteed to return the most complete set of information
    fn list_all_functions(&mut self, project_root: &Path) -> Result<Vec<FunctionInfo>> {
        let am_functions = self.list_autometrics_functions(project_root)?;
        let all_function_definitions = self.list_all_function_definitions(project_root)?;
        let mut info_set: HashMap<FunctionId, FunctionInfo> = am_functions
            .into_iter()
            .map(|full_info| (full_info.id.clone(), full_info))
            .collect();

        // Only the definition field is expected to differ
        // between am_functions and all_function_definitions
        for function in all_function_definitions {
            info_set
                .entry(function.id.clone())
                .and_modify(|info| info.definition = function.definition.clone())
                .or_insert(function);
        }
        Ok(info_set.into_values().collect())
    }

    /// List all the autometricized functions in the given source code.
    fn list_autometrics_functions_in_single_file(
        &mut self,
        source_code: &str,
    ) -> Result<Vec<FunctionInfo>>;

    /// List all the functions defined in the given source code.
    fn list_all_function_definitions_in_single_file(
        &mut self,
        source_code: &str,
    ) -> Result<Vec<FunctionInfo>>;

    /// List all the functions in the given source code, instrumented or just defined.
    ///
    /// This is guaranteed to return the most complete set of information
    fn list_all_functions_in_single_file(
        &mut self,
        source_code: &str,
    ) -> Result<Vec<FunctionInfo>> {
        let am_functions = self.list_autometrics_functions_in_single_file(source_code)?;
        let all_function_definitions =
            self.list_all_function_definitions_in_single_file(source_code)?;
        let mut info_set: HashMap<FunctionId, FunctionInfo> = am_functions
            .into_iter()
            .map(|full_info| (full_info.id.clone(), full_info))
            .collect();

        // Only the definition field is expected to differ
        // between am_functions and all_function_definitions
        for function in all_function_definitions {
            info_set
                .entry(function.id.clone())
                .and_modify(|info| info.definition = function.definition.clone())
                .or_insert(function);
        }
        Ok(info_set.into_values().collect())
    }
}

/// Instrument a file, adding autometrics annotations as necessary.
///
/// Each language is responsible to reuse its queries/create additonal queries to add the
/// necessary code and produce a new version of the file that has _all_ functions instrumented.
///
/// The invariant to maintain here is that after being done with a file, all functions defined
/// in the file should be instrumented.
pub trait InstrumentFile {
    /// Instrument all functions in the file
    fn instrument_source_code(&mut self, source: &str) -> Result<String>;
    /// Instrument all functions under the given project.
    fn instrument_project(
        &mut self,
        project_root: &Path,
        exclude_patterns: Option<&ignore::gitignore::Gitignore>,
    ) -> Result<()>;
}

pub type Result<T> = std::result::Result<T, AmlError>;

#[derive(Debug, Error)]
pub enum AmlError {
    /// Issue when trying to create a Tree-sitter parser.
    #[error("Issue creating the TreeSitter parser")]
    CreateParser(#[from] LanguageError),
    /// Issue when trying to create a Tree-sitter query.
    #[error("Issue creating the TreeSitter query")]
    CreateQuery(#[from] QueryError),
    /// Issue when the query is expected to have the given named capture.
    #[error("The query is missing an expected named capture: {0}")]
    MissingNamedCapture(String),
    /// Issue when parsing source code.
    #[error("Parsing error")]
    Parsing,
    /// Issue when trying to convert an extract of source code to a unicode
    /// String.
    #[error("Invalid text in source")]
    InvalidText,
    /// Issue when trying to extract a path to a project
    #[error("Invalid path to project")]
    InvalidPath,
    /// Issue when trying to interact with the filesystem
    #[error("IO error")]
    IO(#[from] std::io::Error),
}

/// Languages supported by `am_list`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    Rust,
    #[serde(rename = "Golang")]
    Go,
    Typescript,
    Python,
}

impl FromStr for Language {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let discriminant = s.to_lowercase();
        if ["rust", "rs"].contains(&discriminant.as_str()) {
            return Ok(Self::Rust);
        }

        if discriminant == "go" {
            return Ok(Self::Go);
        }

        if ["typescript", "ts", "javascript", "js"].contains(&discriminant.as_str()) {
            return Ok(Self::Typescript);
        }

        if ["python", "py"].contains(&discriminant.as_str()) {
            return Ok(Self::Python);
        }

        Err(format!("Unknown language: {s}"))
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::Rust => write!(f, "Rust"),
            Language::Go => write!(f, "Golang"),
            Language::Typescript => write!(f, "Typescript"),
            Language::Python => write!(f, "Python"),
        }
    }
}

pub fn list_all_project_functions(
    root: &Path,
) -> Result<BTreeMap<PathBuf, (Language, Vec<FunctionInfo>)>> {
    let projects = find_project_roots(root)?;
    let mut res: BTreeMap<PathBuf, (Language, Vec<FunctionInfo>)> = BTreeMap::new();

    // TODO: try to parallelize this loop if possible
    for (path, language) in projects.iter() {
        info!(
            "Listing functions in {} (Language: {})",
            path.display(),
            language
        );
        let project_fns = list_single_project_functions(path, *language, true)?;

        res.entry(path.to_path_buf())
            .or_insert_with(|| (*language, Vec::new()))
            .1
            .extend(project_fns);
    }

    Ok(res)
}

pub fn list_single_project_functions(
    root: &Path,
    language: Language,
    all_functions: bool,
) -> Result<Vec<FunctionInfo>> {
    let mut implementor: Box<dyn ListAmFunctions> = match language {
        Language::Rust => Box::new(crate::rust::Impl {}),
        Language::Go => Box::new(crate::go::Impl {}),
        Language::Typescript => Box::new(crate::typescript::Impl {}),
        Language::Python => Box::new(crate::python::Impl {}),
    };
    let mut res = if all_functions {
        implementor.list_all_functions(root)?
    } else {
        implementor.list_autometrics_functions(root)?
    };
    res.sort();
    Ok(res)
}

pub fn instrument_all_project_files(
    root: &Path,
    exclude_patterns: &ignore::gitignore::Gitignore,
) -> Result<()> {
    let projects = find_project_roots(root)?;

    // TODO: try to parallelize this loop if possible
    for (path, language) in projects.iter() {
        info!(
            "Instrumenting functions in {} (Language: {})",
            path.display(),
            language
        );
        instrument_single_project_files(path, *language, exclude_patterns)?;
    }

    Ok(())
}

pub fn instrument_single_project_files(
    root: &Path,
    language: Language,
    exclude_patterns: &ignore::gitignore::Gitignore,
) -> Result<()> {
    let mut implementor: Box<dyn InstrumentFile> = match language {
        Language::Rust => Box::new(crate::rust::Impl {}),
        Language::Go => Box::new(crate::go::Impl {}),
        Language::Typescript => Box::new(crate::typescript::Impl {}),
        Language::Python => Box::new(crate::python::Impl {}),
    };
    implementor.instrument_project(root, Some(exclude_patterns))
}
