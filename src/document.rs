use anyhow::*;
use std::collections::HashMap;

use tower_lsp::lsp_types::Url;

use crate::parser::Cst;

mod diagnostics;

#[derive(Debug, Clone, Default)]
pub struct DocumentCache(HashMap<Url, Document>);

impl DocumentCache {
    pub fn register_or_update(&mut self, url: &Url, text: String) -> Result<&Document> {
        let document = Document::from_text(text)?;
        self.0.insert(url.to_owned(), document);
        Ok(self.0.get(url).unwrap())
    }

    pub fn get(&self, key: &Url) -> Option<&Document> {
        self.0.get(key)
    }
}

#[derive(Debug, Clone)]
pub struct Document {
    body: String,
    lines: Vec<usize>,
    cst: Cst,
}

/// getter, setter
impl Document {
    /// Get a reference to the document's lines.
    pub fn lines(&self) -> &[usize] {
        self.lines.as_ref()
    }

    /// Get a reference to the document's body.
    pub fn body(&self) -> &str {
        self.body.as_ref()
    }
}

impl Document {
    pub fn from_text(body: String) -> Result<Self> {
        let cst = Cst::parse_source_file(&body)?;
        let mut lines = vec![0usize];
        lines.extend(body.match_indices('\n').map(|(p, _)| p + 1));
        Ok(Self { body, lines, cst })
    }

    pub fn display_cst(&self) -> String {
        format!("{}", self.cst)
    }
}
