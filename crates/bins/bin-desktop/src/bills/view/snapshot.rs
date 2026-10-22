//! What the integration tests read back of the Bills page (ADR-0030), built from the page's view
//! model. The open Dialog is the Dialog host's, so `Shell`'s accessor fills it in.

use super::page::BillsPage;

/// One Schedule row: the Plan it belongs to and the status it shows.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillRowSnapshot {
    pub plan: String,
    pub status: String,
    pub actionable: bool,
}

/// The Bills page's state: plain values, so the module's types stay its own.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillsSnapshot {
    /// `Schedule` or `Planner`.
    pub tab: String,
    /// The selected position in the active tab's rows.
    pub selected: usize,
    pub period: (i32, u32),
    pub all: bool,
    /// The Schedule rows the period and filters leave visible, in table order.
    pub rows: Vec<BillRowSnapshot>,
    /// Every Bill Plan name in Planner order.
    pub plans: Vec<String>,
    /// The status chips switched on, e.g. `Paid`.
    pub statuses_on: Vec<String>,
    /// The focused Schedule filter select, e.g. `Plan`; `None` while none is.
    pub filter_focus: Option<String>,
    /// `Add`, `Edit`, `Pay` or `Skip`; `None` while no Bills Dialog is open.
    pub dialog: Option<String>,
    /// The open Add or Edit Dialog's Name draft.
    pub plan_name: Option<String>,
    /// The open Add or Edit Dialog's Unit, by code.
    pub plan_unit: Option<String>,
    /// The open Pay Dialog's panel, `Match` or `Direct`.
    pub pay_mode: Option<String>,
    /// How many Transactions the open Pay Dialog offers to match.
    pub pay_candidates: Option<usize>,
}

impl BillsPage {
    /// The page for a test, from its view model. The Dialog fields are left empty for `Shell` to
    /// fill.
    #[doc(hidden)]
    pub fn snapshot(&self) -> BillsSnapshot {
        let view = self.view_model();
        let schedule = &view.schedule;
        let selected = match view.tab {
            lib_bills::presenter::BillsTab::Schedule => {
                schedule.rows.iter().position(|row| row.selected)
            }
            lib_bills::presenter::BillsTab::Planner => {
                view.planner.rows.iter().position(|row| row.selected)
            }
        };
        BillsSnapshot {
            tab: format!("{:?}", view.tab),
            selected: selected.unwrap_or(0),
            period: (schedule.period.year, schedule.period.month),
            all: schedule.all,
            rows: schedule
                .rows
                .iter()
                .map(|row| BillRowSnapshot {
                    plan: row.name.clone(),
                    status: format!("{:?}", row.status),
                    actionable: row.actionable,
                })
                .collect(),
            plans: view
                .planner
                .rows
                .iter()
                .map(|row| row.name.clone())
                .collect(),
            statuses_on: schedule
                .filters
                .chips
                .iter()
                .filter(|chip| chip.on)
                .map(|chip| format!("{:?}", chip.status))
                .collect(),
            filter_focus: schedule.filters.focused.map(|field| format!("{field:?}")),
            dialog: None,
            plan_name: None,
            plan_unit: None,
            pay_mode: None,
            pay_candidates: None,
        }
    }
}
