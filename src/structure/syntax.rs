//! 構文解析の結果を格納する構文木の要素。

use std::{collections::HashMap, fmt::Display};

use anyhow::*;
use tower_lsp::lsp_types::Url;
use tree_sitter_todome::syntax::ast::{AstNode, SourceFile};

#[derive(Debug, Clone, Default)]
pub struct DocumentCache(HashMap<Url, Document>);

impl DocumentCache {
    pub fn register_or_update(&mut self, url: &Url, text: String) -> Result<&Document> {
        let document = Document::parse(text)?;
        self.0.insert(url.to_owned(), document);
        Ok(self.0.get(url).unwrap())
    }

    pub fn get(&self, key: &Url) -> Option<&Document> {
        self.0.get(key)
    }
}

#[derive(Debug, Clone)]
pub struct Document {
    text: String,
    lines: Vec<usize>,
    root: SourceFile,
}

/// getter, setter
impl Document {
    /// Get a reference to the document's lines.
    pub fn lines(&self) -> &[usize] {
        self.lines.as_ref()
    }

    /// Get a reference to the document's body.
    pub fn text(&self) -> &str {
        self.text.as_ref()
    }

    /// Get a reference to the document's body.
    pub fn root(&self) -> &SourceFile {
        &self.root
    }

    pub fn into_cst(self) -> SourceFile {
        self.root
    }
}

impl Document {
    pub fn parse(text: String) -> Result<Document> {
        let root = SourceFile::parse(text.clone())?;
        let mut lines = vec![0usize];
        lines.extend(text.match_indices('\n').map(|(p, _)| p + 1));
        Ok(Self { text, lines, root })
    }
}

impl Display for Document {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.root().syntax().display_recursive())
    }
}
