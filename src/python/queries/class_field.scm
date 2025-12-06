; Class field assignments (self.field = value in __init__)
(class_definition
  body: (block
    (function_definition
      name: (identifier) @method_name
      body: (block
        (expression_statement
          (assignment
            left: (attribute
              object: (identifier) @self
              attribute: (identifier) @name)
            right: (_) @value))))
      (#eq? @method_name "__init__")
      (#eq? @self "self")))
