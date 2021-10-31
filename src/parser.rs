use std::fmt::Display;

use anyhow::*;
use chrono::NaiveDate;
use itertools::Itertools;
use regex::Regex;
use tree_sitter::{Node, Parser, Point};

/// ファイルをパースした結果。
#[derive(Debug, Clone)]
pub struct Cst {
    pub substr: String,
    pub range: (Point, Point),
    pub range_bytes: (usize, usize),
    pub rule: Rule,
    pub comments: Vec<Cst>,
}

impl Display for Cst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.stringify(0))
    }
}

impl Cst {
    pub fn parse_source_file(content: &str) -> Result<Cst> {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_todome::language())?;
        let tree = parser
            .parse(content, None)
            .ok_or_else(|| anyhow!("parse failed."))?;
        let node = tree.root_node();
        Cst::parse_from_node(&node, content)
    }

    fn collect_children(node: &Node, content: &str) -> Result<Vec<Cst>> {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .map(|node| Cst::parse_from_node(&node, content))
            .try_collect()
    }

    fn parse_from_node(node: &Node, content: &str) -> Result<Cst> {
        let substr = {
            let start = node.start_byte();
            let end = node.end_byte();
            &content[start..end]
        }
        .to_owned();
        let range = {
            let start = node.start_position();
            let end = node.end_position();
            (start, end)
        };
        let range_bytes = {
            let start = node.start_byte();
            let end = node.end_byte();
            (start, end)
        };

        let comments: Vec<_> = {
            let mut cursor = node.walk();
            node.children_by_field_name("comment", &mut cursor)
                .map(|node| Cst::parse_from_node(&node, content))
                .try_collect()
        }?;

        let rule = match node.kind() {
            "source_file" => {
                let children = Cst::collect_children(node, content)?;
                Rule::SourceFile { children }
            }

            "task" => {
                let status = if let Some(node) = node.child_by_field_name("status") {
                    Some(Box::new(Cst::parse_from_node(&node, content)?))
                } else {
                    None
                };
                let meta = if let Some(node) = node.child_by_field_name("meta") {
                    Cst::collect_children(&node, content)?
                } else {
                    vec![]
                };
                let text = {
                    let node = node
                        .child_by_field_name("text")
                        .ok_or_else(|| anyhow!("field 'text' cannot be found in task."))?;
                    Box::new(Cst::parse_from_node(&node, content)?)
                };
                let children = if let Some(node) = node.child_by_field_name("children") {
                    let mut cursor = node.walk();
                    node.children(&mut cursor)
                        .map(|node| Cst::parse_from_node(&node, content))
                        .try_collect()?
                } else {
                    vec![]
                };
                Rule::Task {
                    status,
                    meta,
                    text,
                    children,
                }
            }

            "header" => {
                let status = if let Some(node) = node.child_by_field_name("status") {
                    Some(Box::new(Cst::parse_from_node(&node, content)?))
                } else {
                    None
                };
                let meta = if let Some(node) = node.child_by_field_name("meta") {
                    Cst::collect_children(&node, content)?
                } else {
                    vec![]
                };
                let children = if let Some(node) = node.child_by_field_name("children") {
                    let mut cursor = node.walk();
                    node.children(&mut cursor)
                        .map(|node| Cst::parse_from_node(&node, content))
                        .try_collect()?
                } else {
                    vec![]
                };
                Rule::Header {
                    status,
                    meta,
                    children,
                }
            }

            "status" => {
                let child = node.child(0).unwrap();
                match child.kind() {
                    "status_todo" => Rule::Status {
                        kind: StatusKind::Todo,
                    },
                    "status_doing" => Rule::Status {
                        kind: StatusKind::Doing,
                    },
                    "status_done" => Rule::Status {
                        kind: StatusKind::Done,
                    },
                    "status_cancel" => Rule::Status {
                        kind: StatusKind::Cancelled,
                    },
                    _ => Rule::Error,
                }
            }

            "priority" => {
                // TODO: もうちょっと真面目な正規表現を書く
                let re = Regex::new(r#"\((.*)\)"#).unwrap();
                let value = re.captures(&substr).unwrap()[1].to_owned();
                Rule::Priority { value }
            }

            "due" => {
                let re = Regex::new(r#"\{(.*)\}"#).unwrap();
                let s_value = &re.captures(&substr).unwrap()[1];
                let value = NaiveDate::parse_from_str(s_value, "%Y-%m-%d")?;
                Rule::Due { value }
            }

            "keyval" => {
                let re = Regex::new(r#"\{(.*):(.*)\}"#).unwrap();
                let caps = &re.captures(&substr).unwrap();
                let key = caps[1].to_owned();
                let value = caps[2].to_owned();
                Rule::KeyVal { key, value }
            }

            "category" => {
                let re = Regex::new(r#"\[(.*)\]"#).unwrap();
                let name = re.captures(&substr).unwrap()[1].to_owned();
                Rule::Category { name }
            }

            "text" => Rule::Text {
                content: substr.clone(),
            },

            "comment" => Rule::Comment {
                content: substr.clone(),
            },

            _ => Rule::Error,
        };
        Ok(Cst {
            substr,
            range,
            range_bytes,
            rule,
            comments,
        })
    }

    /// その構文要素の子に相当する Cst の列を返す。
    /// 構文木として子を持ちえない要素であれば `None` を返す。
    /// 構文木としては子を持ち得る（task や header など）が
    /// 子を持たなかった場合は Some(&[]) を返す。
    fn get_children(&self) -> Option<Vec<&Cst>> {
        match &self.rule {
            Rule::SourceFile { children } => Some(children.iter().collect_vec()),
            Rule::Task {
                status,
                meta,
                text,
                children,
            } => {
                let mut v: Vec<&Cst> = vec![];
                if let Some(b) = status {
                    v.push(b);
                }
                v.extend(meta);
                v.push(text);
                v.extend(children);
                Some(v)
            }
            Rule::Header {
                status,
                meta,
                children,
            } => {
                let mut v: Vec<&Cst> = vec![];
                if let Some(b) = status {
                    v.push(b);
                }
                v.extend(meta);
                v.extend(children);
                Some(v)
            }
            _ => None,
        }
    }

    /// そのカーソルが乗っている Cst の参照の列を返す。
    /// Cst は範囲が狭いものから順に並ぶ。
    pub fn get_csts_on_point(&self, cursor: Point) -> Vec<&Cst> {
        if !self.includes(cursor) {
            return vec![];
        }

        let cst = self
            .get_children()
            .unwrap_or_default()
            .into_iter()
            .find(|cst| cst.includes(cursor));
        let mut v = cst
            .map(|cst| cst.get_csts_on_point(cursor))
            .unwrap_or_default();
        v.push(self);
        v
    }

    fn includes(&self, cursor: Point) -> bool {
        let (start, end) = self.range;
        start <= cursor && cursor <= end
    }

    pub fn rule_name(&self) -> &'static str {
        match self.rule {
            Rule::SourceFile { .. } => "soure_file",
            Rule::Task { .. } => "task",
            Rule::Header { .. } => "header",
            Rule::Status { .. } => "status",
            Rule::Priority { .. } => "priority",
            Rule::Due { .. } => "due",
            Rule::KeyVal { .. } => "keyval",
            Rule::Category { .. } => "category",
            Rule::Text { .. } => "text",
            Rule::Comment { .. } => "comment",
            Rule::Error => "ERROR",
        }
    }

    pub fn search_cst<F>(&self, predicate: F) -> Vec<&Cst>
    where
        F: Fn(&Cst) -> bool,
    {
        let mut v = self.search_cst_aux(&predicate);
        v.reverse();
        v
    }

    fn search_cst_aux<F>(&self, predicate: &F) -> Vec<&Cst>
    where
        F: Fn(&Cst) -> bool,
    {
        let mut csts = self
            .get_children()
            .unwrap_or_default()
            .into_iter()
            .map(|cst| cst.search_cst_aux(predicate))
            .concat();
        if predicate(self) {
            csts.push(self)
        }
        csts
    }

    fn stringify(&self, indent: usize) -> String {
        let mut s = String::new();
        let indent_str = " ".repeat(indent * 2);
        s.push_str(&indent_str);
        s.push_str(&format!("[{}]", self.rule_name(),));
        if !self.substr.contains('\n') && self.substr.len() < 50 {
            s.push_str(&format!(r#" "{}""#, self.substr));
        } else {
            let (start, end) = self.range;
            s.push_str(&format!(
                " ({}:{} .. {}:{})",
                start.row + 1,
                start.column + 1,
                end.row + 1,
                end.column + 1,
            ));
        }
        s.push('\n');
        for child in self.get_children().unwrap_or_default() {
            let text = child.stringify(indent + 1);
            s.push_str(&text);
        }
        s
    }
}

/// 構文規則とその階層構造を表したもの。
/// struct の順番は、実際に構文が登場する順番に統一されている。
#[derive(Debug, Clone)]
pub enum Rule {
    SourceFile {
        children: Vec<Cst>,
    },
    Task {
        status: Option<Box<Cst>>,
        meta: Vec<Cst>,
        text: Box<Cst>,
        children: Vec<Cst>,
    },
    Header {
        status: Option<Box<Cst>>,
        meta: Vec<Cst>,
        children: Vec<Cst>,
    },
    Status {
        kind: StatusKind,
    },
    Priority {
        value: String,
    },
    Due {
        value: NaiveDate,
    },
    KeyVal {
        key: String,
        value: String,
    },
    Category {
        name: String,
    },
    Text {
        content: String,
    },
    Comment {
        content: String,
    },
    Error,
}

#[derive(Debug, Clone)]
pub enum StatusKind {
    Todo,
    Doing,
    Done,
    Cancelled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_source_file() {
        let cst = Cst::parse_source_file("- (A) test").unwrap();
        println!("{:#?}", cst);
    }

    #[test]
    fn get_children() {
        let cst = Cst::parse_source_file(
            r#"
- (A) test
	[cattest] foo # a comment
	bar
            "#,
        )
        .unwrap();
        let csts = cst.get_csts_on_point(Point { row: 2, column: 4 });
        for cst in csts {
            println!("{}", cst);
        }
    }
}
