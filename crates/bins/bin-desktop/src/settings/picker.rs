//! The Settings page picker (DESKTOP-120): at Medium, the caret after the page name, or `p`, drops
//! the page list in index order under the heading. `j`/`k` move through it and `enter` opens the
//! chosen page directly, without passing through the index.

use lib_list_cursor::Step;

use crate::settings::SettingsPage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PagePicker {
    /// The highlighted page, the one `enter` opens.
    chosen: SettingsPage,
}

impl PagePicker {
    /// Opens with the page on show highlighted.
    pub fn new(current: SettingsPage) -> Self {
        Self { chosen: current }
    }

    pub fn chosen(&self) -> SettingsPage {
        self.chosen
    }

    /// Moves the highlight through the pages, stopping at either end.
    pub fn step(&mut self, step: Step) {
        if let Some(page) = lib_list_cursor::step_id(&SettingsPage::ALL, Some(self.chosen), step) {
            self.chosen = page;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_on_the_page_on_show() {
        let picker = PagePicker::new(SettingsPage::Display);
        assert_eq!(picker.chosen(), SettingsPage::Display);
    }

    #[test]
    fn step_moves_the_highlight_and_stops_at_either_end() {
        let mut picker = PagePicker::new(SettingsPage::ALL[0]);
        picker.step(Step::Prev);
        assert_eq!(picker.chosen(), SettingsPage::ALL[0], "stops at the start");
        picker.step(Step::Next);
        assert_eq!(picker.chosen(), SettingsPage::ALL[1]);

        let last = SettingsPage::ALL[SettingsPage::ALL.len() - 1];
        let mut picker = PagePicker::new(last);
        picker.step(Step::Next);
        assert_eq!(picker.chosen(), last, "stops at the end");
    }
}
