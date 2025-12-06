; Docstrings - first expression statement with string in function/class/module
(module
  (expression_statement
    (string) @docstring) @first_stmt
  (#eq? @first_stmt 0))

(function_definition
  body: (block
    (expression_statement
      (string) @docstring)))

(class_definition
  body: (block
    (expression_statement
      (string) @docstring)))
