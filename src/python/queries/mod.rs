//! Tree-sitter query strings used by the Python analyzers and graders.

/// Tree-sitter query that returns import statements.
/// * `module`: module being imported
/// * `name`: alias or specific import name
pub const IMPORT_QUERY: &str = include_str!("import.scm");

/// Tree-sitter query that returns function definitions.
/// * `name`: function name
/// * `params`: function parameters
/// * `return_type`: optional return type annotation
/// * `body`: function body
pub const FUNCTION_DEF_QUERY: &str = include_str!("function_def.scm");

/// Tree-sitter query that returns class definitions.
/// * `name`: class name
/// * `bases`: base classes
/// * `body`: class body
pub const CLASS_DEF_QUERY: &str = include_str!("class_def.scm");

/// Tree-sitter query to check for `if __name__ == "__main__":` blocks.
/// * `body`: the main block body
pub const MAIN_BLOCK_QUERY: &str = include_str!("main_block.scm");

/// Tree-sitter query that returns docstrings.
/// * `docstring`: the docstring content
pub const DOCSTRING_QUERY: &str = include_str!("docstring.scm");

/// Tree-sitter query that returns function/method calls.
/// * `name`: function name being called
/// * `arguments`: call arguments
pub const FUNCTION_CALL_QUERY: &str = include_str!("function_call.scm");

/// Tree-sitter query that returns method definitions within classes.
/// * `name`: method name
/// * `params`: method parameters
/// * `body`: method body
pub const METHOD_DEF_QUERY: &str = include_str!("method_def.scm");

/// Tree-sitter query that returns class field assignments.
/// * `name`: field name
/// * `value`: assigned value
pub const CLASS_FIELD_QUERY: &str = include_str!("class_field.scm");
