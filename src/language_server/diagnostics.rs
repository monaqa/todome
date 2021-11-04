use chrono::{Local, NaiveDate};
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Range};

use crate::structure::syntax::{Document, Due, Rule};

impl Document {
    pub fn get_diagnostics(&self) -> Vec<Diagnostic> {
        let today = Local::today().naive_local();
        [
            self.get_syntax_error(),
            self.get_due_today(&today),
            // self.get_overdue(&today),
        ]
        .concat()
    }

    fn get_syntax_error(&self) -> Vec<Diagnostic> {
        self.root()
            .search(|cst| cst.rule.name() == "ERROR")
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

    fn get_due_today(&self, date: &NaiveDate) -> Vec<Diagnostic> {
        self.root()
            .search(|cst| match cst.rule {
                Rule::Due(Due { value }) => date == &value,
                _ => false,
            })
            .into_iter()
            .filter_map(|cst| {
                let (start, end) = cst.range.convert_into(self)?;
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
