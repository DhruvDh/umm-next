(program
  (block_comment)*
  (line_comment)*
  (class_declaration 
      (class_body
          ((block_comment)*
          (line_comment)*
          (constructor_declaration
			(modifiers)* @modifier
      		(marker_annotation)* @annotation
			    name: (_) @identifier
          parameters: (_)* @parameters
          (throws)* @throws
			))*
      )
	)
)