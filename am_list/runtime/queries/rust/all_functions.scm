((function_item
  name: (identifier) @func.name))

;; It is impossible to do arbitrary levels of nesting, so we just detect module declarations to
;; call this query recursively on the declaration_list of the module.
;; Ref: https://github.com/tree-sitter/tree-sitter/discussions/981
((mod_item
  name: (identifier) @mod.name
  body: (declaration_list) @mod.contents))

;; We want to skip the "bare" function detection (@func.name pattern in this file) when function
;; is within an impl block. The reason is that we cannot properly report the module name (which should
;; be the struct type name) if we use this detection method.
;; Therefore, we skip bare functions that are detected within impl blocks, and instead rely on
;; recursion to find functions within impl blocks.
;;
;; We also consider that an "impl block" is an "in-file" module for the purpose of recursion
;; This allows to detect functions that have the autometrics annotation within an impl block,
;; _while allowing to skip functions in impl blocks detected by the main query_.
((impl_item
  type: (type_identifier) @impl.type
  body: (declaration_list) @impl.contents))
