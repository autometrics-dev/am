((package_clause
   (package_identifier) @pack.name)

 (comment) @dir.comment
 .
 (comment)*
 .
 (function_declaration
   name: (identifier) @func.name)
 (#match? @dir.comment "^//autometrics:(inst|doc)"))


