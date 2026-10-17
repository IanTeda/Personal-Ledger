//! The Settings page picker (DESKTOP-120): at Medium, the caret after the page name, or `p`, drops
//! the page list in index order under the heading. Typing filters it, `enter` opens the chosen
//! page directly, without passing through the index.

use lib_list_cursor::Step;

use crate::{form::field::TextField, settings::SettingsPage};

#[derive(Debug, Clone, PartialEq)]
pub struct PagePicker {
    query: TextField,
    /// The highlighted page; it stays put while it still matches the filter.
    selected: Option<SettingsPage>,
}

impl PagePicker {
    /// Opens with every page listed and the page on show highlighted.
    pub fn new(current: SettingsPage) -> Self {
        Self {
            query: TextField::new(""),
            selected: Some(current),
        }
    }

    pub fn query(&self) -> &str {
        self.query.text()
    }

    /// The pages whose name contains the filter, in index order.
    pub fn matches(&self) -> Vec<SettingsPage> {
        let query = self.query.text().to_lowercase();
        SettingsPage::ALL
            .into_iter()
            .filter(|page| page.label().to_lowercase().contains(&query))
            .collect()
    }

    /// The page `enter` opens: the highlighted one, or the first match once the filter has hidden it.
    pub fn chosen(&self) -> Option<SettingsPage> {
        lib_list_cursor::resolve(self.matches(), self.selected)
    }

    pub fn push(&mut self, ch: char) {
        self.query.push(ch);
    }

    pub fn backspace(&mut self) {
        self.query.backspace();
    }

    /// Moves the highlight through the matches.
    pub fn step(&mut self, step: Step) {
        self.selected = lib_list_cursor::step_id(&self.matches(), self.chosen(), step);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_on_the_page_on_show_with_every_page_listed() {
        let picker = PagePicker::new(SettingsPage::Display);
        assert_eq!(picker.matches(), SettingsPage::ALL.to_vec());
        assert_eq!(picker.chosen(), Some(SettingsPage::Display));
    }

    #[test]
    fn typing_filters_and_falls_back_to_the_first_match() {
        let mut picker = PagePicker::new(SettingsPage::Display);
        for ch in "ta".chars() {
            picker.push(ch);
        }
        assert_eq!(
            picker.matches(),
            vec![SettingsPage::Tags, SettingsPage::DataBackup]
        );
        assert_eq!(picker.chosen(), Some(SettingsPage::Tags));

        picker.step(Step::Next);
        assert_eq!(picker.chosen(), Some(SettingsPage::DataBackup));
        picker.step(Step::Next);
        assert_eq!(
            picker.chosen(),
            Some(SettingsPage::DataBackup),
            "stops at the end"
        );
    }

    #[test]
    fn a_filter_nothing_matches_chooses_nothing() {
        let mut picker = PagePicker::new(SettingsPage::General);
        picker.push('#');
        assert_eq!(picker.chosen(), None);
        picker.backspace();
        assert_eq!(picker.chosen(), Some(SettingsPage::General));
    }
}
