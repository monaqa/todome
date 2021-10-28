use std::fmt::Display;

use anyhow::*;
use chrono::NaiveDate;
use itertools::Itertools;
use regex::Regex;
use tree_sitter::{Node, Parser, Point, Tree, TreeCursor};

pub fn parse_to_cst(text: &str) -> Result<CstNode> {
    let mut parser = Parser::new();
    parser.set_language(tree_sitter_todome::language())?;
    let tree = parser
        .parse(text, None)
        .ok_or_else(|| anyhow!("parse failed."))?;
    let mut cursor = tree.walk();
    let cst = CstNode::from_cursor(&mut cursor, text);
    Ok(cst)
}

#[derive(Debug, Clone)]
pub struct Cst {
    substr: String,
    range: (Point, Point),
    range_bytes: (usize, usize),
    rule: Rule,
    // comments: Vec<Cst>,
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

    pub fn collect_children_by_field_name(
        node: &Node,
        field_name: &str,
        content: &str,
    ) -> Result<Vec<Cst>> {
        let mut cursor = node.walk();
        node.children_by_field_name(field_name, &mut cursor)
            .map(|node| Cst::parse_from_node(&node, content))
            .try_collect()
    }

    pub fn collect_children(node: &Node, content: &str) -> Result<Vec<Cst>> {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .map(|node| Cst::parse_from_node(&node, content))
            .try_collect()
    }

    pub fn parse_from_node(node: &Node, content: &str) -> Result<Cst> {
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
                let children = Cst::collect_children_by_field_name(node, "children", content)?;
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
                let children = Cst::collect_children_by_field_name(node, "children", content)?;
                Rule::Header {
                    status,
                    meta,
                    children,
                }
            }

            "status" => {
                let child = node.child(0).unwrap();
                let kind = match child.kind() {
                    "status_todo" => StatusKind::Todo,
                    "status_doing" => StatusKind::Doing,
                    "status_done" => StatusKind::Done,
                    "status_cancel" => StatusKind::Cancelled,
                    _ => unreachable!(),
                };
                Rule::Status { kind }
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

            s => unreachable!("{}", s),
        };
        Ok(Cst {
            substr,
            range,
            range_bytes,
            rule,
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
    /// Cst は範囲が広いものから順に並ぶ。
    fn get_csts_on_point(&self, cursor: Point) -> Vec<&Cst> {
        todo!()
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
}

#[derive(Debug, Clone)]
pub enum StatusKind {
    Todo,
    Doing,
    Done,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct CstNode {
    kind: String,
    substr: String,
    range: CstRange,
    children: Vec<CstNode>,
    field_name: Option<String>,
    named: bool,
    extra: bool,
    error: bool,
    missing: bool,
}

#[derive(Debug, Clone)]
struct CstRange {
    start: (usize, usize),
    end: (usize, usize),
}

impl CstNode {
    /// TODO: もっと効率良い方法がありそう
    fn from_cursor(cursor: &mut TreeCursor, text: &str) -> CstNode {
        let node = cursor.node();
        let field_name = cursor.field_name().map(|s| s.to_owned());
        let kind = node.kind().to_owned();
        let range = {
            let start = node.start_position();
            let end = node.end_position();
            CstRange {
                start: (start.row, start.column),
                end: (end.row, end.column),
            }
        };
        let substr = {
            let start = node.start_byte();
            let end = node.end_byte();
            &text[start..end]
        }
        .to_owned();
        let children: Vec<_> = node
            .children(cursor)
            .into_iter()
            .map(|child| {
                let mut cursor = child.walk();
                CstNode::from_cursor(&mut cursor, text)
            })
            .collect();
        CstNode {
            kind,
            range,
            substr,
            children,
            field_name,
            named: node.is_named(),
            extra: node.is_extra(),
            error: node.is_error(),
            missing: node.is_missing(),
        }
    }

    fn stringify(&self, indent: usize) -> String {
        if !self.named {
            return "".to_owned();
        }
        let mut s = String::new();
        let indent_str = " ".repeat(indent * 2);
        s.push_str(&indent_str);
        if let Some(field_name) = &self.field_name {
            s.push_str(&format!("{}:", field_name,));
        }
        if self.error {
            s.push_str(&format!("[!{}!]", self.kind,));
        } else if self.missing {
            s.push_str(&format!("[?{}?]", self.kind,));
        } else if self.extra {
            s.push_str(&format!("[%{}%]", self.kind,));
        } else {
            s.push_str(&format!("[{}]", self.kind,));
        }
        if !self.substr.contains('\n') && self.substr.len() < 50 {
            s.push_str(&format!(r#" "{}""#, self.substr));
        } else {
            s.push_str(&format!(
                " ({}:{} .. {}:{})",
                self.range.start.0 + 1,
                self.range.start.1 + 1,
                self.range.end.0 + 1,
                self.range.end.1 + 1,
            ));
        }
        s.push('\n');
        for child in &self.children {
            let text = child.stringify(indent + 1);
            s.push_str(&text);
        }
        s
    }
}

impl Display for CstNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.stringify(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_a_file() {
        let cst = parse_to_cst("- (A) {2012-10-15} test").unwrap();
        println!("{:#?}", cst);
    }

    #[test]
    fn parse_source_file() {
        let cst = Cst::parse_source_file("- (A) test").unwrap();
        println!("{:#?}", cst);
    }
}
