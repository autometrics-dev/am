(import_from_statement
  module_name: (dotted_name (identifier) @import.module)
  (#eq? @import.module "autometrics")
  name:[
        (dotted_name (identifier) @import.name)
        (aliased_import
          name: (dotted_name (identifier) @import.name)
          alias: (identifier) @import.alias)]
  (#eq? @import.name "autometrics"))
