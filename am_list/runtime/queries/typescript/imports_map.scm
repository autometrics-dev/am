;; This query extracts all the imports of the current source
;; NOTE: it is impossible to merge "looking for imports" with the
;; "looking for autometricized functions" queries for at least 2 reasons:
;; - The "call_expression" is not necessarily a sibling node to the imports, and it's not possible
;;   to match a "call_expression" as an arbitrarily deep "cousin" of the import_clause node.
;; - There is no builtin `#prefix?` operator, which makes checking for namespaced imports
;;   impossible to do in 1 query

((import_statement
  (import_clause
   (named_imports
    (import_specifier
     name: (identifier) @inst.ident)))
  source: (string (string_fragment) @inst.source)))

(import_statement
 (import_clause
  (named_imports
   (import_specifier
    name: (identifier) @inst.realname
    alias: (identifier) @inst.ident)))
 source: (string (string_fragment) @inst.source))

(import_statement
 (import_clause
  (namespace_import (identifier) @inst.prefix))
 source: (string (string_fragment) @inst.source))
