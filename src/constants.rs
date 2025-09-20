#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;
use rhai::AST;
use state::InitCell;

lazy_static! {
    /// Rhai script as a AST, behind an mutex.
    pub static ref SCRIPT_AST: Arc<Mutex<AST>> = Arc::new(Mutex::new(AST::empty()));
}

/// Prompt truncation length
pub const PROMPT_TRUNCATE: usize = 15000;

/// file name for JUnit platform console standard jar
pub const JUNIT_PLATFORM: &str = "junit-platform-console-standalone-1.10.2.jar";

/// Tree-sitter query that returns imports made
/// * `path`: java name of the import as it appears in the source code.
/// * `asterisk`: true if the import path ends in an asterisk
pub const IMPORT_QUERY: &str = include_str!("queries/import.scm");

/// Tree-sitter query that returns name of the package
/// * `name`: name of the package
pub const PACKAGE_QUERY: &str = include_str!("queries/package.scm");

/// Tree-sitter query that returns name of the class
/// * `name`: name of the class
pub const CLASSNAME_QUERY: &str = include_str!("queries/class_name.scm");

/// Tree-sitter query that returns name of the interface
/// * `name`: name of the interface
pub const INTERFACENAME_QUERY: &str = include_str!("queries/interface_name.scm");

/// Tree-sitter query that returns name of the JUnit `@Test` annotated methods
/// * `name`: name of the test method
pub const TEST_ANNOTATION_QUERY: &str = include_str!("queries/test_annotation.scm");

/// Tree-sitter query to check the existence of a main method.
pub const MAIN_METHOD_QUERY: &str = include_str!("queries/main_method.scm");

/// Tree-sitter query that returns class declaration statements
/// * `className`: class name
/// * `typeParameters`: type parameters
/// * `interfaces`: interfaces
pub const CLASS_DECLARATION_QUERY: &str = include_str!("queries/class_declaration.scm");

/// * `field`: entire field declaration
pub const CLASS_FIELDS_QUERY: &str = include_str!("queries/class_fields.scm");

/// Tree-sitter query that returns class constructor signatures
/// * `modifier`: constructor modifiers
/// * `annotation`: constructor annotations
/// * `identifier`: constructor identifier
/// * `parameters`: constructor parameters
/// * `throws`: constructor throws
pub const CLASS_CONSTRUCTOR_QUERY: &str = include_str!("queries/class_constructors.scm");

/// Tree-sitter query that returns class method signatures
/// * `modifier`: method modifiers
/// * `annotation`: method annotations
/// * `returnType`: method return type
/// * `identifier`: method identifier
/// * `parameters`: method parameters
/// * `throws`: method throws
pub const CLASS_METHOD_QUERY: &str = include_str!("queries/class_methods.scm");

/// Tree-sitter query that returns interface declaration statements
/// * `identifier`: interface name
/// * `parameters`: type parameters
/// * `extends`: extends interfaces
pub const INTERFACE_DECLARATION_QUERY: &str = include_str!("queries/interface_declaration.scm");

/// Tree-sitter query that returns interface constants
/// * `constant`: entire constant declaration
pub const INTERFACE_CONSTANTS_QUERY: &str = include_str!("queries/interface_constants.scm");

/// Tree-sitter query that returns interface methods signatures
/// * `signature`: entire method signature
pub const INTERFACE_METHODS_QUERY: &str = include_str!("queries/interface_methods.scm");

/// Tree-sitter query that returns method call identifiers
/// * `name`: method call identifier
pub const METHOD_CALL_QUERY: &str = include_str!("queries/method_invocation.scm");

/// Whether to use active retrieval or heuristic based retrieval
pub static USE_ACTIVE_RETRIEVAL: InitCell<bool> = InitCell::new();
