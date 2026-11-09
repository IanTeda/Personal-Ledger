//! The Skip this cycle Dialog (8e): a confirm with nothing to fill in. Skipping destroys nothing
//! (Unskip reverses it), so it isn't drawn destructive. A One-shot Plan's text says the bill is
//! cancelled instead, since it has no next cycle (#368).

use lib_locale::format::format_month_day;
use lib_viper::dialog::{DialogInput, DialogKeys, DialogRouter};
use lib_viper::keymap::{BACK, SELECT};
use lib_viper::presenter::{DialogPresenter, Presenter};
use lib_viper::view::{DialogFrame, Hint};

use crate::interactor::SkipBill;
use crate::protocols::{BillsOutcome, SharedPorts};
use crate::{EntryId, Recurrence, msg};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipEvent {
    /// A Dialog keymap command or a typed key: `select` and `submit` confirm, the rest do nothing
    /// since the Dialog has no field.
    Input(DialogInput),
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkipViewModel {
    pub frame: DialogFrame,
    pub body: String,
    /// The status line's legend while the Dialog is open.
    pub hints: Vec<Hint>,
}

pub struct SkipPresenter {
    ports: SharedPorts,
    entry: EntryId,
    view: SkipViewModel,
}

impl SkipPresenter {
    /// The Dialog on entry `entry`, or `None` when its Plan is gone. Its text is read once: the
    /// Dialog is modal, so nothing renames the Plan under it. `keys` are the Dialog keymap's
    /// chords, which its hints show.
    pub fn new(ports: SharedPorts, entry: EntryId, keys: DialogKeys) -> Option<Self> {
        let plan = ports.borrow().bills().bill_plan(entry.plan_id)?;
        let due = format_month_day(entry.due);
        let body = if plan.recurrence == Recurrence::OneShot {
            msg::bills_skip_body_one_shot(&due, &plan.name)
        } else {
            msg::bills_skip_body(&due, &plan.name)
        };
        Some(Self {
            ports,
            entry,
            view: SkipViewModel {
                frame: DialogFrame {
                    title: msg::bills_skip_title(&plan.name),
                    submit: msg::bills_skip_submit(),
                    can_confirm: true,
                    destructive: false,
                    hints: Vec::new(),
                },
                body,
                hints: vec![
                    keys.hint(BACK, lib_locale::msg::dialog_hint_cancel()),
                    keys.hint(SELECT, lib_locale::msg::dialog_hint_confirm()),
                ],
            },
        })
    }
}

impl Presenter for SkipPresenter {
    type Event = SkipEvent;
    type ViewModel = SkipViewModel;
    type Router<'r> = dyn DialogRouter<BillsOutcome> + 'r;

    /// Confirm skips the entry and closes; an entry settled or superseded underneath closes the
    /// Dialog too, with the refusal for the page to report, since there is no form to show it on.
    fn handle(&mut self, event: SkipEvent, router: &mut Self::Router<'_>) {
        match event {
            SkipEvent::Input(DialogInput::Select | DialogInput::Submit) | SkipEvent::Confirm => {
                let skipped =
                    SkipBill { entry: self.entry }.run(self.ports.borrow_mut().split_mut().bills);
                router.close(Some(match skipped {
                    Ok(()) => BillsOutcome::Skipped,
                    Err(_) => BillsOutcome::SkipRefused,
                }));
            }
            SkipEvent::Input(_) => {}
            SkipEvent::Cancel => router.close(None),
        }
    }

    fn view_model(&self) -> SkipViewModel {
        self.view.clone()
    }
}

impl DialogPresenter for SkipPresenter {
    type Outcome = BillsOutcome;

    fn input_event(input: DialogInput) -> SkipEvent {
        SkipEvent::Input(input)
    }

    fn confirm_event() -> SkipEvent {
        SkipEvent::Confirm
    }

    fn cancel_event() -> SkipEvent {
        SkipEvent::Cancel
    }

    /// Nothing to fill in.
    fn can_confirm(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::{RecordingRouter, Routed};
    use crate::interactor::test_support::entry_of;
    use crate::presenter::test_support::{date, sample};
    use crate::{Resolution, entry};

    #[test]
    fn confirm_skips_the_entry_and_closes() {
        let (ports, shared) = sample();
        let origin = entry_of(&ports.borrow(), "Origin Energy", date(2026, 9, 24));
        let mut skip =
            SkipPresenter::new(shared, origin, DialogKeys::default()).expect("the Plan is there");
        let view = skip.view_model();
        assert_eq!(view.frame.title, "Skip this cycle \u{2014} Origin Energy");
        assert!(view.body.contains("isn't affected"));
        let mut router = RecordingRouter::default();

        skip.handle(SkipEvent::Confirm, &mut router);

        assert_eq!(router.routed, [Routed::Closed(Some(BillsOutcome::Skipped))]);
        assert!(matches!(
            entry(&ports.borrow().bills.entries, origin).map(|e| &e.resolution),
            Some(Resolution::Skipped { .. })
        ));
    }

    #[test]
    fn select_and_submit_confirm_and_other_inputs_do_nothing() {
        for (input, confirms) in [
            (DialogInput::Select, true),
            (DialogInput::Submit, true),
            (DialogInput::NextField, false),
            (DialogInput::SelectNext, false),
            (DialogInput::Char('x'), false),
            (DialogInput::Backspace, false),
        ] {
            let (_ports, shared) = sample();
            let origin = entry_of(&_ports.borrow(), "Origin Energy", date(2026, 9, 24));
            let mut skip = SkipPresenter::new(shared, origin, DialogKeys::default())
                .expect("the Plan is there");
            let mut router = RecordingRouter::default();

            skip.handle(SkipEvent::Input(input), &mut router);

            assert_eq!(
                router.routed,
                if confirms {
                    vec![Routed::Closed(Some(BillsOutcome::Skipped))]
                } else {
                    Vec::new()
                },
                "{input:?}"
            );
        }
    }

    #[test]
    fn an_entry_settled_underneath_closes_with_the_refusal() {
        let (ports, shared) = sample();
        let origin = entry_of(&ports.borrow(), "Origin Energy", date(2026, 9, 24));
        let mut skip =
            SkipPresenter::new(shared, origin, DialogKeys::default()).expect("the Plan is there");
        SkipBill { entry: origin }
            .run(&mut ports.borrow_mut().bills)
            .expect("an open entry");
        let mut router = RecordingRouter::default();

        skip.handle(SkipEvent::Confirm, &mut router);

        assert_eq!(
            router.routed,
            [Routed::Closed(Some(BillsOutcome::SkipRefused))]
        );
    }

    #[test]
    fn a_one_shot_says_the_bill_is_cancelled() {
        let (ports, shared) = sample();
        let id = ports.borrow().bills.plans[0].id;
        ports.borrow_mut().bills.plans[0].recurrence = Recurrence::OneShot;
        let skip = SkipPresenter::new(
            shared,
            EntryId {
                plan_id: id,
                due: date(2026, 9, 24),
            },
            DialogKeys::default(),
        )
        .expect("the Plan is there");

        assert!(
            skip.view_model()
                .body
                .contains("one-time bill is cancelled")
        );
    }
}
