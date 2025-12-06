#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

//! Tree-sitter parser wrapper for Python source code.

use std::fmt::Formatter;

use anyhow::{Context, Result, anyhow};
use tree_sitter::{Language, Node, Query, QueryCursor, StreamingIterator, Tree};

use crate::Dict;

/// A struct that wraps a tree-sitter parser object and source code.
#[derive(Clone)]
pub struct Parser {
    /// The source code being parsed.
    code:  String,
    /// The parse tree.
    _tree: Option<Tree>,
    /// The tree-sitter Python grammar language.
    lang:  tree_sitter::Language,
}

/// Returns the compiled tree-sitter Python language.
fn python_language() -> tree_sitter::Language {
    tree_sitter_python::LANGUAGE.into()
}

impl Default for Parser {
    fn default() -> Self {
        // Fall back to the fallible constructor but keep Default for callers
        // that derive it; panic with context if even the empty parse fails.
        Parser::new(String::new()).expect("Failed to initialize Python parser with empty source")
    }
}

impl std::fmt::Debug for Parser {
    fn fmt(&self, _: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl Parser {
    /// Returns a new parser object.
    ///
    /// * `source_code`: the source code to be parsed
    pub fn new(source_code: String) -> Result<Self> {
        let mut parser = tree_sitter::Parser::new();
        let language = python_language();

        parser
            .set_language(&language)
            .with_context(|| "Failed to load Python grammar")?;
        let tree = parser
            .parse(source_code.as_str(), None)
            .ok_or_else(|| anyhow!("Error parsing Python code"))?;

        Ok(Self {
            code:  source_code,
            _tree: Some(tree),
            lang:  language,
        })
    }

    /// A getter for parser's source code.
    pub fn code(&self) -> &str {
        self.code.as_str()
    }

    /// Returns the parse tree's root node.
    pub fn root_node(&self) -> Result<Node<'_>> {
        self._tree
            .as_ref()
            .map(Tree::root_node)
            .context("Treesitter could not parse code")
    }

    /// Returns the tree-sitter language (useful for custom queries).
    pub fn language(&self) -> &Language {
        &self.lang
    }

    /// A setter for parser's source code.
    pub fn set_code(&mut self, code: String) -> Result<()> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&self.lang)
            .with_context(|| "Failed to load Python grammar")?;

        let tree = parser
            .parse(code.as_str(), None)
            .ok_or_else(|| anyhow!("Error parsing Python code"))?;

        self.code = code;
        self._tree = Some(tree);

        Ok(())
    }

    /// Applies a tree sitter query and returns the result as a collection of
    /// HashMaps.
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

    /// Returns the total number of lines in the source code.
    pub fn line_count(&self) -> usize {
        self.code.lines().count()
    }
}
