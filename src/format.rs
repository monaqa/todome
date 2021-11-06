use std::ops::Deref;

use crate::structure::syntax::{
    Category, Comment, Cst, Document, Due, Header, KeyVal, Priority, Rule, SourceFile, StatusKind,
    Task, Text,
};
use anyhow::*;
use itertools::Itertools;
use regex::Regex;

/// 与えられたドキュメントをフォーマットして文字列に変換する。
pub fn format_lines(text: &str) -> Result<String> {
    let mut lines = text.lines();
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
    meta: Vec<Rule>,
    comment: Option<Comment>,
    text: Option<Text>,
}

impl TodomeLine {
    fn parse(line: &str) -> Result<TodomeLine> {
        let re = Regex::new(r#"^\t*"#)?;
        let caps = re.captures(line).unwrap();
        let indent = caps[0].len();
        let line = line.trim();
        let cst = Document::parse(line.to_owned())?.into_cst();
        let comment_source = cst
            .comments
            .get(0)
            .map(|cst| cst.rule.as_comment().unwrap().clone());

        let mut children = match cst.rule {
            Rule::SourceFile(SourceFile { children }) => children,
            r => unreachable!("Rule: {}", r.name()),
        };
        if children.is_empty() {
            return Ok(TodomeLine {
                indent,
                status: None,
                meta: vec![],
                comment: None,
                text: None,
            });
        }
        let cst = children.swap_remove(0);

        let todome_line = match cst.rule {
            Rule::Task(Task {
                status, meta, text, ..
            }) => {
                let status = status.map(|status| status.rule.as_status().unwrap().kind);
                let meta = meta.iter().map(|cst| cst.rule.clone()).collect_vec();
                let comment = comment_source.or_else(|| {
                    cst.comments
                        .get(0)
                        .map(|cst| cst.rule.as_comment().unwrap().clone())
                });
                let text = Some(text.rule.as_text().unwrap().clone());
                TodomeLine {
                    indent,
                    status,
                    meta,
                    comment,
                    text,
                }
            }
            Rule::Header(Header { status, meta, .. }) => {
                let status = status.map(|status| status.rule.as_status().unwrap().kind);
                let meta = meta.iter().map(|cst| cst.rule.clone()).collect_vec();
                let comment = comment_source.or_else(|| {
                    cst.comments
                        .get(0)
                        .map(|cst| cst.rule.as_comment().unwrap().clone())
                });
                TodomeLine {
                    indent,
                    status,
                    meta,
                    comment,
                    text: None,
                }
            }
            Rule::Comment(comment) => TodomeLine {
                indent,
                status: None,
                meta: vec![],
                comment: Some(comment),
                text: None,
            },
            Rule::Error => return Err(anyhow!("Syntax Error")),
            r => unreachable!("Rule: {}", r.name()),
        };

        Ok(todome_line)
    }

    fn sort_meta(&mut self) {
        self.meta.sort_by_key(rule_ord);
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
                StatusKind::Cancelled => "/ ",
            })
            .unwrap_or_default();

        let meta = self.meta.iter().map(|rule| match rule {
            Rule::Priority(Priority { value }) => format!("({})", value),
            Rule::Due(Due { value }) => format!("({})", value.format("%Y-%m-%d")),
            Rule::KeyVal(KeyVal { key, value }) => format!("{{{}:{}}}", key, value),
            Rule::Category(Category { name }) => format!("[{}]", name),
            _ => "".to_owned(),
        });
        contents.extend(meta);
        contents.extend(
            self.text
                .as_ref()
                .map(|text| text.content.trim().to_owned())
                .into_iter(),
        );
        contents.extend(
            self.comment
                .as_ref()
                .map(|comment| comment.content.clone())
                .into_iter(),
        );

        format!("{}{}{}\n", indent, status, contents.join(" "))
    }
}

fn rule_ord(rule: &Rule) -> i64 {
    match rule {
        Rule::Priority(_) => 1,
        Rule::Due(_) => 2,
        Rule::KeyVal(_) => 4,
        Rule::Category(_) => 3,
        _ => 0,
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
