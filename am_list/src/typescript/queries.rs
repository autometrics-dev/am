use std::path::Path;

use log::warn;
use tree_sitter::{Parser, Query};
use tree_sitter_typescript::language_typescript as language;

use crate::{AmlError, FunctionInfo, Location, Result, FUNC_NAME_CAPTURE};

use super::imports::{Identifier, ImportsMap, Source};

const TYPE_NAME_CAPTURE: &str = "type.name";
const METHOD_NAME_CAPTURE: &str = "method.name";
const WRAPPER_DIRECT_NAME_CAPTURE: &str = "wrapperdirect.name";
const WRAPPER_NAME_CAPTURE: &str = "wrapper.name";
const WRAPPER_ARGS_MODULE_CAPTURE: &str = "module.name";

const IMPORTS_IDENT_NAME_CAPTURE: &str = "inst.ident";
const IMPORTS_REAL_NAME_CAPTURE: &str = "inst.realname";
const IMPORTS_SOURCE_CAPTURE: &str = "inst.source";
const IMPORTS_PREFIX_CAPTURE: &str = "inst.prefix";

fn new_parser() -> Result<Parser> {
    let mut parser = Parser::new();
    parser.set_language(language())?;
    Ok(parser)
}

/// Query wrapper for "all functions in source"
#[derive(Debug)]
pub(super) struct AllFunctionsQuery {
    query: Query,
    /// Index of the capture for a function name.
    func_name_idx: u32,
    /// Index of the capture for the name of a class that is defined in file.
    type_name_idx: u32,
    /// Index of the capture for the contents of a method that is defined in file.
    method_name_idx: u32,
}

impl AllFunctionsQuery {
    pub fn try_new() -> Result<Self> {
        let query = Query::new(
            language(),
            include_str!("../../runtime/queries/typescript/all_functions.scm"),
        )?;
        let func_name_idx = query
            .capture_index_for_name(FUNC_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(FUNC_NAME_CAPTURE.to_string()))?;
        let type_name_idx = query
            .capture_index_for_name(TYPE_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(TYPE_NAME_CAPTURE.to_string()))?;
        let method_name_idx = query
            .capture_index_for_name(METHOD_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(METHOD_NAME_CAPTURE.to_string()))?;

        Ok(Self {
            query,
            func_name_idx,
            type_name_idx,
            method_name_idx,
        })
    }

    pub fn list_function_names(
        &self,
        file_name: &str,
        module_name: &str,
        source: &str,
    ) -> Result<Vec<FunctionInfo>> {
        let mut parser = new_parser()?;
        let parsed_source = parser.parse(source, None).ok_or(AmlError::Parsing)?;
        let mut cursor = tree_sitter::QueryCursor::new();
        let functions = cursor
            .matches(&self.query, parsed_source.root_node(), source.as_bytes())
            .filter_map(|capture| -> Option<FunctionInfo> {
                let func_name_node = capture.nodes_for_capture_index(self.func_name_idx).next();
                let method_name_node = capture.nodes_for_capture_index(self.method_name_idx).next();
                let type_name_node = capture.nodes_for_capture_index(self.type_name_idx).next();
                match (
                    // Test for bare function capture
                    func_name_node
                        .map(|node| node.utf8_text(source.as_bytes()).map(ToString::to_string)),
                    // Test for method name capture
                    method_name_node
                        .map(|node| node.utf8_text(source.as_bytes()).map(ToString::to_string)),
                    // Test for class name capture
                    type_name_node
                        .map(|node| node.utf8_text(source.as_bytes()).map(ToString::to_string)),
                ) {
                    (Some(Ok(bare_function_name)), _, _) => {
                        let start = func_name_node
                            .expect("just extracted a name from the node")
                            .start_position();
                        let end = func_name_node
                            .expect("just extracted a name from the node")
                            .end_position();
                        let instrumentation = None;
                        let definition = Some(Location::from((file_name, start, end)));
                        Some(FunctionInfo {
                            id: (module_name, bare_function_name).into(),
                            instrumentation,
                            definition,
                        })
                    }
                    (_, Some(Ok(method_name)), Some(Ok(class_name))) => {
                        let start = method_name_node
                            .expect("just extracted a name from the node")
                            .start_position();
                        let end = method_name_node
                            .expect("just extracted a name from the node")
                            .end_position();
                        let instrumentation = None;
                        let definition = Some(Location::from((file_name, start, end)));
                        let qual_fn_name = format!("{class_name}.{method_name}");
                        Some(FunctionInfo {
                            id: (module_name, qual_fn_name).into(),
                            instrumentation,
                            definition,
                        })
                    }
                    (_, None, Some(_)) => {
                        warn!("Found a class without a method in the capture");
                        None
                    }
                    (_, Some(_), None) => {
                        warn!("Found a method without a class in the capture");
                        None
                    }
                    (Some(Err(e)), _, _) => {
                        warn!("Could not extract a function name: {e}");
                        None
                    }
                    (_, Some(Err(e)), _) => {
                        warn!("Could not extract a method name: {e}");
                        None
                    }
                    (_, _, Some(Err(e))) => {
                        warn!("Could not extract a class name: {e}");
                        None
                    }
                    _ => None,
                }
            })
            .collect();

        Ok(functions)
    }
}

/// Query wrapper for "all autometrics functions in source"
#[derive(Debug)]
pub(super) struct AmQuery {
    query: Query,
    /// Index of the capture for a class name defined in the file.
    type_name_idx: u32,
    /// Index of the capture for a method name defined in the file.
    method_name_idx: u32,
    /// Index of the capture for the name of the autometrics wrapper that takes
    /// directly the function as argument.
    wrapper_direct_name_idx: u32,
    /// Index of the capture for the name of the autometrics wrapper that takes
    /// 2 arguments.
    wrapper_name_idx: u32,
}

impl AmQuery {
    /// Failible constructor.
    ///
    /// The constructor only fails if the given tree-sitter query does not have the
    /// necessary named captures.
    pub fn try_new() -> Result<Self> {
        let query = Query::new(
            language(),
            include_str!("../../runtime/queries/typescript/autometrics.scm"),
        )?;
        let type_name_idx = query
            .capture_index_for_name(TYPE_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(TYPE_NAME_CAPTURE.to_string()))?;
        let method_name_idx = query
            .capture_index_for_name(METHOD_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(METHOD_NAME_CAPTURE.to_string()))?;
        let wrapper_direct_name_idx = query
            .capture_index_for_name(WRAPPER_DIRECT_NAME_CAPTURE)
            .ok_or_else(|| {
                AmlError::MissingNamedCapture(WRAPPER_DIRECT_NAME_CAPTURE.to_string())
            })?;
        let wrapper_name_idx = query
            .capture_index_for_name(WRAPPER_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(WRAPPER_NAME_CAPTURE.to_string()))?;

        Ok(Self {
            query,
            type_name_idx,
            method_name_idx,
            wrapper_direct_name_idx,
            wrapper_name_idx,
        })
    }

    pub fn list_function_names(
        &self,
        file_name: &str,
        module_name: &str,
        source: &str,
        path: Option<&Path>,
    ) -> Result<Vec<FunctionInfo>> {
        let mut parser = new_parser()?;
        let parsed_source = parser.parse(source, None).ok_or(AmlError::Parsing)?;

        let imports_query = ImportsMapQuery::try_new()?;
        let imports_map = imports_query.list_imports(path, source)?;

        let mut cursor = tree_sitter::QueryCursor::new();
        let wrapper_direct_name = cursor
            .matches(&self.query, parsed_source.root_node(), source.as_bytes())
            .filter_map(|capture| {
                capture
                    .nodes_for_capture_index(self.wrapper_direct_name_idx)
                    .next()
            })
            .map(|node| {
                node.utf8_text(source.as_bytes())
                    .map(ToString::to_string)
                    .map_err(|_| AmlError::InvalidText)
            })
            .next()
            .transpose()?;
        let mut wrapped_fns_list = if wrapper_direct_name.is_none() {
            Vec::new()
        } else {
            let subquery = AmWrapperDirectSubquery::try_new(wrapper_direct_name.unwrap())?;
            subquery.list_function_names(file_name, module_name, source, imports_map)?
        };

        let wrapper_name = cursor
            .matches(&self.query, parsed_source.root_node(), source.as_bytes())
            .filter_map(|capture| {
                capture
                    .nodes_for_capture_index(self.wrapper_name_idx)
                    .next()
            })
            .map(|node| {
                node.utf8_text(source.as_bytes())
                    .map(ToString::to_string)
                    .map_err(|_| AmlError::InvalidText)
            })
            .next()
            .transpose()?;
        if let Some(wrapper_name) = wrapper_name {
            let subquery = AmWrapperSubquery::try_new(wrapper_name)?;
            wrapped_fns_list.extend(subquery.list_function_names(file_name, source)?)
        }

        cursor = tree_sitter::QueryCursor::new();
        let mut method_list: Vec<FunctionInfo> = cursor
            .matches(&self.query, parsed_source.root_node(), source.as_bytes())
            .filter_map(|capture| -> Option<FunctionInfo> {
                // Bare functions are handled by the subquery list_function_names method
                let method_name_node = capture.nodes_for_capture_index(self.method_name_idx).next();
                let type_name_node = capture.nodes_for_capture_index(self.type_name_idx).next();
                match (
                    // Test for Method name capture
                    method_name_node
                        .map(|node| node.utf8_text(source.as_bytes()).map(ToString::to_string)),
                    // Test for class name capture
                    type_name_node
                        .map(|node| node.utf8_text(source.as_bytes()).map(ToString::to_string)),
                ) {
                    (Some(Ok(method_name)), Some(Ok(class_name))) => {
                        let qual_fn_name = format!("{class_name}.{method_name}");
                        let start = method_name_node
                            .expect("just extracted a name from the node")
                            .start_position();
                        let end = method_name_node
                            .expect("just extracted a name from the node")
                            .end_position();
                        let instrumentation = Some(Location::from((file_name, start, end)));
                        let definition = Some(Location::from((file_name, start, end)));
                        Some(FunctionInfo {
                            id: (module_name, qual_fn_name).into(),
                            instrumentation,
                            definition,
                        })
                    }
                    (None, Some(_)) => {
                        warn!("Found a class without a method in the capture");
                        None
                    }
                    (Some(_), None) => {
                        warn!("Found a method without a class in the capture");
                        None
                    }
                    (Some(Err(e)), _) => {
                        warn!("Could not extract a method name: {e}");
                        None
                    }
                    (_, Some(Err(e))) => {
                        warn!("Could not extract a class name: {e}");
                        None
                    }
                    _ => None,
                }
            })
            .collect();

        // Concatenate list of methods and list of wrapped functions
        method_list.append(&mut wrapped_fns_list);
        Ok(method_list)
    }
}

/// Query wrapper for "all function arguments to the given wrapper_name in source"
#[derive(Debug)]
struct AmWrapperSubquery {
    query: Query,
    /// Name of the wrapper function to look for
    // Having the wrapper_name is useful when debugging the queries
    #[allow(dead_code)]
    wrapper_name: String,
    /// Index of the capture for a function name.
    func_name_idx: u32,
    /// Index of the capture for a module name.
    module_name_idx: u32,
}

impl AmWrapperSubquery {
    /// Failible constructor.
    ///
    /// The constructor only fails if the given tree-sitter query does not have the
    /// necessary named captures.
    pub fn try_new(wrapper_name: String) -> Result<Self> {
        let wrapped_query_str = format!(
            include_str!("../../runtime/queries/typescript/wrapper_call.scm.tpl"),
            wrapper_name
        );
        let query = Query::new(language(), &wrapped_query_str)?;
        let func_name_idx = query
            .capture_index_for_name(FUNC_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(FUNC_NAME_CAPTURE.to_string()))?;
        let module_name_idx = query
            .capture_index_for_name(WRAPPER_ARGS_MODULE_CAPTURE)
            .ok_or_else(|| {
                AmlError::MissingNamedCapture(WRAPPER_ARGS_MODULE_CAPTURE.to_string())
            })?;

        Ok(Self {
            query,
            wrapper_name,
            func_name_idx,
            module_name_idx,
        })
    }

    pub fn list_function_names(&self, file_name: &str, source: &str) -> Result<Vec<FunctionInfo>> {
        let mut parser = new_parser()?;
        let parsed_source = parser.parse(source, None).ok_or(AmlError::Parsing)?;
        let mut cursor = tree_sitter::QueryCursor::new();
        let functions = cursor
            .matches(&self.query, parsed_source.root_node(), source.as_bytes())
            .filter_map(|capture| -> Option<FunctionInfo> {
                // Bare function
                let func_name_node = capture.nodes_for_capture_index(self.func_name_idx).next();
                let module_name_node = capture.nodes_for_capture_index(self.module_name_idx).next();

                match (
                    module_name_node
                        .map(|node| node.utf8_text(source.as_bytes()).map(ToString::to_string)),
                    func_name_node
                        .map(|node| node.utf8_text(source.as_bytes()).map(ToString::to_string)),
                ) {
                    (Some(Ok(module)), Some(Ok(function))) => {
                        let start = func_name_node
                            .expect("just extracted a name from the node")
                            .start_position();
                        let end = func_name_node
                            .expect("just extracted a name from the node")
                            .end_position();
                        let definition = None;
                        let instrumentation = Some(Location::from((file_name, start, end)));
                        Some(FunctionInfo {
                            id: (module, function).into(),
                            instrumentation,
                            definition,
                        })
                    }
                    (_, Some(Err(e))) => {
                        warn!("Could not extract a function name: {e}");
                        None
                    }
                    (Some(Err(e)), _) => {
                        warn!("Could not extract a module name: {e}");
                        None
                    }
                    _ => None,
                }
            })
            .collect();
        Ok(functions)
    }
}

/// Query wrapper for functions called in the autometrics wrapper that takes
/// directly the function as arguments.
#[derive(Debug)]
struct AmWrapperDirectSubquery {
    query: Query,
    /// Name of the wrapper function to look for
    // Having the wrapper_name is useful when debugging the queries
    #[allow(dead_code)]
    wrapper_name: String,
    /// Index of the capture for the function name being called in the wrapper.
    func_name_idx: u32,
}

impl AmWrapperDirectSubquery {
    /// Failible constructor.
    ///
    /// The constructor only fails if the given tree-sitter query does not have the
    /// necessary named captures.
    pub fn try_new(wrapper_name: String) -> Result<Self> {
        let wrapped_query_str = format!(
            include_str!("../../runtime/queries/typescript/wrapper_direct_call.scm.tpl"),
            wrapper_name
        );
        let query = Query::new(language(), &wrapped_query_str)?;
        let func_name_idx = query
            .capture_index_for_name(FUNC_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(FUNC_NAME_CAPTURE.to_string()))?;

        Ok(Self {
            query,
            wrapper_name,
            func_name_idx,
        })
    }

    pub fn list_function_names(
        &self,
        file_name: &str,
        module_name: &str,
        source: &str,
        imports_map: ImportsMap,
    ) -> Result<Vec<FunctionInfo>> {
        let mut parser = new_parser()?;
        let parsed_source = parser.parse(source, None).ok_or(AmlError::Parsing)?;
        let mut cursor = tree_sitter::QueryCursor::new();
        let functions = cursor
            .matches(&self.query, parsed_source.root_node(), source.as_bytes())
            .filter_map(|capture| -> Option<FunctionInfo> {
                // Bare function
                let func_name_node = capture.nodes_for_capture_index(self.func_name_idx).next();
                match func_name_node
                    .map(|node| node.utf8_text(source.as_bytes()).map(ToString::to_string))
                {
                    Some(Ok(fn_name)) => {
                        let start = func_name_node
                            .expect("just extracted a name from the node")
                            .start_position();
                        let end = func_name_node
                            .expect("just extracted a name from the node")
                            .end_position();
                        let definition = None;
                        let instrumentation = Some(Location::from((file_name, start, end)));
                        if let Some((ident, source)) =
                            imports_map.resolve_ident(Identifier::from(&fn_name))
                        {
                            Some(FunctionInfo {
                                id: (source, ident).into(),
                                instrumentation,
                                definition,
                            })
                        } else {
                            Some(FunctionInfo {
                                id: (module_name, fn_name).into(),
                                instrumentation,
                                definition,
                            })
                        }
                    }
                    Some(Err(e)) => {
                        warn!("Could not extract a function name: {e}");
                        None
                    }
                    _ => None,
                }
            })
            .collect();
        Ok(functions)
    }
}

/// Query wrapper for imports in the source
#[derive(Debug)]
pub(super) struct ImportsMapQuery {
    query: Query,
    /// Index of the capture for a named import in the source.
    named_import_idx: u32,
    /// Index of the capture for a namespace import in the source.
    prefixed_import_idx: u32,
    /// Index of the capture for the real name of an aliased import in the
    /// source.
    import_og_name_idx: u32,
    /// Index of the capture for the source of the import statement being captured.
    source_idx: u32,
}

impl ImportsMapQuery {
    /// Failible constructor.
    ///
    /// The constructor only fails if the given tree-sitter query does not have the
    /// necessary named captures.
    pub fn try_new() -> Result<Self> {
        let query = Query::new(
            language(),
            include_str!("../../runtime/queries/typescript/imports_map.scm"),
        )?;
        let named_import_idx = query
            .capture_index_for_name(IMPORTS_IDENT_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(IMPORTS_IDENT_NAME_CAPTURE.to_string()))?;
        let prefixed_import_idx = query
            .capture_index_for_name(IMPORTS_PREFIX_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(IMPORTS_PREFIX_CAPTURE.to_string()))?;
        let import_og_name_idx = query
            .capture_index_for_name(IMPORTS_REAL_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(IMPORTS_REAL_NAME_CAPTURE.to_string()))?;
        let source_idx = query
            .capture_index_for_name(IMPORTS_SOURCE_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(IMPORTS_SOURCE_CAPTURE.to_string()))?;

        Ok(Self {
            query,
            named_import_idx,
            prefixed_import_idx,
            import_og_name_idx,
            source_idx,
        })
    }

    pub fn list_imports(&self, file_path: Option<&Path>, source: &str) -> Result<ImportsMap> {
        let mut res = ImportsMap::default();

        let mut parser = new_parser()?;
        let parsed_source = parser.parse(source, None).ok_or(AmlError::Parsing)?;
        let mut cursor = tree_sitter::QueryCursor::new();
        for capture in cursor.matches(&self.query, parsed_source.root_node(), source.as_bytes()) {
            // Check for a namespaced capture
            if let Some(sub_match) = capture
                .nodes_for_capture_index(self.prefixed_import_idx)
                .next()
            {
                let prefix: Identifier = sub_match
                    .utf8_text(source.as_bytes())
                    .map(ToString::to_string)
                    .map_err(|_| AmlError::InvalidText)?
                    .into();
                let import_source: Source = capture
                    .nodes_for_capture_index(self.source_idx)
                    .next()
                    .unwrap_or_else(|| {
                        panic!(
                            "the capture for {} has a capture for {}",
                            IMPORTS_PREFIX_CAPTURE, IMPORTS_SOURCE_CAPTURE
                        )
                    })
                    .utf8_text(source.as_bytes())
                    .map_err(|_| AmlError::InvalidText)?
                    .into();

                res.add_namespace(prefix, import_source.into_canonical(file_path));
            }

            // Check for the other capture
            if let Some(sub_match) = capture
                .nodes_for_capture_index(self.named_import_idx)
                .next()
            {
                let ident_name: Identifier = sub_match
                    .utf8_text(source.as_bytes())
                    .map_err(|_| AmlError::InvalidText)?
                    .into();
                let real_name: Option<Identifier> = capture
                    .nodes_for_capture_index(self.import_og_name_idx)
                    .next()
                    .map(|node| -> Result<&str> {
                        node.utf8_text(source.as_bytes())
                            .map_err(|_| AmlError::InvalidText)
                    })
                    .transpose()?
                    .map(Into::into);
                let import_source: Source = capture
                    .nodes_for_capture_index(self.source_idx)
                    .next()
                    .unwrap_or_else(|| {
                        panic!(
                            "the capture for {} has a capture for {}",
                            IMPORTS_IDENT_NAME_CAPTURE, IMPORTS_SOURCE_CAPTURE
                        )
                    })
                    .utf8_text(source.as_bytes())
                    .map_err(|_| AmlError::InvalidText)?
                    .into();

                if let Some(real_name) = real_name {
                    res.add_aliased_import(
                        ident_name,
                        real_name,
                        import_source.into_canonical(file_path),
                    );
                } else {
                    res.add_named_import(ident_name, import_source.into_canonical(file_path));
                }
            }
        }

        Ok(res)
    }
}
