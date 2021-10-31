use chrono::{Local, NaiveDate};
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Range};

use crate::{parser::Rule, position::ConvertBetweenBytes};

use super::Document;

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
        self.cst
            .search_cst(|cst| cst.rule_name() == "ERROR")
            .into_iter()
            .filter_map(|cst| {
                let range = Range {
                    start: cst.range.0.try_convert(self)?,
                    end: cst.range.1.try_convert(self)?,
                };
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

    fn get_due_today(&self, date: &NaiveDate) -> Vec<Diagnostic> {
        self.cst
            .search_cst(|cst| match cst.rule {
                Rule::Due { value } => date == &value,
                _ => false,
            })
            .into_iter()
            .filter_map(|cst| {
                let range = Range {
                    start: cst.range.0.try_convert(self)?,
                    end: cst.range.1.try_convert(self)?,
                };
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

    fn get_overdue(&self, date: &NaiveDate) -> Vec<Diagnostic> {
        self.cst
            .search_cst(|cst| match cst.rule {
                Rule::Due { value } => date > &value,
                _ => false,
            })
            .into_iter()
            .filter_map(|cst| {
                let range = Range {
                    start: cst.range.0.try_convert(self)?,
                    end: cst.range.1.try_convert(self)?,
                };
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
}
