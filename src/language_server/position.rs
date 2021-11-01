//! テキスト中の位置を表す表現の相互変換を行う。
//! テキスト中の位置を表す表現は以下の3通り。
//!
//! - bytes (usize)
//!   単にバッファの先頭から数えたときのインデックスを表す。
//! - point (tree_sitter::Point)
//!   row と column を zero-based に持つ。 indexing はおそらく UTF-8 がベース。
//! - position (tower_lsp::lsp::Position)
//!   LSP でやり取りするときに用いられる。
//!   row と column を zero-based に持つ。 indexing は UTF-16 がベース。
//!

use itertools::Itertools;
use tower_lsp::lsp_types::Position;
use tree_sitter::Point;

use super::document::Document;

/// Document が与えられた上での usize との相互変換。
pub trait ConvertBetweenBytes: Sized {
    fn try_from_bytes(bytepos: usize, document: &Document) -> Option<Self>;
    fn try_into_bytes(self, document: &Document) -> Option<usize>;

    fn try_convert<T: ConvertBetweenBytes>(self, document: &Document) -> Option<T> {
        T::try_from_bytes(self.try_into_bytes(document)?, document)
    }
}

impl ConvertBetweenBytes for Point {
    fn try_from_bytes(bytepos: usize, document: &Document) -> Option<Self> {
        if bytepos > document.body().len() {
            return None;
        }
        let row = match document.lines().binary_search(&bytepos) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let column = bytepos - document.lines()[row];
        Some(Point { row, column })
    }

    fn try_into_bytes(self, document: &Document) -> Option<usize> {
        let Point { row, column } = self;
        let idxline = document.lines().get(row)?;
        let max_idx = match document.lines().get(row + 1) {
            Some(idx) => *idx,
            None => document.body().len(),
        };
        if (*idxline + column) < max_idx {
            Some(*idxline + column)
        } else {
            None
        }
    }
}

impl ConvertBetweenBytes for Position {
    fn try_from_bytes(bytepos: usize, document: &Document) -> Option<Self> {
        if bytepos > document.body().len() {
            return None;
        }
        let row = match document.lines().binary_search(&bytepos) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let bytes_startline = document.lines()[row];
        let text = &document.body()[bytes_startline..bytepos];
        let character = text.encode_utf16().collect_vec().len();
        Some(Position {
            line: row as u32,
            character: character as u32,
        })
    }

    fn try_into_bytes(self, document: &Document) -> Option<usize> {
        let Position { line, character } = self;
        // position が属する行のテキストを取り出す。
        let start = *document.lines().get(line as usize)?;
        let text = {
            let end = *document
                .lines()
                .get(line as usize + 1)
                .unwrap_or(&document.body().len());
            &document.body()[start..end]
        };
        let vec_utf16 = text.encode_utf16().take(character as usize).collect_vec();
        let text = String::from_utf16_lossy(&vec_utf16);
        let column = text.len();
        Some(start + column)
    }
}
