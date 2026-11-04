//! The Delete document confirm, in the Library and in the Inbox. It names the Document, how many
//! Links go with it and where its file stays, and warns the delete is permanent: there is no Undo,
//! since `u` reverses filing only. A plain confirm, with no name to type back, as the Document's
//! file and every record it linked survive the delete.
//!
//! The Dialog raises its own Toast on landing, so the page only learns which Document went. A
//! Document gone since the Dialog opened is refused with its Message and the Dialog stays open.

use lib_core::DocumentId;
use lib_locale::Segment;
use lib_toast::{Notice, ToastKind};
use lib_viper::dialog::{DialogInput, DialogRouter};
use lib_viper::keymap::{BACK, Keymap, SELECT};
use lib_viper::view::{DialogFrame, Hint, count};

use super::DialogPorts;
use crate::interactor::DeleteDocument;
use crate::protocols::{DocumentsOutcome, SharedRepository};
use crate::{get, msg};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteDocumentEvent {
    /// A Dialog keymap command or a typed key: `select` and `submit` confirm, and every other
    /// key does nothing.
    Input(DialogInput),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteDocumentViewModel {
    pub frame: DialogFrame,
    /// What goes and what stays.
    pub body: String,
    /// The permanence warning, its emphasis tagged `strong`.
    pub warning: Vec<Segment>,
    /// Why the last confirm was refused.
    pub refusal: Option<String>,
}

pub struct DeleteDocumentPresenter {
    documents: SharedRepository,
    id: DocumentId,
    title: String,
    path: String,
    links: usize,
    refusal: Option<String>,
}

impl DeleteDocumentPresenter {
    /// The Dialog for deleting Document `id`, or `None` when it is gone.
    pub fn new(ports: DialogPorts, id: DocumentId) -> Option<Self> {
        let documents = ports.documents;
        let (title, path, links) = {
            let held = documents.borrow().documents();
            let document = get(&held, id)?;
            (
                document.title.clone(),
                document.path.to_string_lossy().into_owned(),
                document.links.len(),
            )
        };
        Some(Self {
            documents,
            id,
            title,
            path,
            links,
            refusal: None,
        })
    }

    pub fn handle(
        &mut self,
        event: DeleteDocumentEvent,
        router: &mut dyn DialogRouter<DocumentsOutcome>,
    ) {
        match event {
            DeleteDocumentEvent::Input(DialogInput::Select | DialogInput::Submit)
            | DeleteDocumentEvent::Confirm => self.confirm(router),
            DeleteDocumentEvent::Input(_) => {}
            DeleteDocumentEvent::Cancel => router.close(None),
        }
    }

    pub fn view_model(&self) -> DeleteDocumentViewModel {
        DeleteDocumentViewModel {
            frame: DialogFrame {
                title: msg::documents_dialog_delete_title(&self.title),
                submit: msg::documents_dialog_delete_submit(),
                can_confirm: true,
                destructive: true,
                hints: Vec::new(),
            },
            body: msg::documents_dialog_delete_body(count(self.links), &self.path),
            warning: msg::documents_dialog_delete_warning(),
            refusal: self.refusal.clone(),
        }
    }

    /// The status line's legend: cancel and confirm.
    pub fn legend<E: Clone>(&self, keymap: &Keymap<E>) -> Vec<Hint> {
        let key = |command| keymap.chord(command).unwrap_or_default().to_string();
        vec![
            Hint::new(key(BACK), msg::documents_dialog_hint_cancel()),
            Hint::new(key(SELECT), msg::documents_dialog_hint_confirm()),
        ]
    }

    fn confirm(&mut self, router: &mut dyn DialogRouter<DocumentsOutcome>) {
        let deleted = DeleteDocument(self.id).run(&mut *self.documents.borrow_mut());
        match deleted {
            Ok(document) => {
                router.notify(Notice::Toast(
                    ToastKind::Success,
                    msg::documents_dialog_toast_deleted(&document.title),
                ));
                router.close(Some(DocumentsOutcome::Deleted(self.id)));
            }
            Err(_) => self.refusal = Some(msg::documents_dialog_delete_refused()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{dialog_ports, locale};
    use super::*;
    use crate::fake::{DocumentsCall, RecordingDocumentsRouter};

    fn open() -> (DeleteDocumentPresenter, DialogPorts, DocumentId, DocumentId) {
        locale();
        let ports = dialog_ports(crate::fake::InMemoryFiles::default());
        let held = ports.documents.borrow().documents();
        let filed = held.iter().find(|d| d.is_filed() && !d.links.is_empty());
        let filed = filed.expect("a linked Filed Document").id;
        let unfiled = held.iter().find(|d| d.is_unfiled()).expect("Unfiled").id;
        let presenter =
            DeleteDocumentPresenter::new(ports.clone(), filed).expect("a stub Document");
        (presenter, ports, filed, unfiled)
    }

    fn plain(segments: &[Segment]) -> String {
        segments.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn it_names_the_document_its_links_and_where_the_file_stays() {
        let (presenter, ports, filed, _) = open();
        let held = ports.documents.borrow().documents();
        let document = get(&held, filed).expect("there");
        let view = presenter.view_model();

        assert_eq!(
            view.frame.title,
            format!("Delete document \u{2014} {}", document.title)
        );
        assert_eq!(view.frame.submit, "Delete document");
        assert!(view.frame.destructive && view.frame.can_confirm);
        // The fixture's Filed Document has one Link, so this pins the singular, not "1 links".
        assert_eq!(document.links.len(), 1);
        assert_eq!(
            view.body,
            format!(
                "Its 1 link goes with it; the record it names stays. The file stays on disk at {}.",
                document.path.display()
            )
        );
        assert_eq!(plain(&view.warning), "Deleting it cannot be undone.");
        let strong: Vec<_> = view
            .warning
            .iter()
            .filter(|segment| segment.tag.as_deref() == Some("strong"))
            .collect();
        assert_eq!(strong.len(), 1);
    }

    #[test]
    fn the_links_clause_is_dropped_at_no_links_but_the_file_sentence_stays() {
        let (_, ports, _, unfiled) = open();
        let held = ports.documents.borrow().documents();
        let document = get(&held, unfiled).expect("there");
        assert!(document.links.is_empty());

        let view = DeleteDocumentPresenter::new(ports, unfiled)
            .expect("a Document")
            .view_model();

        assert_eq!(
            view.body,
            format!("The file stays on disk at {}.", document.path.display())
        );
    }

    #[test]
    fn confirm_deletes_raises_its_toast_and_hands_the_page_the_id() {
        let (mut presenter, ports, filed, _) = open();
        let title = get(&ports.documents.borrow().documents(), filed)
            .expect("there")
            .title
            .clone();
        let mut router = RecordingDocumentsRouter::default();

        presenter.handle(DeleteDocumentEvent::Confirm, &mut router);

        assert!(get(&ports.documents.borrow().documents(), filed).is_none());
        assert_eq!(
            router.take(),
            [
                DocumentsCall::Notice(Notice::Toast(
                    ToastKind::Success,
                    format!("Deleted \"{title}\". The file is still on disk.")
                )),
                DocumentsCall::CloseDialog(Some(DocumentsOutcome::Deleted(filed))),
            ]
        );
    }

    #[test]
    fn select_and_submit_confirm_and_every_other_key_is_swallowed() {
        let (mut presenter, ports, filed, _) = open();
        let before = ports.documents.borrow().documents().len();
        let mut router = RecordingDocumentsRouter::default();
        for input in [
            DialogInput::Char('x'),
            DialogInput::Backspace,
            DialogInput::NextField,
        ] {
            presenter.handle(DeleteDocumentEvent::Input(input), &mut router);
        }
        assert!(router.take().is_empty());
        assert_eq!(ports.documents.borrow().documents().len(), before);

        presenter.handle(DeleteDocumentEvent::Input(DialogInput::Select), &mut router);
        assert_eq!(ports.documents.borrow().documents().len(), before - 1);
        assert!(get(&ports.documents.borrow().documents(), filed).is_none());
    }

    #[test]
    fn cancel_closes_and_writes_nothing() {
        let (mut presenter, ports, ..) = open();
        let before = ports.documents.borrow().documents();
        let mut router = RecordingDocumentsRouter::default();

        presenter.handle(DeleteDocumentEvent::Cancel, &mut router);

        assert_eq!(router.take(), [DocumentsCall::CloseDialog(None)]);
        assert_eq!(ports.documents.borrow().documents(), before);
    }

    #[test]
    fn a_document_gone_since_the_dialog_opened_is_refused_and_the_dialog_stays() {
        let (mut presenter, ports, filed, _) = open();
        ports.documents.borrow_mut().delete_document(filed);
        let mut router = RecordingDocumentsRouter::default();

        presenter.handle(DeleteDocumentEvent::Confirm, &mut router);

        assert!(router.take().is_empty(), "no Toast, and it doesn't close");
        assert_eq!(
            presenter.view_model().refusal.as_deref(),
            Some("This document has since been deleted.")
        );
    }

    #[test]
    fn no_dialog_opens_on_a_document_that_is_gone() {
        let (_, ports, ..) = open();
        assert!(DeleteDocumentPresenter::new(ports, DocumentId::seed(9_999)).is_none());
    }
}
