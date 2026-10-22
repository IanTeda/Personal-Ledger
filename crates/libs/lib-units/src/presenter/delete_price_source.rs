//! The Delete price source Dialog (16z·del): the source's name typed back, and what happens to the
//! Units it prices, which fall back to Manual entry.

use lib_core::PriceSourceId;
use lib_viper::view::{DialogFrame, TextFieldView};

use super::{UnitsOutcome, count, linked_codes};
use crate::interactor::{DeletePriceSource, DeletePriceSourceRefusal};
use crate::msg;
use crate::protocols::{SharedPorts, UnitsRouter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeletePriceSourceEvent {
    /// The confirmation field's whole new text.
    ConfirmText(String),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletePriceSourceViewModel {
    pub frame: DialogFrame,
    /// `This price source values 1 unit. Deleting it cannot be undone.`
    pub warning: String,
    /// `btc falls back to Manual entry and keeps its last price.`; `None` while no Unit uses it.
    pub fallback: Option<String>,
    pub confirm: TextFieldView,
    pub refusal: Option<String>,
}

pub struct DeletePriceSourcePresenter {
    ports: SharedPorts,
    id: PriceSourceId,
    name: String,
    /// The Units it prices, copied when the Dialog opens.
    codes: Vec<String>,
    confirm_text: String,
    refusal: Option<String>,
}

impl DeletePriceSourcePresenter {
    /// The Dialog for source `id`, or `None` when it is gone. Only a Custom source can be deleted,
    /// but that is the Interactor's to refuse.
    pub fn new(ports: SharedPorts, id: PriceSourceId) -> Option<Self> {
        let (name, codes) = {
            let ports = ports.borrow();
            let name = ports.source_catalogue().source(id)?.name.to_string();
            (name, linked_codes(&*ports, id))
        };
        Some(Self {
            ports,
            id,
            name,
            codes,
            confirm_text: String::new(),
            refusal: None,
        })
    }

    pub fn handle(&mut self, event: DeletePriceSourceEvent, router: &mut dyn UnitsRouter) {
        match event {
            DeletePriceSourceEvent::ConfirmText(text) => self.confirm_text = text,
            DeletePriceSourceEvent::Confirm => self.confirm(router),
            DeletePriceSourceEvent::Cancel => router.close_dialog(None),
        }
    }

    pub fn view_model(&self) -> DeletePriceSourceViewModel {
        let units = count(self.codes.len());
        DeletePriceSourceViewModel {
            frame: DialogFrame {
                title: msg::units_price_source_delete_title(&self.name),
                submit: msg::units_price_source_delete_submit(),
                can_confirm: self.confirm_text == self.name,
                destructive: true,
            },
            warning: msg::units_price_source_delete_warning(units),
            fallback: (!self.codes.is_empty())
                .then(|| msg::units_price_source_delete_fallback(units, &self.codes.join(", "))),
            confirm: TextFieldView {
                label: msg::units_price_source_delete_confirm_label(&self.name),
                text: self.confirm_text.clone(),
                placeholder: self.name.clone(),
                focused: true,
            },
            refusal: self.refusal.clone(),
        }
    }

    /// The typed name must match exactly, case and all, as Delete unit's code does.
    fn confirm(&mut self, router: &mut dyn UnitsRouter) {
        if self.confirm_text != self.name {
            return;
        }
        let deleted = {
            let mut ports = self.ports.borrow_mut();
            let ports = ports.split_mut();
            DeletePriceSource(self.id).run(ports.price_sources, ports.units)
        };
        self.refusal = match deleted {
            Ok(deleted) => {
                router.close_dialog(Some(UnitsOutcome::SourceDeleted {
                    name: deleted.source.name,
                    moved: deleted.moved,
                }));
                None
            }
            Err(DeletePriceSourceRefusal::Gone) => Some(msg::units_refusal_price_source_gone()),
            Err(DeletePriceSourceRefusal::BuiltIn) => Some(msg::units_refusal_built_in_delete()),
            Err(DeletePriceSourceRefusal::ManualEntry) => Some(msg::units_refusal_manual_entry()),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::stub;
    use super::*;
    use crate::fake::{RecordingRouter, RouterCall};
    use crate::interactor::AddPriceSource;
    use crate::{COINGECKO, MANUAL_ENTRY, Updates};

    fn send(
        dialog: &mut DeletePriceSourcePresenter,
        events: Vec<DeletePriceSourceEvent>,
    ) -> Vec<RouterCall> {
        let mut router = RecordingRouter::default();
        for event in events {
            dialog.handle(event, &mut router);
        }
        router.calls
    }

    fn typed(name: &str) -> Vec<DeletePriceSourceEvent> {
        vec![
            DeletePriceSourceEvent::ConfirmText(name.into()),
            DeletePriceSourceEvent::Confirm,
        ]
    }

    #[test]
    fn deleting_a_custom_source_moves_its_units_to_manual_entry_and_closes_with_the_count() {
        let (ports, shared) = stub();
        let source = AddPriceSource {
            name: "Broker".into(),
            endpoint: crate::built_in(COINGECKO).unwrap().endpoint.clone(),
            updates: Updates::Daily,
        }
        .run(&mut ports.borrow_mut().price_sources);
        ports.borrow_mut().units.units[1].link.source_id = source.id;
        let mut dialog = DeletePriceSourcePresenter::new(shared, source.id).unwrap();
        let view = dialog.view_model();
        assert_eq!(view.frame.title, "Delete price source — Broker");
        assert_eq!(
            view.warning,
            "This price source values 1 unit. Deleting it cannot be undone."
        );
        assert_eq!(
            view.fallback.as_deref(),
            Some("btc falls back to Manual entry and keeps its last price.")
        );

        assert_eq!(
            send(&mut dialog, typed("broker")),
            [],
            "the match is case-sensitive"
        );
        let calls = send(&mut dialog, typed("Broker"));

        assert_eq!(
            calls,
            [RouterCall::CloseDialog(Some(UnitsOutcome::SourceDeleted {
                name: "Broker".into(),
                moved: 1,
            }))]
        );
        assert_eq!(ports.borrow().units.units[1].link.source_id, MANUAL_ENTRY);
    }

    #[test]
    fn a_built_in_or_a_gone_source_is_refused_as_text() {
        let (ports, shared) = stub();
        let mut built_in = DeletePriceSourcePresenter::new(shared.clone(), COINGECKO).unwrap();
        assert_eq!(send(&mut built_in, typed("CoinGecko")), []);
        assert_eq!(
            built_in.view_model().refusal.as_deref(),
            Some("A built-in price source can't be deleted.")
        );

        let source = AddPriceSource {
            name: "Broker".into(),
            endpoint: crate::built_in(COINGECKO).unwrap().endpoint.clone(),
            updates: Updates::Daily,
        }
        .run(&mut ports.borrow_mut().price_sources);
        let mut gone = DeletePriceSourcePresenter::new(shared, source.id).unwrap();
        ports.borrow_mut().price_sources.records.clear();
        send(&mut gone, typed("Broker"));
        assert_eq!(
            gone.view_model().refusal.as_deref(),
            Some("That price source was removed while the dialog was open.")
        );
    }
}
