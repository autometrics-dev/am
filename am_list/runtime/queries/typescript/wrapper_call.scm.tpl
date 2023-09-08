((call_expression
   function: (identifier) @wrapper.call
   arguments: (arguments
               .
               (object
                (pair
                 key: (property_identifier) @func.prop
                 value: (string (string_fragment) @func.name))
                (pair
                 key: (property_identifier) @mod.prop
                 value: (string (string_fragment) @module.name)))))
 (#eq? @wrapper.call "{0}")
 (#eq? @func.prop "functionName")
 (#eq? @mod.prop "moduleName"))

((call_expression
   function: (identifier) @wrapper.call
   arguments: (arguments
               .
               (object
                (pair
                 key: (property_identifier) @mod.prop
                 value: (string (string_fragment) @module.name))
                (pair
                 key: (property_identifier) @func.prop
                 value: (string (string_fragment) @func.name)))))
 (#eq? @wrapper.call "{0}")
 (#eq? @func.prop "functionName")
 (#eq? @mod.prop "moduleName"))
