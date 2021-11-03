use super::syntax::DocumentSyntax;
use itertools::Itertools;
use tower_lsp::lsp_types::Position;
use tree_sitter::{Node, Point};

#[derive(Debug, Clone, Copy)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

impl TextRange {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn get_text(self, text: &str) -> &str {
        &text[self.start..self.end]
    }

    pub fn from_node(node: &Node) -> Self {
        TextRange::new(node.start_byte(), node.end_byte())
    }

    pub fn convert_into<T: ConvertBetweenBytes>(
        &self,
        document: &DocumentSyntax,
    ) -> Option<(T, T)> {
        let start = T::try_from_bytes(self.start, document)?;
        let end = T::try_from_bytes(self.end, document)?;
        Some((start, end))
    }

    pub fn includes(&self, cursor: usize) -> bool {
        self.start <= cursor && cursor <= self.end
    }
}

/// DocumentSyntax が与えられた上での usize との相互変換。
pub trait ConvertBetweenBytes: Sized {
    fn try_from_bytes(bytepos: usize, document: &DocumentSyntax) -> Option<Self>;
    fn try_into_bytes(self, document: &DocumentSyntax) -> Option<usize>;

    fn try_convert<T: ConvertBetweenBytes>(self, document: &DocumentSyntax) -> Option<T> {
        T::try_from_bytes(self.try_into_bytes(document)?, document)
    }
}

impl ConvertBetweenBytes for Point {
    fn try_from_bytes(bytepos: usize, document: &DocumentSyntax) -> Option<Self> {
        if bytepos > document.text().len() {
            return None;
        }
        let row = match document.lines().binary_search(&bytepos) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let column = bytepos - document.lines()[row];
        Some(Point { row, column })
    }

    fn try_into_bytes(self, document: &DocumentSyntax) -> Option<usize> {
        let Point { row, column } = self;
        let idxline = document.lines().get(row)?;
        let max_idx = match document.lines().get(row + 1) {
            Some(idx) => *idx,
            None => document.text().len(),
        };
        if (*idxline + column) < max_idx {
            Some(*idxline + column)
        } else {
            None
        }
    }
}

impl ConvertBetweenBytes for Position {
    fn try_from_bytes(bytepos: usize, document: &DocumentSyntax) -> Option<Self> {
        if bytepos > document.text().len() {
            return None;
        }
        let row = match document.lines().binary_search(&bytepos) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let bytes_startline = document.lines()[row];
        let text = &document.text()[bytes_startline..bytepos];
        let character = text.encode_utf16().collect_vec().len();
        Some(Position {
            line: row as u32,
            character: character as u32,
        })
    }

    fn try_into_bytes(self, document: &DocumentSyntax) -> Option<usize> {
        let Position { line, character } = self;
        // position が属する行のテキストを取り出す。
        let start = *document.lines().get(line as usize)?;
        let text = {
            let end = *document
                .lines()
                .get(line as usize + 1)
                .unwrap_or(&document.text().len());
            &document.text()[start..end]
        };
        let vec_utf16 = text.encode_utf16().take(character as usize).collect_vec();
        let text = String::from_utf16_lossy(&vec_utf16);
        let column = text.len();
        Some(start + column)
    }
}
