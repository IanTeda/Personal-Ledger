//! The Remove Document type Dialog. A type with Filed files asks where they move, the destination
//! select pre-selected to Other, the Default, and is drawn as a destructive action; a type with
//! none is a plain confirm. Other itself never gets here: the page opens the notice instead.
//!
//! The destination names are read when the Dialog opens, as it is modal and nothing changes them
//! under it.

use lib_core::DocumentTypeId;
use lib_viper::view::{Choice, DialogFrame, SelectView};

use crate::interactor::RemoveDocumentType;
use crate::msg;
use crate::protocols::{DocumentTypesOutcome, DocumentTypesRouter, SharedRepository, TypeChange};
use crate::types::{RemoveError, get, position};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoveTypeEvent {
    /// A destination's name, as the select lists it.
    Destination(String),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveTypeViewModel {
    pub frame: DialogFrame,
    /// What the Dialog says is about to happen to the type and its files.
    pub message: String,
    /// Where the files move, `None` when there are none to move.
    pub destination: Option<SelectView<String>>,
    /// The note under the select.
    pub moved_note: Option<String>,
    /// Why the last confirm was refused.
    pub refusal: Option<String>,
    /// How many Filed files the type has.
    pub files: u32,
}

pub struct RemoveTypePresenter {
    ports: SharedRepository,
    id: DocumentTypeId,
    name: String,
    files: u32,
    /// Where the files can move: every other type, in order.
    destinations: Vec<String>,
    destination: Option<String>,
    error: Option<RemoveError>,
}

impl RemoveTypePresenter {
    /// The Dialog for removing type `id`, or `None` when it is gone.
    pub fn new(ports: SharedRepository, id: DocumentTypeId) -> Option<Self> {
        let types = ports.borrow().document_types();
        let row = get(&types, id)?;
        let destinations = types
            .iter()
            .filter(|other| other.id != id)
            .map(|other| other.name.clone())
            .collect();
        let destination = types
            .iter()
            .find(|other| other.is_default && other.id != id)
            .map(|other| other.name.clone());
        let (name, files) = (row.name.clone(), row.files);
        drop(types);
        Some(Self {
            ports,
            id,
            name,
            files,
            destinations,
            destination,
            error: None,
        })
    }

    pub fn handle(&mut self, event: RemoveTypeEvent, router: &mut dyn DocumentTypesRouter) {
        match event {
            RemoveTypeEvent::Destination(name) => {
                if self.destinations.contains(&name) {
                    self.destination = Some(name);
                    self.error = None;
                }
            }
            RemoveTypeEvent::Confirm => self.confirm(router),
            RemoveTypeEvent::Cancel => router.close_dialog(None),
        }
    }

    pub fn view_model(&self) -> RemoveTypeViewModel {
        let has_files = self.files > 0;
        let files = i64::from(self.files);
        RemoveTypeViewModel {
            frame: DialogFrame {
                title: msg::documents_types_dialog_remove_title(),
                submit: if has_files {
                    msg::documents_types_remove_confirm_move(files)
                } else {
                    msg::documents_types_remove_confirm()
                },
                can_confirm: self.can_confirm(),
                destructive: has_files,
                hints: Vec::new(),
            },
            message: if has_files {
                msg::documents_types_remove_with_files(&self.name, files)
            } else {
                msg::documents_types_remove_no_files(&self.name)
            },
            destination: has_files.then(|| SelectView {
                label: msg::documents_types_remove_destination(),
                options: self
                    .destinations
                    .iter()
                    .map(|name| Choice {
                        value: name.clone(),
                        label: name.clone(),
                    })
                    .collect(),
                chosen: self.destination.clone(),
                focused: true,
            }),
            moved_note: has_files.then(msg::documents_types_remove_moved_note),
            refusal: self.error.map(|_| msg::documents_types_remove_refused()),
            files: self.files,
        }
    }

    fn can_confirm(&self) -> bool {
        self.files == 0 || self.destination.is_some()
    }

    fn confirm(&mut self, router: &mut dyn DocumentTypesRouter) {
        if !self.can_confirm() {
            return;
        }
        let mut repository = self.ports.borrow_mut();
        let types = repository.document_types();
        let place = position(&types, self.id).unwrap_or(0);
        let removed = RemoveDocumentType {
            id: self.id,
            destination: self.destination.clone(),
        }
        .run(&mut *repository);
        let after = repository.document_types();
        drop(repository);
        match removed {
            Ok(()) => router.close_dialog(Some(DocumentTypesOutcome {
                change: TypeChange::Removed,
                // The type now sitting where the removed one did.
                select: after
                    .get(place.min(after.len().saturating_sub(1)))
                    .map(|row| row.id),
            })),
            Err(refusal) => self.error = Some(refusal),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::stub;
    use super::*;
    use crate::fake::{InMemoryDocuments, RecordingRouter, RouterCall};

    use std::cell::RefCell;
    use std::rc::Rc;

    fn open(id: u32) -> (RemoveTypePresenter, Rc<RefCell<InMemoryDocuments>>) {
        let (documents, shared) = stub();
        let presenter =
            RemoveTypePresenter::new(shared, DocumentTypeId::seed(id)).expect("a default type");
        (presenter, documents)
    }

    #[test]
    fn a_type_with_files_asks_where_they_go_with_other_pre_selected() {
        let (presenter, _) = open(3);
        let view = presenter.view_model();

        assert_eq!(view.frame.title, "Remove Document type");
        assert_eq!(view.frame.submit, "Move 31 files and remove");
        assert!(view.frame.destructive);
        assert!(view.frame.can_confirm);
        assert_eq!(view.message, "Tax has 31 Filed files. Move them to:");
        let destination = view.destination.expect("a destination select");
        assert_eq!(destination.chosen.as_deref(), Some("Other"));
        assert_eq!(destination.options.len(), 8);
        assert!(
            destination
                .options
                .iter()
                .all(|choice| choice.label != "Tax")
        );
        assert!(view.moved_note.is_some());
    }

    #[test]
    fn a_type_with_no_files_is_a_plain_confirm() {
        let (documents, shared) = stub();
        documents.borrow_mut().types[2].files = 0;
        let presenter =
            RemoveTypePresenter::new(shared, DocumentTypeId::seed(3)).expect("a default type");
        let view = presenter.view_model();

        assert_eq!(view.frame.submit, "Remove type");
        assert!(!view.frame.destructive);
        assert_eq!(view.destination, None);
        assert_eq!(view.moved_note, None);
        assert_eq!(view.message, "No Filed files use Tax, so nothing moves.");
    }

    #[test]
    fn confirming_moves_the_files_to_the_destination_and_selects_the_neighbour() {
        let (mut presenter, documents) = open(3);
        let mut router = RecordingRouter::default();

        presenter.handle(RemoveTypeEvent::Confirm, &mut router);

        let types = documents.borrow().types.clone();
        assert_eq!(types.len(), 8);
        assert!(types.iter().all(|row| row.id != DocumentTypeId::seed(3)));
        // Other had 2 files and takes Tax's 31.
        assert_eq!(types.last().map(|row| row.files), Some(33));
        assert_eq!(
            router.take(),
            [RouterCall::CloseDialog(Some(DocumentTypesOutcome {
                change: TypeChange::Removed,
                select: Some(types[2].id),
            }))]
        );
    }

    #[test]
    fn another_destination_takes_the_files() {
        let (mut presenter, documents) = open(3);
        let mut router = RecordingRouter::default();

        presenter.handle(RemoveTypeEvent::Destination("Receipts".into()), &mut router);
        presenter.handle(RemoveTypeEvent::Confirm, &mut router);

        let types = documents.borrow().types.clone();
        assert_eq!(types[0].files, 186 + 31);
        assert_eq!(types.last().map(|row| row.files), Some(2));
    }

    #[test]
    fn a_destination_that_is_not_listed_is_ignored() {
        let (mut presenter, _) = open(3);
        let mut router = RecordingRouter::default();

        presenter.handle(RemoveTypeEvent::Destination("Tax".into()), &mut router);
        presenter.handle(RemoveTypeEvent::Destination("Nowhere".into()), &mut router);

        assert_eq!(
            presenter
                .view_model()
                .destination
                .and_then(|select| select.chosen),
            Some("Other".to_string())
        );
    }

    #[test]
    fn removing_the_last_row_selects_the_one_before_it() {
        let (mut presenter, documents) = open(8);
        let mut router = RecordingRouter::default();
        // Eight is the last type that is not the Default.
        documents.borrow_mut().types.swap(7, 8);

        presenter.handle(RemoveTypeEvent::Confirm, &mut router);

        let types = documents.borrow().types.clone();
        assert_eq!(
            router.take(),
            [RouterCall::CloseDialog(Some(DocumentTypesOutcome {
                change: TypeChange::Removed,
                select: types.last().map(|row| row.id),
            }))]
        );
    }

    #[test]
    fn a_refusal_stays_open_with_its_text_and_writes_nothing() {
        let (documents, shared) = stub();
        let mut presenter =
            RemoveTypePresenter::new(shared, DocumentTypeId::seed(3)).expect("a default type");
        // The type is removed while the Dialog is open: the form is valid, the domain refuses.
        documents
            .borrow_mut()
            .types
            .retain(|row| row.id != DocumentTypeId::seed(3));
        let mut router = RecordingRouter::default();

        presenter.handle(RemoveTypeEvent::Confirm, &mut router);

        assert_eq!(router.take(), []);
        assert_eq!(
            presenter.view_model().refusal.as_deref(),
            Some("That type can't be removed now. Check its destination and try again.")
        );
        assert_eq!(documents.borrow().types.len(), 8);
    }

    #[test]
    fn cancel_closes_without_an_outcome_and_writes_nothing() {
        let (mut presenter, documents) = open(3);
        let mut router = RecordingRouter::default();

        presenter.handle(RemoveTypeEvent::Cancel, &mut router);

        assert_eq!(router.take(), [RouterCall::CloseDialog(None)]);
        assert_eq!(documents.borrow().types.len(), 9);
    }

    #[test]
    fn a_gone_type_has_no_dialog() {
        let (_, shared) = stub();

        assert!(RemoveTypePresenter::new(shared, DocumentTypeId::seed(999)).is_none());
    }
}
