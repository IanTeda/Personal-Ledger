//! The pure state behind the dialogs' shared dropdown (the Desktop Accounts map's "select control"
//! decision): `gpui`-free, so stepping, wrapping, committing, cancelling and the capped scroll
//! window are all unit-tested without a window.
//!
//! The options list is *not* stored. It comes from live Settings data (institutions, units), so
//! every operation takes it as an argument, and the selection is stored as the option's value
//! key (an institution name, a unit code) rather than an index -- it survives the list changing
//! underneath it.

use std::ops::Range;

/// Most rows an open list shows at once; the highlight scrolls the window past this.
pub const MAX_VISIBLE_ROWS: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SelectState {
    value: Option<String>,
    open: bool,
    highlight: usize,
    scroll: usize,
}

impl SelectState {
    pub fn new(value: Option<String>) -> Self {
        Self {
            value,
            ..Self::default()
        }
    }

    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// The highlighted row while open (an index into the options).
    pub fn highlight(&self) -> usize {
        self.highlight
    }

    fn position(&self, options: &[String]) -> Option<usize> {
        let value = self.value.as_deref()?;
        options.iter().position(|option| option == value)
    }

    /// Closed and focused, `Up`/`Down` change the value directly, wrapping at either end. With no
    /// value yet, the first step lands on the first (`Down`) or last (`Up`) option.
    pub fn step(&mut self, options: &[String], delta: isize) {
        if options.is_empty() {
            return;
        }
        let next = match self.position(options) {
            Some(index) => wrap(index, delta, options.len()),
            None if delta < 0 => options.len() - 1,
            None => 0,
        };
        self.value = Some(options[next].clone());
    }

    /// Opens the list with the current value highlighted (or the first option when there is none).
    pub fn open(&mut self, options: &[String]) {
        if options.is_empty() {
            return;
        }
        self.open = true;
        self.highlight = self.position(options).unwrap_or(0);
        self.scroll = 0;
        self.follow_highlight(options.len());
    }

    /// Open, `Up`/`Down` move the highlight, wrapping, and the window follows it.
    pub fn move_highlight(&mut self, options: &[String], delta: isize) {
        if !self.open || options.is_empty() {
            return;
        }
        self.highlight = wrap(self.highlight.min(options.len() - 1), delta, options.len());
        self.follow_highlight(options.len());
    }

    /// Takes the highlighted option as the value and closes the list.
    pub fn commit(&mut self, options: &[String]) {
        if self.open
            && let Some(option) = options.get(self.highlight)
        {
            self.value = Some(option.clone());
        }
        self.open = false;
    }

    /// Sets the value to `options[index]` and closes the list -- a click on a row.
    pub fn choose(&mut self, options: &[String], index: usize) {
        if let Some(option) = options.get(index) {
            self.value = Some(option.clone());
        }
        self.open = false;
    }

    /// Closes the list without changing the value.
    pub fn cancel(&mut self) {
        self.open = false;
    }

    /// The options to draw while open: at most [`MAX_VISIBLE_ROWS`], starting where the window
    /// has scrolled to.
    pub fn visible_range(&self, len: usize) -> Range<usize> {
        let start = self.scroll.min(len.saturating_sub(MAX_VISIBLE_ROWS));
        start..(start + MAX_VISIBLE_ROWS).min(len)
    }

    fn follow_highlight(&mut self, len: usize) {
        if self.highlight < self.scroll {
            self.scroll = self.highlight;
        } else if self.highlight >= self.scroll + MAX_VISIBLE_ROWS {
            self.scroll = self.highlight + 1 - MAX_VISIBLE_ROWS;
        }
        self.scroll = self.scroll.min(len.saturating_sub(MAX_VISIBLE_ROWS));
    }
}

fn wrap(index: usize, delta: isize, len: usize) -> usize {
    (index as isize + delta).rem_euclid(len as isize) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(count: usize) -> Vec<String> {
        (0..count).map(|n| format!("option {n}")).collect()
    }

    #[test]
    fn step_changes_the_value_directly_and_wraps() {
        let options = options(3);
        let mut state = SelectState::new(Some("option 1".to_string()));
        state.step(&options, 1);
        assert_eq!(state.value(), Some("option 2"));
        state.step(&options, 1);
        assert_eq!(state.value(), Some("option 0"));
        state.step(&options, -1);
        assert_eq!(state.value(), Some("option 2"));
        assert!(!state.is_open());
    }

    #[test]
    fn step_from_no_value_lands_on_either_end() {
        let options = options(3);
        let mut down = SelectState::default();
        down.step(&options, 1);
        assert_eq!(down.value(), Some("option 0"));
        let mut up = SelectState::default();
        up.step(&options, -1);
        assert_eq!(up.value(), Some("option 2"));
    }

    #[test]
    fn step_over_a_value_missing_from_the_options_starts_over() {
        let options = options(3);
        let mut state = SelectState::new(Some("gone".to_string()));
        state.step(&options, 1);
        assert_eq!(state.value(), Some("option 0"));
    }

    #[test]
    fn open_highlights_the_current_value() {
        let options = options(5);
        let mut state = SelectState::new(Some("option 3".to_string()));
        state.open(&options);
        assert!(state.is_open());
        assert_eq!(state.highlight(), 3);
    }

    #[test]
    fn highlight_wraps_and_commit_takes_it() {
        let options = options(3);
        let mut state = SelectState::new(Some("option 0".to_string()));
        state.open(&options);
        state.move_highlight(&options, -1);
        assert_eq!(state.highlight(), 2);
        state.commit(&options);
        assert_eq!(state.value(), Some("option 2"));
        assert!(!state.is_open());
    }

    #[test]
    fn cancel_closes_without_changing_the_value() {
        let options = options(3);
        let mut state = SelectState::new(Some("option 0".to_string()));
        state.open(&options);
        state.move_highlight(&options, 1);
        state.cancel();
        assert!(!state.is_open());
        assert_eq!(state.value(), Some("option 0"));
    }

    #[test]
    fn commit_on_a_closed_list_changes_nothing() {
        let options = options(3);
        let mut state = SelectState::new(Some("option 1".to_string()));
        state.commit(&options);
        assert_eq!(state.value(), Some("option 1"));
    }

    #[test]
    fn choose_sets_the_value_and_closes() {
        let options = options(4);
        let mut state = SelectState::default();
        state.open(&options);
        state.choose(&options, 2);
        assert_eq!(state.value(), Some("option 2"));
        assert!(!state.is_open());
    }

    #[test]
    fn the_window_is_capped_and_follows_the_highlight_down_and_up() {
        let options = options(10);
        let mut state = SelectState::new(Some("option 0".to_string()));
        state.open(&options);
        assert_eq!(state.visible_range(10), 0..MAX_VISIBLE_ROWS);

        for _ in 0..7 {
            state.move_highlight(&options, 1);
        }
        assert_eq!(state.highlight(), 7);
        assert_eq!(state.visible_range(10), 2..8);

        for _ in 0..6 {
            state.move_highlight(&options, -1);
        }
        assert_eq!(state.highlight(), 1);
        assert_eq!(state.visible_range(10), 1..7);
    }

    #[test]
    fn opening_on_a_late_value_scrolls_it_into_the_window() {
        let options = options(10);
        let mut state = SelectState::new(Some("option 9".to_string()));
        state.open(&options);
        assert_eq!(state.visible_range(10), 4..10);
    }

    #[test]
    fn a_short_list_shows_every_row() {
        let state = SelectState::default();
        assert_eq!(state.visible_range(3), 0..3);
        assert_eq!(state.visible_range(0), 0..0);
    }

    #[test]
    fn an_empty_option_list_is_inert() {
        let mut state = SelectState::default();
        state.step(&[], 1);
        state.open(&[]);
        state.move_highlight(&[], 1);
        state.commit(&[]);
        assert_eq!(state.value(), None);
        assert!(!state.is_open());
    }
}
