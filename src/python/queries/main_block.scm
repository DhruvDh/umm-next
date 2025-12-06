; Main block: if __name__ == "__main__":
(if_statement
  condition: (comparison_operator
    (identifier) @name_var
    (string) @main_str)
  consequence: (block) @body
  (#eq? @name_var "__name__"))
