//! The notice a Location's last Area gets in place of a Remove while it holds Items: there is no
//! other Area to take them. Its one button closes it, and it changes nothing.

use lib_core::AreaId;
use lib_viper::view::DialogFrame;

use super::count;
use crate::msg;
use crate::protocols::{InventoryRouter, SharedInventory};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AreaBlockedEvent {
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaBlockedViewModel {
    /// The title, and **Close** as the one button, always enabled.
    pub frame: DialogFrame,
    pub body: String,
}

pub struct AreaBlockedPresenter {
    name: String,
    location: String,
    items: usize,
}

impl AreaBlockedPresenter {
    /// The notice for Area `id`, or `None` when it is gone.
    pub fn new(inventory: &SharedInventory, id: AreaId) -> Option<Self> {
        let rows = inventory.borrow().inventory();
        let (location, area) = rows.area(id)?;
        Some(Self {
            name: area.name.clone(),
            location: location.name.clone(),
            items: rows.area_items(id),
        })
    }

    pub fn handle(&mut self, event: AreaBlockedEvent, router: &mut dyn InventoryRouter) {
        match event {
            AreaBlockedEvent::Cancel => router.close_dialog(None),
        }
    }

    /// Closing is all there is to confirm.
    pub fn confirm(&mut self, router: &mut dyn InventoryRouter) {
        router.close_dialog(None);
    }

    pub fn view_model(&self) -> AreaBlockedViewModel {
        AreaBlockedViewModel {
            frame: DialogFrame {
                title: msg::inventory_area_blocked_title(),
                submit: msg::inventory_area_blocked_close(),
                can_confirm: true,
                destructive: false,
            },
            body: msg::inventory_area_blocked_body(
                &self.name,
                &self.location,
                &msg::inventory_count_items(count(self.items)),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::stub;
    use super::*;
    use crate::fake::{RecordingRouter, RouterCall};
    use crate::protocols::InventoryRepository as _;

    #[test]
    fn it_names_the_area_its_location_and_items_and_closes_changing_nothing() {
        let (inventory, shared) = stub();
        let storage = inventory.borrow().locations()[1].clone();
        let mut notice = AreaBlockedPresenter::new(&shared, storage.areas[0].id).expect("seeded");
        let mut router = RecordingRouter::default();
        let before = inventory.borrow().clone();

        let view = notice.view_model();
        assert_eq!(view.frame.title, "Can't remove area");
        assert_eq!(view.frame.submit, "Close");
        assert!(view.body.contains("Storage unit") && view.body.contains("18 items"));
        notice.confirm(&mut router);
        notice.handle(AreaBlockedEvent::Cancel, &mut router);

        assert_eq!(
            router.take(),
            [RouterCall::CloseDialog(None), RouterCall::CloseDialog(None)]
        );
        assert_eq!(*inventory.borrow(), before);
        assert!(AreaBlockedPresenter::new(&shared, AreaId::seed(9_999)).is_none());
    }
}
