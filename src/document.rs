use anyhow::*;
use std::collections::HashMap;

use tower_lsp::lsp_types::Url;

use crate::parser::Cst;

#[derive(Debug, Clone, Default)]
pub struct DocumentCache(HashMap<Url, Document>);

impl DocumentCache {
    pub fn register_or_update(&mut self, url: Url, text: String) -> Result<()> {
        let document = Document::from_text(text)?;
        self.0.insert(url, document);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Document {
    body: String,
    cst: Cst,
}

impl Document {
    pub fn from_text(body: String) -> Result<Self> {
        let cst = Cst::parse_source_file(&body)?;
        Ok(Self { body, cst })
    }
}
