(function_declaration
 name: (identifier) @func.name)

(function
 name: (identifier) @func.name)

(class_declaration
 name: (type_identifier) @type.name
 body: (class_body
        [(method_signature
          name: (property_identifier) @method.name)
         (method_definition
          name: (property_identifier) @method.name)]))
