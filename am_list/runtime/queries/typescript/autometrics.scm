;; TODO: this doesn't work as captures aren't shared between patterns.
;; This means we can't use @wrapper.atname in an #eq expression in the (call_expression) pattern afterwards
;; A recursion algorithm that uses a templated query might be the solution
((import_statement
  (import_clause
   (named_imports
    (import_specifier
     .
     name: (identifier) @wrapperdirect.name
     .)))
  source: (string (string_fragment) @lib.atname))
 (#match? @lib.atname "@autometrics\/autometrics|autometrics")
 (#eq? @wrapperdirect.name "autometrics"))

((import_statement
  (import_clause
   (named_imports
    (import_specifier
     name: (identifier) @real.name
     alias: (identifier) @wrapperdirect.name)))
  source: (string (string_fragment) @lib.name))
 (#match? @lib.name "@autometrics\/autometrics|autometrics")
 (#eq? @real.name "autometrics"))



;; TODO: this doesn't work as captures aren't shared between patterns.
;; This means we can't use @wrapperdirect.name in an #eq expression in the (call_expression) pattern afterwards
;; A recursion algorithm that uses a templated query might be the solution
((import_statement
  (import_clause
   (named_imports
    (import_specifier
     .
     name: (identifier) @wrapper.name
     .)))
  source: (string (string_fragment) @lib.name))
 (#match? @lib.name "@autometrics\/autometrics|autometrics")
 (#eq? @wrapper.name "autometrics"))

((import_statement
  (import_clause
   (named_imports
    (import_specifier
     name: (identifier) @real.name
     alias: (identifier) @wrapper.name)))
  source: (string (string_fragment) @lib.name))
 (#match? @lib.name "@autometrics\/autometrics|autometrics")
 (#eq? @real.name "autometrics"))

((class_declaration
  decorator: (decorator (identifier) @decorator.name)
  name: (type_identifier) @type.name
  body: (class_body
         [(method_signature
           name: (property_identifier) @method.name)
          (method_definition
           name: (property_identifier) @method.name)]))
 (#eq? @decorator.name "Autometrics"))
