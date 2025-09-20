(program
  (block_comment)*
  (line_comment)*
  (class_declaration 
      (class_body
          (method_declaration
          	(modifiers)* @modifier
            (marker_annotation)* @annotation
            type_parameters: (_)* @typeParameters
            type: (_) @returnType
            name: (_) @identifier
		        parameters: (_) @parameters
            (throws)* @throws
            )
      )
	)
)