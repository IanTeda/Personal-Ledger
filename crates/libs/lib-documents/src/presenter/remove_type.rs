//! The Remove Document type Dialog. A type with Filed files asks where they move, the destination
//! select pre-selected to Other, the Default, and is drawn as a destructive action; a type with
//! none is a plain confirm. Other itself never gets here: the page opens the notice instead.
//!
//! The destination names are read when the Dialog opens, as it is modal and nothing changes them
//! under it.

use lib_core::DocumentTypeId;
use lib_viper::dialog::{DialogInput, DialogKeys, DialogRouter};
use lib_viper::keymap::{BACK, SELECT};
use lib_viper::presenter::{DialogPresenter, Presenter};
use lib_viper::view::{Choice, DialogFrame, Hint, SelectList, SelectMove, SelectView};

use crate::interactor::RemoveDocumentType;
use crate::msg;
use crate::protocols::{DocumentTypesOutcome, SharedRepository, TypeChange};
use crate::types::{RemoveError, get, position};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoveTypeEvent {
    /// A Dialog command or typed key: `select` commits the destination's open list, else removes,
    /// and `submit` only removes; `Space` and the arrows open and step the destination; typing
    /// and the field keys go nowhere.
    Input(DialogInput),
    /// A destination's name, as the select lists it, closing the list.
    Destination(String),
    /// A click on the destination's field: opens or closes its list.
    ToggleDestinations,
    /// Closes the destination's open list without picking: the host's `back`.
    CloseDestinations,
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
    /// The destination's list: open, and its highlight.
    pub destination_list: SelectList,
    /// The note under the select.
    pub moved_note: Option<String>,
    /// Why the last confirm was refused.
    pub refusal: Option<String>,
    /// How many Filed files the type has.
    pub files: u32,
    /// The status line's legend while the Dialog is open.
    pub hints: Vec<Hint>,
}

pub struct RemoveTypePresenter {
    ports: SharedRepository,
    keys: DialogKeys,
    id: DocumentTypeId,
    name: String,
    files: u32,
    /// Where the files can move: every other type, in order.
    destinations: Vec<String>,
    destination: Option<String>,
    list: SelectList,
    error: Option<RemoveError>,
}

impl RemoveTypePresenter {
    /// The Dialog for removing type `id`, or `None` when it is gone. `keys` are the Dialog keymap's
    /// chords, which its hints show.
    pub fn new(ports: SharedRepository, id: DocumentTypeId, keys: DialogKeys) -> Option<Self> {
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
            keys,
            id,
            name,
            files,
            destinations,
            destination,
            list: SelectList::default(),
            error: None,
        })
    }
}

impl Presenter for RemoveTypePresenter {
    type Event = RemoveTypeEvent;
    type ViewModel = RemoveTypeViewModel;
    type Router<'r> = dyn DialogRouter<DocumentTypesOutcome> + 'r;

    fn handle(&mut self, event: RemoveTypeEvent, router: &mut Self::Router<'_>) {
        match event {
            RemoveTypeEvent::Input(input) => self.input(input, router),
            RemoveTypeEvent::Destination(name) => {
                self.list.close();
                self.pick(name);
            }
            RemoveTypeEvent::ToggleDestinations => {
                if self.files > 0 {
                    let options = self.options();
                    self.list.toggle(&options, self.destination.as_ref());
                }
            }
            RemoveTypeEvent::CloseDestinations => {
                self.list.close();
            }
            RemoveTypeEvent::Confirm => self.save(router),
            RemoveTypeEvent::Cancel => router.close(None),
        }
    }

    fn view_model(&self) -> RemoveTypeViewModel {
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
                options: self.options(),
                chosen: self.destination.clone(),
                focused: true,
            }),
            destination_list: self.list,
            moved_note: has_files.then(msg::documents_types_remove_moved_note),
            refusal: self.error.map(|_| msg::documents_types_remove_refused()),
            files: self.files,
            hints: self.hints(),
        }
    }
}

impl DialogPresenter for RemoveTypePresenter {
    type Outcome = DocumentTypesOutcome;

    fn input_event(input: DialogInput) -> RemoveTypeEvent {
        RemoveTypeEvent::Input(input)
    }

    fn confirm_event() -> RemoveTypeEvent {
        RemoveTypeEvent::Confirm
    }

    fn cancel_event() -> RemoveTypeEvent {
        RemoveTypeEvent::Cancel
    }

    fn can_confirm(&self) -> bool {
        self.files == 0 || self.destination.is_some()
    }

    /// Closes the destination's open list without picking: the host's `back`.
    fn close_open_select(&mut self) -> bool {
        let open = self.list.is_open();
        if open {
            self.list.close();
        }
        open
    }
}

impl RemoveTypePresenter {
    /// Cancel and confirm, from the Dialog keys.
    fn hints(&self) -> Vec<Hint> {
        vec![
            self.keys.hint(BACK, lib_locale::msg::dialog_hint_cancel()),
            self.keys
                .hint(SELECT, lib_locale::msg::dialog_hint_confirm()),
        ]
    }

    /// Reads `input`: the destination, when the type has files, is the only field.
    fn input(&mut self, input: DialogInput, router: &mut dyn DialogRouter<DocumentTypesOutcome>) {
        match input {
            DialogInput::Submit => self.save(router),
            DialogInput::Select if !self.list.is_open() => self.save(router),
            _ if self.files > 0 => {
                let options = self.options();
                if let SelectMove::Chose(name) =
                    self.list.input(input, &options, self.destination.as_ref())
                {
                    self.pick(name);
                }
            }
            _ => {}
        }
    }

    fn pick(&mut self, name: String) {
        if self.destinations.contains(&name) {
            self.destination = Some(name);
            self.error = None;
        }
    }

    /// Every other type, as the destination's options.
    fn options(&self) -> Vec<Choice<String>> {
        self.destinations
            .iter()
            .map(|name| Choice {
                value: name.clone(),
                label: name.clone(),
            })
            .collect()
    }

    fn save(&mut self, router: &mut dyn DialogRouter<DocumentTypesOutcome>) {
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
            Ok(()) => router.close(Some(DocumentTypesOutcome {
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
    use super::super::test_support::sample;
    use super::*;
    use crate::fake::{InMemoryDocuments, RecordingRouter, RouterCall};

    use std::cell::RefCell;
    use std::rc::Rc;

    fn open(id: u32) -> (RemoveTypePresenter, Rc<RefCell<InMemoryDocuments>>) {
        let (documents, shared) = sample();
        let presenter =
            RemoveTypePresenter::new(shared, DocumentTypeId::seed(id), DialogKeys::default())
                .expect("a default type");
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
        let (documents, shared) = sample();
        documents.borrow_mut().types[2].files = 0;
        let presenter =
            RemoveTypePresenter::new(shared, DocumentTypeId::seed(3), DialogKeys::default())
                .expect("a default type");
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
        let (documents, shared) = sample();
        let mut presenter =
            RemoveTypePresenter::new(shared, DocumentTypeId::seed(3), DialogKeys::default())
                .expect("a default type");
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
        let (_, shared) = sample();

        assert!(
            RemoveTypePresenter::new(shared, DocumentTypeId::seed(999), DialogKeys::default())
                .is_none()
        );
    }

    #[test]
    fn the_destination_opens_steps_and_commits_and_select_removes_once_closed() {
        let (mut presenter, documents) = open(3);
        let mut router = RecordingRouter::default();
        let mut press = |presenter: &mut RemoveTypePresenter, input| {
            presenter.handle(RemoveTypeEvent::Input(input), &mut router);
            router.take()
        };

        press(&mut presenter, DialogInput::Char('x'));
        press(&mut presenter, DialogInput::NextField);
        press(&mut presenter, DialogInput::Char(' '));
        assert!(presenter.view_model().destination_list.is_open());
        press(&mut presenter, DialogInput::SelectPrevious);
        assert_eq!(
            press(&mut presenter, DialogInput::Select),
            [],
            "commits the highlight"
        );
        let view = presenter.view_model();
        assert!(!view.destination_list.is_open());
        let chosen = view.destination.and_then(|select| select.chosen);
        assert_ne!(chosen.as_deref(), Some("Other"));

        let calls = press(&mut presenter, DialogInput::Select);
        assert!(matches!(
            calls.as_slice(),
            [RouterCall::CloseDialog(Some(_))]
        ));
        assert_eq!(documents.borrow().types.len(), 8);
    }

    #[test]
    fn a_click_toggles_the_destination_list_and_back_closes_it() {
        let (mut presenter, _) = open(3);
        let mut router = RecordingRouter::default();

        presenter.handle(RemoveTypeEvent::ToggleDestinations, &mut router);
        assert!(presenter.view_model().destination_list.is_open());
        presenter.handle(RemoveTypeEvent::CloseDestinations, &mut router);
        assert!(!presenter.view_model().destination_list.is_open());
        presenter.handle(RemoveTypeEvent::ToggleDestinations, &mut router);
        presenter.handle(RemoveTypeEvent::Destination("Receipts".into()), &mut router);
        let view = presenter.view_model();
        assert!(!view.destination_list.is_open());
        assert_eq!(
            view.destination.and_then(|select| select.chosen).as_deref(),
            Some("Receipts")
        );
    }

    #[test]
    fn the_legend_reads_the_dialog_keys() {
        let (_, shared) = sample();
        let presenter = RemoveTypePresenter::new(
            shared,
            DocumentTypeId::seed(3),
            super::super::test_support::dialog_keys(),
        )
        .expect("a default type");

        let legend: Vec<String> = presenter
            .view_model()
            .hints
            .into_iter()
            .map(|hint| format!("{} {}", hint.chord, hint.action))
            .collect();
        assert_eq!(legend, ["esc cancel", "enter confirm"]);
    }
}
