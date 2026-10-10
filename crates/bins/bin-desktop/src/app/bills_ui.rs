//! The Bills destination's wiring: keys, clicks, the Bill Plan, Pay and Skip dialogs, and the Bills filters. The pure rules live in `bills`; the chrome in `view::bills`, and its View state in the `BillsView` Entity.

use super::{PENDING_G_TIMEOUT, Shell};

use std::rc::Rc;

use crate::{
    bills::{self, edit_bills, edit_bills_and_transactions, pay_form::PayForm},
    categories,
    chrome::dialog_host::OpenDialog,
    navigation::key_router::Movement,
    navigation::nav::{FocusZone, InputMode, Noun},
    payees,
    view::bills as bills_view,
    view::bills::state::{BillsEvent, BillsState},
    view::format,
};

use gpui::{App, Context, Entity, Keystroke};

impl Shell {
    /// The Bill Plans, read through their store.
    pub(super) fn bill_plans<'a>(&self, cx: &'a App) -> &'a [bills::BillPlan] {
        self.bills_store.read(cx).plans()
    }

    /// The Bill Schedule entries, read through their store.
    pub(super) fn bill_entries<'a>(&self, cx: &'a App) -> &'a [bills::BillScheduleEntry] {
        self.bills_store.read(cx).entries()
    }

    /// The Bills page's state, read through its Entity.
    pub(super) fn bills_state<'a>(&self, cx: &'a App) -> &'a BillsState {
        self.bills_view.read(cx).state()
    }

    /// Edits the Bills page's state through its Entity, which notifies.
    pub(super) fn edit_bills_state<R>(
        &self,
        cx: &mut App,
        change: impl FnOnce(&mut BillsState) -> R,
    ) -> R {
        self.bills_view.update(cx, |view, cx| view.edit(cx, change))
    }

    /// The Schedule tab's rows for the shown period (or All), before its filters.
    pub(super) fn bills_unfiltered_rows(&self, cx: &App) -> Vec<bills::ScheduleRow> {
        self.bills_view.read(cx).unfiltered_rows(cx)
    }

    /// The Schedule tab's rows as shown: the period's (or All's), through its filters.
    pub(super) fn bills_schedule_rows(&self, cx: &App) -> Vec<bills::ScheduleRow> {
        self.bills_view.read(cx).schedule_rows(cx)
    }

    /// The selected Schedule row, its stored position clamped to the rows now shown.
    pub(super) fn selected_bill_row(&self, cx: &App) -> Option<bills::ScheduleRow> {
        self.bills_view.read(cx).selected_row(cx)
    }

    /// The Planner tab's selected Bill Plan's id, its stored position clamped to the Plans.
    pub(super) fn selected_bill_plan(&self, cx: &App) -> Option<u32> {
        self.bills_view.read(cx).selected_plan(cx)
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` step the active Bills tab's row selection; `Enter` on a
    /// Paid Schedule row opens its Transaction, and on a Planner row edits its Bill Plan. While `f`
    /// has a Schedule filter select focused, they drive the select instead.
    pub(super) fn apply_bills_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) {
        let state = self.bills_state(cx);
        if state.tab == bills::BillsTab::Schedule && state.filter_focus.is_some() {
            self.apply_bills_filter_select_movement(movement, cx);
            return;
        }
        self.bills_view
            .update(cx, |view, cx| view.apply_movement(movement, cx));
    }

    /// Carries out what the Bills page asked for. The page holds only its own state; the dialogs,
    /// the Transaction jump and the filter choices are `Shell`'s.
    pub(super) fn handle_bills_event(&mut self, event: BillsEvent, cx: &mut Context<'_, Self>) {
        match event {
            BillsEvent::AddPlan => self.open_add_bill_plan_dialog(cx),
            BillsEvent::EditPlan(id) => self.open_edit_bill_plan_dialog(id, cx),
            BillsEvent::Pay(row) => self.open_pay_bill_dialog(row, cx),
            BillsEvent::Skip(row) => self.open_skip_bill_dialog(row, cx),
            BillsEvent::OpenTransaction(id) => self.open_bill_transaction(id, cx),
            BillsEvent::FocusNextFilter => self.cycle_bills_filter_focus(cx),
            BillsEvent::TabChanged => {
                self.chrome.status_message = None;
                self.reset_view_scroll(cx);
            }
        }
        cx.notify();
    }

    /// `tab` on the Bills page switches its tab (the handoff's `tab switch view`) rather than
    /// cycling focus; `shift-tab` still cycles focus, so the View zone can always be left. Runs
    /// before the router, like the Colour Theme grid's own `tab`.
    pub(super) fn handle_bills_tab_key(&mut self, keystroke: &Keystroke, cx: &mut App) -> bool {
        let pending_g_active = self
            .pending_g
            .is_some_and(|since| since.elapsed() <= PENDING_G_TIMEOUT);
        let modifiers = &keystroke.modifiers;
        if keystroke.key != "tab"
            || modifiers.shift
            || modifiers.control
            || modifiers.alt
            || modifiers.platform
            || pending_g_active
            || self.nav.mode() != InputMode::Normal
            || self.nav.noun() != Noun::Bills
            || self.nav.focus() != FocusZone::View
        {
            return false;
        }
        let next = self.bills_state(cx).tab.next();
        self.set_bills_tab(next, cx);
        true
    }

    pub(super) fn set_bills_tab(&mut self, tab: bills::BillsTab, cx: &mut App) {
        self.bills_view.update(cx, |view, cx| view.set_tab(tab, cx));
    }

    /// A Schedule filter select's options, and the index of its current value. Category and
    /// Account list only those some Bill Plan uses.
    pub(super) fn bills_filter_options(
        &self,
        field: bills_view::filters::FilterField,
        cx: &App,
    ) -> (Vec<String>, usize) {
        use bills_view::filters::FilterField;
        let filters = &self.bills_state(cx).filters;
        let scoped = |all: String, choices: Vec<(u32, String)>, current: Option<u32>| {
            let index = current
                .and_then(|id| choices.iter().position(|(choice, _)| *choice == id))
                .map_or(0, |position| position + 1);
            let labels = std::iter::once(all)
                .chain(choices.into_iter().map(|(_, name)| name))
                .collect();
            (labels, index)
        };
        match field {
            FilterField::Plan => scoped(
                crate::msg::desktop_bills_filter_all_bills(),
                self.bills_filter_choices(field, cx),
                filters.plan_id,
            ),
            FilterField::Category => scoped(
                crate::msg::desktop_bills_filter_all_categories(),
                self.bills_filter_choices(field, cx),
                filters.category_id,
            ),
            FilterField::Account => scoped(
                crate::msg::desktop_bills_filter_all_accounts(),
                self.bills_filter_choices(field, cx),
                filters.account_id,
            ),
        }
    }

    /// The Bill Plan, Category or Account choices behind a scope select (after its "All" option),
    /// as `(id, name)`.
    pub(super) fn bills_filter_choices(
        &self,
        field: bills_view::filters::FilterField,
        cx: &App,
    ) -> Vec<(u32, String)> {
        use bills_view::filters::FilterField;
        let plans = self.bill_plans(cx);
        let mut choices: Vec<(u32, String)> = match field {
            FilterField::Plan => {
                return bills::planner_order(plans)
                    .into_iter()
                    .map(|plan| (plan.id, plan.name.clone()))
                    .collect();
            }
            FilterField::Category => self
                .categories
                .iter()
                .filter(|c| plans.iter().any(|p| p.category_id == c.id))
                .map(|c| (c.id, c.name.clone()))
                .collect(),
            FilterField::Account => self
                .accounts
                .read(cx)
                .accounts()
                .iter()
                .filter(|a| plans.iter().any(|p| p.account_id == a.id))
                .map(|a| (a.id, a.name.clone()))
                .collect(),
        };
        choices.sort_by_key(|(_, name)| name.to_lowercase());
        choices
    }

    /// Sets a Schedule filter from its select's option `index`.
    pub(super) fn apply_bills_filter_option(
        &mut self,
        field: bills_view::filters::FilterField,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        use bills_view::filters::FilterField;
        let id = index.checked_sub(1).and_then(|i| {
            self.bills_filter_choices(field, cx)
                .get(i)
                .map(|(id, _)| *id)
        });
        self.edit_bills_state(cx, |state| {
            let filters = &mut state.filters;
            match field {
                FilterField::Plan => filters.plan_id = id,
                FilterField::Category => filters.category_id = id,
                FilterField::Account => filters.account_id = id,
            }
            state.selected = 0;
        });
    }

    /// A closed select state on a field's current value.
    pub(super) fn bills_filter_select_state(
        &self,
        field: bills_view::filters::FilterField,
        cx: &App,
    ) -> crate::form::select::SelectState {
        let (options, index) = self.bills_filter_options(field, cx);
        crate::form::select::SelectState::new(options.get(index).cloned())
    }

    /// `f` steps focus along the filter row's selects, then off it.
    pub(super) fn cycle_bills_filter_focus(&mut self, cx: &mut Context<'_, Self>) {
        use bills_view::filters::FilterField;
        let focused = self
            .bills_state(cx)
            .filter_focus
            .as_ref()
            .map(|(field, _)| *field);
        let next = match focused {
            None => Some(FilterField::Plan),
            Some(field) => FilterField::ORDER
                .iter()
                .position(|f| *f == field)
                .and_then(|i| FilterField::ORDER.get(i + 1))
                .copied(),
        };
        let focus = next.map(|field| (field, self.bills_filter_select_state(field, cx)));
        self.edit_bills_state(cx, |state| state.filter_focus = focus);
    }

    /// `j`/`k` on a focused select: step its value while closed (applying it at once), move the
    /// highlight while open; `Enter` opens it, or commits the highlight.
    pub(super) fn apply_bills_filter_select_movement(
        &mut self,
        movement: Movement,
        cx: &mut Context<'_, Self>,
    ) {
        let Some((field, mut state)) = self.edit_bills_state(cx, |s| s.filter_focus.take()) else {
            return;
        };
        let (options, _) = self.bills_filter_options(field, cx);
        let delta = match movement {
            Movement::Next => Some(1),
            Movement::Prev => Some(-1),
            _ => None,
        };
        let mut chosen = false;
        match (movement, delta) {
            (_, Some(delta)) if state.is_open() => state.move_highlight(&options, delta),
            (_, Some(delta)) => {
                state.step(&options, delta);
                chosen = true;
            }
            (Movement::Enter, _) if state.is_open() => {
                state.commit(&options);
                chosen = true;
            }
            (Movement::Enter, _) => state.open(&options),
            _ => {}
        }
        if chosen
            && let Some(index) = state
                .value()
                .and_then(|value| options.iter().position(|option| option == value))
        {
            self.apply_bills_filter_option(field, index, cx);
        }
        self.edit_bills_state(cx, |s| s.filter_focus = Some((field, state)));
    }

    pub(super) fn handle_bills_filter_chip_click(
        &mut self,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        self.bills_view
            .update(cx, |view, cx| view.toggle_filter_chip(index, cx));
        cx.notify();
    }

    /// Clicking a select opens its list (closing any other), or closes its own open list.
    pub(super) fn handle_bills_filter_field_click(
        &mut self,
        field: bills_view::filters::FilterField,
        cx: &mut Context<'_, Self>,
    ) {
        let open_here = self
            .bills_state(cx)
            .filter_focus
            .as_ref()
            .is_some_and(|(focused, state)| *focused == field && state.is_open());
        let mut state = self.bills_filter_select_state(field, cx);
        if !open_here {
            let (options, _) = self.bills_filter_options(field, cx);
            state.open(&options);
        }
        self.edit_bills_state(cx, |s| s.filter_focus = Some((field, state)));
        cx.notify();
    }

    pub(super) fn handle_bills_filter_option_click(
        &mut self,
        field: bills_view::filters::FilterField,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        self.apply_bills_filter_option(field, index, cx);
        let state = self.bills_filter_select_state(field, cx);
        self.edit_bills_state(cx, |s| s.filter_focus = Some((field, state)));
        cx.notify();
    }

    /// The Bills page's own `p`/`s`/`e`/`n`/`f`/`[`/`]`/`0`/`1`–`5` (only while it is the active noun and the view has
    /// focus, in `Normal` mode).
    /// The Schedule tab's filter selects: the first `Esc` closes an open list, the next leaves the
    /// filter row. Returns whether it consumed the `Esc`.
    pub(super) fn close_bills_filter_focus(&mut self, cx: &mut Context<'_, Self>) -> bool {
        self.edit_bills_state(cx, |bills| {
            let Some((_, state)) = bills.filter_focus.as_mut() else {
                return false;
            };
            if state.is_open() {
                state.cancel();
            } else {
                bills.filter_focus = None;
            }
            true
        })
    }

    pub(super) fn handle_bills_key(
        &mut self,
        keystroke: &Keystroke,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if self.nav.noun() != Noun::Bills || self.nav.focus() != FocusZone::View {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform {
            return false;
        }
        let shift = modifiers.shift;
        self.bills_view
            .update(cx, |view, cx| view.handle_key(&keystroke.key, shift, cx))
    }

    /// What the Add and Edit bill plan selects choose from, copied into the form as it opens. On
    /// Edit the Plan's own Payee stays listed even if it has since been deactivated.
    pub(super) fn bill_plan_source(
        &self,
        keep_payee: Option<u32>,
        cx: &App,
    ) -> bills::form::BillPlanSource {
        bills::form::BillPlanSource::new(
            &self.categories,
            self.accounts.read(cx).accounts(),
            self.payees_list(cx),
            keep_payee,
            crate::msg::desktop_payees_category_none(),
            bills_view::planner::recurrence_label,
            self.today,
            self.settings_date_style,
        )
    }

    pub(super) fn open_add_bill_plan_dialog(&mut self, cx: &mut Context<'_, Self>) {
        let form = bills::form::BillPlanForm::new(self.bill_plan_source(None, cx));
        self.open_dialog(OpenDialog::Bills(Box::new(bills::BillsDialog::Add(form))));
    }

    /// Opens the Edit dialog pre-filled from Bill Plan `id`.
    pub(super) fn open_edit_bill_plan_dialog(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        let Some(plan) = bills::get(self.bill_plans(cx), id) else {
            return;
        };
        let form =
            bills::form::BillPlanForm::from_plan(plan, self.bill_plan_source(plan.payee_id, cx));
        self.open_dialog(OpenDialog::Bills(Box::new(bills::BillsDialog::Edit(
            id, form,
        ))));
    }

    /// The form behind the open Add or Edit bill plan dialog, if that is what's open.
    pub(super) fn bill_plan_form_mut(&mut self) -> Option<&mut bills::form::BillPlanForm> {
        self.bills_dialog_mut()
            .and_then(bills::BillsDialog::plan_form_mut)
    }

    /// Applies a confirmed Bills dialog (`Enter` and the confirm button).
    pub(super) fn apply_bills_dialog(
        &mut self,
        dialog: bills::BillsDialog,
        cx: &mut Context<'_, Self>,
    ) {
        match dialog {
            bills::BillsDialog::Add(form) => self.apply_bill_plan(None, form, cx),
            bills::BillsDialog::Edit(id, form) => self.apply_bill_plan(Some(id), form, cx),
            bills::BillsDialog::Pay(form) => self.apply_pay_bill(form, cx),
            bills::BillsDialog::Skip(entry) => self.apply_skip_bill(entry, cx),
        }
    }

    /// **Add bill plan** / **Save**: adds or edits the Plan (Schedule regeneration is
    /// `bills::insert_plan`'s, `edit_plan`'s and `set_active`'s) and selects it on the Planner
    /// tab. A refused Save reopens the dialog with the error shown.
    pub(super) fn apply_bill_plan(
        &mut self,
        editing: Option<u32>,
        mut form: bills::form::BillPlanForm,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(draft) = form.draft() else {
            return;
        };
        let (today, is_active) = (self.today, form.is_active);
        let accounts = self.accounts.read(cx).accounts().to_vec();
        let categories = &self.categories;
        let result = edit_bills(&self.bills_store, cx, |plans, entries| match editing {
            None => bills::insert_plan(plans, entries, &draft, categories, &accounts, today),
            Some(id) => bills::edit_plan(plans, entries, id, &draft, categories, &accounts, today)
                .and_then(|()| bills::set_active(plans, entries, id, is_active, today))
                .map(|()| id),
        });
        match result {
            Ok(id) => {
                if self.bills_state(cx).tab == bills::BillsTab::Planner {
                    let at = bills::planner_order(self.bill_plans(cx))
                        .iter()
                        .position(|plan| plan.id == id)
                        .unwrap_or(0);
                    self.edit_bills_state(cx, |state| state.selected = at);
                }
            }
            Err(error) => {
                form.error = Some(error);
                let dialog = match editing {
                    None => bills::BillsDialog::Add(form),
                    Some(id) => bills::BillsDialog::Edit(id, form),
                };
                self.open_dialog(OpenDialog::Bills(Box::new(dialog)));
            }
        }
    }

    pub(super) fn handle_bill_plan_field_click(
        &mut self,
        field: bills::form::BillPlanField,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.bill_plan_form_mut() {
            match field {
                bills::form::BillPlanField::Category
                | bills::form::BillPlanField::Unit
                | bills::form::BillPlanField::Account
                | bills::form::BillPlanField::Payee
                | bills::form::BillPlanField::Recurrence => form.click_select(field),
                bills::form::BillPlanField::Active => {
                    form.focus(field);
                    form.toggle();
                }
                _ => form.focus(field),
            }
        }
        cx.notify();
    }

    pub(super) fn handle_bill_plan_option_click(
        &mut self,
        field: bills::form::BillPlanField,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.bill_plan_form_mut() {
            form.choose(field, index);
        }
        cx.notify();
    }

    pub(super) fn handle_bill_plan_amount_kind_click(
        &mut self,
        kind: bills::AmountKind,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.bill_plan_form_mut() {
            form.set_amount_kind(kind);
        }
        cx.notify();
    }

    pub(super) fn handle_bills_dialog_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.close_dialog();
        cx.notify();
    }

    pub(super) fn handle_bills_dialog_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_open_dialog(cx);
        cx.notify();
    }

    /// Opens the Pay dialog (8d) on an open Schedule row, with its Match candidates worked out now;
    /// a row with nothing to pay says so instead.
    pub(super) fn open_pay_bill_dialog(
        &mut self,
        row: bills::ScheduleRow,
        cx: &mut Context<'_, Self>,
    ) {
        let plan = bills::get(self.bill_plans(cx), row.id.plan_id);
        let Some(plan) = plan.filter(|_| row.is_actionable()) else {
            self.chrome.status_message = Some(crate::msg::desktop_status_bill_not_actionable());
            return;
        };
        let candidates = bills::match_candidates(
            self.bill_plans(cx),
            self.bill_entries(cx),
            self.transactions(cx),
            self.accounts.read(cx).accounts(),
            &self.categories,
            row.id,
        );
        let preselected = bills::preselected_candidate(plan, &candidates, self.transactions(cx));
        let form = PayForm::new(
            row.id,
            plan,
            candidates,
            preselected,
            self.today,
            self.settings_date_style,
        );
        self.open_dialog(OpenDialog::Bills(Box::new(bills::BillsDialog::Pay(form))));
    }

    pub(super) fn pay_form_mut(&mut self) -> Option<&mut PayForm> {
        match self.bills_dialog_mut() {
            Some(bills::BillsDialog::Pay(form)) => Some(form),
            _ => None,
        }
    }

    /// **Create transaction & mark paid** / **Match & mark paid**: settles the entry through
    /// `bills::pay` or `bills::match_split`. A refused settle reopens the dialog with the error
    /// shown.
    pub(super) fn apply_pay_bill(&mut self, mut form: PayForm, cx: &mut Context<'_, Self>) {
        let entry = form.entry;
        let Some(action) = form.action() else {
            return;
        };
        let accounts = self.accounts.read(cx).accounts().to_vec();
        let categories = &self.categories;
        let result = edit_bills_and_transactions(
            &self.bills_store,
            &self.transactions_store,
            cx,
            |plans, entries, transactions| match action {
                bills::pay_form::PayAction::Pay { amount, date } => {
                    bills::pay(plans, entries, transactions, entry, &amount, date).map(|_| ())
                }
                bills::pay_form::PayAction::Match(split) => bills::match_split(
                    plans,
                    entries,
                    transactions,
                    &accounts,
                    categories,
                    entry,
                    split,
                ),
            },
        );
        if let Err(error) = result {
            form.error = Some(error);
            self.open_dialog(OpenDialog::Bills(Box::new(bills::BillsDialog::Pay(form))));
        }
    }

    pub(super) fn handle_pay_bill_mode_click(
        &mut self,
        mode: bills::pay_form::PayMode,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.pay_form_mut() {
            form.set_mode(mode);
        }
        cx.notify();
    }

    pub(super) fn handle_pay_bill_choice_click(
        &mut self,
        choice: bills::pay_form::MatchChoice,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.pay_form_mut() {
            form.choose(choice);
        }
        cx.notify();
    }

    pub(super) fn handle_pay_bill_field_click(
        &mut self,
        field: bills::pay_form::PayField,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.pay_form_mut() {
            form.focus(field);
        }
        cx.notify();
    }

    /// The Pay dialog (8d) over the Schedule, or nothing once its entry's Plan is gone.
    pub(super) fn render_pay_bill_dialog(
        &self,
        form: &PayForm,
        entity: &Entity<Self>,
        cx: &App,
    ) -> Option<gpui::AnyElement> {
        let plan = bills::get(self.bill_plans(cx), form.entry.plan_id)?;
        let on_mode_click: bills_view::pay_dialog::OnModeClick = {
            let entity = entity.clone();
            Rc::new(move |mode, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_pay_bill_mode_click(mode, cx));
            })
        };
        let on_choice_click: bills_view::pay_dialog::OnChoiceClick = {
            let entity = entity.clone();
            Rc::new(move |choice, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_pay_bill_choice_click(choice, cx)
                });
            })
        };
        let on_field_click: bills_view::pay_dialog::OnFieldClick = {
            let entity = entity.clone();
            Rc::new(move |field, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_pay_bill_field_click(field, cx));
            })
        };
        let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let account = self
            .accounts
            .read(cx)
            .accounts()
            .iter()
            .find(|a| a.id == plan.account_id)
            .map(|a| a.name.clone())
            .unwrap_or_default();
        let payee = plan
            .payee_id
            .and_then(|id| payees::get(self.payees_list(cx), id))
            .map_or_else(crate::msg::desktop_bills_pay_no_payee, |p| p.name.clone());
        let category = categories::path(&self.categories, plan.category_id).unwrap_or_default();
        Some(bills_view::pay_dialog::render(
            bills_view::pay_dialog::PayDialogProps {
                plan_name: &plan.name,
                due: lib_locale::format::format_month_day(form.entry.due),
                form,
                candidates: self.pay_bill_candidate_rows(form, cx),
                planned: format!("{} {}", format::amount(&plan.planned_amount).1, plan.unit),
                chips: [category, payee, account],
                amount_invalid: form.amount_invalid(),
                date_error: form.date_error(),
                error: form
                    .error
                    .as_ref()
                    .map(|_| crate::msg::desktop_bills_pay_error_gone()),
                valid: form.action().is_some(),
                handlers: bills_view::pay_dialog::PayDialogHandlers {
                    on_mode_click,
                    on_choice_click,
                    on_field_click,
                    on_cancel: plain(Shell::handle_bills_dialog_cancel),
                    on_confirm: plain(Shell::handle_bills_dialog_confirm),
                },
            },
            cx,
        ))
    }

    /// The Pay dialog's Match candidates, worded for its radio list.
    pub(super) fn pay_bill_candidate_rows(
        &self,
        form: &PayForm,
        cx: &App,
    ) -> Vec<bills_view::pay_dialog::CandidateRow> {
        form.candidates
            .iter()
            .filter_map(|split_ref| {
                let transaction = self
                    .transactions(cx)
                    .iter()
                    .find(|t| t.id == split_ref.transaction_id)?;
                let split = transaction.splits.get(split_ref.split_index)?;
                let payee = split
                    .payee_id
                    .and_then(|id| payees::get(self.payees_list(cx), id))
                    .map(|p| p.name.clone())
                    .or_else(|| transaction.description.clone())
                    .unwrap_or_default();
                let account = self
                    .accounts
                    .read(cx)
                    .accounts()
                    .iter()
                    .find(|a| a.id == transaction.account_id)
                    .map(|a| a.name.clone())
                    .unwrap_or_default();
                Some(bills_view::pay_dialog::CandidateRow {
                    summary: format!(
                        "{payee} \u{b7} {} \u{b7} {}",
                        lib_locale::format::format_month_day(transaction.date),
                        format::signed_amount(&split.amount).1,
                    ),
                    account,
                })
            })
            .collect()
    }

    /// Opens the Skip dialog (8e) on an open Schedule row; a row with nothing to skip says so
    /// instead.
    pub(super) fn open_skip_bill_dialog(&mut self, row: bills::ScheduleRow, cx: &App) {
        if !row.is_actionable() || bills::get(self.bill_plans(cx), row.id.plan_id).is_none() {
            self.chrome.status_message = Some(crate::msg::desktop_status_bill_not_actionable());
            return;
        }
        self.open_dialog(OpenDialog::Bills(Box::new(bills::BillsDialog::Skip(
            row.id,
        ))));
    }

    /// **Skip this cycle**: resolves the entry through `bills::skip`. The dialog carries no form
    /// to show an error on, so a refused skip (the entry resolved or superseded underneath it)
    /// closes with the reason in the status line.
    pub(super) fn apply_skip_bill(&mut self, entry: bills::EntryId, cx: &mut App) {
        let skipped = edit_bills(&self.bills_store, cx, |plans, entries| {
            bills::skip(plans, entries, entry)
        });
        if skipped.is_err() {
            self.chrome.status_message = Some(crate::msg::desktop_bills_skip_error_gone());
        }
    }

    /// The Skip dialog (8e) over the Schedule, or nothing once its entry's Plan is gone.
    pub(super) fn render_skip_bill_dialog(
        &self,
        entry: bills::EntryId,
        entity: &Entity<Self>,
        cx: &App,
    ) -> Option<gpui::AnyElement> {
        let plan = bills::get(self.bill_plans(cx), entry.plan_id)?;
        let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        Some(bills_view::skip_dialog::render(
            bills_view::skip_dialog::SkipDialogProps {
                plan_name: &plan.name,
                due: lib_locale::format::format_month_day(entry.due),
                one_shot: plan.recurrence == bills::Recurrence::OneShot,
                on_cancel: plain(Shell::handle_bills_dialog_cancel),
                on_confirm: plain(Shell::handle_bills_dialog_confirm),
            },
            cx,
        ))
    }

    /// A Dashboard Needs Attention Bill row's hand-off: the Bills Schedule tab, on the period that
    /// shows the entry, with it selected.
    pub(super) fn open_bill_entry(&mut self, id: bills::EntryId, cx: &mut App) {
        let Some(entry) = bills::entry(self.bill_entries(cx), id) else {
            return;
        };
        let period = bills::schedule_period(entry, self.today);
        self.set_bills_tab(bills::BillsTab::Schedule, cx);
        self.edit_bills_state(cx, |state| {
            state.period = period;
            state.all = false;
            // Reset so no filter hides the entry being handed off to.
            state.filters = bills::history::BillFilters::default();
        });
        let at = self
            .bills_schedule_rows(cx)
            .iter()
            .position(|row| row.id == id)
            .unwrap_or(0);
        self.edit_bills_state(cx, |state| state.selected = at);
        self.nav.set_noun(Noun::Bills);
        self.reset_view_scroll(cx);
    }

    /// A Paid Schedule row's hand-off: the Transactions page with its Matched Transaction selected,
    /// the date range widened to reach it when it falls before this year. A no-op for any other row.
    pub(super) fn open_bill_transaction(&mut self, id: bills::EntryId, cx: &mut Context<'_, Self>) {
        let Some(bills::Resolution::Paid(split)) =
            bills::entry(self.bill_entries(cx), id).map(|entry| entry.resolution.clone())
        else {
            return;
        };
        self.open_transaction_row(split.transaction_id, cx);
    }

    pub(super) fn handle_bills_add_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_add_bill_plan_dialog(cx);
        cx.notify();
    }

    pub(super) fn handle_bills_tab_click(
        &mut self,
        tab: bills::BillsTab,
        cx: &mut Context<'_, Self>,
    ) {
        self.set_bills_tab(tab, cx);
        cx.notify();
    }

    pub(super) fn handle_bills_period_prev(&mut self, cx: &mut Context<'_, Self>) {
        self.bills_view
            .update(cx, |view, cx| view.shift_period(false, cx));
        cx.notify();
    }

    pub(super) fn handle_bills_period_next(&mut self, cx: &mut Context<'_, Self>) {
        self.bills_view
            .update(cx, |view, cx| view.shift_period(true, cx));
        cx.notify();
    }

    pub(super) fn handle_bills_all_click(&mut self, cx: &mut Context<'_, Self>) {
        self.bills_view.update(cx, |view, cx| view.toggle_all(cx));
        cx.notify();
    }

    pub(super) fn handle_bills_row_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.edit_bills_state(cx, |state| state.selected = index);
        cx.notify();
    }

    pub(super) fn handle_bills_edit_plan_click(
        &mut self,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        self.edit_bills_state(cx, |state| state.selected = index);
        if let Some(id) = self.selected_bill_plan(cx) {
            self.open_edit_bill_plan_dialog(id, cx);
        }
        cx.notify();
    }

    pub(super) fn handle_bills_pay_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.edit_bills_state(cx, |state| state.selected = index);
        if let Some(row) = self.selected_bill_row(cx) {
            self.open_pay_bill_dialog(row, cx);
        }
        cx.notify();
    }

    pub(super) fn handle_bills_skip_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.edit_bills_state(cx, |state| state.selected = index);
        if let Some(row) = self.selected_bill_row(cx) {
            self.open_skip_bill_dialog(row, cx);
        }
        cx.notify();
    }

    pub(super) fn handle_bills_view_transaction_click(
        &mut self,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        self.edit_bills_state(cx, |state| state.selected = index);
        if let Some(row) = self.selected_bill_row(cx) {
            self.open_bill_transaction(row.id, cx);
        }
        cx.notify();
    }
}
