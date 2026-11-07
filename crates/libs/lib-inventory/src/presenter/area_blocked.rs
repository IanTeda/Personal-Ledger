//! The notice a Location's last Area gets in place of a Remove while it holds Items: there is no
//! other Area to take them. Its one button closes it, and it changes nothing.

use lib_core::AreaId;
use lib_viper::dialog::{DialogInput, DialogKeys, DialogRouter};
use lib_viper::keymap::{BACK, SELECT};
use lib_viper::presenter::{DialogPresenter, Presenter};
use lib_viper::view::{DialogFrame, Hint, count};

use crate::msg;
use crate::protocols::{InventoryOutcome, SharedInventory};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AreaBlockedEvent {
    /// A Dialog command or typed key: `select` and `submit` close it, and nothing else does
    /// anything.
    Input(DialogInput),
    /// Closes it, as its one button does.
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaBlockedViewModel {
    /// The title, and **Close** as the one button, always enabled.
    pub frame: DialogFrame,
    pub body: String,
    /// The status line's legend while the Dialog is open.
    pub hints: Vec<Hint>,
}

pub struct AreaBlockedPresenter {
    keys: DialogKeys,
    name: String,
    location: String,
    items: usize,
}

impl AreaBlockedPresenter {
    /// The notice for Area `id`, or `None` when it is gone. `keys` are the Dialog keymap's chords,
    /// which its hints show.
    pub fn new(inventory: &SharedInventory, id: AreaId, keys: DialogKeys) -> Option<Self> {
        let rows = inventory.borrow().inventory();
        let (location, area) = rows.area(id)?;
        Some(Self {
            keys,
            name: area.name.clone(),
            location: location.name.clone(),
            items: rows.area_items(id),
        })
    }
}

impl Presenter for AreaBlockedPresenter {
    type Event = AreaBlockedEvent;
    type ViewModel = AreaBlockedViewModel;
    type Router<'r> = dyn DialogRouter<InventoryOutcome> + 'r;

    fn handle(&mut self, event: AreaBlockedEvent, router: &mut Self::Router<'_>) {
        match event {
            // Closing is all there is to confirm.
            AreaBlockedEvent::Input(DialogInput::Select | DialogInput::Submit)
            | AreaBlockedEvent::Confirm
            | AreaBlockedEvent::Cancel => router.close(None),
            AreaBlockedEvent::Input(_) => {}
        }
    }

    fn view_model(&self) -> AreaBlockedViewModel {
        AreaBlockedViewModel {
            frame: DialogFrame {
                title: msg::inventory_area_blocked_title(),
                submit: msg::inventory_area_blocked_close(),
                can_confirm: true,
                destructive: false,
                hints: Vec::new(),
            },
            hints: self.hints(),
            body: msg::inventory_area_blocked_body(
                &self.name,
                &self.location,
                &msg::inventory_count_items(count(self.items)),
            ),
        }
    }
}

impl DialogPresenter for AreaBlockedPresenter {
    type Outcome = InventoryOutcome;

    fn input_event(input: DialogInput) -> AreaBlockedEvent {
        AreaBlockedEvent::Input(input)
    }

    fn confirm_event() -> AreaBlockedEvent {
        AreaBlockedEvent::Confirm
    }

    fn cancel_event() -> AreaBlockedEvent {
        AreaBlockedEvent::Cancel
    }

    fn can_confirm(&self) -> bool {
        true
    }
}

impl AreaBlockedPresenter {
    /// Cancel and confirm, from the Dialog keys.
    fn hints(&self) -> Vec<Hint> {
        vec![
            self.keys.hint(BACK, lib_locale::msg::dialog_hint_cancel()),
            self.keys
                .hint(SELECT, lib_locale::msg::dialog_hint_confirm()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::sample;
    use super::*;
    use crate::fake::{RecordingRouter, RouterCall};
    use crate::protocols::InventoryRepository as _;

    #[test]
    fn it_names_the_area_its_location_and_items_and_closes_changing_nothing() {
        let (inventory, shared) = sample();
        let storage = inventory.borrow().locations()[1].clone();
        let mut notice =
            AreaBlockedPresenter::new(&shared, storage.areas[0].id, DialogKeys::default())
                .expect("seeded");
        let mut router = RecordingRouter::default();
        let before = inventory.borrow().clone();

        let view = notice.view_model();
        assert_eq!(view.frame.title, "Can't remove area");
        assert_eq!(view.frame.submit, "Close");
        assert!(view.body.contains("Storage unit") && view.body.contains("18 items"));
        notice.handle(AreaBlockedEvent::Input(DialogInput::Char('x')), &mut router);
        notice.handle(AreaBlockedEvent::Input(DialogInput::NextField), &mut router);
        assert_eq!(router.take(), [], "only select and submit close it");
        notice.handle(AreaBlockedEvent::Confirm, &mut router);
        notice.handle(AreaBlockedEvent::Cancel, &mut router);
        notice.handle(AreaBlockedEvent::Input(DialogInput::Select), &mut router);
        notice.handle(AreaBlockedEvent::Input(DialogInput::Submit), &mut router);

        assert_eq!(router.take(), vec![RouterCall::CloseDialog(None); 4]);
        assert_eq!(*inventory.borrow(), before);
        assert!(
            AreaBlockedPresenter::new(&shared, AreaId::seed(9_999), DialogKeys::default())
                .is_none()
        );
    }
}
