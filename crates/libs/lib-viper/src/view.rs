//! The pieces a Presenter builds a form or Dialog's view model from, as plain data a View only
//! draws. A text field's caret and a select's open list aren't here: they are the View's, as
//! widget mechanics of its toolkit.

/// One option of a select or segmented control: the value it sets and the text it shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice<T> {
    pub value: T,
    pub label: String,
}

/// A text field as drawn: its label, what it holds, its placeholder and whether it has focus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextFieldView {
    pub label: String,
    pub text: String,
    pub placeholder: String,
    pub focused: bool,
}

/// A select or segmented control as drawn: its label, options, the value chosen and whether it
/// has focus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectView<T> {
    pub label: String,
    pub options: Vec<Choice<T>>,
    pub chosen: Option<T>,
    pub focused: bool,
}

/// A Dialog's buttons and title: the submit's label and whether it is enabled, and whether it
/// destroys something (drawn in the danger colour).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogFrame {
    pub title: String,
    pub submit: String,
    pub can_confirm: bool,
    pub destructive: bool,
}
