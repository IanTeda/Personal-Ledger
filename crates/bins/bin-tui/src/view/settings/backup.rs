//! Settings › Backup (TUI-036): a LOCAL block, the store and its last snapshot, then a GIT BACKUP
//! block. Git Backup commits a text dump of the Ledger plus its Documents through LFS, only to a
//! private repository (ADR-0033), so its block has a contents row, and each commit message starts
//! with a fixed `yyyy-mm-dd hh:mm ·` prefix, drawn as a dim chip. `e` opens Set up Git backup
//! (`popup::settings::git_backup`), which the Settings View draws over the page and which takes
//! every key while open.
//!
//! A wireframe: there is no backup or Git Backup writer yet, so the facts are fixed values, `b`,
//! `X`, `P` and the overlay's Save flash the Shell's "not yet built", and the overlay tests on a
//! stub.

use chrono::{NaiveDate, NaiveTime};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use lib_locale::format::{format_time_minutes, upper};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::{moment, render_fact, render_keys};
use crate::colours::Colours;
use crate::msg;
use crate::popup::settings::git_backup::GitBackupPopup;
use crate::popup::settings::overlay::Outcome;
use crate::view::Action;

/// wireframe: the Ledger's size on disk, which is also the first push's.
pub(super) const SIZE: &str = "2.84 mb";

/// wireframe: the store and its last snapshot.
const STORE: &str = "~/.ledger/personal";
const LAST_SNAPSHOT: Option<NaiveDate> = NaiveDate::from_ymd_opt(2026, 9, 12);

/// wireframe: the Git Backup's Configuration and what it last saw.
const REMOTE: &str = "git@github.com:teda/ledger-backup.git";
const BRANCH: &str = "main";
const SSH_KEY: &str = "~/.ssh/id_ed25519";
const DOCUMENTS: i64 = 412;
const CHECKED: Option<NaiveTime> = NaiveTime::from_hms_opt(14, 0, 0);
/// The commit-message prefix is the push's time in a fixed form, not the Locale's (ADR-0033).
const LAST_PUSH: &str = "2026-09-14 14:00 ·";

#[derive(Default)]
pub(super) struct BackupPage {
    /// `Some` while Set up Git backup is open.
    overlay: Option<GitBackupPopup>,
}

impl BackupPage {
    pub(super) fn overlay_open(&self) -> bool {
        self.overlay.is_some()
    }

    /// Closes the overlay, discarding what was typed; `false` when it wasn't open.
    pub(super) fn close_overlay(&mut self) -> bool {
        self.overlay.take().is_some()
    }

    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if let Some(overlay) = &mut self.overlay {
            return Some(match overlay.handle_key(key) {
                Outcome::Stay => Action::NoOp,
                Outcome::Cancel => {
                    self.overlay = None;
                    Action::NoOp
                }
                Outcome::Save => {
                    self.overlay = None;
                    Action::FlashNotYetBuilt("git-backup save")
                }
            });
        }
        if !matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT) {
            return None;
        }
        match key.code {
            KeyCode::Char('b') => Some(Action::FlashNotYetBuilt("backup")),
            KeyCode::Char('X') => Some(Action::FlashNotYetBuilt("export csv")),
            KeyCode::Char('P') => Some(Action::FlashNotYetBuilt("git-backup push")),
            KeyCode::Char('e') => {
                self.overlay = Some(GitBackupPopup::new(REMOTE, BRANCH, SIZE));
                Some(Action::NoOp)
            }
            _ => None,
        }
    }

    pub(super) fn hints() -> Vec<(String, String)> {
        vec![
            ("b".into(), msg::tui_settings_hint_back_up()),
            ("X".into(), msg::tui_settings_hint_export_csv()),
            ("e".into(), msg::tui_settings_hint_edit_git_backup()),
            ("P".into(), msg::tui_settings_hint_push()),
        ]
    }

    pub(super) fn render(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        let rows = Layout::vertical([Constraint::Length(1); 16]).split(area);
        render_block_heading(frame, rows[0], &msg::tui_settings_backup_local(), None, c);
        render_fact(
            frame,
            rows[1],
            &msg::tui_settings_backup_store(),
            Line::from(STORE),
            c,
        );
        render_fact(
            frame,
            rows[2],
            &msg::tui_settings_backup_last_snapshot(),
            Line::from(moment(LAST_SNAPSHOT, 23, 10)),
            c,
        );
        render_keys(
            frame,
            rows[3],
            &[
                ("b", msg::tui_settings_backup_back_up_now()),
                ("X", msg::tui_settings_backup_export()),
            ],
        );

        render_block_heading(
            frame,
            rows[5],
            &msg::tui_settings_backup_git(),
            Some(("e", msg::tui_settings_hint_edit())),
            c,
        );
        let checked = CHECKED.map(format_time_minutes).unwrap_or_default();
        let git: [(String, Line<'_>); 7] = [
            (msg::tui_settings_git_remote(), Line::from(REMOTE)),
            (msg::tui_settings_git_branch(), Line::from(BRANCH)),
            (
                msg::tui_settings_backup_visibility(),
                Line::from(vec![
                    Span::styled("● ", c.accent()),
                    Span::raw(msg::tui_settings_backup_private(&checked)),
                ]),
            ),
            (
                msg::tui_settings_git_authentication(),
                Line::from(format!("{} · {SSH_KEY}", msg::tui_settings_git_auth_ssh())),
            ),
            (
                msg::tui_settings_backup_contents(),
                Line::from(msg::tui_settings_backup_contents_value(DOCUMENTS)),
            ),
            (
                msg::tui_settings_backup_schedule(),
                Line::from(msg::tui_settings_backup_schedule_value()),
            ),
            (
                msg::tui_settings_git_commit_message(),
                Line::from(vec![
                    Span::styled(LAST_PUSH, c.muted()),
                    Span::raw(" "),
                    Span::raw(msg::tui_settings_git_commit_default()),
                ]),
            ),
        ];
        for (index, (label, value)) in git.into_iter().enumerate() {
            render_fact(frame, rows[6 + index], &label, value, c);
        }
        render_keys(
            frame,
            rows[13],
            &[("P", msg::tui_settings_backup_push_now())],
        );
        frame.render_widget(
            Paragraph::new(Span::styled(
                msg::tui_settings_git_schedule_note_after_changes(),
                c.muted(),
            )),
            rows[15],
        );
    }

    pub(super) fn render_overlay(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        if let Some(overlay) = &self.overlay {
            overlay.render(frame, area, c);
        }
    }
}

/// A block's heading, upper case and bold, with its own key at the right.
fn render_block_heading(
    frame: &mut Frame<'_>,
    area: Rect,
    label: &str,
    key: Option<(&str, String)>,
    c: &Colours,
) {
    let mut spans = vec![Span::styled(
        upper(label),
        Style::default().add_modifier(Modifier::BOLD),
    )];
    if let Some((key, text)) = key {
        spans.push(Span::raw("   "));
        spans.push(Span::styled(
            key,
            Style::default().add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(format!(" {text}"), c.muted()));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::View;
    use crate::view::settings::tests::render_at;
    use crate::view::settings::{SettingsPage, SettingsView};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn backup_page() -> SettingsView {
        let mut view = SettingsView::new();
        view.update(&Action::OpenSettingsPage(SettingsPage::Backup));
        view
    }

    #[test]
    fn at_96_by_30_the_page_draws_the_local_and_git_backup_blocks() {
        crate::locale::init_for_tests();
        let text = render_at(&backup_page(), 96, 30);
        for needle in [
            "Settings › Backup",
            "this device · local files · 2.84 mb",
            "LOCAL",
            "store location",
            "~/.ledger/personal",
            "last snapshot",
            "sep 12, 2026 · 11:10",
            "b back up now",
            "X export ledger (csv)",
            "GIT BACKUP",
            "e edit",
            "git@github.com:teda/ledger-backup.git",
            "● private · checked 2:00",
            "SSH key · ~/.ssh/id_ed25519",
            "ledger text dump · 412 documents via LFS",
            "after changes · at most hourly",
            "2026-09-14 14:00 · ledger backup",
            "P push now",
            "After changes waits for 5 minutes of quiet",
        ] {
            assert!(text.contains(needle), "{needle} missing:\n{text}");
        }
    }

    #[test]
    fn b_x_and_p_flash_not_yet_built() {
        let mut view = backup_page();
        for (code, command) in [
            (KeyCode::Char('b'), "backup"),
            (KeyCode::Char('X'), "export csv"),
            (KeyCode::Char('P'), "git-backup push"),
        ] {
            assert_eq!(
                view.handle_key(KeyEvent::new(code, KeyModifiers::SHIFT)),
                Some(Action::FlashNotYetBuilt(command))
            );
        }
    }

    #[test]
    fn e_opens_set_up_git_backup_whose_token_renders_masked_and_esc_closes_it_first() {
        crate::locale::init_for_tests();
        let mut view = backup_page();
        view.handle_key(key(KeyCode::Char('e')));
        assert!(view.popup_open() && view.editing());
        // Authentication: Access token, then type one into the field under it.
        view.handle_key(key(KeyCode::Tab));
        view.handle_key(key(KeyCode::Tab));
        view.handle_key(key(KeyCode::Right));
        view.handle_key(key(KeyCode::Tab));
        for ch in "ghp_secret".chars() {
            view.handle_key(key(KeyCode::Char(ch)));
        }
        let text = render_at(&view, 96, 30);
        for needle in [
            "set up git backup",
            "Use a private repository.",
            "••••••••••",
            "yyyy-mm-dd hh:mm · ledger backup",
            "Saving makes the first push (2.84 mb).",
            "untested",
        ] {
            assert!(text.contains(needle), "{needle} missing:\n{text}");
        }
        assert!(!text.contains("ghp_secret"), "the token shows:\n{text}");

        view.handle_key(key(KeyCode::Esc));
        assert!(!view.popup_open());
        assert!(view.back(), "the page steps back to the index");
    }
}
