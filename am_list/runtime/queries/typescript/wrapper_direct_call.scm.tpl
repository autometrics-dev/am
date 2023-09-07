((call_expression
   function: (identifier) @wrapper.call
   arguments: (arguments (identifier) @func.name))
 (#eq? @wrapper.call "{0}"))

((call_expression
   function: (identifier) @wrapper.call
   arguments: (arguments (member_expression) @func.name))
 (#eq? @wrapper.call "{0}"))

((call_expression
   function: (identifier) @wrapper.call
   arguments: (arguments
                (function
                  name: (identifier) @func.name)))
 (#eq? @wrapper.call "{0}"))
