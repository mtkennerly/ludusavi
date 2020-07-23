// Iced has built-in support for some keyboard shortcuts. This module provides
// support for implementing other shortcuts until Iced provides its own support.

use copypasta::ClipboardProvider;

pub enum Shortcut {
    Undo,
    Redo,
    ClipboardCopy,
    ClipboardCut,
}

pub fn copy_to_clipboard_from_iced(text: &str, cursor: &iced_native::text_input::Cursor) {
    let value = iced_native::text_input::Value::new(text);
    match cursor.state(&value) {
        iced_native::text_input::cursor::State::Selection { start, end } => {
            let _ = copy_to_clipboard(&text[std::cmp::min(start, end)..std::cmp::max(start, end)]);
        }
        iced_native::text_input::cursor::State::Index(_) => {
            let _ = copy_to_clipboard(&text);
        }
    };
}

pub fn copy_to_clipboard(text: &str) -> Result<(), ()> {
    if let Ok(mut ctx) = copypasta::ClipboardContext::new() {
        match ctx.set_contents(text.to_owned()) {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    } else {
        Err(())
    }
}

#[realia::dep_from_registry("ludusavi", "iced")]
fn get_cut_result(original: &str, _modified: &str) -> String {
    // Can't return the modified content yet because it can cause a panic
    // in Iced. See here: https://github.com/hecrj/iced/issues/443
    original.to_owned()
}

#[realia::not(dep_from_registry("ludusavi", "iced"))]
fn get_cut_result(_original: &str, modified: &str) -> String {
    // The panic has been fixed in Iced's latest code, so this is safe:
    modified.to_owned()
}

pub fn cut_to_clipboard_from_iced(text: &str, cursor: &iced_native::text_input::Cursor) -> String {
    let value = iced_native::text_input::Value::new(text);
    match cursor.state(&value) {
        iced_native::text_input::cursor::State::Selection { start, end } => {
            match cut_to_clipboard(&text, std::cmp::min(start, end), std::cmp::max(start, end)) {
                Ok(remaining) => {
                    // TODO: Clear the previous cursor selection.
                    get_cut_result(&text, &remaining)
                }
                Err(_) => text.to_owned(),
            }
        }
        _ => text.to_owned(),
    }
}

pub fn cut_to_clipboard(text: &str, start: usize, end: usize) -> Result<String, ()> {
    let cut = &text[start..end];
    let remaining = format!("{}{}", &text[0..start], &text[end..text.len()]);
    match copy_to_clipboard(&cut) {
        Ok(_) => Ok(remaining),
        Err(_) => Err(()),
    }
}

pub struct TextHistory {
    history: std::collections::VecDeque<String>,
    limit: usize,
    position: usize,
}

impl Default for TextHistory {
    fn default() -> Self {
        Self::new("", 100)
    }
}

impl TextHistory {
    pub fn new(initial: &str, limit: usize) -> Self {
        let mut history = std::collections::VecDeque::<String>::new();
        history.push_back(initial.to_string());
        Self {
            history,
            limit,
            position: 0,
        }
    }

    pub fn push(&mut self, text: &str) {
        if self.current() == text {
            return;
        }
        if self.position + 1 < self.history.len() {
            self.history.truncate(self.position + 1);
        }
        if self.position + 1 >= self.limit {
            self.history.pop_front();
        }
        self.history.push_back(text.to_string());
        self.position = self.history.len() - 1;
    }

    pub fn current(&self) -> String {
        match self.history.get(self.position) {
            Some(x) => x.to_string(),
            None => "".to_string(),
        }
    }

    pub fn undo(&mut self) -> String {
        self.position = if self.position == 0 { 0 } else { self.position - 1 };
        self.current()
    }

    pub fn redo(&mut self) -> String {
        self.position = std::cmp::min(self.position + 1, self.history.len() - 1);
        self.current()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_history() {
        let mut ht = TextHistory::new("initial", 3);

        assert_eq!(ht.current(), "initial");
        assert_eq!(ht.undo(), "initial");
        assert_eq!(ht.redo(), "initial");

        ht.push("a");
        assert_eq!(ht.current(), "a");
        assert_eq!(ht.undo(), "initial");
        assert_eq!(ht.undo(), "initial");
        assert_eq!(ht.redo(), "a");
        assert_eq!(ht.redo(), "a");

        // Duplicates are ignored:
        ht.push("a");
        ht.push("a");
        ht.push("a");
        assert_eq!(ht.undo(), "initial");

        // History is clipped at the limit:
        ht.push("b");
        ht.push("c");
        ht.push("d");
        assert_eq!(ht.undo(), "c");
        assert_eq!(ht.undo(), "b");
        assert_eq!(ht.undo(), "b");

        // Redos are lost on push:
        ht.push("e");
        assert_eq!(ht.current(), "e");
        assert_eq!(ht.redo(), "e");
        assert_eq!(ht.undo(), "b");
        assert_eq!(ht.undo(), "b");
    }
}
