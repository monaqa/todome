use chrono::{Local, NaiveDate};
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Range};

use crate::structure::syntax::{Document, StatusKind};

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
            .search(|cst| cst.rule.name() == "ERROR", true, false)
            .into_iter()
            .filter_map(|cst| {
                let (start, end) = cst.range.convert_into(self)?;
                let range = Range { start, end };
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
            })
            .collect()
    }

    fn get_overdue(&self, date: &NaiveDate) -> Vec<Diagnostic> {
        self.root()
            .search_task(|context| {
                // dbg!(context);
                let is_done_or_cancel = context
                    .explicit_status
                    .last()
                    .map(|status| *status == StatusKind::Done || *status == StatusKind::Cancelled)
                    .unwrap_or(false);
                let is_due_today = context
                    .explicit_due
                    .last()
                    .map(|due| due < date)
                    .unwrap_or(false);
                !is_done_or_cancel && is_due_today
            })
            .into_iter()
            .filter_map(|cst| {
                let task = cst.rule.as_task()?;
                // タスクのテキスト部分だけをハイライトする（子タスクにはハイライトを付けない）
                let (start, _) = cst.range.convert_into(self)?;
                let (_, end) = task.text.range.convert_into(self)?;
                let range = Range { start, end };
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
            .search_task(|context| {
                // dbg!(context);
                let is_done_or_cancel = context
                    .explicit_status
                    .last()
                    .map(|status| *status == StatusKind::Done || *status == StatusKind::Cancelled)
                    .unwrap_or(false);
                let is_due_today = context
                    .explicit_due
                    .last()
                    .map(|due| due == date)
                    .unwrap_or(false);
                !is_done_or_cancel && is_due_today
            })
            .into_iter()
            .filter_map(|cst| {
                let task = cst.rule.as_task()?;
                // タスクのテキスト部分だけをハイライトする（子タスクにはハイライトを付けない）
                let (start, _) = cst.range.convert_into(self)?;
                let (_, end) = task.text.range.convert_into(self)?;
                let range = Range { start, end };
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
