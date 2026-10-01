# Handoff: Notifications

## Overview
**Notifications** is Personal Ledger's single inbox for things that need the user: bills, cards, cash, budgets, imports and sync, and inventory. It also holds the **rules** that raise them. The destination has two tabs:

- **5a Inbox:** notifications grouped by day, a filter row, and a detail pane showing the numbers behind the alert, one primary action and the rule that raised it.
- **5b Rules:** every alert type as one row (when it fires, in-app vs desktop, on/off), with an editor for the selected rule and delivery settings that apply to all rules.

Frames are section 5 of `Ledger Desktop Shell.dc.html`.

## About the design files
`Notifications.dc.html` is a **high-fidelity HTML design reference**. It is not production code. Recreate it in the target codebase (**Rust + GPUI**) using that codebase's patterns. Open the file in a browser from this folder; it loads `support.js`, `styles.css` and `_ds_bundle.js` from the same folder.

The shell (header, primary rail, status bar) is specified in `design_handoff_shell_settings/`. Only what Notifications depends on is repeated here.

## Fidelity
**High fidelity.** Colours, type, spacing, rules and states are final. Sample data is illustrative (today = Mon 14 Sep 2026).

## Frame & shell
- **Frame:** 1280 × 800. Header 48px, body `flex:1`, status bar 28px.
- **Header breadcrumb:** `ledger › notifications`.
- **Primary rail (206px):** Notifications is active. It is the **last item in LEDGER**: Dashboard `g d` · Transactions `g l` · Documents `g f` · **Notifications `g m`**.
  - **Badge:** `background:#ec3013; color:#f3f2f2; 10px 800; padding:1px 5px; margin-right:4px`, placed before the binding.
  - **Count:** the badge counts **unread** by default. Rules › Badge counts can switch it to "Needs action only".
  - Hide the badge at 0. It shows on every destination, not only here.
- **Content pane:** `flex:1; min-width:0; padding:22px 28px; overflow:hidden; flex column`. There is no second rail.

## Page anatomy (both tabs)
1. **Page heading row:** `flex; align-items:flex-end; justify-content:space-between; gap:16px; margin-bottom:16px`.
   - Left: `h2` "Notifications" (28px/800, `margin:0 0 4px`), with a subline (11.5px #9b9797):
     - Inbox: "3 unread · 2 need action · kept for 90 days"
     - Rules: "13 rules · 12 on · checked after every sync and at 07:00"
   - Right: **Mark all read** and **Snooze all until tomorrow** (`.btn.btn-secondary`, 32px).
2. **Rule:** 2px `rgba(32,30,29,.38)`, `flex:none`, `margin-bottom:14px`.
3. **Toolbar:** `flex; align-items:center; justify-content:space-between; gap:12px; margin-bottom:12px`.
   - **Tab segmented control:** Inbox | Rules.
     - Segment: `padding:6px 12px; border:1px solid rgba(32,30,29,.30); 12px #605d5d`.
     - Active segment: `border-color:#201e1d; background:#201e1d; color:#f3f2f2; 800`.
     - Adjacent segments share borders (`border-left:none` after the first).
     - "Inbox" carries an inline red count chip (`#ec3013`, 10px, `padding:0 4px; margin-left:4px`).
   - **Inbox right:** a filter segmented control **All 9 · Unread 3 · Needs action 2 · Snoozed 1**, then a kind `select` ("Any kind"; 30px, `border:1px solid rgba(32,30,29,.30)`, 12px).
   - **Rules right:** "thresholds use your units · aud" (11.5px #9b9797).
4. **Body:** `flex:1; min-height:0; flex; gap:28px`. The list is `flex:3` (`align-self:flex-start`) and the side pane is `flex:2` (`flex column; gap:14px`).
5. **Status bar** (keys below).

---

## 5a · Inbox

### List
- **Container:** `border:1px solid rgba(32,30,29,.30); 12.5px`.
- **Group header:** `flex; padding:7px 12px; background:#eae9e9; 800 10px/1 Archivo; .11em; #605d5d; border-bottom:1px solid rgba(32,30,29,.30)`. The label is on the left and the group count on the right.
  - Groups: **TODAY · MON 14 SEP**, **THIS WEEK** (since Monday), **EARLIER IN SEPTEMBER** (the current month), then by month.

**Notification row:** `flex; gap:10px; align-items:flex-start; padding:6px 12px; border-bottom:1px solid #d7d3d3`. Its columns, in order:

| Column | Spec |
| --- | --- |
| Unread marker | 7×7 square, `margin-top:5px`. `#ec3013` when unread, transparent when read. |
| Kind | 70px, `800 10px/1 Archivo .11em #9b9797`, `margin-top:3px`: CARD · IMPORT · BUDGET · BILL · CASH · INVENTORY · SYNC. |
| Text | `flex:1; min-width:0; flex column; gap:2px`. The title is 12.5px, **800 when unread** and 400 when read. Detail line below it: 11.5px #605d5d, `text-wrap:pretty`. |
| Meta | Right-aligned column, gap 4. The time is 11px #9b9797 tabular: `HH:MM` today, weekday this week (`sat`), `DD mon` earlier. Below it, rows that need action show **ACTION** (10px 800 .06em `#ae1800`). |

- **Selected row:** `background:#e2e0df; box-shadow:inset 0 0 0 2px #201e1d`.
- **Snoozed row:** placed last, all text `#9b9797`, no marker. The detail line states the wake date ("Snoozed until 30 nov — decide on roll-over.").

**Sample rows**, in order:
1. CARD · Amex interest-free ends Fri 18 sep · unread · ACTION · *selected*
2. IMPORT · 4 ANZ transactions need a category · unread · ACTION
3. BUDGET · Groceries is at 92% with 16 days left · unread
4. BILL · Origin Energy is 18% higher than usual
5. CASH · Lowest point ahead: 72,118.40 on 30 sep
6. BILL · Home loan repayment paid
7. INVENTORY · 3 items over 1,000.00 have no receipt
8. SYNC · Sync server unreachable for 2 hours
9. CASH · Westpac term deposit matures 14 dec · snoozed

### Detail pane (selected notification)
- **Kicker:** `800 10px .11em`, `#ae1800` when the item needs action ("CREDIT CARD · NEEDS ACTION"), otherwise #9b9797.
- **Title:** `h3` 19px/800/1.2.
- **Explanation:** 12.5px #605d5d, line-height 1.45. It must say when the item clears ("This stays in **Needs action** until the payment is recorded, then clears itself.").
- **Facts table:** `border:1px solid rgba(32,30,29,.30); 12.5px`. Rows are `flex; padding:6px 12px; border-bottom:1px solid #d7d3d3` (none on the last), with the label `flex:1 #605d5d` and a tabular value. Sample:
  - Statement balance **2,318.44**
  - Due fri 18 sep · 4 days
  - Pay from ANZ Everyday · 4,182.55 → 1,864.11
  - If missed ≈ 46.35 interest a month
  - Free to spend: already counted
- **Actions:** one primary action named for the fix, plus **Snooze** and **Dismiss** (secondary). All are 32px, gap 8.
  - Primary action per kind:

    | Kind | Primary action |
    | --- | --- |
    | CARD | Pay card… |
    | IMPORT | Categorise |
    | BUDGET | Open budget |
    | BILL | Open bill / Pay… |
    | CASH | Open Cash / Transfer… |
    | INVENTORY | Add receipt… |
    | SYNC | Open Sync server |
- **Divider:** 2px `rgba(32,30,29,.20)`.
- **WHY YOU'RE SEEING THIS:** a caps label, then 11.5px #605d5d text naming the rule in 800 #201e1d, with its timing and delivery, then an **Edit rule** link (`#ec3013`; it jumps to 5b with that rule selected). A second line names the source record ("Opens from **Credit cards › Amex Platinum** statement, 31 aug.").

### Status bar (5a)
- **Left:** `NORMAL` · `j/k row · enter open source · p act · z snooze · x dismiss · R mark all read · tab rules`
- **Right:** "desktop alerts on · quiet 21:00–07:00"

---

## 5b · Rules

### Rules table
- **Container:** `border:1px solid rgba(32,30,29,.30); 12.5px`.
- **Column header:** `#eae9e9; 800 10px .11em #605d5d`. Columns: RULE `flex:1` · WHEN 150px · IN APP 52px (centred) · DESKTOP 60px (centred) · ON 32px (centred).
- **Group label row:** `padding:7px 12px 5px; 800 10px .11em #9b9797; border-bottom:1px solid #d7d3d3`.
- **Rule row:** `flex; align-items:center; padding:5px 12px; border-bottom:1px solid #d7d3d3`. The WHEN text is #605d5d.
- **Checkbox:** 14×14 square.
  - On: `#201e1d` fill with a `#f3f2f2` check (1.8 stroke).
  - Off: `border:1.5px solid rgba(32,30,29,.45)`.
- **Rule turned off:** the whole row turns #9b9797 and the IN APP / DESKTOP boxes render empty (the delivery choices are kept, just not applied).
- **Selected rule:** `background:#e2e0df; box-shadow:inset 0 0 0 2px #201e1d`, with the rule name in 800.

**Rules and defaults:**

| Group | Rule | When (default) | In app | Desktop | On |
| --- | --- | --- | --- | --- | --- |
| BILLS & CARDS | Bill due | 3 days before | ✓ | ✓ | ✓ |
| | Bill higher than usual | 15% over 12-month average | ✓ | – | ✓ |
| | Interest-free period ending | 5 days before due | ✓ | ✓ | ✓ |
| | Card near limit | 80% of limit | ✓ | ✓ | ✓ |
| CASH | Projected low below buffer | within 60 days | ✓ | ✓ | ✓ |
| | Large transaction | over 1,000.00 | ✓ | – | ✓ |
| | Term deposit maturing | 14 days before | ✓ | ✓ | ✓ |
| BUDGETS | Category nearing limit | at 90% | ✓ | – | ✓ |
| | Category over limit | at 100% | ✓ | ✓ | ✓ |
| IMPORTS & SYNC | Uncategorised after import | any | ✓ | – | ✓ |
| | Sync failing | for 2 hours | ✓ | ✓ | ✓ |
| INVENTORY | Item missing proof | value over 1,000.00 | ✓ | – | ✓ |
| | Warranty expiring | 30 days before | – | – | **off** |

### Side pane
**EDIT RULE** (caps label), with the rule name as an `h3` (19px/800). Then the fields:
- **Field label style:** 800 12px, 6px above the control. Inputs are 30px tall with `border:1px solid rgba(32,30,29,.30)`, square.
- **Notify:** a number input (64px, value `5`) followed by "days before the statement due date" (12.5px #605d5d). The field's shape follows the rule type: days before / percent / amount / duration.
- **Applies to:** a select, "All credit cards (1)". The scope depends on the rule (accounts, budgets, categories, items).
- **Deliver:** "In app" and "Desktop alert" checkboxes, gap 18.
- **Needs action:** a "Keep until the statement is paid" checkbox, with the hint "Clears itself when a payment covering the balance is recorded." (11.5px #605d5d). Only rules with a resolvable condition offer this.
- **Buttons:** **Save rule** (primary) and **Reset to default** (secondary), 32px.

After a 2px `rgba(32,30,29,.20)` divider comes **DELIVERY · ALL RULES**:
- **Quiet hours:** two 72px time inputs, `21:00` "to" `07:00`, with the hint "Desktop alerts wait until morning; the inbox still fills."
- **Badge counts:** a segmented control, **Unread** (active) | **Needs action only**.

### Status bar (5b)
- **Left:** `NORMAL` · `j/k rule · space on/off · a in app · d desktop · enter edit · tab inbox`
- **Right:** the desktop-permission state. The mock shows "macOS notifications allowed"; use platform wording (e.g. "desktop notifications allowed" on GNOME), and "desktop notifications blocked — allow…" when denied.

---

## Interactions & behaviour
- **Read vs resolved are separate.**
  - Selecting a row marks it read: the marker clears and the title drops to 400.
  - **Needs action** items keep their ACTION tag until the condition clears (the card is paid, the import is categorised), and then auto-resolve.
  - **Dismiss** removes an informational item; on a needs-action item it asks for confirmation.
- **Snooze (`z`):** opens a small menu: Later today (17:00) · Tomorrow 08:00 · Next week · Pick a date…. A snoozed item leaves its group, sits greyed at the bottom, and returns as unread at its wake time.
- **Primary action (`p`):** runs the item's fix in place (Pay card… opens the Credit cards payment dialog, 9b). **Enter** opens the source record instead.
- **Edit rule:** jumps to 5b with that rule selected. `tab` switches back to Inbox.
- **Rule evaluation:** rules run after every sync and once each morning (07:00).
  - A rule never raises a duplicate for the same source and period. It updates the existing item instead (e.g. Groceries 92% → 96% rewrites the detail line and its time).
- **Desktop alerts:** sent only for rules with Desktop ticked, outside quiet hours.
  - Alerts during quiet hours are held and delivered at the end of quiet hours, collapsed into one summary alert when there are more than 3.
  - Clicking a desktop alert focuses the app on that item.
- **Retention:** 90 days for read and resolved items. Unresolved needs-action items are never pruned.
- **Empty states:**
  - Inbox empty: "Nothing needs you." with a link to Rules.
  - A filter with no matches: a single line, "No snoozed notifications."
- **Rule edits** save only with **Save rule**. The on/off and delivery checkboxes in the table save immediately.

## Keyboard
| Key | Action |
| --- | --- |
| `g m` | Open Notifications (the Inbox tab). |
| `tab` | Switch Inbox ⇄ Rules. |
| `j` / `k` | Next / previous row. |
| `enter` | Inbox: open the source record. Rules: focus the rule editor. |
| `p` | Run the primary action. |
| `z` | Snooze. |
| `x` | Dismiss. |
| `R` | Mark all read. |
| `space` | Rules: turn the rule on or off. |
| `a` / `d` | Rules: toggle in-app / desktop delivery. |
| `esc` | Close the snooze menu, or leave the rule editor. |

Palette: `:notifications`, `:notifications rules`, `:snooze <when>`.

## State
```
notifications
  tab: Inbox | Rules
  filter: All | Unread | NeedsAction | Snoozed
  kind: Option<Kind>
  selectedId: Option<NotificationId>
  selectedRuleId: Option<RuleId>
  ruleDraft: Option<RuleDraft>          // the editor, saved on "Save rule"

Notification { id, ruleId, kind: Card|Import|Budget|Bill|Cash|Inventory|Sync,
               source: RecordRef, title, detail, facts: Vec<(label, value)>,
               createdAt, updatedAt, readAt?, resolvedAt?, dismissedAt?, snoozedUntil?,
               needsAction: bool, primaryAction: ActionId }
Rule         { id, group, name, threshold: Days(n)|Percent(n)|Amount(m)|Duration(h)|Any,
               scope: RuleScope, inApp: bool, desktop: bool, enabled: bool,
               keepUntilResolved: bool, isDefault: bool }
Delivery     { quietStart: Time, quietEnd: Time, badgeCounts: Unread|NeedsActionOnly }
```
- The badge count, the group counts and the filter counts are all derived.
- `needsAction` items resolve when the rule's condition re-evaluates false.
- Notifications and Rules are ledger data and sync. Desktop permission and quiet hours are device Configuration.

## Design tokens
- **Colour:**
  - Ground `#f3f2f2` · chrome `#eae9e9` · selection well `#e2e0df`.
  - Ink `#201e1d` · secondary `#605d5d` · tertiary `#9b9797` · on dark `#f3f2f2` / `#bab6b6`.
  - Accent `#ec3013` (unread marker, badges, links) · text-safe accent `#ae1800` (ACTION, needs-action kicker).
  - Rules: strong `rgba(32,30,29,.38)` · divider `rgba(32,30,29,.20)` · row `#d7d3d3` · border `rgba(32,30,29,.30)`.
- **Type:**
  - Archivo, weights 400 and 800.
  - Sizes 10 / 10.5 / 11 / 11.5 / 12 / 12.5 / 13 / 19 / 28px.
  - Caps labels `.11em`. Tabular numerals on every figure and time.
- **Spacing:** 2, 4, 5, 6, 7, 8, 9, 10, 12, 14, 16, 18, 22, 28px.
- **Radius:** 0 everywhere. **Shadows:** none (selection uses an inset 2px ring).
- **Focus:** `outline:2px solid #ec3013; outline-offset:2px`.
- **Components:** `.btn.btn-primary`, `.btn.btn-secondary` from Modernist (`styles.css`).

## Assets
- **Bell icon (rail):** Lucide-style, 14px, 1.5 stroke, drawn as `M4 11.5V7a4 4 0 0 1 8 0v4.5l1.5 1.5h-11z` plus `M6.5 14.5h3`.
- **Checkboxes, markers and segmented controls** are drawn with CSS. Implement them as native GPUI elements.
- No images.

## Files
- `Notifications.dc.html` — design reference: 5a Inbox, 5b Rules.
- `styles.css`, `_ds_bundle.js` — Modernist tokens and components.
- `support.js` — runtime for viewing the reference.
- `README.md` — this document.
