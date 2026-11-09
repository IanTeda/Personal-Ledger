//! The Import step as a page a Client hosts: closed until `:import` opens it on the statement, then
//! the [`ImportPresenter`] it holds, until a commit, a cancel or the Transactions destination being
//! entered closes it again. It owns the keymaps, the rows' and the open select's, so a Client asks
//! it for the keys its state takes, and an open select is its Filter mode, which swallows a key
//! the select's keymap doesn't bind as an open list does in the Dialogs.

use std::convert::Infallible;
use std::rc::Rc;

use lib_config::KeyBindingConfig;
use lib_toast::{Notice, Notify};
use lib_viper::keymap::Keymap;
use lib_viper::presenter::{PageMode, PagePresenter, Presenter};
use lib_viper::view::{Hint, legend};

use super::{ImportEvent, ImportPresenter, ImportViewModel};
use crate::keymap::{CONTINUE, import_keymap, import_select_keymap};
use crate::plan::ImportCommitted;
use crate::protocols::{ImportPorts, ImportRouter};

/// The step as drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportStepViewModel {
    /// The open step, `None` while it is closed.
    pub step: Option<ImportViewModel>,
    /// The status line's legend, each chord read from the keymap the step takes keys through now.
    pub hints: Vec<Hint>,
    /// The chord **continue** is on, which the footer shows beside it, so an override shows.
    pub continue_chord: String,
}

pub struct ImportStepPresenter {
    /// The ports a step opens on, built afresh each time so the statement is dated from the day it
    /// opens.
    ports: Rc<dyn Fn() -> ImportPorts>,
    open: Option<ImportPresenter>,
    keymap: Keymap<ImportEvent>,
    /// The keys while a row's select is open.
    select_keymap: Keymap<ImportEvent>,
}

impl ImportStepPresenter {
    /// The step, closed, opening on the ports `ports` builds, its keys under `config`.
    pub fn new(ports: impl Fn() -> ImportPorts + 'static, config: &KeyBindingConfig) -> Self {
        Self {
            ports: Rc::new(ports),
            open: None,
            keymap: import_keymap(config),
            select_keymap: import_select_keymap(config),
        }
    }

    /// Whether the step shows in place of the Transactions list.
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    fn select_open(&self) -> bool {
        self.open
            .as_ref()
            .is_some_and(|step| step.view_model().open_select.is_some())
    }
}

/// The step's Router around one call of the open step: it notes a close for the step to act on,
/// and passes each Notice on.
struct Closing<'a, 'r> {
    router: &'a mut (dyn ImportRouter + 'r),
    closed: Option<Option<ImportCommitted>>,
}

impl Notify for Closing<'_, '_> {
    fn notify(&mut self, notice: Notice) {
        self.router.notify(notice);
    }
}

impl ImportRouter for Closing<'_, '_> {
    fn close_import(&mut self, committed: Option<ImportCommitted>) {
        self.closed = Some(committed);
    }
}

impl Presenter for ImportStepPresenter {
    type Event = ImportEvent;
    type ViewModel = ImportStepViewModel;
    type Router<'r> = dyn ImportRouter + 'r;

    /// Opens, leaves or rebinds the step; any other event is the open step's. A close the open
    /// step asks for drops it, then tells `router` what its commit wrote.
    fn handle(&mut self, event: ImportEvent, router: &mut Self::Router<'_>) {
        match event {
            ImportEvent::OpenStep => self.open = Some(ImportPresenter::new((self.ports)())),
            ImportEvent::Leave => self.open = None,
            ImportEvent::KeysChanged(config) => {
                self.keymap = import_keymap(&config);
                self.select_keymap = import_select_keymap(&config);
            }
            event => {
                let Some(step) = self.open.as_mut() else {
                    return;
                };
                let mut closing = Closing {
                    router,
                    closed: None,
                };
                step.handle(event, &mut closing);
                if let Some(committed) = closing.closed {
                    self.open = None;
                    router.close_import(committed);
                }
            }
        }
    }

    fn view_model(&self) -> ImportStepViewModel {
        let step = self.open.as_ref().map(ImportPresenter::view_model);
        let hints = step
            .as_ref()
            .map(|view| legend(&view.hints, self.keymap()))
            .unwrap_or_default();
        ImportStepViewModel {
            step,
            hints,
            continue_chord: self.keymap.chord(CONTINUE).unwrap_or_default().to_owned(),
        }
    }

    /// A closed step takes only what opens it or its keys; there are no rows to act on.
    fn takes(&self, event: &ImportEvent) -> bool {
        self.open.is_some()
            || matches!(
                event,
                ImportEvent::OpenStep | ImportEvent::Leave | ImportEvent::KeysChanged(_)
            )
    }
}

impl PagePresenter for ImportStepPresenter {
    /// The step opens no Dialog.
    type Outcome = Infallible;

    /// The step opens no Dialog, so there is nothing to tell it.
    fn dialog_closed(_outcome: Option<Infallible>) -> Option<ImportEvent> {
        None
    }

    /// The open select's keys while one is open, else the rows'.
    fn keymap(&self) -> &Keymap<ImportEvent> {
        if self.select_open() {
            &self.select_keymap
        } else {
            &self.keymap
        }
    }

    /// An open select takes every key until it closes.
    fn mode(&self) -> Option<PageMode> {
        self.select_open().then_some(PageMode::Filter)
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use lib_list_cursor::Step;

    use super::*;
    use crate::fake::{RecordingRouter, stub_ports};
    use crate::presenter::RowSelect;

    fn step() -> ImportStepPresenter {
        lib_locale::init_with_layers("en-US", &[crate::msg::LAYER]);
        let today = NaiveDate::from_ymd_opt(2026, 9, 28).expect("a September day");
        ImportStepPresenter::new(move || stub_ports(today).0, &KeyBindingConfig::default())
    }

    #[test]
    fn the_step_is_closed_until_opened_and_a_closed_step_takes_no_row_key() {
        let mut step = step();
        let mut router = RecordingRouter::default();

        assert!(!step.is_open());
        assert_eq!(step.view_model().step, None);
        assert!(step.view_model().hints.is_empty());
        assert!(!step.takes(&ImportEvent::Move(Step::Next)));
        assert!(step.takes(&ImportEvent::OpenStep));

        step.handle(ImportEvent::OpenStep, &mut router);

        assert!(step.is_open());
        assert_eq!(step.view_model().step.map(|view| view.rows.len()), Some(18));
        assert_eq!(step.view_model().continue_chord, "enter");
    }

    #[test]
    fn an_open_select_is_a_mode_with_its_own_keymap() {
        let mut step = step();
        let mut router = RecordingRouter::default();
        step.handle(ImportEvent::OpenStep, &mut router);
        assert_eq!(step.mode(), None);
        assert_eq!(
            step.keymap().event("p"),
            Some(ImportEvent::Open(RowSelect::Payee))
        );

        step.handle(ImportEvent::Open(RowSelect::Category), &mut router);

        assert_eq!(step.mode(), Some(PageMode::Filter));
        assert_eq!(
            step.keymap().event("j"),
            Some(ImportEvent::Highlight(Step::Next))
        );
        assert_eq!(step.keymap().event("p"), None, "the select swallows it");
    }

    #[test]
    fn a_cancel_closes_the_step_and_tells_the_router_nothing_was_committed() {
        let mut step = step();
        let mut router = RecordingRouter::default();
        step.handle(ImportEvent::OpenStep, &mut router);

        step.handle(ImportEvent::Cancel, &mut router);

        assert!(!step.is_open());
        assert_eq!(router.closed, [None]);
    }

    #[test]
    fn leaving_closes_the_step_without_a_word_to_the_router() {
        let mut step = step();
        let mut router = RecordingRouter::default();
        step.handle(ImportEvent::OpenStep, &mut router);

        step.handle(ImportEvent::Leave, &mut router);

        assert!(!step.is_open());
        assert!(router.closed.is_empty());
    }

    #[test]
    fn rebound_keys_reach_the_step_and_its_footer() {
        let mut step = step();
        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert(CONTINUE.to_string(), "ctrl+s".to_string());

        step.handle(
            ImportEvent::KeysChanged(config),
            &mut RecordingRouter::default(),
        );

        assert_eq!(step.view_model().continue_chord, "ctrl+s");
        assert_eq!(step.keymap().event("ctrl+s"), Some(ImportEvent::Continue));
    }
}
