(program
  (block_comment)*
  (line_comment)*
  (interface_declaration 
  name: (_) @identifier
  type_parameters: (_)* @parameters
  (extends_interfaces)* @extends)
)