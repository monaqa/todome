use super::syntax::Document;
use itertools::Itertools;
use tower_lsp::lsp_types::Position;
use tree_sitter::Point;

pub trait PosInto<T> {
    fn try_pos_into(self, document: &Document) -> Option<T>;
}

pub trait PosFrom<T>: Sized {
    fn try_pos_from(pos: T, document: &Document) -> Option<Self>;
}

impl<T, U> PosInto<T> for U
where
    T: PosFrom<U>,
{
    fn try_pos_into(self, document: &Document) -> Option<T> {
        T::try_pos_from(self, document)
    }
}

impl<S, U> PosFrom<S> for U
where
    S: PosInto<usize>,
    U: PosFrom<usize>,
{
    fn try_pos_from(pos: S, document: &Document) -> Option<Self> {
        let t = pos.try_pos_into(document)?;
        U::try_pos_from(t, document)
    }
}

impl PosFrom<usize> for Point {
    fn try_pos_from(pos: usize, document: &Document) -> Option<Self> {
        if pos > document.text().len() {
            return None;
        }
        let row = match document.lines().binary_search(&pos) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let column = pos - document.lines()[row];
        Some(Point { row, column })
    }
}

impl PosFrom<Point> for usize {
    fn try_pos_from(pos: Point, document: &Document) -> Option<Self> {
        let Point { row, column } = pos;
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

impl PosFrom<usize> for Position {
    fn try_pos_from(pos: usize, document: &Document) -> Option<Self> {
        if pos > document.text().len() {
            return None;
        }
        let row = match document.lines().binary_search(&pos) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let bytes_startline = document.lines()[row];
        let text = &document.text()[bytes_startline..pos];
        let character = text.encode_utf16().collect_vec().len();
        Some(Position {
            line: row as u32,
            character: character as u32,
        })
    }
}

impl PosFrom<Position> for usize {
    fn try_pos_from(pos: Position, document: &Document) -> Option<Self> {
        let Position { line, character } = pos;
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

impl PosFrom<tower_lsp::lsp_types::Range> for (usize, usize) {
    fn try_pos_from(pos: tower_lsp::lsp_types::Range, document: &Document) -> Option<Self> {
        let tower_lsp::lsp_types::Range { start, end } = pos;
        let start = start.try_pos_into(document)?;
        let end = end.try_pos_into(document)?;
        Some((start, end))
    }
}

impl PosFrom<(usize, usize)> for tower_lsp::lsp_types::Range {
    fn try_pos_from(pos: (usize, usize), document: &Document) -> Option<Self> {
        let start = pos.0.try_pos_into(document)?;
        let end = pos.1.try_pos_into(document)?;
        Some(tower_lsp::lsp_types::Range { start, end })
    }
}
