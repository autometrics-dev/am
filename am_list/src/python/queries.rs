use crate::{AmlError, FunctionInfo, Location, Result, FUNC_NAME_CAPTURE};
use tree_sitter::{Parser, Query};
use tree_sitter_python::language;

const IMPORT_ALIAS_CAPTURE: &str = "import.alias";

fn new_parser() -> Result<Parser> {
    let mut parser = Parser::new();
    parser.set_language(language())?;
    Ok(parser)
}

fn get_node_qualname(node: &tree_sitter::Node, source: &str) -> Result<String> {
    let mut parts = Vec::new();
    let mut node = node.clone().parent().ok_or(AmlError::InvalidText)?;
    while let Some(parent) = node.parent() {
        match parent.kind() {
            "class_definition" | "function_definition" => {
                let name = parent
                    .named_child(0)
                    .ok_or(AmlError::InvalidText)?
                    .utf8_text(source.as_bytes())
                    .map(ToString::to_string)
                    .map_err(|_| AmlError::InvalidText)?;
                if parent.kind() == "class_definition" {
                    parts.push(name);
                } else {
                    parts.extend(vec!["<locals>".to_string(), name]);
                }
            }
            _ => {}
        }
        node = parent;
    }
    parts.reverse();
    Ok(parts.join("."))
}

/// Query wrapper for "all autometrics functions in source"
#[derive(Debug)]
pub(super) struct AmQuery {
    query: Query,
    /// Index of the capture for a function name.
    func_name_idx: u32,
}

impl AmQuery {
    /// Failible constructor.
    ///
    /// The constructor only fails if the given tree-sitter query does not have the
    /// necessary named captures.
    pub fn try_new(decorator_name: &str) -> Result<Self> {
        let am_query_str = format!(
            include_str!("../../runtime/queries/python/autometrics.scm.tpl"),
            decorator_name
        );
        let query = Query::new(language(), &am_query_str)?;
        let func_name_idx = query
            .capture_index_for_name(FUNC_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(FUNC_NAME_CAPTURE.to_string()))?;
        Ok(Self {
            query,
            func_name_idx,
        })
    }

    pub fn list_function_names(
        &self,
        file_name: &str,
        source: &str,
        module_name: &str,
    ) -> Result<Vec<FunctionInfo>> {
        let mut parser = new_parser()?;
        let parsed_source = parser.parse(source, None).ok_or(AmlError::Parsing)?;

        let mut cursor = tree_sitter::QueryCursor::new();
        cursor
            .matches(&self.query, parsed_source.root_node(), source.as_bytes())
            .filter_map(|m| {
                let node = m.nodes_for_capture_index(self.func_name_idx).next()?;
                let start = node.start_position();
                let end = node.end_position();
                let instrumentation = Some(Location::from((file_name, start, end)));
                let definition = Some(Location::from((file_name, start, end)));

                let func_name = node.utf8_text(source.as_bytes()).ok()?.to_string();
                let qualname = get_node_qualname(&node, source).ok()?;
                let full_name = if qualname.is_empty() {
                    func_name
                } else {
                    format!("{}.{}", qualname, func_name)
                };
                Some(Ok(FunctionInfo {
                    id: (module_name, full_name).into(),
                    instrumentation,
                    definition,
                }))
            })
            .collect::<std::result::Result<Vec<_>, _>>()
    }
}

/// Query wrapper for autometrics decorator imports in source
#[derive(Debug)]
pub(super) struct AmImportQuery {
    query: Query,
    /// Index of the capture for import alias
    import_alias_idx: u32,
}

impl AmImportQuery {
    /// Failible constructor.
    ///
    /// The constructor only fails if the given tree-sitter query does not have the
    /// necessary named captures.
    pub fn try_new() -> Result<Self> {
        let query = Query::new(
            language(),
            include_str!("../../runtime/queries/python/import.scm"),
        )?;
        let import_alias_idx = query
            .capture_index_for_name(IMPORT_ALIAS_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(IMPORT_ALIAS_CAPTURE.to_string()))?;
        Ok(Self {
            query,
            import_alias_idx,
        })
    }

    pub fn get_decorator_name(&self, source: &str) -> Result<String> {
        let mut parser = new_parser()?;
        let parsed_source = parser.parse(source, None).ok_or(AmlError::Parsing)?;

        let mut cursor = tree_sitter::QueryCursor::new();
        let matches = cursor
            .matches(&self.query, parsed_source.root_node(), source.as_bytes())
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(AmlError::InvalidText);
        }
        let alias = matches[0]
            .captures
            .iter()
            .find(|c| c.index == self.import_alias_idx)
            .map(|c| c.node.utf8_text(source.as_bytes()).map(ToString::to_string));
        match alias {
            Some(Ok(alias)) => Ok(alias),
            None => Ok("autometrics".to_string()),
            _ => Err(AmlError::InvalidText),
        }
    }
}

/// Query wrapper for "all functions in source"
#[derive(Debug)]
pub(super) struct AllFunctionsQuery {
    query: Query,
    /// Index of the capture for a function name.
    func_name_idx: u32,
}

impl AllFunctionsQuery {
    /// Failible constructor.
    ///
    /// The constructor only fails if the given tree-sitter query does not have the
    /// necessary named captures.
    pub fn try_new() -> Result<Self> {
        let query = Query::new(
            language(),
            include_str!("../../runtime/queries/python/all_functions.scm"),
        )?;
        let func_name_idx = query
            .capture_index_for_name(FUNC_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(FUNC_NAME_CAPTURE.to_string()))?;

        Ok(Self {
            query,
            func_name_idx,
        })
    }

    pub fn list_function_names(
        &self,
        file_name: &str,
        source: &str,
        module_name: &str,
    ) -> Result<Vec<FunctionInfo>> {
        let mut parser = new_parser()?;
        let parsed_source = parser.parse(source, None).ok_or(AmlError::Parsing)?;

        let mut cursor = tree_sitter::QueryCursor::new();
        cursor
            .matches(&self.query, parsed_source.root_node(), source.as_bytes())
            .filter_map(|capture| -> Option<Result<FunctionInfo>> {
                let node = capture
                    .captures
                    .iter()
                    .find(|c| c.index == self.func_name_idx)?
                    .node;
                let start = node.start_position();
                let end = node.end_position();
                let instrumentation = None;
                let definition = Some(Location::from((file_name, start, end)));
                let func_name = node.utf8_text(source.as_bytes()).ok()?.to_string();
                let qualname = get_node_qualname(&node, source).ok()?;
                let full_name = if qualname.is_empty() {
                    func_name
                } else {
                    format!("{}.{}", qualname, func_name)
                };
                Some(Ok(FunctionInfo {
                    id: (module_name, full_name).into(),
                    instrumentation,
                    definition,
                }))
            })
            .collect::<std::result::Result<Vec<_>, _>>()
    }
}
