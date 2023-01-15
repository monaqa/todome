use std::fmt::Display;

use chrono::NaiveDate;
use itertools::Itertools;
use regex::Regex;
use tree_sitter_todome::syntax::ast::{Item, Memo, Meta, SourceFile, StatusKind, Text};

/// 与えられたドキュメントをフォーマットして文字列に変換する。
pub fn format_lines(text: &str) -> anyhow::Result<String> {
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

#[derive(Debug, Clone, Default)]
pub struct MetaData {
    priority: Option<String>,
    date: [Option<NaiveDate>; 3],
    category: Vec<String>,
}

impl MetaData {
    fn from_metas(metas: &[Meta]) -> Self {
        let mut data = MetaData::default();
        metas.iter().for_each(|meta| match meta {
            Meta::Priority(p) => data.priority = Some(p.value()),
            Meta::Date(d) => {
                data.date[0] = d.start().or(data.date[0]);
                data.date[1] = d.target().or(data.date[1]);
                data.date[2] = d.deadline().or(data.date[2]);
            }
            Meta::Keyval(_) => {}
            Meta::Category(c) => data.category.push(c.name()),
        });
        data
    }
}

impl Display for MetaData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(priority) = &self.priority {
            write!(f, "({priority}) ")?
        };
        match self.date {
            [Some(start), Some(target), Some(deadline)] => {
                write!(
                    f,
                    "({}~{} {}!) ",
                    start.format("%Y-%m-%d"),
                    target.format("%Y-%m-%d"),
                    deadline.format("%Y-%m-%d"),
                )?;
            }
            [None, Some(target), Some(deadline)] => {
                write!(
                    f,
                    "({} {}!) ",
                    target.format("%Y-%m-%d"),
                    deadline.format("%Y-%m-%d"),
                )?;
            }
            [Some(start), None, Some(deadline)] => {
                write!(
                    f,
                    "({}~{}!) ",
                    start.format("%Y-%m-%d"),
                    deadline.format("%Y-%m-%d"),
                )?;
            }
            [Some(start), Some(target), None] => {
                write!(
                    f,
                    "({}~{}) ",
                    start.format("%Y-%m-%d"),
                    target.format("%Y-%m-%d"),
                )?;
            }
            [Some(start), None, None] => write!(f, "({}~) ", start.format("%Y-%m-%d"),)?,
            [None, Some(target), None] => write!(f, "({}) ", target.format("%Y-%m-%d"),)?,
            [None, None, Some(deadline)] => write!(f, "({}!) ", deadline.format("%Y-%m-%d"),)?,
            _ => {}
        }
        let cats = self.category.iter().map(|c| format!("[{c}] ")).join("");
        write!(f, "{cats}")?;
        Ok(())
    }
}

impl TodomeLine {
    fn parse(line: &str) -> anyhow::Result<TodomeLine> {
        let re = Regex::new(r#"^\t*"#)?;
        let caps = re.captures(line).unwrap();
        let indent = caps[0].len();
        let line = line.trim();

        let source_file = SourceFile::parse(line.to_owned())?;
        let item = source_file.items().into_iter().next();
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
                meta: task.meta(),
                memo: task.memo(),
                text: task.text(),
            },
            Item::Header(header) => TodomeLine {
                indent,
                status: header.status().map(|s| s.kind()),
                meta: header.meta(),
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

        let meta = MetaData::from_metas(&self.meta).to_string();

        // contents.push(meta);
        let text: String = self
            .text
            .as_ref()
            .map(|text| text.body().trim().to_owned())
            .unwrap_or_default();
        let memo: String = self
            .memo
            .as_ref()
            .map(|memo| format!("# {}", memo.body().trim()))
            .unwrap_or_default();

        let content = [meta.trim(), &text, &memo]
            .iter()
            .filter(|s| !s.is_empty())
            .join(" ");

        format!("{indent}{status}{content}\n")
    }
}

fn meta_ord(meta: &Meta) -> i64 {
    match meta {
        Meta::Priority(_) => 1,
        Meta::Date(_) => 2,
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
