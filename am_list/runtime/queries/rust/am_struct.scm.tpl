((impl_item
   type: (type_identifier) @type.impl
   body: (declaration_list
           (function_item
             name: (identifier) @func.name)))
 (#match? @type.impl "({}){{1,1}}"))
