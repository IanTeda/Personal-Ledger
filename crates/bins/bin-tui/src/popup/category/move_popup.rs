//! The "move" popup — `m` on a Categories tree row, or `:cat move <cat> <parent>`
//! (`docs/ux/tui/categories/README.md` "5b — Move"). Genuinely interactive and genuinely
//! mutates the tree, unlike `popup::unit`'s forms (still wireframe-only) — the only thing
//! stopping this popup being wireframe-only too is that "Categories: fixture data seam and
//! mutable View state pattern" already built a real, mutable tree for it to act on.
//!
//! **Reaching the tree it needs**: `MovePopup` is `Shell`-owned (`Shell::category_popup`,
//! matching `popup::unit`'s own structure, per this ticket's instruction), but the Category
//! tree lives inside `CategoriesView` (`view::categories::CategoriesView::store`) — `Shell`
//! itself owns no tree. Every method here that needs to read it (`render`, `tab_complete`,
//! resolving `^n`/`^s`) takes `store: &dyn CategoryStore` as a parameter, which `Shell` gets
//! via `View::category_store()` at the point it needs it, rather than this popup holding its
//! own reference or a stale clone. Writing back happens the other way: a resolved `^n`/`^s`
//! becomes a `Copy`-friendly `Action::CreateCategoryChild`/`Action::MoveCategory` that `Shell`
//! relays to `View::update`, which is where `CategoriesView` actually mutates its own `store`
//! — this popup never mutates anything itself.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
};

use lib_core::RowID;

use crate::category::CategoryStore;
use crate::popup::REFERENCE_TERMINAL_WIDTH;

/// The theme's one accent colour, per `docs/ux/tui/README.md`'s style table — used here for
/// the input cursor and the moving node's marked landing row.
const ACCENT: Color = Color::Red;

/// Fraction of `REFERENCE_TERMINAL_WIDTH` the popup takes, per the handoff's "~88% width" —
/// matches `popup::unit`'s own forms.
const POPUP_WIDTH_PERCENT: u32 = 88;

const POPUP_WIDTH: u16 = ((REFERENCE_TERMINAL_WIDTH as u32 * POPUP_WIDTH_PERCENT) / 100) as u16;

/// Width of the field label column (`new parent`, `recomputes`, …) — sized to the longest
/// label, plus a gap.
const LABEL_WIDTH: u16 = "transactions".len() as u16 + 1;

/// How many of the landing parent's children (moving node included) the "lands as" preview
/// shows before summarising the rest — keeps the popup's height bounded for a parent with a
/// great many children.
const MAX_LANDING_CHILDREN: usize = 6;

/// The `:cat move` popup's own draft state — just the typed `new parent` path. Everything
/// else (whether it resolves, what it would refuse, what "lands as" shows) is recomputed live
/// against the store on every render/keystroke rather than cached here, so it can never go
/// stale while the popup is open.
pub struct MovePopup {
    moving_id: RowID,
    input: String,
}

/// What the currently-typed `input` resolves to.
enum Resolution {
    /// Every path segment matched an existing category — `RowID` is its id.
    Existing(RowID),
    /// Every segment but the last matched; the last (`name`) doesn't exist yet under the
    /// matched `parent` — `^n` can create it.
    Creatable { parent: RowID, name: String },
    /// The path doesn't match anything, and doesn't leave exactly one creatable segment
    /// either (e.g. more than one missing level, or no matching root at all).
    Invalid,
}

impl MovePopup {
    /// Opens a popup moving `moving_id`, prefilling `input` with its current parent's path —
    /// continuing to type from there covers the common case of moving to a sibling or a
    /// nested child of the current location.
    pub fn new(store: &dyn CategoryStore, moving_id: RowID) -> Self {
        let input = store
            .find(moving_id)
            .and_then(|node| node.parent_id)
            .map(|parent_id| ancestor_names(store, parent_id).join("/"))
            .unwrap_or_default();
        Self { moving_id, input }
    }

    pub fn moving_id(&self) -> RowID {
        self.moving_id
    }

    pub fn push_char(&mut self, c: char) {
        self.input.push(c);
    }

    pub fn backspace(&mut self) {
        self.input.pop();
    }

    /// `Tab`: completes the input's last path segment against the first matching category
    /// name (alphabetical, case-insensitive) — a narrow-to-one-candidate step, not full
    /// argument completion, mirroring the command popup's own `Tab` semantics.
    pub fn tab_complete(&mut self, store: &dyn CategoryStore) {
        let Some(candidate) = completions(store, &self.input).into_iter().next() else {
            return;
        };
        let mut segments: Vec<&str> = self.input.split('/').collect();
        segments.pop();
        segments.push(&candidate);
        self.input = segments.join("/");
    }

    /// The existing category `input` currently resolves to, if any — what `^s` would move
    /// into.
    pub fn resolved_parent(&self, store: &dyn CategoryStore) -> Option<RowID> {
        match resolve(store, &self.input) {
            Resolution::Existing(id) => Some(id),
            _ => None,
        }
    }

    /// The parent/name `^n` would create, if `input`'s last segment is the only missing part
    /// of an otherwise-resolved path.
    pub fn creatable(&self, store: &dyn CategoryStore) -> Option<(RowID, String)> {
        match resolve(store, &self.input) {
            Resolution::Creatable { parent, name } => Some((parent, name)),
            _ => None,
        }
    }

    /// Renders the floating overlay, centred and sized to its own dynamic content (the "lands
    /// as" fragment's height varies with how many children the landing parent has), within
    /// `area` — the full terminal area, per §3a's "centred floating overlay" every form in
    /// this design reuses.
    pub fn render(&self, frame: &mut Frame, area: Rect, store: &dyn CategoryStore) {
        let Some(moving) = store.find(self.moving_id) else {
            return;
        };
        let resolution = resolve(store, &self.input);
        let landing_lines = landing_fragment_lines(store, &resolution, moving.id);
        let content_rows = 6 + landing_lines.len() as u16 + 1 + 3 + 2;
        let popup = popup_rect(area, content_rows);

        frame.render_widget(Clear, popup);
        let block = Block::bordered();
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let mut constraints = vec![
            Constraint::Length(1), // title
            Constraint::Length(1), // rule
            Constraint::Length(1), // moving
            Constraint::Length(1), // from
            Constraint::Length(1), // new parent
            Constraint::Length(1), // completion
            Constraint::Length(1), // blank spacer
            Constraint::Length(1), // "lands as" heading
        ];
        constraints.extend(std::iter::repeat_n(
            Constraint::Length(1),
            landing_lines.len(),
        ));
        constraints.push(Constraint::Length(1)); // blank spacer
        constraints.extend([
            Constraint::Length(1), // recomputes
            Constraint::Length(1), // transactions
            Constraint::Length(1), // refuses
            Constraint::Length(1), // rule
            Constraint::Length(1), // footer hints
        ]);
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        render_title(frame, rows[0]);
        frame.render_widget(
            Block::new().borders(ratatui::widgets::Borders::BOTTOM),
            rows[1],
        );
        render_field(
            frame,
            rows[2],
            "moving",
            Line::from(format!(
                "{} · {} · {} txns",
                moving.name,
                if store.children(moving.id).is_empty() {
                    "leaf".to_string()
                } else {
                    format!("{} children", store.children(moving.id).len())
                },
                moving.transaction_count
            )),
        );
        let from_text = moving
            .parent_id
            .map(|parent_id| ancestor_names(store, parent_id).join(" / "))
            .unwrap_or_else(|| "—".to_string());
        render_field(frame, rows[3], "from", Line::from(from_text));

        render_new_parent_field(frame, rows[4], &self.input);
        render_completion_row(frame, rows[5], store, &self.input);
        // rows[6] is left blank — breathing space above the "lands as" preview.

        let depth_tag = match &resolution {
            Resolution::Existing(id) => format!("depth {}", store.depth(*id) + 1),
            _ => String::new(),
        };
        render_lands_as_heading(frame, rows[7], &depth_tag);
        for (index, line) in landing_lines.iter().enumerate() {
            frame.render_widget(line.clone(), rows[8 + index]);
        }

        let after_landing = 8 + landing_lines.len();
        // rows[after_landing] is left blank — breathing space below the "lands as" preview.
        render_field(
            frame,
            rows[after_landing + 1],
            "recomputes",
            Line::from(recomputes_text(store, self.moving_id, &resolution)),
        );
        render_field(
            frame,
            rows[after_landing + 2],
            "transactions",
            Line::from(format!(
                "{} · unchanged, still this category",
                moving.transaction_count
            )),
        );
        render_field(
            frame,
            rows[after_landing + 3],
            "refuses",
            Line::from("its own descendants · the other root, while it has transactions"),
        );
        frame.render_widget(
            Block::new().borders(ratatui::widgets::Borders::BOTTOM),
            rows[after_landing + 4],
        );
        render_footer_hints(frame, rows[after_landing + 5]);
    }
}

/// The title row: "move" flush left, the `:cat move` command dim and right-aligned — echoing
/// `popup::unit::new`'s own title-row convention.
fn render_title(frame: &mut Frame, area: Rect) {
    let tag = ":cat move";
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);
    frame.render_widget(Paragraph::new("move"), columns[0]);
    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim)).alignment(Alignment::Right),
        columns[1],
    );
}

/// One `label   value` row — mirrors `popup::unit::new`'s own `render_field`.
fn render_field(frame: &mut Frame, area: Rect, label: &str, value: Line<'static>) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(LABEL_WIDTH), Constraint::Min(0)])
        .split(area);
    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(Paragraph::new(Span::styled(label, dim)), columns[0]);
    frame.render_widget(Paragraph::new(value), columns[1]);
}

/// The `new parent` field: the focused input box glyph (`┌`) per the handoff's drawn example,
/// the typed text, and a trailing accent cursor.
fn render_new_parent_field(frame: &mut Frame, area: Rect, input: &str) {
    let cursor = Style::default().fg(ACCENT);
    render_field(
        frame,
        area,
        "\u{250c} new parent",
        Line::from(vec![
            Span::raw(input.to_string()),
            Span::styled("\u{258c}", cursor),
        ]),
    );
}

/// The `completion` row beneath it: up to a few candidate names sharing the input's last
/// segment as a prefix, dim, with a trailing `· tab` hint.
fn render_completion_row(frame: &mut Frame, area: Rect, store: &dyn CategoryStore, input: &str) {
    let candidates = completions(store, input);
    let dim = Style::default().add_modifier(Modifier::DIM);
    let text = if candidates.is_empty() {
        "no matches · tab".to_string()
    } else {
        format!("{}  · tab", candidates.join(" · "))
    };
    render_field(
        frame,
        area,
        "\u{2514} completion",
        Line::from(Span::styled(text, dim)),
    );
}

/// The "lands as" heading: the label, and — once the input resolves to an existing category —
/// the depth the moving node would land at.
fn render_lands_as_heading(frame: &mut Frame, area: Rect, depth_tag: &str) {
    let dim = Style::default().add_modifier(Modifier::DIM);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(depth_tag.chars().count() as u16),
        ])
        .split(area);
    frame.render_widget(Paragraph::new(Span::styled("lands as", dim)), columns[0]);
    frame.render_widget(
        Paragraph::new(Span::styled(depth_tag.to_string(), dim)).alignment(Alignment::Right),
        columns[1],
    );
}

/// The "lands as" preview's own content lines — a simplified fragment (the resolved parent's
/// name, then its children with the moving node inserted at its sorted landing position, not
/// the handoff's full nested multi-level box-drawing tree) real enough to be "the check the
/// user actually performs", without re-implementing the tree pane's own recursive guide
/// renderer just for a preview.
fn landing_fragment_lines<'a>(
    store: &dyn CategoryStore,
    resolution: &Resolution,
    moving_id: RowID,
) -> Vec<Paragraph<'a>> {
    let dim = Style::default().add_modifier(Modifier::DIM);
    let accent = Style::default().fg(ACCENT);

    match resolution {
        Resolution::Existing(parent_id) => {
            if let Some(refusal) = store.validate_move(moving_id, *parent_id).err() {
                return vec![Paragraph::new(Span::styled(refusal.to_string(), accent))];
            }

            let moving_name = store
                .find(moving_id)
                .map(|node| node.name.clone())
                .unwrap_or_default();
            let mut names: Vec<String> = store
                .children(*parent_id)
                .iter()
                .map(|child| child.name.clone())
                .collect();
            names.push(moving_name.clone());
            names.sort_by_key(|name| name.to_lowercase());

            let parent_name = store
                .find(*parent_id)
                .map(|node| node.name.clone())
                .unwrap_or_default();
            let mut lines = vec![Paragraph::new(parent_name)];

            let shown = names.len().min(MAX_LANDING_CHILDREN);
            for name in &names[..shown] {
                if *name == moving_name {
                    lines.push(Paragraph::new(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(format!("{name} · moves here"), accent),
                    ])));
                } else {
                    lines.push(Paragraph::new(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(name.clone(), dim),
                    ])));
                }
            }
            if names.len() > shown {
                lines.push(Paragraph::new(Span::styled(
                    format!("  … {} more", names.len() - shown),
                    dim,
                )));
            }
            lines
        }
        Resolution::Creatable { parent, name } => {
            let parent_name = store
                .find(*parent)
                .map(|node| node.name.clone())
                .unwrap_or_default();
            vec![Paragraph::new(Span::styled(
                format!("^n creates \"{name}\" under {parent_name}, then lands there"),
                dim,
            ))]
        }
        Resolution::Invalid => vec![Paragraph::new(Span::styled(
            "type a full existing path, or one new segment to create with ^n",
            dim,
        ))],
    }
}

/// The `recomputes` row: how many ancestor chains' rollups would need recomputing — the
/// moving node's own old and new ancestor chains, per the handoff's "recomputes rollups on 3
/// ancestors". Informational only; `CategoryStore::move_to`'s rollups are always derived live,
/// so nothing is actually cached to invalidate — this states the handoff's own framing rather
/// than describing a real cache.
fn recomputes_text(store: &dyn CategoryStore, moving_id: RowID, resolution: &Resolution) -> String {
    let old_ancestors = store
        .find(moving_id)
        .and_then(|node| node.parent_id)
        .map(|parent_id| store.depth(parent_id))
        .unwrap_or(0);
    let new_ancestors = match resolution {
        Resolution::Existing(id) => store.depth(*id),
        Resolution::Creatable { parent, .. } => store.depth(*parent) + 1,
        Resolution::Invalid => 0,
    };
    format!("rollups on {} ancestors", old_ancestors.max(new_ancestors))
}

/// The window footer hint row — matches the handoff's own `tab` / `^n` / `^s` / `esc` key set.
fn render_footer_hints(frame: &mut Frame, area: Rect) {
    const HINTS: &[(&str, &str)] = &[
        ("tab", "complete parent"),
        ("^n", "new parent"),
        ("^s", "move"),
        ("esc", "cancel"),
    ];
    let key_style = Style::default().add_modifier(Modifier::BOLD);
    let label_style = Style::default().add_modifier(Modifier::DIM);

    let mut spans = Vec::with_capacity(HINTS.len() * 3);
    for (index, (key, label)) in HINTS.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(*key, key_style));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(*label, label_style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// `id`'s ancestor names, root to self, lowercase — the shared basis for both the `from`
/// field's spaced display and the `new parent` input's compact one.
fn ancestor_names(store: &dyn CategoryStore, id: RowID) -> Vec<String> {
    let mut names = Vec::new();
    let mut current = store.find(id);
    while let Some(node) = current {
        names.push(node.name.to_lowercase());
        current = node.parent_id.and_then(|parent_id| store.find(parent_id));
    }
    names.reverse();
    names
}

/// Resolves `input` (a `/`-separated path, matched case-insensitively segment by segment
/// against root names then descendant names) against `store`. Only ever reports one missing
/// segment as creatable — a path missing more than one level in a row is `Invalid`, per the
/// handoff's own single-level "`^n` creates a missing parent inline" (not a whole missing
/// chain).
fn resolve(store: &dyn CategoryStore, input: &str) -> Resolution {
    let segments: Vec<&str> = input
        .split('/')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect();
    let Some((first, rest)) = segments.split_first() else {
        return Resolution::Invalid;
    };

    let Some(mut current_id) = store
        .nodes()
        .iter()
        .find(|node| node.parent_id.is_none() && node.name.eq_ignore_ascii_case(first))
        .map(|node| node.id)
    else {
        return Resolution::Invalid;
    };

    for (index, segment) in rest.iter().enumerate() {
        let children = store.children(current_id);
        match children
            .iter()
            .find(|child| child.name.eq_ignore_ascii_case(segment))
        {
            Some(child) => current_id = child.id,
            None if index == rest.len() - 1 => {
                return Resolution::Creatable {
                    parent: current_id,
                    name: (*segment).to_string(),
                };
            }
            None => return Resolution::Invalid,
        }
    }

    Resolution::Existing(current_id)
}

/// Candidate category names sharing `input`'s last segment as a case-insensitive prefix,
/// sorted, deduplicated, capped to a handful — the `completion` row and what `Tab` accepts
/// the first of.
fn completions(store: &dyn CategoryStore, input: &str) -> Vec<String> {
    let last_segment = input.rsplit('/').next().unwrap_or("").to_lowercase();
    let mut names: Vec<String> = store
        .nodes()
        .iter()
        .filter(|node| node.name.to_lowercase().starts_with(&last_segment))
        .map(|node| node.name.clone())
        .collect();
    names.sort_by_key(|name| name.to_lowercase());
    names.dedup();
    names.truncate(4);
    names
}

/// Computes a centred popup `Rect` sized to `content_rows` plus its top/bottom border —
/// mirrors `popup::unit::delete`'s own dynamic `popup_rect`.
fn popup_rect(area: Rect, content_rows: u16) -> Rect {
    let width = POPUP_WIDTH.min(area.width);
    let x = area.x + (area.width.saturating_sub(width)) / 2;

    let height = (content_rows + 2).min(area.height);
    let anchor_y = area.y + (area.height / 6).max(1);
    let y = anchor_y.min(area.y + area.height.saturating_sub(height));

    Rect {
        x,
        y,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;
    use crate::category::CategoryFixture;

    fn find_by_name(store: &CategoryFixture, name: &str) -> RowID {
        store
            .nodes()
            .iter()
            .find(|node| node.name == name)
            .unwrap_or_else(|| panic!("fixture should seed a category named {name}"))
            .id
    }

    fn render(popup: &MovePopup, store: &dyn CategoryStore) -> String {
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| popup.render(frame, frame.area(), store))
            .expect("rendering the popup should not error");

        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }
        text
    }

    #[test]
    fn new_prefills_input_with_the_moving_nodes_current_parent_path() {
        let store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries");
        let popup = MovePopup::new(&store, groceries);
        assert_eq!(popup.input, "expenses/food");
    }

    #[test]
    fn renders_without_panicking() {
        let store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries");
        render(&MovePopup::new(&store, groceries), &store);
    }

    #[test]
    fn shows_the_moving_and_from_fields() {
        let store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries");
        let text = render(&MovePopup::new(&store, groceries), &store);

        assert!(
            text.contains("Groceries · leaf · 148 txns"),
            "moving field missing"
        );
        assert!(text.contains("expenses / food"), "from field missing");
    }

    #[test]
    fn resolves_an_existing_full_path() {
        let store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries");
        let transport = find_by_name(&store, "Transport");
        let mut popup = MovePopup::new(&store, groceries);
        popup.input = "expenses/transport".to_string();

        assert_eq!(popup.resolved_parent(&store), Some(transport));
    }

    #[test]
    fn resolves_a_creatable_missing_last_segment() {
        let store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries");
        let food = find_by_name(&store, "Food");
        let mut popup = MovePopup::new(&store, groceries);
        popup.input = "expenses/food/daily".to_string();

        assert_eq!(popup.creatable(&store), Some((food, "daily".to_string())));
        assert_eq!(popup.resolved_parent(&store), None);
    }

    #[test]
    fn an_unresolvable_path_is_neither_resolved_nor_creatable() {
        let store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries");
        let mut popup = MovePopup::new(&store, groceries);
        popup.input = "not/a/real/path/at/all".to_string();

        assert_eq!(popup.resolved_parent(&store), None);
        assert_eq!(popup.creatable(&store), None);
    }

    #[test]
    fn tab_completes_the_last_segment_to_the_first_alphabetical_match() {
        let store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries");
        let mut popup = MovePopup::new(&store, groceries);
        popup.input = "expenses/h".to_string();

        popup.tab_complete(&store);
        assert_eq!(popup.input, "expenses/Health");
    }

    #[test]
    fn shows_the_lands_as_preview_with_the_moving_node_marked() {
        let store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries");
        let mut popup = MovePopup::new(&store, groceries);
        popup.input = "expenses/transport".to_string();

        let text = render(&popup, &store);
        assert!(text.contains("lands as"), "lands as heading missing");
        assert!(
            text.contains("Groceries · moves here"),
            "moved-node marker missing"
        );
        // Transport's existing children (Fuel, Public Transit, Parking) should still be
        // listed for context.
        assert!(
            text.contains("Fuel"),
            "existing sibling missing from preview"
        );
    }

    #[test]
    fn shows_a_refusal_in_the_lands_as_preview_for_a_cycle() {
        let store = CategoryFixture::new();
        let food = find_by_name(&store, "Food");
        let groceries = find_by_name(&store, "Groceries");
        let mut popup = MovePopup::new(&store, food);
        popup.input = "expenses/food/groceries".to_string();

        let text = render(&popup, &store);
        assert!(
            text.contains("can't move into its own descendant set"),
            "cycle refusal missing from lands-as preview, got: {text}"
        );
        let _ = groceries;
    }

    #[test]
    fn shows_the_footer_hints() {
        let store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries");
        let text = render(&MovePopup::new(&store, groceries), &store);
        for key in ["tab", "^n", "^s", "esc"] {
            assert!(text.contains(key), "{key} hint missing");
        }
    }

    #[test]
    fn popup_rect_stays_within_the_terminal_area() {
        let area = Rect::new(0, 0, 96, 30);
        let popup = popup_rect(area, 20);
        assert!(popup.x + popup.width <= area.width);
        assert!(popup.y + popup.height <= area.height);
    }
}
