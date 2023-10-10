((package_clause
   (package_identifier) @pack.name)

 (function_declaration
   name: (identifier) @func.name))

((package_clause
   (package_identifier) @pack.name)

 (method_declaration
   receiver: (parameter_list
               (parameter_declaration
                          type: (type_identifier) @type.name))
   name: (field_identifier) @func.name))

((package_clause
   (package_identifier) @pack.name)

 (method_declaration
   receiver: (parameter_list
               (parameter_declaration
                          type: (pointer_type (type_identifier) @type.name)))
   name: (field_identifier) @func.name))
