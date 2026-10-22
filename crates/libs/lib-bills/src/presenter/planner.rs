//! The Planner tab (8b): the Bill Plans themselves, active ones by name and then inactive ones
//! ([`planner_order`]). Not period-scoped. `Enter`, `e` and a row's edit button open the Plan's
//! Edit form.

use lib_core::BillPlanId;
use lib_list_cursor::Step;
use lib_locale::Segment;
use lib_locale::format::upper;

use super::{count, money, recurrence_label};
use crate::protocols::{BillsRouter, SharedPorts};
use crate::{AmountKind, BillPlan, inactive_count, msg, planner_order};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlannerEvent {
    /// `j`/`k`, `g g`, `G`, `Ctrl-d`/`Ctrl-u` over the Plans.
    Move(Step),
    /// A click on row `index`.
    Select(usize),
    /// `Enter` and `e`: the selected Plan's Edit form.
    Edit,
    /// A row's edit button: select it, then edit.
    EditRow(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannerViewModel {
    /// The header's meta line: `10 bill plans`.
    pub meta_plans: String,
    /// `· 1 inactive`, its count strong; `None` while every Plan is active.
    pub meta_inactive: Option<Vec<Segment>>,
    pub columns: PlannerColumns,
    pub rows: Vec<PlanLine>,
    /// What shows instead of the table while there are no Plans.
    pub empty: Option<String>,
    pub footnote: String,
}

/// The table's headings, upper-cased.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannerColumns {
    pub name: String,
    pub category: String,
    pub account: String,
    pub planned: String,
    pub recurs: String,
    pub lead: String,
    pub active: String,
    pub actions: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanLine {
    pub id: BillPlanId,
    pub name: String,
    pub category: String,
    pub account: String,
    /// The Planned Amount, `~`-prefixed when Estimated.
    pub planned: String,
    pub recurs: String,
    /// `3d`, or `—` when unset.
    pub lead: String,
    /// An active Plan with an Attention Lead, tinted to draw the eye to LEAD.
    pub has_lead: bool,
    pub active: String,
    /// Drawn muted throughout.
    pub inactive: bool,
    pub edit: String,
    pub selected: bool,
}

pub struct PlannerPresenter {
    ports: SharedPorts,
    /// A position in Planner order, clamped wherever it is read.
    selected: usize,
}

impl PlannerPresenter {
    pub fn new(ports: SharedPorts) -> Self {
        Self { ports, selected: 0 }
    }

    pub fn handle(&mut self, event: PlannerEvent, router: &mut dyn BillsRouter) {
        match event {
            PlannerEvent::Move(step) => {
                let len = self.plans().len();
                self.selected = lib_list_cursor::step(self.clamped(len), len, step);
            }
            PlannerEvent::Select(index) => self.selected = index,
            PlannerEvent::Edit => self.edit(router),
            PlannerEvent::EditRow(index) => {
                self.selected = index;
                self.edit(router);
            }
        }
    }

    /// Selects Plan `id`; leaves the selection for a Plan that is gone.
    pub(super) fn select_plan(&mut self, id: BillPlanId) {
        if let Some(at) = planner_order(&self.plans())
            .iter()
            .position(|plan| plan.id == id)
        {
            self.selected = at;
        }
    }

    pub(super) fn reset_selection(&mut self) {
        self.selected = 0;
    }

    fn plans(&self) -> Vec<BillPlan> {
        self.ports.borrow().bills().bill_plans()
    }

    fn clamped(&self, len: usize) -> usize {
        self.selected.min(len.saturating_sub(1))
    }

    fn edit(&mut self, router: &mut dyn BillsRouter) {
        let plans = self.plans();
        let ordered = planner_order(&plans);
        if let Some(plan) = ordered.get(self.clamped(ordered.len())) {
            router.open_plan_form(Some(plan.id));
        }
    }

    /// The status line's right-hand text: how many Plans there are.
    pub fn status(&self) -> String {
        msg::bills_status_plans(count(self.plans().len()))
    }

    pub fn view_model(&self) -> PlannerViewModel {
        let ports = self.ports.borrow();
        let ports = &*ports;
        let plans = ports.bills().bill_plans();
        let ordered = planner_order(&plans);
        let selected = (!ordered.is_empty()).then(|| self.clamped(ordered.len()));
        let categories = ports.categories().bill_categories();
        let accounts = ports.accounts().bill_accounts();
        let inactive = inactive_count(&plans);
        let rows = ordered
            .iter()
            .enumerate()
            .map(|(index, plan)| {
                let planned = money(ports, &plan.planned_amount, plan.unit);
                PlanLine {
                    id: plan.id,
                    name: plan.name.clone(),
                    category: categories
                        .iter()
                        .find(|category| category.id == plan.category_id)
                        .map_or_else(msg::bills_none, |category| category.name.clone()),
                    account: accounts
                        .iter()
                        .find(|account| account.id == plan.account_id)
                        .map_or_else(msg::bills_none, |account| account.name.clone()),
                    planned: if plan.amount_kind == AmountKind::Estimated {
                        format!("~{planned}")
                    } else {
                        planned
                    },
                    recurs: recurrence_label(plan.recurrence),
                    lead: plan.attention_lead.map_or_else(msg::bills_none, |days| {
                        msg::bills_lead_days(&days.to_string())
                    }),
                    has_lead: plan.attention_lead.is_some() && plan.is_active,
                    active: if plan.is_active {
                        msg::bills_active_yes()
                    } else {
                        msg::bills_active_no()
                    },
                    inactive: !plan.is_active,
                    edit: msg::bills_row_edit(),
                    selected: selected == Some(index),
                }
            })
            .collect();
        PlannerViewModel {
            meta_plans: msg::bills_planner_plans(count(plans.len())),
            meta_inactive: (inactive > 0)
                .then(|| msg::bills_planner_inactive(&inactive.to_string())),
            columns: PlannerColumns {
                name: upper(&msg::bills_column_name()),
                category: upper(&msg::bills_column_category()),
                account: upper(&msg::bills_column_account()),
                planned: upper(&msg::bills_column_planned()),
                recurs: upper(&msg::bills_column_recurs()),
                lead: upper(&msg::bills_column_lead()),
                active: upper(&msg::bills_column_active()),
                actions: upper(&lib_locale::msg::column_actions()),
            },
            empty: plans.is_empty().then(msg::bills_planner_empty),
            rows,
            footnote: msg::bills_planner_footnote(),
        }
    }
}
