//! Tree-sitter query strings used by the Java analyzers and graders.

/// Tree-sitter query that returns imports made
/// * `path`: java name of the import as it appears in the source code.
/// * `asterisk`: true if the import path ends in an asterisk
pub const IMPORT_QUERY: &str = include_str!("import.scm");

/// Tree-sitter query that returns name of the package
/// * `name`: name of the package
pub const PACKAGE_QUERY: &str = include_str!("package.scm");

/// Tree-sitter query that returns name of the class
/// * `name`: name of the class
pub const CLASSNAME_QUERY: &str = include_str!("class_name.scm");

/// Tree-sitter query that returns name of the interface
/// * `name`: name of the interface
pub const INTERFACENAME_QUERY: &str = include_str!("interface_name.scm");

/// Tree-sitter query that returns name of the JUnit `@Test` annotated methods
/// * `name`: name of the test method
pub const TEST_ANNOTATION_QUERY: &str = include_str!("test_annotation.scm");

/// Tree-sitter query to check the existence of a main method.
pub const MAIN_METHOD_QUERY: &str = include_str!("main_method.scm");

/// Tree-sitter query that returns class declaration statements
/// * `className`: class name
/// * `typeParameters`: type parameters
/// * `interfaces`: interfaces
pub const CLASS_DECLARATION_QUERY: &str = include_str!("class_declaration.scm");

/// * `field`: entire field declaration
pub const CLASS_FIELDS_QUERY: &str = include_str!("class_fields.scm");

/// Tree-sitter query that returns class constructor signatures
/// * `modifier`: constructor modifiers
/// * `annotation`: constructor annotations
/// * `identifier`: constructor identifier
/// * `parameters`: constructor parameters
/// * `throws`: constructor throws
pub const CLASS_CONSTRUCTOR_QUERY: &str = include_str!("class_constructors.scm");

/// Tree-sitter query that returns class method signatures
/// * `modifier`: method modifiers
/// * `annotation`: method annotations
/// * `returnType`: method return type
/// * `identifier`: method identifier
/// * `parameters`: method parameters
/// * `throws`: method throws
pub const CLASS_METHOD_QUERY: &str = include_str!("class_methods.scm");

/// Tree-sitter query that returns interface declaration statements
/// * `identifier`: interface name
/// * `parameters`: type parameters
/// * `extends`: extends interfaces
pub const INTERFACE_DECLARATION_QUERY: &str = include_str!("interface_declaration.scm");

/// Tree-sitter query that returns interface constants
/// * `constant`: entire constant declaration
pub const INTERFACE_CONSTANTS_QUERY: &str = include_str!("interface_constants.scm");

/// Tree-sitter query that returns interface methods signatures
/// * `signature`: entire method signature
pub const INTERFACE_METHODS_QUERY: &str = include_str!("interface_methods.scm");

/// Tree-sitter query that returns method call identifiers
/// * `name`: method call identifier
pub const METHOD_CALL_QUERY: &str = include_str!("method_invocation.scm");
