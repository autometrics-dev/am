(decorated_definition
  (decorator
    [(identifier) @decorator.name
      (call (identifier) @decorator.name)])
  (#eq? @decorator.name "{0}")
  definition: (function_definition
               name: (identifier) @func.name))
