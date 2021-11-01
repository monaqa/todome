use chrono::NaiveDate;

use crate::parser::{Cst, Rule, StatusKind};

#[derive(Debug, Clone)]
pub struct FormattingOption {}

impl FormattingOption {
    pub fn new() -> Self {
        Self {}
    }
}

/// format された Cst のデータを格納する構造体。
/// TODO: オプション次第では、rule 以外にも保持しておきたい情報も格納
#[derive(Debug, Clone)]
pub struct FormattedCst {
    rule: FormattedRule,
    comments: Vec<FormattedCst>,
}

/// 構文規則とその階層構造を表したもの。
/// struct の順番は、実際に構文が登場する順番に統一されている。
#[derive(Debug, Clone)]
pub enum FormattedRule {
    SourceFile {
        children: Vec<FormattedCst>,
    },
    Task {
        status: Option<Box<FormattedCst>>,
        meta: Vec<FormattedCst>,
        text: Box<FormattedCst>,
        children: Vec<FormattedCst>,
    },
    Header {
        status: Option<Box<FormattedCst>>,
        meta: Vec<FormattedCst>,
        children: Vec<FormattedCst>,
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
    Tag {
        name: String,
    },
    Comment {
        content: String,
    },
    Error,
}

impl FormattedCst {
    pub fn from_cst(cst: &Cst) -> FormattedCst {
        match &cst.rule {
            Rule::SourceFile { children } => {
                let children = children.iter().map(FormattedCst::from_cst).collect();
                let rule = FormattedRule::SourceFile { children };
                FormattedCst {
                    rule,
                    comments: vec![],
                }
            }
            Rule::Task {
                status,
                meta,
                text,
                children,
            } => {
                let status = status
                    .as_ref()
                    .map(|cst| Box::new(FormattedCst::from_cst(cst)));
                let meta = meta.iter().map(FormattedCst::from_cst).collect();
                let text = Box::new(FormattedCst::from_cst(cst));
                let children = children.iter().map(FormattedCst::from_cst).collect();
                let rule = FormattedRule::Task {
                    status,
                    meta,
                    text,
                    children,
                };
                FormattedCst {
                    rule,
                    comments: vec![],
                }
            }
            Rule::Header {
                status,
                meta,
                children,
            } => {
                todo!()
            }
            Rule::Status { kind } => {
                todo!()
            }
            Rule::Priority { value } => {
                todo!()
            }
            Rule::Due { value } => {
                todo!()
            }
            Rule::KeyVal { key, value } => {
                todo!()
            }
            Rule::Category { name } => {
                todo!()
            }
            Rule::Text { content, tags } => {
                todo!()
            }
            Rule::Comment { content } => {
                todo!()
            }
            Rule::Tag { name } => {
                todo!()
            }
            Rule::Error => {
                todo!()
            }
        }
    }

    pub fn to_formatted_string(&self, opt: &FormattingOption) -> String {
        match &self.rule {
            FormattedRule::SourceFile { children } => {
                todo!()
            }
            FormattedRule::Task {
                status,
                meta,
                text,
                children,
            } => {
                todo!()
            }
            FormattedRule::Header {
                status,
                meta,
                children,
            } => {
                todo!()
            }
            FormattedRule::Status { kind } => {
                todo!()
            }
            FormattedRule::Priority { value } => {
                todo!()
            }
            FormattedRule::Due { value } => {
                todo!()
            }
            FormattedRule::KeyVal { key, value } => {
                todo!()
            }
            FormattedRule::Category { name } => {
                todo!()
            }
            FormattedRule::Text { content } => {
                todo!()
            }
            FormattedRule::Comment { content } => {
                todo!()
            }
            FormattedRule::Error => {
                todo!()
            }
            FormattedRule::Tag { name } => todo!(),
        }
    }
}
