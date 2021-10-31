use chrono::NaiveDate;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Range};

use crate::position::ConvertBetweenBytes;

use super::Document;

impl Document {
    pub fn get_diagnostics(&self) -> Vec<Diagnostic> {
        [self.get_syntax_error()].concat()
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
                    message: "Syntax Error".to_owned(),
                    related_information: None,
                    tags: None,
                    data: None,
                })
            })
            .collect()
    }
}
