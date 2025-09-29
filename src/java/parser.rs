use std::fmt::Formatter;

use anyhow::{Context, Result, anyhow};
use tree_sitter::{Query, QueryCursor, StreamingIterator, Tree};

use crate::Dict;
#[derive(Clone)]
/// A struct that wraps a tree-sitter parser object and source code
pub struct Parser {
    /// the source code being parsed
    code:  String,
    /// the parse tree
    _tree: Option<Tree>,
    /// the tree-sitter java grammar language
    lang:  tree_sitter::Language,
}

fn java_language() -> tree_sitter::Language {
    tree_sitter_java::LANGUAGE.into()
}

impl Default for Parser {
    fn default() -> Self {
        let mut parser = tree_sitter::Parser::new();
        let language = java_language();
        parser
            .set_language(&language)
            .expect("Error loading Java grammar");
        let tree = parser.parse("", None);

        Self {
            code:  String::new(),
            _tree: tree,
            lang:  language,
        }
    }
}

impl std::fmt::Debug for Parser {
    fn fmt(&self, _: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl Parser {
    /// Returns a new parser object
    ///
    /// * `source_code`: the source code to be parsed
    /// * `lang`: the tree-sitter grammar to use
    pub fn new(source_code: String) -> Result<Self> {
        let mut parser = tree_sitter::Parser::new();
        let language = java_language();

        parser
            .set_language(&language)
            .with_context(|| "Failed to load Java grammar")?;
        let tree = parser
            .parse(source_code.as_str(), None)
            .ok_or_else(|| anyhow!("Error parsing Java code"))?;

        Ok(Self {
            code:  source_code,
            _tree: Some(tree),
            lang:  language,
        })
    }

    /// A getter for parser's source code
    pub fn code(&self) -> &str {
        self.code.as_str()
    }

    /// A setter for parser's source code
    pub fn set_code(&mut self, code: String) -> Result<()> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&self.lang)
            .with_context(|| "Failed to load Java grammar")?;

        let tree = parser
            .parse(code.as_str(), None)
            .ok_or_else(|| anyhow!("Error parsing Java code"))?;

        self.code = code;
        self._tree = Some(tree);

        Ok(())
    }

    /// Applies a tree sitter query and returns the result as a collection of
    /// HashMaps
    ///
    /// * `q`: the tree-sitter query to be applied
    pub fn query(&self, q: &str) -> Result<Vec<Dict>> {
        let mut results = vec![];
        let tree = self
            ._tree
            .as_ref()
            .context("Treesitter could not parse code")?;

        let query = Query::new(&self.lang, q)
            .with_context(|| format!("Failed to compile tree-sitter query: {q}"))?;
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), self.code.as_bytes());
        let mut capture_indices = Vec::new();

        for name in query.capture_names() {
            let index = query
                .capture_index_for_name(name)
                .ok_or_else(|| anyhow!("Capture name {name} has no index associated."))?;
            capture_indices.push((index, name.to_string()));
        }

        while let Some(m) = matches.next() {
            let mut result = Dict::new();

            for (index, name) in &capture_indices {
                let value = match m.captures.iter().find(|c| c.index == *index) {
                    Some(v) => v,
                    None => continue,
                };

                let value = value
                    .node
                    .utf8_text(self.code.as_bytes())
                    .with_context(|| {
                        format!(
                            "Cannot match query result indices with source code for capture name: \
                             {name}."
                        )
                    })?;

                result.insert(name.clone(), value.to_string());
            }
            results.push(result);
        }

        Ok(results)
    }

    /// Returns the text and 1-based starting line number for each occurrence of
    /// the requested capture in the supplied query.
    pub fn query_capture_positions(
        &self,
        q: &str,
        capture_name: &str,
    ) -> Result<Vec<(String, usize)>> {
        let tree = self
            ._tree
            .as_ref()
            .context("Treesitter could not parse code")?;

        let query = Query::new(&self.lang, q)
            .with_context(|| format!("Failed to compile tree-sitter query: {q}"))?;
        let capture_index = query
            .capture_index_for_name(capture_name)
            .ok_or_else(|| anyhow!("Capture name {capture_name} not present in query"))?;

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), self.code.as_bytes());
        let mut results = Vec::new();

        while let Some(m) = matches.next() {
            for capture in m.captures.iter().filter(|c| c.index == capture_index) {
                let text = capture
                    .node
                    .utf8_text(self.code.as_bytes())
                    .context("Cannot map capture to source text")?;
                let line = capture.node.start_position().row + 1; // 1-based
                results.push((text.to_string(), line));
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::java::queries::CLASSNAME_QUERY;

    #[test]
    fn query_returns_class_name_capture() {
        let parser = Parser::new("class Foo {}".to_string()).expect("parser should initialize");
        let captures = parser.query(CLASSNAME_QUERY).expect("query should succeed");
        assert_eq!(captures.len(), 1);
        assert_eq!(captures[0].get("name").map(String::as_str), Some("Foo"));
    }

    #[test]
    fn query_errors_on_invalid_query() {
        let parser = Parser::new("class Foo {}".to_string()).expect("parser should initialize");
        let err = parser.query("(invalid").unwrap_err();
        let message = err.to_string();
        assert!(message.contains("Failed to compile tree-sitter query"));
    }

    #[test]
    fn query_capture_positions_errors_on_missing_capture() {
        let parser = Parser::new("class Foo {}".to_string()).expect("parser should initialize");
        let err = parser
            .query_capture_positions(CLASSNAME_QUERY, "not_a_capture")
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("Capture name not_a_capture not present in query")
        );
    }

    #[test]
    fn set_code_updates_tree() {
        let mut parser = Parser::new("class Foo {}".to_string()).expect("parser should initialize");
        parser
            .set_code("class Bar {}".to_string())
            .expect("set_code should succeed");

        let captures = parser.query(CLASSNAME_QUERY).expect("query should succeed");
        assert_eq!(captures.len(), 1);
        assert_eq!(captures[0].get("name").map(String::as_str), Some("Bar"));
    }
}
