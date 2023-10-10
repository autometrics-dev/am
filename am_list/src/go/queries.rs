use crate::{AmlError, FunctionInfo, Location, Result, FUNC_NAME_CAPTURE};
use log::error;
use tree_sitter::{Parser, Query};
use tree_sitter_go::language;

const PACK_NAME_CAPTURE: &str = "pack.name";
const TYPE_NAME_CAPTURE: &str = "type.name";

fn new_parser() -> Result<Parser> {
    let mut parser = Parser::new();
    parser.set_language(language())?;
    Ok(parser)
}

/// Query wrapper for "all autometrics functions in source"
#[derive(Debug)]
pub(super) struct AmQuery {
    query: Query,
    /// Index of the capture for a Type, in the case of methods.
    type_name_idx: u32,
    /// Index of the capture for a function name.
    func_name_idx: u32,
    /// Index of the capture for the package name.
    mod_name_idx: u32,
}

impl AmQuery {
    /// Failible constructor.
    ///
    /// The constructor only fails if the given tree-sitter query does not have the
    /// necessary named captures.
    pub fn try_new() -> Result<Self> {
        let query = Query::new(
            language(),
            include_str!("../../runtime/queries/go/autometrics.scm"),
        )?;
        let type_name_idx = query
            .capture_index_for_name(TYPE_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(TYPE_NAME_CAPTURE.to_string()))?;
        let func_name_idx = query
            .capture_index_for_name(FUNC_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(FUNC_NAME_CAPTURE.to_string()))?;
        let mod_name_idx = query
            .capture_index_for_name(PACK_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(PACK_NAME_CAPTURE.to_string()))?;

        Ok(Self {
            query,
            type_name_idx,
            func_name_idx,
            mod_name_idx,
        })
    }

    pub fn list_function_names(&self, file_name: &str, source: &str) -> Result<Vec<FunctionInfo>> {
        let mut parser = new_parser()?;
        let parsed_source = parser.parse(source, None).ok_or(AmlError::Parsing)?;

        let mut cursor = tree_sitter::QueryCursor::new();
        cursor
            .matches(&self.query, parsed_source.root_node(), source.as_bytes())
            .filter_map(|capture| -> Option<Result<FunctionInfo>> {
                let module = capture
                    .nodes_for_capture_index(self.mod_name_idx)
                    .next()
                    .map(|node| node.utf8_text(source.as_bytes()).map(ToString::to_string))?;
                let type_name = capture
                    .nodes_for_capture_index(self.type_name_idx)
                    .next()
                    .map(|node| node.utf8_text(source.as_bytes()).map(ToString::to_string))
                    .transpose();
                let fn_node = capture.nodes_for_capture_index(self.func_name_idx).next()?;
                let fn_name = fn_node
                    .utf8_text(source.as_bytes())
                    .map(ToString::to_string);
                let start = fn_node.start_position();
                let end = fn_node.end_position();
                let instrumentation = Some(Location::from((file_name, start, end)));
                let definition = Some(Location::from((file_name, start, end)));

                match (module, type_name, fn_name) {
                    (Ok(module), Ok(type_name), Ok(function)) => Some(Ok(FunctionInfo {
                        id: (
                            module,
                            format!(
                                "{}{function}",
                                if let Some(go_type) = type_name {
                                    format!("{go_type}.")
                                } else {
                                    String::new()
                                }
                            ),
                        )
                            .into(),
                        instrumentation,
                        definition,
                    })),
                    (Err(err_mod), _, _) => {
                        error!("could not fetch the package name: {err_mod}");
                        Some(Err(AmlError::InvalidText))
                    }
                    (_, Err(err_typ), _) => {
                        error!("could not fetch the package name: {err_typ}");
                        Some(Err(AmlError::InvalidText))
                    }
                    (_, _, Err(err_fn)) => {
                        error!("could not fetch the package name: {err_fn}");
                        Some(Err(AmlError::InvalidText))
                    }
                }
            })
            .collect::<std::result::Result<Vec<_>, _>>()
    }
}

/// Query wrapper for "all functions in source"
#[derive(Debug)]
pub(super) struct AllFunctionsQuery {
    query: Query,
    /// Index of the capture for a Type, in the case of methods.
    type_name_idx: u32,
    /// Index of the capture for a function name.
    func_name_idx: u32,
    /// Index of the capture for the package name.
    mod_name_idx: u32,
}

impl AllFunctionsQuery {
    /// Failible constructor.
    ///
    /// The constructor only fails if the given tree-sitter query does not have the
    /// necessary named captures.
    pub fn try_new() -> Result<Self> {
        let query = Query::new(
            language(),
            include_str!("../../runtime/queries/go/all_functions.scm"),
        )?;
        let type_name_idx = query
            .capture_index_for_name(TYPE_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(TYPE_NAME_CAPTURE.to_string()))?;
        let func_name_idx = query
            .capture_index_for_name(FUNC_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(FUNC_NAME_CAPTURE.to_string()))?;
        let mod_name_idx = query
            .capture_index_for_name(PACK_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(PACK_NAME_CAPTURE.to_string()))?;

        Ok(Self {
            query,
            type_name_idx,
            func_name_idx,
            mod_name_idx,
        })
    }

    pub fn list_function_names(&self, file_name: &str, source: &str) -> Result<Vec<FunctionInfo>> {
        let mut parser = new_parser()?;
        let parsed_source = parser.parse(source, None).ok_or(AmlError::Parsing)?;

        let mut cursor = tree_sitter::QueryCursor::new();
        cursor
            .matches(&self.query, parsed_source.root_node(), source.as_bytes())
            .filter_map(|capture| -> Option<Result<FunctionInfo>> {
                let module = capture
                    .nodes_for_capture_index(self.mod_name_idx)
                    .next()
                    .map(|node| node.utf8_text(source.as_bytes()).map(ToString::to_string))?;
                let type_name = capture
                    .nodes_for_capture_index(self.type_name_idx)
                    .next()
                    .map(|node| node.utf8_text(source.as_bytes()).map(ToString::to_string))
                    .transpose();
                let fn_node = capture.nodes_for_capture_index(self.func_name_idx).next()?;
                let fn_name = fn_node
                    .utf8_text(source.as_bytes())
                    .map(ToString::to_string);
                let start = fn_node.start_position();
                let end = fn_node.end_position();
                let instrumentation = None;
                let definition = Some(Location::from((file_name, start, end)));

                match (module, type_name, fn_name) {
                    (Ok(module), Ok(type_name), Ok(function)) => Some(Ok(FunctionInfo {
                        id: (
                            module,
                            format!(
                                "{}{function}",
                                if let Some(go_type) = type_name {
                                    format!("{go_type}.")
                                } else {
                                    String::new()
                                }
                            ),
                        )
                            .into(),
                        instrumentation,
                        definition,
                    })),
                    (Err(err_mod), _, _) => {
                        error!("could not fetch the package name: {err_mod}");
                        Some(Err(AmlError::InvalidText))
                    }
                    (_, Err(err_typ), _) => {
                        error!("could not fetch the package name: {err_typ}");
                        Some(Err(AmlError::InvalidText))
                    }
                    (_, _, Err(err_fn)) => {
                        error!("could not fetch the package name: {err_fn}");
                        Some(Err(AmlError::InvalidText))
                    }
                }
            })
            .collect::<std::result::Result<Vec<_>, _>>()
    }
}
