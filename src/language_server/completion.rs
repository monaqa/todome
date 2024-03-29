use std::collections::HashSet;

use anyhow::*;
use chrono::{Duration, Local};
use log::debug;
use tower_lsp::lsp_types::{CompletionItem, CompletionTextEdit, TextEdit};
use tree_sitter::Point;
use tree_sitter_todome::syntax::ast::{AstNode, Category, Tag};

use crate::structure::{position::PosInto, syntax::Document};

impl Document {
    pub fn get_completion(
        &self,
        params: &tower_lsp::lsp_types::CompletionParams,
    ) -> Result<Vec<CompletionItem>> {
        let cursor = {
            let cursor = params.text_document_position.position;
            let cursor = cursor.try_pos_into(self);
            if cursor.is_none() {
                return Ok(vec![]);
            }
            cursor.unwrap()
        };
        let nodes = self.root().syntax().dig(cursor - 1);
        if let Some(node) = nodes.get(0) {
            debug!("kind: {}", node.green().kind().as_str());
            if ["subtext", "memo"].contains(&node.green().kind().as_str()) {
                debug!("returning ok");
                return Ok(vec![]);
            }
        }

        // trigger_character と CST の種類（もし Error でなければ）によって補完候補を出し分ける
        let trigger_character = params
            .context
            .as_ref()
            .and_then(|ctx| ctx.trigger_character.as_deref());
        let rule = nodes.get(1).map(|node| node.green().kind().as_str());

        let completions = match (trigger_character, rule) {
            (Some("["), _) | (_, Some("category")) => {
                // category name の completion
                self.get_category_completions(cursor)
            }
            (Some("("), _) | (_, Some("due")) | (_, Some("priority")) => {
                // due の completion
                self.get_due_completion(cursor)
            }
            (Some("@"), _) | (_, Some("tag")) => {
                // tag name の completion
                self.get_tag_completions(cursor)
            }
            _ => return Ok(vec![]),
        };

        Ok(completions)
    }

    fn get_category_completions(&self, cursor: usize) -> Vec<CompletionItem> {
        let range = {
            let row = {
                let point: Option<Point> = cursor.try_pos_into(self);
                if point.is_none() {
                    return vec![];
                }
                point.unwrap().row
            };
            let start_of_line = self.lines()[row];
            let before_cursor = &self.text()[start_of_line..cursor];
            let after_cursor = &self.text()[cursor..cursor + 1];
            let pos_open_bracket = before_cursor.rfind('[').unwrap_or(before_cursor.len());
            let pos_close_bracket = usize::from(after_cursor == "]");
            (start_of_line + pos_open_bracket, cursor + pos_close_bracket)
                .try_pos_into(self)
                .unwrap()
        };

        let categories: HashSet<String> = self
            .root()
            .syntax()
            .children_recursive()
            .into_iter()
            .filter_map(Category::cast)
            .map(|category| category.name())
            .collect();
        categories
            .into_iter()
            .map(|s| {
                let new_text = format!("[{}]", s);
                let edit = TextEdit {
                    range,
                    new_text: new_text.clone(),
                };
                CompletionItem {
                    label: new_text,
                    kind: None,
                    detail: None,
                    documentation: None,
                    deprecated: None,
                    preselect: None,
                    sort_text: None,
                    filter_text: None,
                    insert_text: None,
                    insert_text_format: None,
                    insert_text_mode: None,
                    text_edit: Some(CompletionTextEdit::Edit(edit)),
                    additional_text_edits: None,
                    command: None,
                    commit_characters: None,
                    data: None,
                    tags: None,
                }
            })
            .collect()
    }

    fn get_tag_completions(&self, cursor: usize) -> Vec<CompletionItem> {
        let range = {
            let row = {
                let point: Option<Point> = cursor.try_pos_into(self);
                if point.is_none() {
                    return vec![];
                }
                point.unwrap().row
            };

            let start_of_line = self.lines()[row];
            let before_cursor = &self.text()[start_of_line..cursor];
            let pos_open_bracket = before_cursor.rfind('@').unwrap_or(before_cursor.len());
            (start_of_line + pos_open_bracket, cursor)
                .try_pos_into(self)
                .unwrap()
        };

        let tags: HashSet<String> = self
            .root()
            .syntax()
            .children_recursive()
            .into_iter()
            .filter_map(Tag::cast)
            .map(|tag| tag.name())
            .collect();
        tags.into_iter()
            .map(|s| {
                let new_text = format!("@{}", s);
                let edit = TextEdit {
                    range,
                    new_text: new_text.clone(),
                };
                CompletionItem {
                    label: new_text,
                    kind: None,
                    detail: None,
                    documentation: None,
                    deprecated: None,
                    preselect: None,
                    sort_text: None,
                    filter_text: None,
                    insert_text: None,
                    insert_text_format: None,
                    insert_text_mode: None,
                    text_edit: Some(CompletionTextEdit::Edit(edit)),
                    additional_text_edits: None,
                    command: None,
                    commit_characters: None,
                    data: None,
                    tags: None,
                }
            })
            .collect()
    }

    fn get_due_completion(&self, cursor: usize) -> Vec<CompletionItem> {
        let range = {
            let row = {
                let point: Option<Point> = cursor.try_pos_into(self);
                if point.is_none() {
                    return vec![];
                }
                point.unwrap().row
            };
            let start_of_line = self.lines()[row];
            let before_cursor = &self.text()[start_of_line..cursor];
            let after_cursor = &self.text()[cursor..cursor + 1];
            let pos_open_paren = before_cursor.rfind('(').unwrap_or(before_cursor.len());
            let pos_close_paren = usize::from(after_cursor == ")");
            (start_of_line + pos_open_paren, cursor + pos_close_paren)
                .try_pos_into(self)
                .unwrap()
        };

        let now = Local::now().naive_local().date();
        let candidates = [
            (now, "today"),
            (now + Duration::days(1), "tomorrow"),
            (now + Duration::days(2), "2 days later"),
            (now + Duration::days(7), "1 week later"),
        ];
        candidates
            .into_iter()
            .map(|(date, desc)| {
                let new_text = format!("({})", date.format("%Y-%m-%d"));
                let edit = TextEdit {
                    range,
                    new_text: new_text.clone(),
                };
                CompletionItem {
                    label: new_text,
                    kind: None,
                    detail: Some(desc.to_owned()),
                    documentation: None,
                    deprecated: None,
                    preselect: None,
                    sort_text: None,
                    filter_text: None,
                    insert_text: None,
                    insert_text_format: None,
                    insert_text_mode: None,
                    text_edit: Some(CompletionTextEdit::Edit(edit)),
                    additional_text_edits: None,
                    command: None,
                    commit_characters: None,
                    data: None,
                    tags: None,
                }
            })
            .collect()
    }
}
