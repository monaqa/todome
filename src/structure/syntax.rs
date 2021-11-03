//! 構文解析の結果を格納する構文木の要素。

use anyhow::*;
use chrono::NaiveDate;
use itertools::Itertools;
use regex::Regex;
use tree_sitter::{Node, Parser, Point};

use super::position::TextRange;

pub struct DocumentSyntax {
    text: String,
    lines: Vec<usize>,
    root: Cst,
}

/// getter, setter
impl DocumentSyntax {
    /// Get a reference to the document's lines.
    pub fn lines(&self) -> &[usize] {
        self.lines.as_ref()
    }

    /// Get a reference to the document's body.
    pub fn text(&self) -> &str {
        self.text.as_ref()
    }

    /// Get a reference to the document's body.
    pub fn root(&self) -> &Cst {
        &self.root
    }
}

impl DocumentSyntax {
    pub fn parse(text: String) -> Result<DocumentSyntax> {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_todome::language())?;
        let tree = parser
            .parse(&text, None)
            .ok_or_else(|| anyhow!("parse failed."))?;
        let node = tree.root_node();
        let root = Cst::parse_node(&node, &text)?;
        let mut lines = vec![0usize];
        lines.extend(text.match_indices('\n').map(|(p, _)| p + 1));
        Ok(Self { text, lines, root })
    }

    pub fn stringify(&self) -> String {
        self.root().stringify(0, self)
    }
}

#[derive(Debug, Clone)]
pub struct Cst {
    pub range: TextRange,
    pub rule: Rule,
}

impl Cst {
    fn parse_node(node: &Node, content: &str) -> Result<Cst> {
        let range = TextRange::from_node(node);
        let substr = {
            let range = TextRange::from_node(node);
            range.get_text(content)
        };

        let rule = match node.kind() {
            "source_file" => {
                let children = Cst::collect_children(node, content)?;
                SourceFile { children }.into()
            }

            "task" => {
                let status = Cst::child_by_field_name("status", node, content)?.map(Box::new);
                let meta = Cst::collect_children_by_field_name("node", node, content)?;
                let text = Cst::child_by_field_name("text", node, content)?
                    .ok_or_else(|| anyhow!("Cannot find 'text' field in task."))?;
                let text = Box::new(text);
                let children = Cst::collect_children_by_field_name("children", node, content)?;
                Task {
                    status,
                    meta,
                    text,
                    children,
                }
                .into()
            }

            "header" => {
                let status = Cst::child_by_field_name("status", node, content)?.map(Box::new);
                let meta = Cst::collect_children_by_field_name("node", node, content)?;
                let children = Cst::collect_children_by_field_name("children", node, content)?;
                Header {
                    status,
                    meta,
                    children,
                }
                .into()
            }

            "status" => {
                let child = node
                    .child(0)
                    .ok_or_else(|| anyhow!("Cannot find status kind."))?;
                match child.kind() {
                    "status_todo" => Status {
                        kind: StatusKind::Todo,
                    }
                    .into(),
                    "status_doing" => Status {
                        kind: StatusKind::Doing,
                    }
                    .into(),
                    "status_done" => Status {
                        kind: StatusKind::Done,
                    }
                    .into(),
                    "status_cancel" => Status {
                        kind: StatusKind::Cancelled,
                    }
                    .into(),
                    _ => Rule::Error,
                }
            }

            "priority" => {
                let re = Regex::new(r#"\((.*)\)"#).unwrap();
                let value = re.captures(substr).unwrap()[1].to_owned();
                Priority { value }.into()
            }

            "due" => {
                let re = Regex::new(r#"\((.*)\)"#).unwrap();
                let s_value = &re.captures(substr).unwrap()[1];
                let value = NaiveDate::parse_from_str(s_value, "%Y-%m-%d")?;
                Due { value }.into()
            }

            "keyval" => {
                let re = Regex::new(r#"\{(.*):(.*)\}"#).unwrap();
                let caps = &re.captures(substr).unwrap();
                let key = caps[1].to_owned();
                let value = caps[2].to_owned();
                KeyVal { key, value }.into()
            }

            "category" => {
                let re = Regex::new(r#"\[(.*)\]"#).unwrap();
                let name = re.captures(substr).unwrap()[1].to_owned();
                Category { name }.into()
            }

            "text" => {
                let tags = Cst::collect_children(node, content)?;
                Text {
                    content: substr.to_owned(),
                    tags,
                }
                .into()
            }

            "tag" => {
                let re = Regex::new(r#"@(.*)"#).unwrap();
                let name = re.captures(substr).unwrap()[1].to_owned();
                Tag { name }.into()
            }

            _ => unreachable!(),
        };
        Ok(Cst { range, rule })
    }

    fn collect_children(node: &Node, content: &str) -> Result<Vec<Cst>> {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .map(|node| Cst::parse_node(&node, content))
            .try_collect()
    }

    fn child_by_field_name(name: &str, node: &Node, content: &str) -> Result<Option<Cst>> {
        let result = if let Some(node) = node.child_by_field_name(name) {
            Some(Cst::parse_node(&node, content)?)
        } else {
            None
        };
        Ok(result)
    }

    fn collect_children_by_field_name(name: &str, node: &Node, content: &str) -> Result<Vec<Cst>> {
        let mut cursor = node.walk();
        node.children_by_field_name(name, &mut cursor)
            .map(|node| Cst::parse_node(&node, content))
            .try_collect()
    }
}

impl Cst {
    pub fn get_children(&self) -> Option<Vec<&Cst>> {
        let v = match &self.rule {
            Rule::SourceFile(SourceFile { children }) => children.iter().collect(),
            Rule::Task(Task {
                status,
                meta,
                text,
                children,
            }) => {
                let mut v: Vec<&Cst> = vec![];
                if let Some(b) = status {
                    v.push(b)
                };
                v.extend(meta);
                v.push(text);
                v.extend(children);
                v
            }
            Rule::Header(Header {
                status,
                meta,
                children,
            }) => {
                let mut v: Vec<&Cst> = vec![];
                if let Some(b) = status {
                    v.push(b)
                };
                v.extend(meta);
                v.extend(children);
                v
            }
            Rule::Text(Text { tags, .. }) => tags.iter().collect(),
            _ => return None,
        };
        Some(v)
    }

    pub fn search<F>(&self, predicate: F) -> Vec<&Cst>
    where
        F: Fn(&Cst) -> bool,
    {
        let mut v = self.search_aux(&predicate);
        v.reverse();
        v
    }

    fn search_aux<F>(&self, predicate: &F) -> Vec<&Cst>
    where
        F: Fn(&Cst) -> bool,
    {
        let mut csts = self
            .get_children()
            .unwrap_or_default()
            .into_iter()
            .map(|cst| cst.search_aux(&predicate))
            .concat();
        if predicate(self) {
            csts.push(self)
        }
        csts
    }

    // root_cst.(&cst) で、root_cst を始祖とするすべての CST のうち、
    // cst の祖先に相当する CST の列を返す。
    //
    pub fn ancestor(&self, child: &Cst) -> Vec<&Cst> {
        let point = child.range.start;
        self.get_csts_on_cursor(point)
    }

    pub fn get_csts_on_cursor(&self, cursor: usize) -> Vec<&Cst> {
        if !self.range.includes(cursor) {
            return vec![];
        }

        let cst = self
            .get_children()
            .unwrap_or_default()
            .into_iter()
            .find(|cst| cst.range.includes(cursor));
        let mut v = cst
            .map(|cst| cst.get_csts_on_cursor(cursor))
            .unwrap_or_default();
        v.push(self);
        v
    }

    fn stringify(&self, indent: usize, document: &DocumentSyntax) -> String {
        let substr = self.range.get_text(document.text());
        let mut s = String::new();
        let indent_str = " ".repeat(indent * 2);
        s.push_str(&indent_str);
        s.push_str(&format!("[{}]", self.rule.name(),));
        if !substr.contains('\n') && substr.len() < 50 {
            s.push_str(&format!(r#" "{}""#, substr));
        } else if let Some((start, end)) = self.range.convert_into::<Point>(document) {
            s.push_str(&format!(
                " ({}:{} .. {}:{})",
                start.row + 1,
                start.column + 1,
                end.row + 1,
                end.column + 1,
            ));
        } else {
            unreachable!()
        }
        s.push('\n');
        for child in self.get_children().unwrap_or_default() {
            let text = child.stringify(indent + 1, document);
            s.push_str(&text);
        }
        s
    }
}

#[derive(Debug, Clone)]
pub enum Rule {
    SourceFile(SourceFile),
    Task(Task),
    Header(Header),
    Status(Status),
    Priority(Priority),
    Due(Due),
    KeyVal(KeyVal),
    Category(Category),
    Text(Text),
    Tag(Tag),
    Comment(Comment),
    Error,
}

impl Rule {
    fn name(&self) -> &'static str {
        match &self {
            Rule::SourceFile(_) => "source_file",
            Rule::Task(_) => "task",
            Rule::Header(_) => "header",
            Rule::Status(_) => "status",
            Rule::Priority(_) => "priority",
            Rule::Due(_) => "due",
            Rule::KeyVal(_) => "keyval",
            Rule::Category(_) => "category",
            Rule::Text(_) => "text",
            Rule::Tag(_) => "tag",
            Rule::Comment(_) => "comment",
            Rule::Error => "ERROR",
        }
    }
}

impl From<SourceFile> for Rule {
    fn from(v: SourceFile) -> Self {
        Self::SourceFile(v)
    }
}
impl From<Task> for Rule {
    fn from(v: Task) -> Self {
        Self::Task(v)
    }
}
impl From<Header> for Rule {
    fn from(v: Header) -> Self {
        Self::Header(v)
    }
}
impl From<Status> for Rule {
    fn from(v: Status) -> Self {
        Self::Status(v)
    }
}
impl From<Priority> for Rule {
    fn from(v: Priority) -> Self {
        Self::Priority(v)
    }
}
impl From<Due> for Rule {
    fn from(v: Due) -> Self {
        Self::Due(v)
    }
}
impl From<KeyVal> for Rule {
    fn from(v: KeyVal) -> Self {
        Self::KeyVal(v)
    }
}
impl From<Category> for Rule {
    fn from(v: Category) -> Self {
        Self::Category(v)
    }
}
impl From<Text> for Rule {
    fn from(v: Text) -> Self {
        Self::Text(v)
    }
}
impl From<Tag> for Rule {
    fn from(v: Tag) -> Self {
        Self::Tag(v)
    }
}
impl From<Comment> for Rule {
    fn from(v: Comment) -> Self {
        Self::Comment(v)
    }
}

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub children: Vec<Cst>,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub status: Option<Box<Cst>>,
    pub meta: Vec<Cst>,
    pub text: Box<Cst>,
    pub children: Vec<Cst>,
}

#[derive(Debug, Clone)]
pub struct Header {
    pub status: Option<Box<Cst>>,
    pub meta: Vec<Cst>,
    pub children: Vec<Cst>,
}

#[derive(Debug, Clone)]
pub struct Status {
    pub kind: StatusKind,
}

#[derive(Debug, Clone)]
pub struct Priority {
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct Due {
    pub value: NaiveDate,
}

#[derive(Debug, Clone)]
pub struct KeyVal {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct Category {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Text {
    pub content: String,
    pub tags: Vec<Cst>,
}

#[derive(Debug, Clone)]
pub struct Tag {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Comment {
    pub content: String,
}

#[derive(Debug, Clone, Copy)]
pub enum StatusKind {
    Todo,
    Doing,
    Done,
    Cancelled,
}
