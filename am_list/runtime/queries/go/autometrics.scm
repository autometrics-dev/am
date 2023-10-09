((package_clause
  (package_identifier) @pack.name)

 (comment) @dir.comment
 .
 (comment)*
 .
 (function_declaration
  name: (identifier) @func.name)
 (#match? @dir.comment "^//autometrics:(inst|doc)"))



((package_clause
  (package_identifier) @pack.name)

 (comment) @dir.comment
 .
 (comment)*
 .
 (method_declaration
  receiver: (parameter_list
              (parameter_declaration
                type: (type_identifier) @type.name))
  name: (field_identifier) @func.name)
 (#match? @dir.comment "^//autometrics:(inst|doc)"))

((package_clause
  (package_identifier) @pack.name)

 (comment) @dir.comment
 .
 (comment)*
 .
 (method_declaration
  receiver: (parameter_list
              (parameter_declaration
                type: (pointer_type (type_identifier) @type.name)))
  name: (field_identifier) @func.name)
 (#match? @dir.comment "^//autometrics:(inst|doc)"))
