//! The pure state behind a dialog's text input: append-only typing and `Backspace`, the only
//! editing a Dialog offers (there is no caret). Forms hold one per text field and hand the
//! focused one to `dialog_host`, which does the typing for every Dialog in one place.

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextField {
    text: String,
}

impl TextField {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn into_text(self) -> String {
        self.text
    }

    pub fn is_blank(&self) -> bool {
        self.text.trim().is_empty()
    }

    pub fn push(&mut self, ch: char) {
        self.text.push(ch);
    }

    /// A no-op on an empty field, as `String::pop` is.
    pub fn backspace(&mut self) {
        self.text.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_backspace_edit_the_end_of_the_text() {
        let mut field = TextField::new("au");
        field.push('d');
        assert_eq!(field.text(), "aud");
        field.backspace();
        field.backspace();
        assert_eq!(field.text(), "a");
    }

    #[test]
    fn backspace_on_an_empty_field_is_a_no_op() {
        let mut field = TextField::default();
        field.backspace();
        assert_eq!(field.text(), "");
    }

    #[test]
    fn whitespace_only_text_is_blank() {
        assert!(TextField::new("  ").is_blank());
        assert!(!TextField::new(" a ").is_blank());
    }
}
