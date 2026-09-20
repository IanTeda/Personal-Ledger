# Research: virtualised Transactions list for the desktop app

**Question.** The desktop Transactions view needs a table of about 700 stub rows (later thousands), one row per transaction, columns status / flag / DATE / ACCOUNT / PAYEE / CATEGORY / TAGS / AMOUNT / RUNNING. It needs keyboard row selection (`j`/`k`, `g`/`G`, half-page) that keeps the selected row in view, a filter header, a column header and a footer that stay put while only the rows scroll, per-row hover and dark selected-row styling, and a "Row density" preference (compact / regular / roomy) that changes row height. The fallback to avoid is one plain `div` per row inside an `overflow_y_scroll` container. This note is primary-source research only: it reads `gpui` 0.2.2 and `gpui-component` 0.5.1 as vendored in the local cargo registry (`~/.cargo/registry/src/index.crates.io-*/`) and the existing `bin-desktop` code on the `concept` branch. Paths below are relative to `gpui-0.2.2/src/` or `gpui-component-0.5.1/src/` unless stated. Line numbers are for those exact versions. Nothing was compiled; the sketch in section 6 is compiling-in-principle only.

## 1. Recommendation

Hand-roll the list on `gpui::uniform_list` with a `UniformListScrollHandle`, and keep selection state in the view (not in `gpui_component::Table`). Reasons, in order of weight:

- **Keyboard ownership.** `bin-desktop`'s `Shell` already routes every key through a `gpui`-free `route_key` and a `Movement` enum (`Next`/`Prev`/`First`/`Last`/`HalfPageDown`/`HalfPageUp`, `crates/bins/bin-desktop/src/key_router.rs` on `concept`). `Table` instead binds its own `up`/`down`/`left`/`right`/`escape` actions in a `"Table"` key context (`table/mod.rs:19-28`, `:119`) and owns `selected_row` privately. It has no half-page or first/last API. We would be driving it from outside via `set_selected_row` and fighting its own bindings.
- **Theming cost.** `Table` paints from `cx.theme()` (gpui-component's `ActiveTheme`): `table`, `table_head`, `table_hover`, `table_active`, `table_active_border`, `table_row_border`, `table_even`, `border`, `radius` (`table/state.rs:643, 891-1131`, `table/mod.rs:118-125`). ADR-0016 scopes that theme to chart and table widgets only, and the shell uses hand-rolled zero-radius Modernist tokens (`theme.rs`). Adopting `Table` for the real Transactions view means either mapping every Modernist token into a second theme, or living with a visibly different table. The per-row hooks are narrow: `TableDelegate::render_tr` returns a `Stateful<Div>` whose style is merged in via `refine_style` (`table/state.rs:983-999`) but the selected/hover backgrounds are hard-coded from the theme.
- **Row height is not ours to set.** `Table` takes its row height from `Size::table_row_height()`, which is 26 / 30 / 32 / 40 px for XSmall / Small / Medium / Large (`styled.rs:250-257`). Density would have to be mapped onto that fixed set via `Table::with_size` (`table/mod.rs:95-101`), and roomy or compact values outside it are not possible.
- **What `Table` does give us for free** is sortable/resizable/movable columns, a sticky header, scrollbars, load-more and a visible-rows callback. The Transactions spec does not ask for column resize or reorder, and sorting is by a filter header we build ourselves.

`Table` remains the right answer for quick, theme-agnostic data grids (it is what `feasibility_demo.rs` proves). The hand-rolled list costs roughly 150 lines of row/header code we would otherwise have to restyle anyway, and `Table` itself is built on the same `uniform_list` (`table/state.rs:1325`), so nothing is lost at the virtualisation layer.

## 2. Q1 — How `uniform_list` is driven, and variable row height

**Signature** (`elements/uniform_list.rs:21-26`):

```rust
pub fn uniform_list<R: IntoElement>(
    id: impl Into<ElementId>,
    item_count: usize,
    f: impl 'static + Fn(Range<usize>, &mut Window, &mut App) -> Vec<R>,
) -> UniformList
```

- `item_count` is a plain `usize` passed on every render; the list is rebuilt every frame, so changing the count (a filter narrowing the rows) needs no special call.
- The closure receives the *visible* index range and returns exactly one element per index. It is `'static + Fn`, so it cannot borrow the view: capture an `Rc`/`Arc` clone of the data, or use `cx.processor(|this, range, window, cx| ...)` (`app/context.rs:262-272`), which is what `gpui-component`'s `Table` does (`table/state.rs:1325-1330`).
- The base style already sets `overflow.y = Scroll` (`:32`). The default sizing behaviour is `ListSizingBehavior::Auto` (`elements/list.rs:99-105`), meaning the list takes its size from the flex layout around it, not from its items; `Infer` sizes it to `item_count * row_height` clamped to available height (`uniform_list.rs:268-293`).

**The fixed-height requirement.** The module doc says it plainly (`:1-5`): the list "measures the first element and then lays out all remaining elements in a line based on that measurement". `measure_item` (`:650-672`) calls the render closure with `ix..ix+1` (item 0 by default, or `with_width_from_item`, `:614-617`) and lays it out with `AvailableSpace::MinContent` height. That single measured height is then used for content height (`:344-347`), scroll maths and every item's origin (`:479`) and forced item height (`:486-490`). Consequences:

- Every row must have a definite height: give the row `.h(px(..))`. A row whose height depends on its content will be measured from row 0 only and clipped or gapped elsewhere.
- The render closure is called with a one-item range twice per frame for measuring (`request_layout` at `:262` and `prepaint` at `:338`), in addition to the real visible range. It must be a pure function of the index and must not, for example, count "rendered rows" as a side effect.

**Density-dependent row height is workable, with no list rebuild.** Because the row height is re-measured from the closure every frame, a height derived from a setting simply works: read the current density into a local (`let row_h = self.density.row_height();`) before building the list and capture it in the closure (`.h(row_h)`). Nothing else needs recreating; keep the same element id and the same `UniformListScrollHandle`. One catch: the scroll offset is stored in pixels (`ScrollHandle::offset`, `elements/div.rs:3083-3086`), so after a density change the same pixel offset points at a different row. Re-anchor by calling `scroll_to_item_strict(selected, ScrollStrategy::Center)` (`uniform_list.rs:159-166`) on the same frame as the preference change. The density change is not a stale-state hazard within the list, but it is a re-anchoring one.

## 3. Q2 — Scrolling a row into view, and reading the visible range

**Scroll into view.** `UniformListScrollHandle` (`:77-80`) is `Rc<RefCell<UniformListScrollState>>`, held in the view and passed to the list with `.track_scroll(handle.clone())` (`:675-679`) each frame. Its methods (`:141-209`):

- `scroll_to_item(ix, strategy)`: non-strict; does nothing if the item is already fully visible.
- `scroll_to_item_strict(ix, strategy)`: always positions per the strategy, even if visible.
- `_with_offset` variants shrink the effective viewport by N rows from the edge the strategy points at (useful for "scrolloff").

These only store a `DeferredScrollToItem` (`:97-107`); the scroll is applied in the next `prepaint` (`:351-358`, `:393-448`). The view must therefore also call `cx.notify()`.

**`ScrollStrategy`** (`:83-95`) has three variants: `Top`, `Center`, `Bottom`. The important detail is how non-strict scrolling combines with the strategy. Reading `:393-448`: the code first computes a minimal correction (item above the viewport goes to the top edge, item below goes to the bottom edge), and then applies `strategy` only when the item was outside the viewport *before* that correction (`:414-418`, using the old `scroll_top`). So for an item that was already fully visible, nothing happens; for an item that was off-screen, the strategy chooses the final placement. The trap: `scroll_to_item(ix + 1, Top)` on `j` past the bottom edge will put the new row at the *top* of the viewport, jumping a full page. The correct pairing for keyboard stepping is `Bottom` when moving down and `Top` when moving up, which is exactly what `gpui-component`'s own `TableState::set_selected_row` does (`table/state.rs:228-240`). Use `Center` for jumps (`g`/`G`, half-page, density re-anchor) if a centred landing is wanted.

**Reading the visible range.** There is no public visible-range getter on `UniformListScrollHandle`. `ScrollHandle::top_item()`/`bottom_item()` (`elements/div.rs:3093-3129`) binary-search `child_bounds`, which is only populated for a normal `div` scroll container's children, so they are useless for a `uniform_list`. What is available, all through `handle.0.borrow()` (the field is `pub`, `:80`):

- `base_handle.offset()` (`div.rs:3083`): current scroll offset; `offset.y` is `<= 0`.
- `base_handle.bounds()` (`div.rs:3131`): list bounds, written each prepaint (`div.rs:1761-1764`); `bounds.size.height` is the viewport height. It is zero until the first paint.
- `last_item_size` (`:115`, written at `:353-356`): `.item` is the padded viewport size, `.contents` the total content size.

The list computes its own range as `first = floor(-offset.y / row_h)`, `last = ceil((-offset.y + viewport_h) / row_h)` (`:450-457`); the view can reproduce that. Half-page movement is then `page = (viewport_h / row_h).floor().max(1.0) as usize`, `selected ± page` clamped to `0..len`. `is_scrollable()` (`:227-233`) is also public.

## 4. Q3 — Composing inside a page with fixed header, column header and footer

`UniformList` is a `Styled + InteractiveElement` element (`:236-240`, `:706-709`), so it takes flex styles like any `div`. It scrolls internally, so the page is a flex column where only the list flexes:

```text
v_flex (page)   size_full, overflow_hidden
├─ filter header       flex_none (fixed height)
├─ column header       flex_none (fixed height)
├─ uniform_list        flex_1, min_h_0   <- the only scrolling part
└─ footer              flex_none
```

- Give the list `flex_1()` (`styled.rs:165` and neighbours) so it takes the leftover height, plus `min_h_0()` so a flex item is allowed to shrink below its content height. Without `min_h_0` a tall list can push the footer out of the window. Set `overflow_hidden()` on the page column so nothing bleeds.
- Header, column header and footer must not shrink: use `flex_none()` (or `flex_shrink_0()`, `styled.rs:221`) with fixed heights.
- Keep the default `ListSizingBehavior::Auto` (`list.rs:99-105`): with `Infer`, a short list would report `item_count * row_h` and the footer would float up under the last row instead of sitting at the bottom.
- `track_scroll` binds the handle so the list writes `bounds`/`max_offset` into it (`div.rs:1728-1764`), which is what makes section 3's range reads work.
- The column header must line up with row cells because it is outside the scroll area. Use one shared column-width definition (fixed `px` widths, or `flex_basis`-style widths) applied to both header and row cells. A scrollbar in the list will steal width from rows but not from the header; `gpui-component`'s own `Table` avoids this by rendering its header inside the same container and reading column bounds back (`table/state.rs:1327-1330` comment). Budget for it, or reserve a scrollbar gutter on the header.

## 5. Q4 — `gpui_component::Table` versus hand-rolled

`Table` is `RenderOnce` over an `Entity<TableState<D>>` (`table/mod.rs:60-134`), with `D: TableDelegate` (`table/delegate.rs`) providing `columns_count`, `rows_count`, `column`, `render_td`, and optionally `render_tr`, `render_th`, `render_empty`, `perform_sort`, `move_column`, `load_more`, `visible_rows_changed`. Internally it is a `uniform_list` (`table/state.rs:1325-1364`) with `ListSizingBehavior::Auto` and `track_scroll(self.vertical_scroll_handle.clone())`, so its virtualisation and scroll-into-view behaviour are the same as section 2/3 describe.

| Concern | Hand-rolled `uniform_list` | `gpui_component::Table` |
|---|---|---|
| Virtualisation | Yes, direct | Yes (same `uniform_list`) |
| Row height / density | Any `px` value from our setting | Fixed set of four via `Size` (`styled.rs:250-257`) |
| Keyboard ownership | Ours; fits `route_key`/`Movement` | Own `up`/`down`/`left`/`right`/`escape` bindings in a `"Table"` key context; half-page and `g`/`G` absent; must call `set_selected_row` from outside |
| Selection, hover styling | Ours (Modernist tokens) | Theme tokens `table_active`, `table_hover`, `table_even`, hard-coded in `render_table_row` (`table/state.rs:983-1131`); only partially overridable via `render_tr` |
| Fixed header / footer | We compose | Header is built in; footer and filter header are ours regardless |
| Sorting, resize, reorder, load-more | We build (not needed now) | Built in |
| Theming | Zero extra cost | Needs gpui-component `Theme` populated and initialised; visual mismatch with the zero-radius Modernist look unless tokens are mapped (ADR-0016 scopes that theme to charts and tables only) |
| Code volume | More (row, header, scroll glue) | Less, but a delegate plus a second theme |

The trade-off favours hand-rolled once the user-visible requirements (density, keyboard model, Modernist styling) are all things `Table` either hard-codes or duplicates. If a future screen needs a plain sortable, resizable grid with no Modernist constraints, `Table` is the better tool.

## 6. Code sketch

Compiling in principle against gpui 0.2.2; `Transaction`, `Density`, `theme::*` and `render_row` are the app's own. Keys arrive as a `Movement` already parsed by `route_key`.

```rust
use std::{ops::Range, rc::Rc};
use gpui::{
    Context, IntoElement, Render, ScrollStrategy, UniformListScrollHandle, Window, div,
    prelude::*, px, uniform_list,
};

pub struct TransactionsView {
    rows: Rc<Vec<Transaction>>,       // 'static-capturable snapshot of the filtered rows
    selected: usize,
    density: Density,                 // compact / regular / roomy
    scroll: UniformListScrollHandle,  // create once with ::new()
}

impl Density {
    fn row_height(self) -> gpui::Pixels {
        match self { Self::Compact => px(24.), Self::Regular => px(30.), Self::Roomy => px(38.) }
    }
}

impl TransactionsView {
    fn row_height(&self) -> gpui::Pixels { self.density.row_height() }

    /// Viewport height in rows; 0 before the first paint, so clamp to 1.
    fn page_rows(&self) -> usize {
        let vh = self.scroll.0.borrow().base_handle.bounds().size.height;
        ((vh / self.row_height()).floor() as usize).max(1)
    }

    pub fn apply(&mut self, m: Movement, cx: &mut Context<Self>) {
        let last = self.rows.len().saturating_sub(1);
        let page = self.page_rows();
        let (next, strategy) = match m {
            Movement::Next         => ((self.selected + 1).min(last), ScrollStrategy::Bottom),
            Movement::Prev         => (self.selected.saturating_sub(1), ScrollStrategy::Top),
            Movement::First        => (0, ScrollStrategy::Top),
            Movement::Last         => (last, ScrollStrategy::Bottom),
            Movement::HalfPageDown => ((self.selected + page / 2).min(last), ScrollStrategy::Center),
            Movement::HalfPageUp   => (self.selected.saturating_sub(page / 2), ScrollStrategy::Center),
            Movement::Enter        => return,
        };
        self.selected = next;
        // Non-strict: no-op while the row is already visible (uniform_list.rs:146-153).
        self.scroll.scroll_to_item(next, strategy);
        cx.notify(); // the scroll is applied in the next prepaint
    }

    pub fn set_density(&mut self, d: Density, cx: &mut Context<Self>) {
        self.density = d;
        // Scroll offset is in pixels, so re-anchor after the height changes.
        self.scroll.scroll_to_item_strict(self.selected, ScrollStrategy::Center);
        cx.notify();
    }
}

impl Render for TransactionsView {
    fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let row_h = self.row_height();
        let selected = self.selected;
        let rows = self.rows.clone();

        let list = uniform_list(
            "transactions-rows",
            rows.len(),
            move |range: Range<usize>, _w, _cx| {
                // Pure function of the index: also called with ix..ix+1 to measure row 0.
                range
                    .map(|ix| render_row(ix, &rows[ix], ix == selected, row_h))
                    .collect::<Vec<_>>()
            },
        )
        .track_scroll(self.scroll.clone())
        .flex_1()
        .min_h_0();

        div()
            .flex().flex_col().size_full().overflow_hidden()
            .child(div().flex_none().h(px(40.)).child("filter header"))
            .child(div().flex_none().h(px(28.)).child("column header (same widths as rows)"))
            .child(list)
            .child(div().flex_none().h(px(28.)).child("footer"))
    }
}

/// Each row needs a definite height and an id; hover and selected styling come from our tokens.
fn render_row(ix: usize, tx: &Transaction, selected: bool, h: gpui::Pixels) -> impl IntoElement {
    div()
        .id(("tx-row", ix))
        .flex().flex_row().items_center().w_full().h(h)
        .when(selected, |d| d.bg(theme::INK).text_color(theme::INK_ON_DARK))
        .when(!selected, |d| d.hover(|d| d.bg(theme::HOVER_TINT)))
        // ... one child per column, with fixed widths shared with the column header
}
```

Focus and routing: the view is reached from `Shell`'s existing `handle_key_down` (`Movement` from `route_key`) and stays focus-free itself; no `track_focus` on the list is required.

## 7. Open risks

1. **Scroll-strategy jump (biggest).** Non-strict `scroll_to_item` applies `Top`/`Bottom`/`Center` to the *final* placement whenever the target was off-screen (`uniform_list.rs:414-418`). Getting the direction-to-strategy pairing wrong (or using strict) makes the selection jump by a page on every step at the viewport edge. Cover with an on-screen check: step off the bottom edge and confirm the row lands at the bottom.
2. **Visible-range reads are unofficial.** They rely on the `pub` `Rc<RefCell<UniformListScrollState>>` and the `base_handle` fields, which `gpui` 0.2.x may change (the crate is pre-1.0 and Zed-internal). Isolate them in one helper. `bounds()` is zero before the first paint, so the first half-page press can give 0; clamp.
3. **Header/row width drift.** The column header is outside the scroll area, so a vertical scrollbar or fractional widths can misalign columns. Use one shared width table and reserve a gutter.
4. **`'static` closure and data snapshots.** The list closure cannot borrow the view; capturing an `Rc<Vec<_>>` means a filter edit must swap the `Rc` (cheap) rather than mutate in place. Thousands of rows are fine; verify allocation cost of cloning the `Rc` per frame is nil (it is a refcount bump).
5. **Density height re-anchoring.** Pixel scroll offsets do not survive a row-height change, so `set_density` must re-anchor as sketched; also verify the selected row is still fully visible after a switch to roomy.
6. **Per-frame measurement call.** The closure is invoked for row 0 twice per frame just to measure (`:262`, `:338`). If `render_row` gets expensive (formatting money, tag chips) precompute display strings per row rather than in the closure.
7. **Not compiled or run.** The sketch and the strategy analysis come from reading source; no rendering was tried. A spike with 700 stub rows should confirm section 3's `min_h_0` layout and section 2's density behaviour before committing.
