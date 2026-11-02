//! The Import files Dialog: one typed path per line. Each path that can become a Document lands
//! in the Inbox; when none can, the Dialog stays open and says why each failed, until the paths
//! change.

use lib_toast::{Notice, ToastKind};
use lib_viper::keymap::{BACK, Keymap, SUBMIT};
use lib_viper::view::{DialogFrame, Hint};

use super::{DialogPorts, count};
use crate::interactor::ImportDocuments;
use crate::protocols::{DocumentsOutcome, DocumentsRouter};
use crate::{ImportOutcome, Source, msg};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportEvent {
    /// The paths' whole new text, as the View's input holds it.
    Paths(String),
    /// `Enter`: a new line, unless the last is blank already.
    Newline,
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportViewModel {
    pub frame: DialogFrame,
    pub label: String,
    /// The typed text, a path a line.
    pub paths: String,
    pub placeholder: String,
    /// Why each path of the last attempt, which imported none, failed.
    pub notes: Vec<String>,
    pub note: String,
}

pub struct ImportPresenter {
    ports: DialogPorts,
    paths: String,
    notes: Vec<String>,
}

impl ImportPresenter {
    pub fn new(ports: DialogPorts) -> Self {
        Self {
            ports,
            paths: String::new(),
            notes: Vec::new(),
        }
    }

    pub fn handle(&mut self, event: ImportEvent, router: &mut dyn DocumentsRouter) {
        match event {
            ImportEvent::Paths(text) => {
                self.paths = text;
                self.notes.clear();
            }
            ImportEvent::Newline => {
                if !self.paths.is_empty() && !self.paths.ends_with('\n') {
                    self.paths.push('\n');
                }
            }
            ImportEvent::Confirm => self.confirm(router),
            ImportEvent::Cancel => router.close_dialog(None),
        }
    }

    pub fn view_model(&self) -> ImportViewModel {
        ImportViewModel {
            frame: DialogFrame {
                title: msg::documents_dialog_import_title(),
                submit: msg::documents_dialog_import_submit(),
                can_confirm: !self.lines().is_empty(),
                destructive: false,
                hints: Vec::new(),
            },
            label: msg::documents_dialog_field_paths(),
            paths: self.paths.clone(),
            placeholder: msg::documents_dialog_field_paths_placeholder(),
            notes: self.notes.clone(),
            note: msg::documents_dialog_import_note(),
        }
    }

    /// The status line's legend: `Enter` types a new line, so `submit` imports.
    pub fn legend<E: Clone>(&self, keymap: &Keymap<E>) -> Vec<Hint> {
        let key = |command| keymap.chord(command).unwrap_or_default().to_string();
        vec![
            Hint::new(key(BACK), msg::documents_dialog_hint_cancel()),
            Hint::new(key(SUBMIT), msg::documents_dialog_hint_confirm()),
        ]
    }

    /// The non-blank typed lines.
    fn lines(&self) -> Vec<String> {
        self.paths
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect()
    }

    fn confirm(&mut self, router: &mut dyn DocumentsRouter) {
        let typed = self.lines();
        if typed.is_empty() {
            return;
        }
        let outcomes = ImportDocuments {
            typed,
            today: self.ports.today,
            source: Source::Import,
        }
        .run(&*self.ports.files, &mut *self.ports.documents.borrow_mut());
        let imported = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, ImportOutcome::Imported(_)))
            .count();
        let notes = import_notes(&outcomes);
        if imported == 0 {
            self.notes = notes;
            return;
        }
        let toast = if notes.is_empty() {
            msg::documents_toast_imported(count(imported))
        } else {
            msg::documents_toast_import_partial(&imported.to_string(), &notes.len().to_string())
        };
        router.notify(Notice::Toast(ToastKind::Success, toast));
        router.close_dialog(Some(DocumentsOutcome::Imported));
    }
}

/// Why each line of an import that didn't land didn't.
fn import_notes(outcomes: &[ImportOutcome]) -> Vec<String> {
    outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            ImportOutcome::Imported(_) => None,
            ImportOutcome::NotFound(path) => Some(msg::documents_dialog_error_path_missing(path)),
            ImportOutcome::Unsupported(path) => Some(format!(
                "{path}: {}",
                msg::documents_dialog_error_unsupported()
            )),
            ImportOutcome::Already(title) => Some(msg::documents_dialog_error_already(title)),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::test_support::dialog_ports;
    use super::*;
    use crate::fake::{DocumentsCall, InMemoryFiles, RecordingDocumentsRouter};

    fn import() -> (ImportPresenter, DialogPorts, RecordingDocumentsRouter) {
        let ports = dialog_ports(
            InMemoryFiles::default()
                .with("/tmp/a.pdf", 10)
                .with("/tmp/b.png", 10),
        );
        (
            ImportPresenter::new(ports.clone()),
            ports,
            RecordingDocumentsRouter::default(),
        )
    }

    #[test]
    fn enter_types_a_new_line_and_only_typed_paths_can_confirm() {
        let (mut presenter, _, mut router) = import();
        assert!(!presenter.view_model().frame.can_confirm);
        presenter.handle(ImportEvent::Newline, &mut router);
        assert_eq!(presenter.view_model().paths, "", "no line to end yet");

        presenter.handle(ImportEvent::Paths("/tmp/a.pdf".to_string()), &mut router);
        presenter.handle(ImportEvent::Newline, &mut router);
        presenter.handle(ImportEvent::Newline, &mut router);
        assert_eq!(presenter.view_model().paths, "/tmp/a.pdf\n");
        assert!(presenter.view_model().frame.can_confirm);
    }

    #[test]
    fn an_import_that_takes_nothing_stays_open_and_says_why_each_failed() {
        let (mut presenter, ports, mut router) = import();
        let before = ports.documents.borrow().documents();
        presenter.handle(
            ImportEvent::Paths("/tmp/missing.pdf\n/tmp/notes.txt".to_string()),
            &mut router,
        );

        presenter.handle(ImportEvent::Confirm, &mut router);

        assert!(router.take().is_empty(), "the Dialog stays open");
        assert_eq!(
            presenter.view_model().notes,
            [
                "There is no file at /tmp/missing.pdf.",
                "/tmp/notes.txt: Only PDF, JPG and PNG files can be Documents.",
            ]
        );
        assert_eq!(ports.documents.borrow().documents(), before);

        presenter.handle(ImportEvent::Paths("/tmp/a.pdf".to_string()), &mut router);
        assert!(
            presenter.view_model().notes.is_empty(),
            "a change clears them"
        );
    }

    #[test]
    fn an_import_lands_in_the_inbox_and_counts_what_it_left() {
        let (mut presenter, ports, mut router) = import();
        let before = crate::inbox_count(&ports.documents.borrow().documents());
        presenter.handle(
            ImportEvent::Paths("/tmp/a.pdf\n/tmp/b.png\n/tmp/missing.pdf".to_string()),
            &mut router,
        );

        presenter.handle(ImportEvent::Confirm, &mut router);

        assert_eq!(
            crate::inbox_count(&ports.documents.borrow().documents()),
            before + 2
        );
        assert_eq!(
            router.take(),
            [
                DocumentsCall::Notice(Notice::Toast(
                    ToastKind::Success,
                    "2 imported, 1 not imported.".to_string()
                )),
                DocumentsCall::CloseDialog(Some(DocumentsOutcome::Imported)),
            ]
        );
    }
}
