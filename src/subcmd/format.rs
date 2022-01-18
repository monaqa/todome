use anyhow::*;
use itertools::Itertools;
use regex::Regex;
use tree_sitter_todome::syntax::ast::{Item, Memo, Meta, Priority, SourceFile, StatusKind, Text};

/// 与えられたドキュメントをフォーマットして文字列に変換する。
pub fn format_lines(text: &str) -> Result<String> {
    let lines = text.lines();
    let mut todome_lines = vec![];

    for line in lines {
        let mut todome_line = TodomeLine::parse(line)?;
        todome_line.sort_meta();
        todome_lines.push(todome_line.stringify());
    }

    // 1行ずつフォーマットを掛けていく

    Ok(todome_lines.join(""))
}

#[derive(Debug, Clone)]
pub struct TodomeLine {
    indent: usize,
    status: Option<StatusKind>,
    meta: Vec<Meta>,
    memo: Option<Memo>,
    text: Option<Text>,
}

impl TodomeLine {
    fn parse(line: &str) -> Result<TodomeLine> {
        let re = Regex::new(r#"^\t*"#)?;
        let caps = re.captures(line).unwrap();
        let indent = caps[0].len();
        let line = line.trim();

        let source_file = SourceFile::parse(line.to_owned())?;
        let item = source_file.items().next();
        if item.is_none() {
            return Ok(TodomeLine {
                indent,
                status: None,
                meta: vec![],
                memo: None,
                text: None,
            });
        }
        let item = item.unwrap();
        let todome_line = match item {
            Item::Task(task) => TodomeLine {
                indent,
                status: task.status().map(|s| s.kind()),
                meta: task.meta().collect_vec(),
                memo: task.memo(),
                text: task.text(),
            },
            Item::Header(header) => TodomeLine {
                indent,
                status: header.status().map(|s| s.kind()),
                meta: header.meta().collect_vec(),
                memo: header.memo(),
                text: None,
            },
            Item::Memo(memo) => TodomeLine {
                indent,
                status: None,
                meta: vec![],
                memo: Some(memo),
                text: None,
            },
        };

        Ok(todome_line)
    }

    fn sort_meta(&mut self) {
        self.meta.sort_by_key(meta_ord);
    }

    fn stringify(&self) -> String {
        let indent = "\t".repeat(self.indent);
        let mut contents = vec![];

        let status = self
            .status
            .map(|kind| match kind {
                StatusKind::Todo => "+ ",
                StatusKind::Doing => "* ",
                StatusKind::Done => "- ",
                StatusKind::Cancel => "= ",
                StatusKind::Other => "/ ",
            })
            .unwrap_or_default();

        let meta = self.meta.iter().map(|meta| match meta {
            Meta::Priority(p) => format!("({})", p.value()),
            Meta::Due(d) => format!("({})", d.value()),
            Meta::Keyval(k) => format!("{{{}:{}}}", k.key(), k.value()),
            Meta::Category(c) => format!("[{}]", c.name()),
        });

        contents.extend(meta);
        contents.extend(
            self.text
                .as_ref()
                .map(|text| text.body().trim().to_owned())
                .into_iter(),
        );
        contents.extend(
            self.memo
                .as_ref()
                .map(|memo| format!("# {}", memo.body().trim()))
                .into_iter(),
        );

        format!("{}{}{}\n", indent, status, contents.join(" "))
    }
}

fn meta_ord(meta: &Meta) -> i64 {
    match meta {
        Meta::Priority(_) => 1,
        Meta::Due(_) => 2,
        Meta::Keyval(_) => 4,
        Meta::Category(_) => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_line() {
        dbg!(TodomeLine::parse(r#""#).unwrap());
        dbg!(TodomeLine::parse(r#"	"#).unwrap());
        dbg!(TodomeLine::parse(r#"適当なタスク"#).unwrap());
        dbg!(TodomeLine::parse(r#"(A) 適当なタスク"#).unwrap());
        dbg!(TodomeLine::parse(r#"(A) # 適当なヘッダ"#).unwrap());
        dbg!(TodomeLine::parse(r#"適当なタスク # コメント"#).unwrap());
    }
}
