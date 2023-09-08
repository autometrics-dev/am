use crate::{AmlError, FunctionInfo, Location, Result, FUNC_NAME_CAPTURE};
use log::{trace, warn};
use tree_sitter::{Node, Parser, Query};
use tree_sitter_rust::language;

const ANNOTATED_IMPL_NAME_CAPTURE: &str = "type.impl";
const ANNOTATED_IMPL_METHOD_NAME_CAPTURE: &str = "inner.func.name";
const MOD_NAME_CAPTURE: &str = "mod.name";
const MOD_CONTENTS_CAPTURE: &str = "mod.contents";
const IMPL_NAME_CAPTURE: &str = "impl.type";
const IMPL_CONTENTS_CAPTURE: &str = "impl.contents";

const GRAMMAR_IMPL_ITEM_NODE_KIND: &str = "impl_item";
const GRAMMAR_MOD_ITEM_NODE_KIND: &str = "mod_item";

fn new_parser() -> Result<Parser> {
    let mut parser = Parser::new();
    parser.set_language(language())?;
    Ok(parser)
}

fn is_within_mod_item(node: Node, max_parent: Option<Node>, source: &str) -> bool {
    let mut walk = node;
    loop {
        if walk.kind() == GRAMMAR_MOD_ITEM_NODE_KIND {
            trace!(
                "Node was inside a mod.\nNode:{}\nMax Parent:{}\n",
                node.utf8_text(source.as_bytes())
                    .map(ToString::to_string)
                    .unwrap(),
                if let Some(node) = max_parent {
                    node.utf8_text(source.as_bytes())
                        .map(ToString::to_string)
                        .unwrap()
                } else {
                    source.to_string()
                }
            );
            break true;
        }
        if let Some(parent) = walk.parent() {
            if max_parent.map_or(false, |max_parent| parent.id() == max_parent.id()) {
                break false;
            }

            walk = parent;
            continue;
        }
        break false;
    }
}

fn is_within_impl_item(node: Node, max_parent: Option<Node>, source: &str) -> bool {
    let mut walk = node;
    loop {
        if walk.kind() == GRAMMAR_IMPL_ITEM_NODE_KIND {
            trace!(
                "Node was inside a impl block.\nNode:{}\nMax Parent:{}\n",
                node.utf8_text(source.as_bytes())
                    .map(ToString::to_string)
                    .unwrap(),
                if let Some(node) = max_parent {
                    node.utf8_text(source.as_bytes())
                        .map(ToString::to_string)
                        .unwrap()
                } else {
                    source.to_string()
                }
            );
            break true;
        }
        if let Some(parent) = walk.parent() {
            if max_parent.map_or(false, |max_parent| parent.id() == max_parent.id()) {
                break false;
            }

            walk = parent;
            continue;
        }
        break false;
    }
}

/// Query wrapper for "all autometrics functions in source"
#[derive(Debug)]
pub(super) struct AmQuery {
    query: Query,
    /// Index of the capture for a function name.
    func_name_idx: u32,
    /// Index of the capture for the type name of an `#[autometrics]`-annotated impl block
    ///
    /// This is an option, because when we want to list all functions, we do not want to use
    /// this capture ever (we will instead recurse into every impl block.)
    annotated_impl_type_name_idx: u32,
    /// Index of the capture for a method name within an `#[autometrics]`-annotated impl block
    ///
    /// This is an option, because when we want to list all functions, we do not want to use
    /// this capture ever (we will instead recurse into every impl block.)
    annotated_impl_method_name_idx: u32,
    /// Index of the capture for the name of a module that is defined in file.
    mod_name_idx: u32,
    /// Index of the capture for the contents of a module that is defined in file.
    mod_contents_idx: u32,
    /// Index of a capture for the type name associated to any impl block in the file.
    impl_type_idx: u32,
    /// Index of a capture for the contents (the declarations) associated to any impl block in the file.
    impl_contents_idx: u32,
}

impl AmQuery {
    /// Failible constructor.
    ///
    /// The constructor only fails if the given tree-sitter query does not have the
    /// necessary named captures.
    pub fn try_new() -> Result<Self> {
        let query = Query::new(
            language(),
            include_str!("../../runtime/queries/rust/autometrics.scm"),
        )?;

        let func_name_idx = query
            .capture_index_for_name(FUNC_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(FUNC_NAME_CAPTURE.into()))?;
        let annotated_impl_type_name_idx = query
            .capture_index_for_name(ANNOTATED_IMPL_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(ANNOTATED_IMPL_NAME_CAPTURE.into()))?;
        let annotated_impl_method_name_idx = query
            .capture_index_for_name(ANNOTATED_IMPL_METHOD_NAME_CAPTURE)
            .ok_or_else(|| {
                AmlError::MissingNamedCapture(ANNOTATED_IMPL_METHOD_NAME_CAPTURE.into())
            })?;
        let mod_name_idx = query
            .capture_index_for_name(MOD_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(MOD_NAME_CAPTURE.into()))?;
        let mod_contents_idx = query
            .capture_index_for_name(MOD_CONTENTS_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(MOD_NAME_CAPTURE.into()))?;
        let impl_type_idx = query
            .capture_index_for_name(IMPL_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(IMPL_NAME_CAPTURE.into()))?;
        let impl_contents_idx = query
            .capture_index_for_name(IMPL_CONTENTS_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(IMPL_CONTENTS_CAPTURE.into()))?;

        Ok(Self {
            query,
            func_name_idx,
            annotated_impl_type_name_idx,
            annotated_impl_method_name_idx,
            mod_name_idx,
            mod_contents_idx,
            impl_type_idx,
            impl_contents_idx,
        })
    }

    pub fn list_function_names(
        &self,
        file_name: &str,
        module: String,
        source: &str,
    ) -> Result<Vec<FunctionInfo>> {
        let mut parser = new_parser()?;
        let parsed_source = parser.parse(source, None).ok_or(AmlError::Parsing)?;
        self.list_function_rec(file_name, module, None, parsed_source.root_node(), source)
    }

    fn list_function_rec(
        &self,
        file_name: &str,
        current_module: String,
        current_type: Option<String>,
        node: Node,
        source: &str,
    ) -> Result<Vec<FunctionInfo>> {
        let mut res = Vec::new();
        let mut cursor = tree_sitter::QueryCursor::new();

        // Detect all functions directly in module scope
        let direct_names = self.list_direct_function_names(
            &mut cursor,
            node,
            file_name,
            source,
            &current_type,
            &current_module,
        );
        res.extend(direct_names);

        // Detect all methods from annotated impl blocks directly in module scope
        let impl_block_methods = self.list_annotated_impl_block_methods(
            &mut cursor,
            node,
            file_name,
            source,
            &current_module,
        );
        res.extend(impl_block_methods);

        // Detect all functions in submodule scope
        for capture in cursor.matches(&self.query, node, source.as_bytes()) {
            if let Some(mod_name_node) = capture.nodes_for_capture_index(self.mod_name_idx).next() {
                // We only want to consider module nodes that are direct children of the currently iterating node,
                // because the recursion will cleanly look for deeply nested module declarations.
                if mod_name_node
                    .parent()
                    .unwrap_or_else(|| panic!("The rust tree-sitter grammar guarantees that a mod_item:name has a mod_item as parent. {} capture is supposed to capture a mod_item:name", MOD_NAME_CAPTURE))
                    .parent() != Some(node) {
                    continue;
                }

                let mod_name = {
                    match mod_name_node
                        .utf8_text(source.as_bytes())
                        .map(ToString::to_string)
                    {
                        Ok(val) => val,
                        Err(e) => {
                            warn!("Error while extracting the module name: {e}");
                            continue;
                        }
                    }
                };

                if let Some(contents_node) = capture
                    .nodes_for_capture_index(self.mod_contents_idx)
                    .next()
                {
                    let new_module = if current_module.is_empty() {
                        mod_name
                    } else {
                        format!("{current_module}::{mod_name}")
                    };
                    trace!(
                        "Recursing into mod {}\n{}\n\n\n",
                        new_module,
                        contents_node
                            .utf8_text(source.as_bytes())
                            .map(ToString::to_string)
                            .unwrap()
                    );
                    let inner = self.list_function_rec(
                        file_name,
                        new_module,
                        current_type.clone(),
                        contents_node,
                        source,
                    )?;
                    res.extend(inner)
                }
            }

            if let Some(impl_type_node) = capture.nodes_for_capture_index(self.impl_type_idx).next()
            {
                // We only want to consider impl blocks that are direct children of the currently iterating node,
                // because the recursion will cleanly look for deeply nested module declarations.
                if impl_type_node
                    .parent()
                    .unwrap_or_else(|| panic!("The rust tree-sitter grammar guarantees that a impl_item:type_identifier has a impl_item as parent. {} capture is supposed to capture a mod_item:name", MOD_NAME_CAPTURE))
                    .parent() != Some(node) {
                    continue;
                }

                let type_name = {
                    match impl_type_node
                        .utf8_text(source.as_bytes())
                        .map(ToString::to_string)
                    {
                        Ok(val) => val,
                        Err(e) => {
                            warn!("Error extracting the struct name: {e}");
                            continue;
                        }
                    }
                };

                if let Some(contents_node) = capture
                    .nodes_for_capture_index(self.impl_contents_idx)
                    .next()
                {
                    trace!(
                        "Recursing into impl block {}::{}\n{}\n\n\n",
                        current_module,
                        type_name,
                        contents_node
                            .utf8_text(source.as_bytes())
                            .map(ToString::to_string)
                            .unwrap()
                    );
                    let inner = self.list_function_rec(
                        file_name,
                        current_module.clone(),
                        Some(type_name),
                        contents_node,
                        source,
                    )?;
                    res.extend(inner)
                }
            }
        }

        Ok(res)
    }

    fn list_direct_function_names(
        &self,
        cursor: &mut tree_sitter::QueryCursor,
        node: Node,
        file_name: &str,
        source: &str,
        current_type: &Option<String>,
        current_module: &str,
    ) -> Vec<FunctionInfo> {
        cursor
            .matches(&self.query, node, source.as_bytes())
            .filter_map(|capture| -> Option<FunctionInfo> {
                let fn_node: Node = capture.nodes_for_capture_index(self.func_name_idx).next()?;

                // Ignore the matches that are within a mod_item, as the recursion will catch it later with the fully qualified module name.
                if is_within_mod_item(fn_node, Some(node), source) {
                    return None;
                }

                // Ignore the matches that are within a impl_item, as the impl_block_names variable below catches those, applying the
                // fully qualified module name.
                if is_within_impl_item(fn_node, Some(node), source) {
                    return None;
                }

                let start = fn_node.start_position();
                let end = fn_node.end_position();
                let instrumentation = Some(Location::from((file_name, start, end)));
                let definition = Some(Location::from((file_name, start, end)));

                let fn_name: std::result::Result<String, std::str::Utf8Error> = fn_node
                    .utf8_text(source.as_bytes())
                    .map(ToString::to_string);

                let type_prefix: String = current_type
                    .as_ref()
                    .map(|t| format!("{t}::"))
                    .unwrap_or_default();

                match fn_name {
                    Ok(f) => Some(FunctionInfo {
                        id: (current_module, format!("{type_prefix}{f}")).into(),
                        instrumentation,
                        definition,
                    }),
                    Err(e) => {
                        warn!("Could not get the method name: {e}");
                        None
                    }
                }
            })
            .collect()
    }

    fn list_annotated_impl_block_methods(
        &self,
        cursor: &mut tree_sitter::QueryCursor,
        node: Node,
        file_name: &str,
        source: &str,
        current_module: &str,
    ) -> Vec<FunctionInfo> {
        cursor
            .matches(&self.query, node, source.as_bytes())
            .filter_map(|capture| -> Option<FunctionInfo> {
                let fn_node: Node = capture
                    .nodes_for_capture_index(self.annotated_impl_method_name_idx)
                    .next()?;

                // Ignore the matches that are within a mod_item, as the recursion will catch it later with the fully qualified module name.
                if is_within_mod_item(fn_node, Some(node), source) {
                    return None;
                }

                let fn_name = fn_node
                    .utf8_text(source.as_bytes())
                    .map(ToString::to_string);
                let struct_name = capture
                    .nodes_for_capture_index(self.annotated_impl_type_name_idx)
                    .next()
                    .map(|node| node.utf8_text(source.as_bytes()).map(ToString::to_string))?;

                let start = fn_node.start_position();
                let end = fn_node.end_position();
                let instrumentation = Some(Location::from((file_name, start, end)));
                let definition = Some(Location::from((file_name, start, end)));

                match (struct_name, fn_name) {
                    (Ok(s), Ok(f)) => Some(FunctionInfo {
                        id: (current_module, format!("{s}::{f}")).into(),
                        instrumentation,
                        definition,
                    }),
                    (Err(e), _) => {
                        warn!("Could not extract the name of the struct: {e}");
                        None
                    }
                    (_, Err(e)) => {
                        warn!("Could not extract the name of the method: {e}");
                        None
                    }
                }
            })
            .collect()
    }
}

/// Query wrapper for "all functions in source"
#[derive(Debug)]
pub(super) struct AllFunctionsQuery {
    query: Query,
    /// Index of the capture for a function name.
    func_name_idx: u32,
    /// Index of the capture for the name of a module that is defined in file.
    mod_name_idx: u32,
    /// Index of the capture for the contents of a module that is defined in file.
    mod_contents_idx: u32,
    /// Index of a capture for the type name associated to any impl block in the file.
    impl_type_idx: u32,
    /// Index of a capture for the contents (the declarations) associated to any impl block in the file.
    impl_contents_idx: u32,
}

impl AllFunctionsQuery {
    /// Failible constructor.
    ///
    /// The constructor only fails if the given tree-sitter query does not have the
    /// necessary named captures.
    pub fn try_new() -> Result<Self> {
        let query = Query::new(
            language(),
            include_str!("../../runtime/queries/rust/all_functions.scm"),
        )?;

        let func_name_idx = query
            .capture_index_for_name(FUNC_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(FUNC_NAME_CAPTURE.into()))?;
        let mod_name_idx = query
            .capture_index_for_name(MOD_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(MOD_NAME_CAPTURE.into()))?;
        let mod_contents_idx = query
            .capture_index_for_name(MOD_CONTENTS_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(MOD_NAME_CAPTURE.into()))?;
        let impl_type_idx = query
            .capture_index_for_name(IMPL_NAME_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(IMPL_NAME_CAPTURE.into()))?;
        let impl_contents_idx = query
            .capture_index_for_name(IMPL_CONTENTS_CAPTURE)
            .ok_or_else(|| AmlError::MissingNamedCapture(IMPL_CONTENTS_CAPTURE.into()))?;

        Ok(Self {
            query,
            func_name_idx,
            mod_name_idx,
            mod_contents_idx,
            impl_type_idx,
            impl_contents_idx,
        })
    }

    pub fn list_function_names(
        &self,
        file_name: &str,
        module: String,
        source: &str,
    ) -> Result<Vec<FunctionInfo>> {
        let mut parser = new_parser()?;
        let parsed_source = parser.parse(source, None).ok_or(AmlError::Parsing)?;
        self.list_function_rec(file_name, module, None, parsed_source.root_node(), source)
    }

    fn list_function_rec(
        &self,
        file_name: &str,
        current_module: String,
        current_type: Option<String>,
        node: Node,
        source: &str,
    ) -> Result<Vec<FunctionInfo>> {
        let mut res = Vec::new();
        let mut cursor = tree_sitter::QueryCursor::new();

        // Detect all functions directly in module scope
        let direct_names = self.list_direct_function_names(
            &mut cursor,
            node,
            file_name,
            source,
            current_type,
            &current_module,
        );
        res.extend(direct_names);

        // Detect all functions in submodule scope
        for capture in cursor.matches(&self.query, node, source.as_bytes()) {
            if let Some(mod_name_node) = capture.nodes_for_capture_index(self.mod_name_idx).next() {
                // We only want to consider module nodes that are direct children of the currently iterating node,
                // because the recursion will cleanly look for deeply nested module declarations.
                if mod_name_node
                    .parent()
                    .unwrap_or_else(|| panic!("The rust tree-sitter grammar guarantees that a mod_item:name has a mod_item as parent. {} capture is supposed to capture a mod_item:name", MOD_NAME_CAPTURE))
                    .parent() != Some(node) {
                    continue;
                }

                let mod_name = {
                    match mod_name_node
                        .utf8_text(source.as_bytes())
                        .map(ToString::to_string)
                    {
                        Ok(val) => val,
                        Err(e) => {
                            warn!("Could not extract module name from a capture: {e}");
                            continue;
                        }
                    }
                };

                if let Some(contents_node) = capture
                    .nodes_for_capture_index(self.mod_contents_idx)
                    .next()
                {
                    let new_module = if current_module.is_empty() {
                        mod_name
                    } else {
                        format!("{current_module}::{mod_name}")
                    };
                    trace!(
                        "Recursing into mod {}\n{}\n\n\n",
                        new_module,
                        contents_node
                            .utf8_text(source.as_bytes())
                            .map(ToString::to_string)
                            .unwrap()
                    );
                    let inner =
                        self.list_function_rec(file_name, new_module, None, contents_node, source)?;
                    res.extend(inner.into_iter())
                }
            }

            if let Some(impl_type_node) = capture.nodes_for_capture_index(self.impl_type_idx).next()
            {
                // We only want to consider impl blocks that are direct children of the currently iterating node,
                // because the recursion will cleanly look for deeply nested module declarations.
                if impl_type_node
                    .parent()
                    .unwrap_or_else(|| panic!("The rust tree-sitter grammar guarantees that a impl_item:type_identifier has a impl_item as parent. {} capture is supposed to capture a mod_item:name", MOD_NAME_CAPTURE))
                    .parent() != Some(node) {
                    continue;
                }

                let type_name = {
                    match impl_type_node
                        .utf8_text(source.as_bytes())
                        .map(ToString::to_string)
                    {
                        Ok(val) => val,
                        Err(e) => {
                            warn!("Could not extract the type name of the impl block: {e}");
                            continue;
                        }
                    }
                };

                if let Some(contents_node) = capture
                    .nodes_for_capture_index(self.impl_contents_idx)
                    .next()
                {
                    trace!(
                        "Recursing into impl block {}::{}\n{}\n\n\n",
                        current_module,
                        type_name,
                        contents_node
                            .utf8_text(source.as_bytes())
                            .map(ToString::to_string)
                            .unwrap()
                    );
                    let inner = self.list_function_rec(
                        file_name,
                        current_module.clone(),
                        Some(type_name),
                        contents_node,
                        source,
                    )?;
                    res.extend(inner.into_iter())
                }
            }
        }

        Ok(res)
    }

    fn list_direct_function_names(
        &self,
        cursor: &mut tree_sitter::QueryCursor,
        node: Node,
        file_name: &str,
        source: &str,
        current_type: Option<String>,
        current_module: &str,
    ) -> Vec<FunctionInfo> {
        cursor
            .matches(&self.query, node, source.as_bytes())
            .filter_map(|capture| -> Option<FunctionInfo> {
                let fn_node: Node = capture.nodes_for_capture_index(self.func_name_idx).next()?;

                // Ignore the matches that are within a mod_item, as the recursion will catch it later with the fully qualified module name.
                if is_within_mod_item(fn_node, Some(node), source) {
                    return None;
                }

                // Ignore the matches that are within a impl_item, as the impl_block_names variable below catches those, applying the
                // fully qualified module name.
                if is_within_impl_item(fn_node, Some(node), source) {
                    return None;
                }

                let fn_name: std::result::Result<String, std::str::Utf8Error> = fn_node
                    .utf8_text(source.as_bytes())
                    .map(ToString::to_string);

                let type_prefix: String = current_type
                    .as_ref()
                    .map(|t| format!("{t}::"))
                    .unwrap_or_default();

                let start = fn_node.start_position();
                let end = fn_node.end_position();
                let instrumentation = None;
                let definition = Some(Location::from((file_name, start, end)));

                match fn_name {
                    Ok(f) => Some(FunctionInfo {
                        id: (current_module, format!("{type_prefix}{f}")).into(),
                        instrumentation,
                        definition,
                    }),
                    Err(e) => {
                        warn!("Could not get the method name: {e}");
                        None
                    }
                }
            })
            .collect()
    }
}
