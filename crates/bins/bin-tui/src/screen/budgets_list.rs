//! The Budgets list screen (CC-TUI-010) — the horizontal progress-bar-vs-"today"-line
//! visualization carried forward from the closed TUI feasibility map (Ian confirmed this
//! exact design in that map's prototype; only the underlying spend calculation is new here).
//! Needs Category names for its rows and per-Budget spend progress, so `init()` also kicks
//! off a Categories load (reusing `CategoriesLoaded`) and a progress computation for every
//! loaded Budget.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, Paragraph},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::{Action, InputMode},
    db,
    screen::Screen,
};

/// One cell of a rendered progress bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BarCell {
    Filled,
    Empty,
    /// The "today" marker — always rendered distinctly, regardless of fill state.
    Today,
}

/// Builds a `width`-cell bar: `spend_fraction` of it filled (capped visually at `width` even
/// if over budget), with a `Today` marker at `today_fraction`'s position.
fn bar_cells(spend_fraction: f64, today_fraction: f64, width: usize) -> Vec<BarCell> {
    if width == 0 {
        return Vec::new();
    }
    let filled = ((spend_fraction.max(0.0)) * width as f64).round() as usize;
    let filled = filled.min(width);
    let today_index = ((today_fraction.clamp(0.0, 1.0)) * (width - 1) as f64).round() as usize;

    (0..width)
        .map(|i| {
            if i == today_index {
                BarCell::Today
            } else if i < filled {
                BarCell::Filled
            } else {
                BarCell::Empty
            }
        })
        .collect()
}

const BAR_WIDTH: usize = 24;

enum Status {
    Loading,
    Loaded(Vec<lib_database::Budgets>),
    Failed(String),
}

/// Lists every active Budget with its spend-progress bar; the entry point for creating,
/// editing, and deleting Budgets.
pub struct BudgetsListScreen {
    status: Status,
    categories: Vec<lib_database::Categories>,
    progress: Vec<(lib_core::RowID, lib_database::BudgetProgress)>,
    selected: usize,
    pending_delete: Option<lib_core::RowID>,
    error: Option<String>,
    action_tx: Option<UnboundedSender<Action>>,
}

impl BudgetsListScreen {
    pub fn new() -> Self {
        Self {
            status: Status::Loading,
            categories: Vec::new(),
            progress: Vec::new(),
            selected: 0,
            pending_delete: None,
            error: None,
            action_tx: None,
        }
    }

    fn budgets(&self) -> &[lib_database::Budgets] {
        match &self.status {
            Status::Loaded(budgets) => budgets,
            _ => &[],
        }
    }

    fn selected_id(&self) -> Option<lib_core::RowID> {
        self.budgets().get(self.selected).map(|b| b.id)
    }

    fn category_name(&self, id: lib_core::RowID) -> &str {
        self.categories
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.name.as_str())
            .unwrap_or("(unknown)")
    }

    fn progress_for(&self, id: lib_core::RowID) -> Option<&lib_database::BudgetProgress> {
        self.progress
            .iter()
            .find(|(bid, _)| *bid == id)
            .map(|(_, p)| p)
    }

    fn move_selection(&mut self, delta: isize) {
        let len = self.budgets().len();
        if len == 0 {
            return;
        }
        let next = (self.selected as isize + delta).rem_euclid(len as isize);
        self.selected = next as usize;
        self.pending_delete = None;
        self.error = None;
    }

    async fn load() -> lib_database::Result<Vec<lib_database::Budgets>> {
        let pool = db::connect().await?;
        lib_database::Budgets::find_all_active(&pool).await
    }

    async fn delete(id: lib_core::RowID) -> lib_database::Result<()> {
        let pool = db::connect().await?;
        lib_database::Budgets::delete_by_id(id, &pool).await
    }

    /// Computes progress for every given Budget, in order, against one connection.
    async fn load_progress(
        budgets: Vec<lib_database::Budgets>,
    ) -> lib_database::Result<Vec<(lib_core::RowID, lib_database::BudgetProgress)>> {
        let pool = db::connect().await?;
        let mut results = Vec::with_capacity(budgets.len());
        for budget in budgets {
            let progress = budget.current_progress(&pool).await?;
            results.push((budget.id, progress));
        }
        Ok(results)
    }

    fn spawn_progress_load(&self, budgets: Vec<lib_database::Budgets>) {
        let Some(action_tx) = self.action_tx.clone() else {
            return;
        };
        tokio::spawn(async move {
            let action = match Self::load_progress(budgets).await {
                Ok(progress) => Action::BudgetProgressLoaded(progress),
                Err(err) => Action::BudgetProgressLoadFailed(err.to_string()),
            };
            let _ = action_tx.send(action);
        });
    }
}

impl Default for BudgetsListScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for BudgetsListScreen {
    fn init(&mut self, action_tx: UnboundedSender<Action>) {
        let load_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = match Self::load().await {
                Ok(budgets) => Action::BudgetsLoaded(budgets),
                Err(err) => Action::BudgetsLoadFailed(err.to_string()),
            };
            let _ = load_tx.send(action);
        });

        let categories_tx = action_tx.clone();
        tokio::spawn(async move {
            let action = async {
                let pool = db::connect().await?;
                lib_database::Categories::find_all_active(&pool).await
            }
            .await;
            let action = match action {
                Ok(categories) => Action::CategoriesLoaded(categories),
                Err(err) => Action::CategoriesLoadFailed(err.to_string()),
            };
            let _ = categories_tx.send(action);
        });

        self.action_tx = Some(action_tx);
    }

    fn handle_key(&mut self, key: KeyEvent, _mode: InputMode) -> Option<Action> {
        if let Some(pending_id) = self.pending_delete {
            self.pending_delete = None;
            return match key.code {
                KeyCode::Char('y') => {
                    if let Some(action_tx) = self.action_tx.clone() {
                        tokio::spawn(async move {
                            let action = match Self::delete(pending_id).await {
                                Ok(()) => Action::BudgetDeleted(pending_id),
                                Err(err) => Action::BudgetDeleteFailed(err.to_string()),
                            };
                            let _ = action_tx.send(action);
                        });
                    }
                    Some(Action::NoOp)
                }
                _ => Some(Action::NoOp),
            };
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_selection(-1);
                Some(Action::NoOp)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_selection(1);
                Some(Action::NoOp)
            }
            KeyCode::Enter => self
                .budgets()
                .get(self.selected)
                .cloned()
                .map(|budget| Action::OpenBudgetDetail(Some(budget))),
            KeyCode::Char('n') => Some(Action::OpenBudgetDetail(None)),
            KeyCode::Char('d') if self.selected_id().is_some() => {
                self.pending_delete = self.selected_id();
                Some(Action::NoOp)
            }
            _ => None,
        }
    }

    fn update(&mut self, action: &Action) {
        match action {
            Action::BudgetsLoaded(budgets) => {
                self.status = Status::Loaded(budgets.clone());
                if self.selected >= budgets.len() && !budgets.is_empty() {
                    self.selected = budgets.len() - 1;
                }
                self.spawn_progress_load(budgets.clone());
            }
            Action::BudgetsLoadFailed(message) => {
                self.status = Status::Failed(message.clone());
            }
            Action::CategoriesLoaded(categories) => self.categories = categories.clone(),
            Action::BudgetProgressLoaded(progress) => {
                for (id, new_progress) in progress {
                    match self.progress.iter_mut().find(|(pid, _)| pid == id) {
                        Some((_, existing)) => *existing = new_progress.clone(),
                        None => self.progress.push((*id, new_progress.clone())),
                    }
                }
            }
            Action::BudgetProgressLoadFailed(message) => {
                self.error = Some(message.clone());
            }
            Action::BudgetSaved(saved) => {
                if let Status::Loaded(budgets) = &mut self.status {
                    match budgets.iter_mut().find(|b| b.id == saved.id) {
                        Some(existing) => *existing = saved.clone(),
                        None => budgets.push(saved.clone()),
                    }
                }
                self.spawn_progress_load(vec![saved.clone()]);
            }
            Action::BudgetDeleted(id) => {
                if let Status::Loaded(budgets) = &mut self.status {
                    budgets.retain(|b| b.id != *id);
                }
                self.progress.retain(|(pid, _)| pid != id);
            }
            Action::BudgetSaveFailed(message) | Action::BudgetDeleteFailed(message) => {
                self.error = Some(message.clone());
            }
            _ => {}
        }
    }

    fn title(&self) -> &'static str {
        "Budgets"
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);

        match &self.status {
            Status::Loading => {
                frame.render_widget(
                    Paragraph::new("Loading Budgets...")
                        .block(Block::bordered().title(" Budgets ")),
                    rows[0],
                );
            }
            Status::Failed(message) => {
                frame.render_widget(
                    Paragraph::new(format!("Failed to load Budgets: {message}"))
                        .style(Style::default().fg(Color::Red))
                        .block(Block::bordered().title(" Budgets ")),
                    rows[0],
                );
            }
            Status::Loaded(budgets) => {
                let items: Vec<ListItem> = budgets
                    .iter()
                    .enumerate()
                    .map(|(index, budget)| {
                        let selected = index == self.selected;
                        let name_style = if selected {
                            Style::default().fg(Color::Black).bg(Color::Cyan)
                        } else {
                            Style::default()
                        };

                        let header = Line::from(vec![
                            Span::styled(
                                format!("{:<20}", self.category_name(budget.category_id)),
                                name_style,
                            ),
                            Span::raw(format!(
                                "  {} / {} ({})",
                                self.progress_for(budget.id)
                                    .map(|p| p.spend.to_string())
                                    .unwrap_or_else(|| "…".to_string()),
                                budget.limit_amount,
                                budget.period.as_str(),
                            )),
                        ]);

                        let bar_line = match self.progress_for(budget.id) {
                            Some(progress) => {
                                let fill_color = if progress.is_ahead_of_pace() {
                                    Color::Red
                                } else {
                                    Color::Green
                                };
                                let mut spans: Vec<Span> = bar_cells(
                                    progress.spend_fraction(),
                                    progress.today_fraction,
                                    BAR_WIDTH,
                                )
                                .into_iter()
                                .map(|cell| match cell {
                                    BarCell::Filled => {
                                        Span::styled("█", Style::default().fg(fill_color))
                                    }
                                    BarCell::Empty => {
                                        Span::styled("░", Style::default().fg(Color::DarkGray))
                                    }
                                    BarCell::Today => Span::styled(
                                        "▐",
                                        Style::default()
                                            .fg(Color::White)
                                            .add_modifier(ratatui::style::Modifier::BOLD),
                                    ),
                                })
                                .collect();
                                if progress.spend_fraction() > 1.0 {
                                    spans.push(Span::styled(
                                        " OVER",
                                        Style::default()
                                            .fg(Color::Red)
                                            .add_modifier(ratatui::style::Modifier::BOLD),
                                    ));
                                }
                                Line::from(spans)
                            }
                            None => Line::raw("  computing progress..."),
                        };

                        ListItem::new(vec![header, bar_line, Line::raw("")])
                    })
                    .collect();

                frame.render_widget(
                    List::new(items)
                        .block(Block::bordered().title(format!(" Budgets ({}) ", budgets.len()))),
                    rows[0],
                );
            }
        }

        let footer = if self.pending_delete.is_some() {
            "Delete this Budget? y: confirm, any other key: cancel".to_string()
        } else if let Some(error) = &self.error {
            format!("Error: {error}")
        } else {
            "↑/k ↓/j: move  Enter: edit  n: new  d: delete  Esc: back".to_string()
        };
        frame.render_widget(Paragraph::new(footer), rows[1]);
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyModifiers;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn mock_budget(
        category_id: lib_core::RowID,
        unit_id: lib_core::RowID,
    ) -> lib_database::Budgets {
        let now = chrono::Utc::now();
        lib_database::Budgets {
            id: lib_core::RowID::new(),
            category_id,
            unit_id,
            limit_amount: lib_core::Money::mock(),
            period: lib_core::BudgetPeriod::Monthly,
            is_active: true,
            created_on: now,
            updated_on: now,
        }
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn render(status: Status) {
        let mut screen = BudgetsListScreen::new();
        screen.status = status;
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| screen.view(frame, frame.area()))
            .expect("rendering the Budgets list screen should not error");
    }

    #[test]
    fn renders_loading_without_panicking() {
        render(Status::Loading);
    }

    #[test]
    fn renders_failed_without_panicking() {
        render(Status::Failed("connection refused".to_string()));
    }

    #[test]
    fn renders_loaded_without_progress_without_panicking() {
        render(Status::Loaded(vec![mock_budget(
            lib_core::RowID::new(),
            lib_core::RowID::new(),
        )]));
    }

    #[test]
    fn renders_loaded_with_progress_without_panicking() {
        let mut screen = BudgetsListScreen::new();
        let budget = mock_budget(lib_core::RowID::new(), lib_core::RowID::new());
        screen.update(&Action::BudgetsLoaded(vec![budget.clone()]));
        screen.update(&Action::BudgetProgressLoaded(vec![(
            budget.id,
            lib_database::BudgetProgress {
                spend: "50".parse().unwrap(),
                limit_amount: "100".parse().unwrap(),
                period_start: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                period_end: chrono::NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
                today_fraction: 0.5,
            },
        )]));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| screen.view(frame, frame.area()))
            .expect("rendering with progress should not error");
    }

    #[test]
    fn n_opens_a_create_form() {
        let mut screen = BudgetsListScreen::new();
        assert_eq!(
            screen.handle_key(key(KeyCode::Char('n')), InputMode::Navigation),
            Some(Action::OpenBudgetDetail(None))
        );
    }

    #[test]
    fn enter_opens_an_edit_form_for_the_selected_budget() {
        let mut screen = BudgetsListScreen::new();
        let budget = mock_budget(lib_core::RowID::new(), lib_core::RowID::new());
        screen.update(&Action::BudgetsLoaded(vec![budget.clone()]));

        assert_eq!(
            screen.handle_key(key(KeyCode::Enter), InputMode::Navigation),
            Some(Action::OpenBudgetDetail(Some(budget)))
        );
    }

    #[test]
    fn d_arms_delete_confirmation_and_a_non_y_key_cancels_it() {
        let mut screen = BudgetsListScreen::new();
        screen.update(&Action::BudgetsLoaded(vec![mock_budget(
            lib_core::RowID::new(),
            lib_core::RowID::new(),
        )]));

        screen.handle_key(key(KeyCode::Char('d')), InputMode::Navigation);
        assert!(screen.pending_delete.is_some());

        screen.handle_key(key(KeyCode::Char('x')), InputMode::Navigation);
        assert!(screen.pending_delete.is_none());
    }

    #[test]
    fn budget_deleted_removes_it_from_the_loaded_list() {
        let mut screen = BudgetsListScreen::new();
        let budget = mock_budget(lib_core::RowID::new(), lib_core::RowID::new());
        screen.update(&Action::BudgetsLoaded(vec![budget.clone()]));
        screen.update(&Action::BudgetDeleted(budget.id));

        assert!(screen.budgets().is_empty());
    }

    #[test]
    fn category_name_resolves_once_loaded() {
        let mut screen = BudgetsListScreen::new();
        let now = chrono::Utc::now();
        let category = lib_database::Categories {
            id: lib_core::RowID::new(),
            code: "GRO.001".to_string(),
            name: "Groceries".to_string(),
            description: None,
            url_slug: None,
            category_type: lib_core::CategoryTypes::Expense,
            color: None,
            icon: None,
            is_active: true,
            created_on: now,
            updated_on: now,
        };
        screen.update(&Action::CategoriesLoaded(vec![category.clone()]));

        assert_eq!(screen.category_name(category.id), "Groceries");
        assert_eq!(screen.category_name(lib_core::RowID::new()), "(unknown)");
    }

    #[test]
    fn bar_cells_fills_proportionally_and_marks_today() {
        let cells = bar_cells(0.5, 0.75, 10);
        assert_eq!(cells.len(), 10);
        assert_eq!(
            cells.iter().filter(|c| **c == BarCell::Filled).count(),
            5,
            "half the bar should be filled"
        );
        assert_eq!(cells[7], BarCell::Today, "today marker at ~75%");
    }

    #[test]
    fn bar_cells_caps_fill_at_width_when_over_budget() {
        let cells = bar_cells(1.5, 0.5, 10);
        assert_eq!(
            cells.iter().filter(|c| **c == BarCell::Filled).count(),
            9,
            "fill caps at width, minus the one today-marker cell"
        );
    }
}
