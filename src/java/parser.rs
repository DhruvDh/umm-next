use std::fmt::Formatter;

use anyhow::{Context, Result, bail};
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
            .expect("Error loading Java grammar");
        let tree = parser
            .parse(source_code.as_str(), None)
            .context("Error parsing Java code")?;

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
    pub fn set_code(&mut self, code: String) {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&self.lang)
            .expect("Error loading Java grammar");

        let tree = parser
            .parse(code.as_str(), None)
            .expect("Error parsing Java code");

        self.code = code;
        self._tree = Some(tree);
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

        let query = Query::new(&self.lang, q).unwrap();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), self.code.as_bytes());
        let capture_names = query.capture_names();

        while let Some(m) = matches.next() {
            let mut result = Dict::new();

            for name in capture_names {
                let index = query.capture_index_for_name(name);
                let index = match index {
                    Some(i) => i,
                    None => bail!(
                        "Error while querying source code. Capture name: {} has no index \
                         associated.",
                        name
                    ),
                };

                let value = m.captures.iter().find(|c| c.index == index);
                let value = match value {
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

                result.insert(name.to_string(), value.to_string());
            }
            results.push(result);
        }

        Ok(results)
    }
}
