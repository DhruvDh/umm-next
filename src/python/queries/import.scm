; Import statements
(import_statement
  name: (dotted_name) @module)

(import_from_statement
  module_name: (dotted_name) @module
  name: (dotted_name)? @name)

(import_from_statement
  module_name: (relative_import) @module
  name: (dotted_name)? @name)
