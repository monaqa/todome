//! 構文解析の結果を格納する構文木の要素。

use std::{collections::HashMap, fmt::Display};

use anyhow::*;
use chrono::NaiveDate;
use itertools::Itertools;
use log::info;
use regex::Regex;
use tower_lsp::lsp_types::Url;
use tree_sitter::{Node, Parser, Point};

use super::position::TextRange;

#[derive(Debug, Clone, Default)]
pub struct DocumentCache(HashMap<Url, Document>);

impl DocumentCache {
    pub fn register_or_update(&mut self, url: &Url, text: String) -> Result<&Document> {
        let document = Document::parse(text)?;
        self.0.insert(url.to_owned(), document);
        Ok(self.0.get(url).unwrap())
    }

    pub fn get(&self, key: &Url) -> Option<&Document> {
        self.0.get(key)
    }
}

#[derive(Debug, Clone)]
pub struct Document {
    text: String,
    lines: Vec<usize>,
    root: Cst,
}

/// getter, setter
impl Document {
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

    pub fn into_cst(self) -> Cst {
        self.root
    }
}

impl Document {
    pub fn parse(text: String) -> Result<Document> {
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
}

impl Display for Document {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.root().stringify(0, self))
    }
}

#[derive(Debug, Clone)]
pub struct Cst {
    pub range: TextRange,
    pub rule: Rule,
    pub comments: Vec<Cst>,
    pub errors: Vec<Cst>,
}

impl Cst {
    fn parse_node(node: &Node, content: &str) -> Result<Cst> {
        let range = TextRange::from_node(node);
        let substr = {
            let range = TextRange::from_node(node);
            range.get_text(content)
        };
        let errors = {
            let mut cursor = node.walk();
            node.children(&mut cursor)
                .filter(|node| node.is_error())
                .map(|node| Cst::parse_node(&node, content))
                .try_collect()?
        };
        let comments = {
            let mut cursor = node.walk();
            node.children(&mut cursor)
                .filter(|node| node.kind() == "comment")
                .map(|node| Cst::parse_node(&node, content))
                .try_collect()?
        };

        let rule = match node.kind() {
            "source_file" => {
                let children = Cst::collect_children(node, content)?;
                SourceFile { children }.into()
            }

            "task" => {
                let status = Cst::child_by_field_name("status", node, content)?.map(Box::new);
                let meta = if let Some(node) = node.child_by_field_name("meta") {
                    let mut cursor = node.walk();
                    node.children(&mut cursor)
                        .map(|node| Cst::parse_node(&node, content))
                        .try_collect()?
                } else {
                    vec![]
                };
                let text = Cst::child_by_field_name("text", node, content)?
                    .ok_or_else(|| anyhow!("Cannot find 'text' field in task."))?;
                let text = Box::new(text);
                let children = if let Some(node) = node.child_by_field_name("children") {
                    let mut cursor = node.walk();
                    node.children(&mut cursor)
                        .map(|node| Cst::parse_node(&node, content))
                        .try_collect()?
                } else {
                    vec![]
                };
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
                let meta = if let Some(node) = node.child_by_field_name("meta") {
                    let mut cursor = node.walk();
                    node.children(&mut cursor)
                        .map(|node| Cst::parse_node(&node, content))
                        .try_collect()?
                } else {
                    vec![]
                };
                let children = if let Some(node) = node.child_by_field_name("children") {
                    let mut cursor = node.walk();
                    node.children(&mut cursor)
                        .map(|node| Cst::parse_node(&node, content))
                        .try_collect()?
                } else {
                    vec![]
                };
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

            "ERROR" => {
                info!("error");
                return Ok(Cst {
                    range,
                    rule: Rule::Error,
                    comments: vec![],
                    errors: vec![],
                });
            }

            "comment" => {
                let content = substr.to_owned();
                Comment { content }.into()
            }

            s => unreachable!("rule name: {}", s),
        };
        Ok(Cst {
            range,
            rule,
            comments,
            errors,
        })
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
}

impl Cst {
    /// すべての子要素を取得する。
    pub fn get_children(&self, include_error: bool, include_comment: bool) -> Vec<&Cst> {
        let mut v = match &self.rule {
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
            _ => vec![],
        };
        if include_error {
            v.extend(self.errors.iter());
        }
        if include_comment {
            v.extend(self.comments.iter());
        }
        v
    }

    pub fn search<F>(&self, predicate: F, include_error: bool, include_comment: bool) -> Vec<&Cst>
    where
        F: Fn(&Cst) -> bool,
    {
        let mut v = self.search_aux(&predicate, include_error, include_comment);
        v.reverse();
        v
    }

    fn search_aux<F>(&self, predicate: &F, include_error: bool, include_comment: bool) -> Vec<&Cst>
    where
        F: Fn(&Cst) -> bool,
    {
        let mut csts = self
            .get_children(include_error, include_comment)
            .into_iter()
            .map(|cst| cst.search_aux(predicate, include_error, include_comment))
            .concat();
        if predicate(self) {
            csts.push(self)
        }
        csts
    }

    pub fn search_task<F>(&self, predicate: F) -> Vec<&Cst>
    where
        F: Fn(&Context) -> bool,
    {
        let context = Context::default();
        self.search_task_aux(&context, &predicate)
    }

    fn search_task_aux<F>(&self, context: &Context, predicate: &F) -> Vec<&Cst>
    where
        F: Fn(&Context) -> bool,
    {
        let context = context.with_cst(self);
        let mut csts = vec![];
        if predicate(&context) {
            csts.push(self);
        }
        let children: &[_] = match &self.rule {
            Rule::SourceFile(SourceFile { children }) => children,
            Rule::Header(Header { children, .. }) => children,
            Rule::Task(Task { children, .. }) => children,
            _ => &[],
        };
        let iter = children
            .iter()
            .map(|cst| cst.search_task_aux(&context, predicate));
        csts.extend(iter.concat());
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
            .get_children(true, true)
            .into_iter()
            .find(|cst| cst.range.includes(cursor));
        let mut v = cst
            .map(|cst| cst.get_csts_on_cursor(cursor))
            .unwrap_or_default();
        v.push(self);
        v
    }

    /// 自身もしくは子のうち、範囲が range と一致する Cst を集める。
    pub fn get_csts_on_range(&self, range: TextRange) -> Vec<&Cst> {
        self.get_csts_on_cursor(range.start)
            .into_iter()
            .filter(|cst| cst.range == range)
            .collect_vec()
    }

    fn stringify(&self, indent: usize, document: &Document) -> String {
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
        for child in self.get_children(true, true) {
            let text = child.stringify(indent + 1, document);
            s.push_str(&text);
        }
        s
    }
}

#[derive(Debug, Clone, Default)]
pub struct Context {
    pub explicit_status: Vec<StatusKind>,
    pub explicit_priority: Vec<String>,
    pub explicit_keyval: HashMap<String, String>,
    pub explicit_due: Vec<NaiveDate>,
    pub categories: Vec<String>,
}

impl Context {
    fn with_cst(&self, cst: &Cst) -> Context {
        match &cst.rule {
            Rule::SourceFile(_) => Context::default(),
            Rule::Task(Task { status, meta, .. }) => {
                let mut context = self.clone();
                if let Some(status) = status {
                    let status_kind = status.rule.as_status().unwrap().kind;
                    context.explicit_status.push(status_kind);
                };
                for cst in meta {
                    match &cst.rule {
                        Rule::Priority(Priority { value }) => {
                            context.explicit_priority.push(value.clone());
                        }
                        Rule::Due(Due { value }) => context.explicit_due.push(*value),
                        Rule::KeyVal(KeyVal { key, value }) => {
                            context.explicit_keyval.insert(key.clone(), value.clone());
                        }
                        Rule::Category(Category { name }) => context.categories.push(name.clone()),
                        _ => unreachable!(),
                    }
                }
                context
            }
            Rule::Header(Header { status, meta, .. }) => {
                let mut context = self.clone();
                if let Some(status) = status {
                    let status_kind = status.rule.as_status().unwrap().kind;
                    context.explicit_status.push(status_kind);
                };
                for cst in meta {
                    match &cst.rule {
                        Rule::Priority(Priority { value }) => {
                            context.explicit_priority.push(value.clone());
                        }
                        Rule::Due(Due { value }) => context.explicit_due.push(*value),
                        Rule::KeyVal(KeyVal { key, value }) => {
                            context.explicit_keyval.insert(key.clone(), value.clone());
                        }
                        Rule::Category(Category { name }) => context.categories.push(name.clone()),
                        _ => unreachable!(),
                    }
                }
                context
            }
            Rule::Error | Rule::Comment(_) => self.clone(),
            rule => unreachable!("rule: {}", rule.name()),
        }
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
    pub fn as_source_file(&self) -> Option<&SourceFile> {
        if let Self::SourceFile(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_task(&self) -> Option<&Task> {
        if let Self::Task(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_header(&self) -> Option<&Header> {
        if let Self::Header(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_status(&self) -> Option<&Status> {
        if let Self::Status(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_priority(&self) -> Option<&Priority> {
        if let Self::Priority(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_due(&self) -> Option<&Due> {
        if let Self::Due(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_key_val(&self) -> Option<&KeyVal> {
        if let Self::KeyVal(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_category(&self) -> Option<&Category> {
        if let Self::Category(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_text(&self) -> Option<&Text> {
        if let Self::Text(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_tag(&self) -> Option<&Tag> {
        if let Self::Tag(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_comment(&self) -> Option<&Comment> {
        if let Self::Comment(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the rule is [`Error`].
    ///
    /// [`Error`]: Rule::Error
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error)
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

impl Rule {
    pub fn name(&self) -> &'static str {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Todo,
    Doing,
    Done,
    Cancelled,
}
