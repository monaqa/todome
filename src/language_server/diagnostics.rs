use chrono::{Local, NaiveDate};
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity};
use tree_sitter_todome::syntax::ast::AstNode;

use crate::structure::{position::PosInto, syntax::Document};

impl Document {
    pub fn get_diagnostics(&self) -> Vec<Diagnostic> {
        let today = Local::today().naive_local();
        [
            self.get_syntax_error(),
            self.get_due_today(&today),
            self.get_overdue(&today),
        ]
        .concat()
    }

    fn get_syntax_error(&self) -> Vec<Diagnostic> {
        self.root()
            .syntax()
            .children_recursive()
            .into_iter()
            .filter_map(|n| {
                if n.green().kind().as_str() == "ERROR" {
                    let range = n.range().try_pos_into(self)?;
                    Some(Diagnostic {
                        range,
                        severity: Some(DiagnosticSeverity::Error),
                        code: None,
                        code_description: None,
                        source: Some("todome".to_owned()),
                        message: "Syntax error".to_owned(),
                        related_information: None,
                        tags: None,
                        data: None,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    fn get_overdue(&self, date: &NaiveDate) -> Vec<Diagnostic> {
        self.root()
            .items_nested()
            .filter(|item| item.as_task().is_some())
            .filter(|item| {
                let is_valid = item
                    .scoped_statuses()
                    .next()
                    .map(|status| status.is_valid())
                    .unwrap_or(true);
                let is_overdue = item
                    .scoped_dues()
                    .next()
                    .and_then(|due| due.try_as_date())
                    .map(|due| due < *date)
                    .unwrap_or(false);
                is_valid && is_overdue
            })
            .filter_map(|item| {
                let task = item.as_task()?;
                let range = task.text()?.syntax().range().try_pos_into(self)?;
                Some(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::Error),
                    code: None,
                    code_description: None,
                    source: Some("todome".to_owned()),
                    message: "This task is overdue.".to_owned(),
                    related_information: None,
                    tags: None,
                    data: None,
                })
            })
            .collect()
    }

    fn get_due_today(&self, date: &NaiveDate) -> Vec<Diagnostic> {
        self.root()
            .items_nested()
            .filter(|item| item.as_task().is_some())
            .filter(|item| {
                let is_valid = item
                    .scoped_statuses()
                    .next()
                    .map(|status| status.is_valid())
                    .unwrap_or(true);
                let is_due_today = item
                    .scoped_dues()
                    .next()
                    .and_then(|due| due.try_as_date())
                    .map(|due| due == *date)
                    .unwrap_or(false);
                is_valid && is_due_today
            })
            .filter_map(|item| {
                let task = item.as_task()?;
                let range = task.text()?.syntax().range().try_pos_into(self)?;
                Some(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::Warning),
                    code: None,
                    code_description: None,
                    source: Some("todome".to_owned()),
                    message: "This task is due today.".to_owned(),
                    related_information: None,
                    tags: None,
                    data: None,
                })
            })
            .collect()
    }
}
