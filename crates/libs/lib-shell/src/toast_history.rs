//! The session Toast history Dialog's Presenter (#1043): the Toasts the Shell's Presenter holds,
//! newest first, as rows of text. It is read-only: `Esc` and the Close button close it, viewing
//! dismisses nothing, and every key is left to the Shell. The card scrolls by pointer, in gpui.

use chrono::{DateTime, Local};
use lib_locale::format::format_time;
use lib_toast::{History, ToastKind};
use lib_viper::dialog::DialogRouter;

/// One history row as text, so the ordering and merging are testable without a window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToastHistoryRow {
    pub kind: ToastKind,
    /// The local time the Toast was last raised.
    pub time: String,
    pub text: String,
    /// `×N` once the Toast merged duplicates.
    pub badge: Option<String>,
}

/// What the Toast history card shows: the rows, newest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToastHistoryViewModel {
    pub rows: Vec<ToastHistoryRow>,
}

/// The Toast history Dialog's Presenter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToastHistoryPresenter {
    view_model: ToastHistoryViewModel,
}

impl ToastHistoryPresenter {
    /// Opens over the Toasts raised so far.
    pub fn open(history: &History<DateTime<Local>>) -> Self {
        Self {
            view_model: ToastHistoryViewModel {
                rows: rows(history),
            },
        }
    }

    /// Reads the history again after a Toast was raised behind the card. The Toast stack is hidden
    /// while the card is up, so the card is the only place a new Toast shows.
    pub fn refresh(&mut self, history: &History<DateTime<Local>>) {
        self.view_model.rows = rows(history);
    }

    pub fn view_model(&self) -> &ToastHistoryViewModel {
        &self.view_model
    }

    /// The Close button.
    pub fn confirm(&mut self, router: &mut impl DialogRouter<()>) {
        router.close(Some(()));
    }

    /// `Esc`, the Shell's `back`.
    pub fn cancel(&mut self, router: &mut impl DialogRouter<()>) {
        router.close(None);
    }
}

/// The history's rows, newest first.
fn rows(history: &History<DateTime<Local>>) -> Vec<ToastHistoryRow> {
    history
        .iter()
        .map(|entry| ToastHistoryRow {
            kind: entry.kind(),
            time: format_time(entry.last_raised().time()),
            text: entry.text().to_string(),
            badge: (entry.count() > 1).then(|| format!("×{}", entry.count())),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use lib_toast::{Notice, Notify, Toasts};

    use super::*;

    #[derive(Default)]
    struct Closing {
        closed: Vec<Option<()>>,
    }

    impl Notify for Closing {
        fn notify(&mut self, _notice: Notice) {}
    }

    impl DialogRouter<()> for Closing {
        fn close(&mut self, outcome: Option<()>) {
            self.closed.push(outcome);
        }
    }

    fn at(second: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(2026, 9, 28, 15, 4, second)
            .single()
            .expect("test time is unambiguous")
    }

    fn open(toasts: &Toasts<DateTime<Local>>) -> ToastHistoryPresenter {
        ToastHistoryPresenter::open(toasts.history())
    }

    #[test]
    fn an_empty_history_opens_with_no_rows() {
        let toasts = Toasts::default();

        assert!(open(&toasts).view_model().rows.is_empty());
    }

    #[test]
    fn a_toast_raised_behind_the_card_shows_on_refresh() {
        let mut toasts = Toasts::default();
        toasts.raise(ToastKind::Info, "before", at(1));
        let mut history = open(&toasts);
        toasts.raise(ToastKind::Error, "behind", at(2));

        history.refresh(toasts.history());

        let texts: Vec<_> = history
            .view_model()
            .rows
            .iter()
            .map(|row| row.text.as_str())
            .collect();
        assert_eq!(texts, ["behind", "before"]);
    }

    #[test]
    fn rows_list_newest_first() {
        let mut toasts = Toasts::default();
        toasts.raise(ToastKind::Info, "older", at(1));
        toasts.raise(ToastKind::Error, "newer", at(2));

        let texts: Vec<_> = open(&toasts)
            .view_model()
            .rows
            .iter()
            .map(|row| (row.kind, row.text.clone()))
            .collect();

        assert_eq!(
            texts,
            [
                (ToastKind::Error, "newer".to_string()),
                (ToastKind::Info, "older".to_string())
            ]
        );
    }

    #[test]
    fn a_merged_duplicate_is_one_row_with_its_count_and_last_time() {
        let mut toasts = Toasts::default();
        toasts.raise(ToastKind::Success, "Deleted tag Food", at(1));
        toasts.raise(ToastKind::Success, "Deleted tag Food", at(2));

        let presenter = open(&toasts);
        let rows = &presenter.view_model().rows;

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].badge.as_deref(), Some("×2"));
        assert_eq!(rows[0].time, format_time(at(2).time()));
    }

    #[test]
    fn the_close_button_closes_it_with_an_outcome_and_esc_without() {
        let mut presenter = open(&Toasts::default());
        let mut router = Closing::default();

        presenter.confirm(&mut router);
        presenter.cancel(&mut router);

        assert_eq!(router.closed, [Some(()), None]);
    }
}
