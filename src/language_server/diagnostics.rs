use chrono::{Duration, Local, NaiveDate};
use log::debug;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, DiagnosticTag, Range};
use tree_sitter_todome::syntax::ast::{AstNode, Task};

use crate::structure::{position::PosInto, syntax::Document};

fn default_diag() -> Diagnostic {
    Diagnostic {
        source: Some("todome".to_owned()),
        ..Default::default()
    }
}

impl Document {
    pub fn get_diagnostics(&self) -> Vec<Diagnostic> {
        let today = Local::today().naive_local();
        [self.get_syntax_error(), self.get_date_diagnostics(today)].concat()
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

    fn get_date_diagnostics(&self, today: NaiveDate) -> Vec<Diagnostic> {
        self.root()
            .items_nested()
            .into_iter()
            .filter(|item| {
                // status が valid のものだけ
                item.scoped_statuses()
                    .into_iter()
                    .next()
                    .map(|status| status.is_valid())
                    .unwrap_or(true)
            })
            .flat_map(|item| {
                if let Some(task) = item.as_task() {
                    self.get_date_diags_for_task(task, today)
                } else {
                    vec![]
                }
            })
            .collect()
    }

    /// 特定のタスクに対し、日付に関連する diagnostics を生成する。
    ///
    /// * 日付設定に矛盾がある
    ///     * [ERROR] start <= target <= deadline が満たされていない
    /// * 開始前
    ///     * [INFO (unused)] start > today
    /// * 期日が近い
    ///     * [INFO] deadline < today + 7
    ///     * [INFO] target == today
    ///     * [WARNING] deadline == today
    /// * 期日を過ぎている
    ///     * [ERROR] deadline < today
    ///     * [WARNING] target < today
    fn get_date_diags_for_task(&self, task: &Task, today: NaiveDate) -> Vec<Diagnostic> {
        let Some(date) = task.meta().into_iter().find_map(|meta| meta.as_due().cloned())
        else {
            return vec![]
        };

        let start = date.start();
        let target = date.target();
        let deadline = date.deadline();

        let mut diags = vec![];

        if let (Some(start), Some(target)) = (start, target) {
            if start > target {
                let range = date
                    .syntax()
                    .range()
                    .try_pos_into(self)
                    .expect("failed to convert position.");
                diags.push(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::Error),
                    message: "start date must be earlier than target date.".to_owned(),
                    ..default_diag()
                })
            }
        }

        if let (Some(target), Some(deadline)) = (target, deadline) {
            if target > deadline {
                let range = date
                    .syntax()
                    .range()
                    .try_pos_into(self)
                    .expect("failed to convert position.");
                diags.push(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::Error),
                    message: "target date must be earlier than deadline.".to_owned(),
                    ..default_diag()
                })
            }
        }

        if let (Some(start), Some(deadline)) = (start, deadline) {
            if start > deadline {
                let range = date
                    .syntax()
                    .range()
                    .try_pos_into(self)
                    .expect("failed to convert position.");
                diags.push(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::Error),
                    message: "start date must be earlier than deadline.".to_owned(),
                    ..default_diag()
                })
            }
        }

        if let Some(start) = start {
            if start > today {
                let range = task
                    .syntax()
                    .range()
                    .try_pos_into(self)
                    .expect("failed to convert position.");
                diags.push(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::Hint),
                    message: "this task is not started yet.".to_owned(),
                    tags: Some(vec![DiagnosticTag::Unnecessary]),
                    ..default_diag()
                })
            }
        }

        if let Some(target) = target {
            let range = task
                .syntax()
                .range()
                .try_pos_into(self)
                .expect("failed to convert position.");
            match target {
                d if d < today => diags.push(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::Warning),
                    message: "target date of this task is over.".to_owned(),
                    ..default_diag()
                }),
                d if d == today => diags.push(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::Information),
                    message: "this task is targeted today.".to_owned(),
                    ..default_diag()
                }),
                _ => {}
            }
        }

        if let Some(deadline) = deadline {
            let range = task
                .syntax()
                .range()
                .try_pos_into(self)
                .expect("failed to convert position.");
            match deadline {
                d if d < today => diags.push(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::Error),
                    message: "this task is OVERDUE!".to_owned(),
                    ..default_diag()
                }),
                d if d == today => diags.push(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::Warning),
                    message: "this task is due today.".to_owned(),
                    ..default_diag()
                }),
                d if d <= today - Duration::days(7) => diags.push(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::Warning),
                    message: "this task is due in 1 week.".to_owned(),
                    ..default_diag()
                }),
                _ => {}
            }
        }

        diags
    }
}
