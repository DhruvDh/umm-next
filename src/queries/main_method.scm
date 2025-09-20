(method_declaration
	(modifiers) @modifier
    type: (void_type) @return_type
    name: (identifier) @name
    parameters: (formal_parameters
      (formal_parameter
          type: (array_type
          	element: (type_identifier) @para_type
            dimensions: (dimensions) @dim
          )
          name: (identifier) @para_name
      )
    )
    (#eq? @name "main")
    (#eq? @return_type "void")
    (#eq? @para_type "String")
    (#eq? @dim "[]")
) @body