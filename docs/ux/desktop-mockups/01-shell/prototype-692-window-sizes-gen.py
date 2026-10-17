"""PROTOTYPE generator for the 01-shell Window Size section (#692). Throwaway: prints HTML to stdout."""

INK, INK2, INK3 = "#201e1d", "#605d5d", "#9b9797"
RULE = "rgba(32,30,29,.38)"
LABEL = "font:800 10px/1 'Archivo';letter-spacing:.11em;color:#9b9797"

ICON = {
    "dashboard": '<rect x="1.5" y="1.5" width="5.5" height="6.5"></rect><rect x="9" y="1.5" width="5.5" height="4"></rect><rect x="1.5" y="10" width="5.5" height="4.5"></rect><rect x="9" y="7.5" width="5.5" height="7"></rect>',
    "transactions": '<line x1="2" y1="4" x2="14" y2="4"></line><line x1="2" y1="8" x2="14" y2="8"></line><line x1="2" y1="12" x2="10" y2="12"></line>',
    "documents": '<path d="M3.5 1.5h6l3 3v10h-9z"></path><polyline points="9.5,1.5 9.5,4.5 12.5,4.5"></polyline><line x1="5.5" y1="8" x2="10.5" y2="8"></line><line x1="5.5" y1="11" x2="10.5" y2="11"></line>',
    "events": '<path d="M3 14.5V1.5"></path><path d="M3 2h9.5l-2 3 2 3H3"></path>',
    "notifications": '<path d="M4 11.5V7a4 4 0 0 1 8 0v4.5l1.5 1.5h-11z"></path><path d="M6.5 14.5h3"></path>',
    "cash": '<rect x="1.5" y="3.5" width="13" height="10"></rect><path d="M1.5 6.5h10.5"></path><circle cx="11.5" cy="10" r="1"></circle>',
    "inventory": '<path d="M1.5 5L8 1.5 14.5 5v6L8 14.5 1.5 11z"></path><polyline points="1.5,5 8,8.5 14.5,5"></polyline><line x1="8" y1="8.5" x2="8" y2="14.5"></line>',
    "loans": '<line x1="13" y1="3" x2="3" y2="13"></line><circle cx="4.5" cy="4.5" r="2"></circle><circle cx="11.5" cy="11.5" r="2"></circle>',
    "credit": '<rect x="1.5" y="3.5" width="13" height="9"></rect><line x1="1.5" y1="6.5" x2="14.5" y2="6.5"></line><line x1="4" y1="10" x2="7.5" y2="10"></line>',
    "investments": '<line x1="3" y1="14" x2="3" y2="10"></line><line x1="8" y1="14" x2="8" y2="6.5"></line><line x1="13" y1="14" x2="13" y2="2.5"></line>',
    "bills": '<line x1="2" y1="3" x2="14" y2="3"></line><line x1="2" y1="8" x2="14" y2="8"></line><line x1="2" y1="13" x2="14" y2="13"></line>',
    "budgets": '<rect x="1.5" y="5" width="13" height="6"></rect><line x1="10" y1="3" x2="10" y2="13"></line>',
    "reports": '<polyline points="2,12 6,7 9,10 14,3"></polyline>',
    "settings": '<path d="M8 1.5v3M8 11.5v3M1.5 8h3M11.5 8h3M3.4 3.4l2.1 2.1M10.5 10.5l2.1 2.1M12.6 3.4l-2.1 2.1M5.5 10.5l-2.1 2.1"></path><circle cx="8" cy="8" r="2.2"></circle>',
    # Settings pages
    "general": '<line x1="2" y1="4" x2="14" y2="4"></line><line x1="2" y1="12" x2="14" y2="12"></line><rect x="4" y="2.5" width="3" height="3"></rect><rect x="9" y="10.5" width="3" height="3"></rect>',
    "display": '<rect x="1.5" y="2.5" width="13" height="9"></rect><line x1="5.5" y1="14" x2="10.5" y2="14"></line><line x1="8" y1="11.5" x2="8" y2="14"></line>',
    "units": '<circle cx="8" cy="8" r="6.5"></circle><path d="M10 5.5H7a1.25 1.25 0 0 0 0 2.5h2a1.25 1.25 0 0 1 0 2.5H6M8 4v1.5M8 10.5V12"></path>',
    "institutions": '<path d="M1.5 5.5 8 2l6.5 3.5z"></path><line x1="3.5" y1="7" x2="3.5" y2="12"></line><line x1="8" y1="7" x2="8" y2="12"></line><line x1="12.5" y1="7" x2="12.5" y2="12"></line><line x1="1.5" y1="14" x2="14.5" y2="14"></line>',
    "accounts": '<rect x="1.5" y="3.5" width="13" height="10"></rect><path d="M1.5 6.5h10.5"></path><circle cx="11.5" cy="10" r="1"></circle>',
    "categories": '<rect x="1.5" y="1.5" width="5" height="5"></rect><rect x="9.5" y="1.5" width="5" height="5"></rect><rect x="1.5" y="9.5" width="5" height="5"></rect><rect x="9.5" y="9.5" width="5" height="5"></rect>',
    "tags": '<path d="M8.5 1.5H14.5V7.5L8 14 1.5 7.5z"></path>',
    "payees": '<circle cx="8" cy="5.5" r="2.75"></circle><path d="M2.5 14c0-2.8 2.5-4.5 5.5-4.5s5.5 1.7 5.5 4.5"></path>',
    "sync": '<path d="M13.5 6.5A5.5 5.5 0 0 0 3 5M2.5 9.5A5.5 5.5 0 0 0 13 11"></path><polyline points="3,1.5 3,5 6.5,5"></polyline><polyline points="13,14.5 13,11 9.5,11"></polyline>',
    "backup": '<ellipse cx="8" cy="3.5" rx="5.5" ry="2"></ellipse><path d="M2.5 3.5v9c0 1.1 2.5 2 5.5 2s5.5-.9 5.5-2v-9M2.5 8c0 1.1 2.5 2 5.5 2s5.5-.9 5.5-2"></path>',
    "logs": '<rect x="1.5" y="2.5" width="13" height="11"></rect><polyline points="4,6 6,8 4,10"></polyline><line x1="7.5" y1="10.5" x2="11.5" y2="10.5"></line>',
    "about": '<circle cx="8" cy="8" r="6.5"></circle><line x1="8" y1="7" x2="8" y2="11.5"></line><line x1="8" y1="4.5" x2="8" y2="5.2"></line>',
}

RAIL = [
    ("LEDGER", [("dashboard", "Dashboard", "g d"), ("transactions", "Transactions", "g l"), ("documents", "Documents", "g f"), ("events", "Events", "g e"), ("notifications", "Notifications", "g m")]),
    ("NET WORTH", [("cash", "Cash", "g c"), ("inventory", "Inventory", "g o"), ("loans", "Loans", "g n"), ("credit", "Credit cards", "g k"), ("investments", "Investments", "g i")]),
    ("PLAN", [("bills", "Bills", "g w"), ("budgets", "Budgets", "g b"), ("reports", "Reports", "g r")]),
]

# (icon, full label, short label, one-line description) in SettingsPage order (crates/bins/bin-desktop/src/settings/page.rs)
PAGES = [
    ("general", "General", "General", "ledger name, owner and start of the financial year"),
    ("display", "Display", "Display", "date format, separators, row density, colour theme"),
    ("units", "Units", "Units", "currencies and other units, and the default"),
    ("institutions", "Institutions", "Institutions", "banks, brokers and lenders"),
    ("accounts", "Accounts", "Accounts", "every account, its institution and its unit"),
    ("categories", "Categories", "Categories", "income and expense categories"),
    ("tags", "Tags", "Tags", "free labels across transactions"),
    ("payees", "Payees", "Payees", "who you pay and who pays you"),
    ("documents", "Documents", "Documents", "document types and where files are kept"),
    ("inventory", "Inventory", "Inventory", "item types and depreciation"),
    ("sync", "Sync server", "Sync", "server address, sign-in, interval"),
    ("backup", "Data &amp; backup", "Backup", "backups, export and the ledger file"),
    ("logs", "Tracing (Logs)", "Logs", "log level and the live log"),
    ("about", "About", "About", "version, licences and credits"),
]


def svg(name, size=14, stroke=INK):
    return f'<svg width="{size}" height="{size}" viewBox="0 0 16 16" fill="none" stroke="{stroke}" stroke-width="1.5" style="flex:none">{ICON[name]}</svg>'


def k(key):
    return f'<span style="color:{INK};font-weight:800">{key}</span>'


def header(crumb, sync=True, crumb_html=None):
    ctrl = lambda inner, fill="": f'<div style="width:26px;height:26px;display:flex;align-items:center;justify-content:center;background:rgba(32,30,29,.08)"><svg width="10" height="10" viewBox="0 0 10 10" {fill} stroke="{INK}" stroke-width="1.4">{inner}</svg></div>'
    crumb_html = crumb_html or f'<span style="color:{INK3};font-size:12px;white-space:nowrap">{crumb}</span>'
    s = f'''
  <div style="height:48px;flex:none;display:flex;align-items:center;gap:12px;padding:0 10px 0 12px;background:#eae9e9;border-bottom:2px solid {RULE}">
    <div style="width:28px;height:28px;flex:none;display:flex;align-items:center;justify-content:center"><svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="{INK}" stroke-width="1.5"><rect x="1.5" y="2.5" width="13" height="11"></rect><line x1="6" y1="2.5" x2="6" y2="13.5"></line></svg></div>
    <div style="display:flex;align-items:baseline;gap:8px;min-width:0;flex:1;overflow:hidden">
      <span style="font-weight:800;font-size:13.5px;letter-spacing:-.01em;flex:none">Personal Ledger</span>
      {crumb_html}
    </div>'''
    if sync:
        s += f'''
    <div style="display:flex;align-items:center;gap:6px;color:{INK2};font-size:12px;flex:none"><svg width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="#2ecc71" stroke-width="2"><polyline points="3,8.5 6.5,12 13,4.5"></polyline></svg><span>Synced @ 14:22</span></div>'''
    s += f'''
    <div style="display:flex;gap:2px;margin-left:10px;flex:none">{ctrl('<line x1="1" y1="5" x2="9" y2="5"></line>')}{ctrl('<rect x="1.2" y="1.2" width="7.6" height="7.6"></rect>', 'fill="none"')}{ctrl('<line x1="1.5" y1="1.5" x2="8.5" y2="8.5"></line><line x1="8.5" y1="1.5" x2="1.5" y2="8.5"></line>')}</div>
  </div>'''
    return s


def full_rail(active):
    s = f'\n    <div style="width:206px;flex:none;background:#eae9e9;border-right:2px solid {RULE};display:flex;flex-direction:column;padding:14px 0 10px">'
    for gi, (group, items) in enumerate(RAIL):
        s += f'\n      <div style="{LABEL};padding:{16 if gi else 0}px 14px 8px">{group}</div>'
        for icon, label, key in items:
            s += rail_item(icon, label, key, icon == active)
    s += f'\n      <div style="flex:1"></div>\n      <div style="height:2px;background:rgba(32,30,29,.20);margin:8px 0"></div>'
    s += rail_item("settings", "Settings", "g s", active == "settings")
    return s + "\n    </div>"


def rail_item(icon, label, key, on):
    badge = f'<span style="background:#ec3013;color:#f3f2f2;font-size:10px;font-weight:800;padding:1px 5px;margin-right:4px">3</span>' if icon == "notifications" else ""
    if on:
        return f'\n      <div style="display:flex;align-items:center;gap:9px;padding:7px 14px;background:{INK};color:#f3f2f2">{svg(icon, stroke="#f3f2f2")}<span style="font-weight:800;flex:1">{label}</span><span style="font-size:11px;color:#bab6b6">{key}</span></div>'
    return f'\n      <div style="display:flex;align-items:center;gap:9px;padding:7px 14px">{svg(icon)}<span style="flex:1">{label}</span>{badge}<span style="font-size:11px;color:{INK3}">{key}</span></div>'


def icon_rail(active):
    s = f'\n    <div style="width:52px;flex:none;background:#eae9e9;border-right:2px solid {RULE};display:flex;flex-direction:column;align-items:center;padding:12px 0;gap:2px">'
    for gi, (_, items) in enumerate(RAIL):
        if gi:
            s += '\n      <div style="width:24px;height:1px;background:rgba(32,30,29,.20);margin:5px 0"></div>'
        for icon, _, _ in items:
            s += icon_cell(icon, icon == active)
    s += '\n      <div style="flex:1"></div>'
    s += icon_cell("settings", active == "settings")
    return s + "\n    </div>"


def icon_cell(icon, on):
    dot = '<span style="position:absolute;top:3px;right:3px;width:6px;height:6px;background:#ec3013"></span>' if icon == "notifications" else ""
    bg = f"background:{INK};" if on else ""
    return f'\n      <div style="position:relative;width:36px;height:32px;flex:none;display:flex;align-items:center;justify-content:center;{bg}">{svg(icon, 16, "#f3f2f2" if on else INK)}{dot}</div>'


def settings_index(active="display", width=232, filter_box=False, footer=True):
    s = f'\n    <div style="width:{width}px;flex:none;border-right:2px solid {RULE};display:flex;flex-direction:column;background:#f3f2f2">'
    s += f'\n      <div style="padding:14px 14px 8px;{LABEL};color:{INK}">SETTINGS</div>\n      <div style="height:2px;flex:none;background:{RULE}"></div>'
    if filter_box:
        s += f'\n      <div style="margin:10px 14px 4px;height:30px;box-sizing:border-box;display:flex;align-items:center;gap:6px;font-size:12.5px;padding:0 10px;border:1px solid rgba(32,30,29,.30);color:{INK3}"><span style="color:{INK};font-weight:800">/</span>filter pages</div>'
    for icon, label, _, _ in PAGES:
        if icon == active:
            s += f'\n      <div style="padding:7px 14px;background:{INK};color:#f3f2f2;font-weight:800">{label}</div>'
        else:
            s += f'\n      <div style="padding:7px 14px">{label}</div>'
    s += '\n      <div style="flex:1"></div>'
    if footer:
        s += f'\n      <div style="padding:10px 14px;border-top:1px solid #d7d3d3;color:{INK3};font-size:11px">preferences sync · configuration local</div>'
    return s + "\n    </div>"


def narrow_index(active="display"):
    s = f'\n    <div style="width:120px;flex:none;border-right:2px solid {RULE};display:flex;flex-direction:column;background:#f3f2f2">'
    s += f'\n      <div style="padding:14px 12px 8px;{LABEL};color:{INK}">SETTINGS</div>\n      <div style="height:2px;flex:none;background:{RULE}"></div>'
    for icon, _, short, _ in PAGES:
        on = icon == active
        st = f"background:{INK};color:#f3f2f2;font-weight:800;" if on else ""
        s += f'\n      <div style="display:flex;align-items:center;gap:8px;padding:7px 12px;font-size:12px;white-space:nowrap;{st}">{svg(icon, 14, "#f3f2f2" if on else INK)}<span style="overflow:hidden;text-overflow:ellipsis">{short}</span></div>'
    s += '\n      <div style="flex:1"></div>'
    return s + "\n    </div>"


def display_form(stack, name):
    radio = lambda group, opts, sel: '<div class="seg">' + "".join(f'<label class="seg-opt"><input type="radio" name="{group}-{name}"{" checked=\"checked\"" if i == sel else ""}>{o}</label>' for i, o in enumerate(opts)) + "</div>"
    form = f'''
        <div style="width:300px;flex:none;display:flex;flex-direction:column;gap:18px">
          <div class="field"><label>Date format</label>{radio("df", ["12 sep 2026", "12/09/2026", "ISO"], 0)}</div>
          <div class="field"><label>Decimal &amp; thousands separator</label>{radio("ds", ["1,234.56", "1.234,56", "1 234.56"], 0)}</div>
          <div class="field"><label>Row density</label>{radio("rd", ["compact", "regular", "roomy"], 1)}</div>
          <div class="field"><label>Status glyphs</label><div style="display:flex;flex-direction:column;gap:7px;margin-top:2px"><label class="radio"><input type="radio" name="gl-{name}" checked="checked"><span class="dot"></span>unicode — ○ ◐ ● ⚑</label><label class="radio"><input type="radio" name="gl-{name}"><span class="dot"></span>ascii fallback — o / x !</label></div></div>
        </div>'''
    row = lambda g, d, p, a, last=False, gc=INK: f'<div style="display:flex;padding:8px 12px;font-variant-numeric:tabular-nums;{"" if last else "border-bottom:1px solid #d7d3d3"}"><span style="width:18px;color:{gc}">{g}</span><span style="width:92px">{d}</span><span style="flex:1">{p}</span><span style="width:96px;text-align:right">{a}</span></div>'
    preview = f'''
        <div style="flex:1;min-width:{300 if stack else 0}px;max-width:460px">
          <div style="{LABEL};margin-bottom:10px">PREVIEW</div>
          <div style="border:1px solid rgba(32,30,29,.30);background:#eae9e9">
            <div style="display:flex;padding:7px 12px;{LABEL};color:{INK2};border-bottom:1px solid #d7d3d3"><span style="width:18px"></span><span style="width:92px">DATE</span><span style="flex:1">PAYEE</span><span style="width:96px;text-align:right">AMOUNT</span></div>
            {row("●", "12 sep 2026", "Woolworths Metro", "−86.40")}
            {row("◐", "10 sep 2026", "Telstra", "−99.00")}
            {row("⚑", "09 sep 2026", "Dept of Education", "+4,210.00", True, "#ec3013")}
          </div>
          <p style="margin-top:14px;font-size:12px;color:{INK2};max-width:420px;text-wrap:pretty">Display settings are Configuration, not Preferences: read from <code>personal-ledger.conf</code> at start-up and written back here.</p>
        </div>'''
    direction = "flex-direction:column;gap:28px" if stack else "gap:40px"
    return f'\n      <div style="display:flex;{direction}">{form}{preview}\n      </div>'


def page_title(title_html, right):
    return f'''
      <div style="display:flex;align-items:baseline;justify-content:space-between;gap:12px">
        {title_html}
        <span style="font-size:11.5px;color:{INK3};white-space:nowrap">{right}</span>
      </div>
      <div style="height:2px;flex:none;background:{RULE};margin:14px 0 18px"></div>'''


def display_page(name, stack=False, cap=True, title_html=None, above="", right="client-scoped · never synced"):
    title_html = title_html or '<h4 style="margin:0;font-size:20px">Display</h4>'
    capw = "max-width:840px;" if cap else ""
    return f'''
    <div style="flex:1;min-width:0;display:flex;flex-direction:column;padding:22px 28px;overflow:hidden;position:relative">
     <div style="{capw}width:100%">{above}{page_title(title_html, right)}{display_form(stack, name)}
     </div>
    </div>'''


def status(hints, path, dropped=None):
    hint = " · ".join(f"{k(key)} {what}" for key, what in hints)
    s = f'''
  <div style="height:28px;flex:none;display:flex;align-items:center;gap:14px;padding:0 12px;background:#eae9e9;border-top:2px solid {RULE};font-size:11.5px;color:{INK2};white-space:nowrap;overflow:hidden">
    <span style="background:{INK};color:#f3f2f2;font-weight:800;font-size:10px;letter-spacing:.1em;padding:2px 7px">NORMAL</span>
    <span>{hint}</span>
    <div style="flex:1"></div>
    <span>{path}</span>
  </div>'''
    return s


SETTINGS_HINTS = [("j/k", "section"), ("tab", "next field"), ("b", "expand sidebar"), ("q", "back to dashboard")]
LONG_PATH, SHORT_PATH = "~/documents/ledgers/teda.pldb", "…/teda.pldb"


def card(cid, size, label, w, h, inner, overlay=""):
    return f'''
<div class="dv-opt" id="{cid}" data-window-size="{size}">
<div class="dv-olabel"><a class="dv-oid" href="#{cid}">{cid}</a><b>{size} · {w} × {h}.</b> {label}</div>
<div class="dv-card" style="width:{w}px;height:{h}px;display:flex;flex-direction:column;font-size:13px;color:{INK};position:relative">
{inner}{overlay}
</div>
</div>
'''


def body(*parts):
    return '\n  <div style="flex:1;display:flex;min-height:0;position:relative">' + "".join(parts) + "\n  </div>"


def dimmer(left=0):
    return f'<div style="position:absolute;left:{left}px;top:0;right:0;bottom:0;background:rgba(32,30,29,.30);z-index:4"></div>'


out = []
n = iter(range(105, 200))
nid = lambda: f"DESKTOP-{next(n):03d}"


def turn(tid, title, intro, cards, nxt=""):
    out.append(f'''
<section class="dv-turn" id="{tid}">
<div class="dv-thd"><a class="dv-tid" href="#{tid}">{tid[1:]}</a><span class="dv-tname">{title}</span></div>
<p class="dv-next" style="margin:0 0 22px;max-width:1100px">{intro}</p>
<div class="dv-opts">
{"".join(cards)}
</div>
{nxt}
</section>''')


# ---- 2: reference frames -------------------------------------------------------------------
M_H = 1040
cards = []
cards.append(card(nid(), "Medium", "Baseline: today's Chrome with only the Medium rule applied, the primary rail as icons. The docked 232px Settings index stays, so the page gets 960 − 52 − 232 = 676px and the preview stacks under the form. This is the layout the four options in turn 3 replace. Status line keeps every hint at 960; the ledger path shortens to <code>…/teda.pldb</code>.", 960, M_H,
    header("settings › display") + body(icon_rail("settings"), settings_index(), display_page("m-ref", stack=True)) + status(SETTINGS_HINTS, SHORT_PATH)))
cards.append(card(nid(), "Large", "Today's Chrome, unchanged: expanded primary rail (206px), docked Settings index (232px), page content capped at 840px.", 1280, 800,
    header("settings › display") + body(full_rail("settings"), settings_index(), display_page("l-ref")) + status([("j/k", "section"), ("tab", "next field"), ("b", "collapse sidebar"), ("q", "back to dashboard")], LONG_PATH)))
cards.append(card(nid(), "X-Large", "Same Chrome as Large. The Settings page caps its content at 840px and leaves the rest of the View empty, so a form never stretches to 1,400px. Settings docks no detail pane; Views that want one at X-Large decide that per View.", 1920, 1080,
    header("settings › display") + body(full_rail("settings"), settings_index(), display_page("xl-ref")) + status([("j/k", "section"), ("tab", "next field"), ("b", "collapse sidebar"), ("q", "back to dashboard")], LONG_PATH)))
turn("t2", "Window Sizes · reference frames",
     "Settings › Display at each Window Size's reference frame (ADR-0042). Medium is 720–1199px wide, Large 1200–1599px, X-Large 1600px and up; height never picks a Window Size.", cards)

# ---- 3: Medium Settings navigation options ---------------------------------------------------
cards = []
drawer_hints = [("j/k", "section"), ("tab", "next field"), ("\\", "pages")]


def drawer_overlay():
    s = f'<div style="position:absolute;left:52px;top:48px;bottom:28px;width:260px;z-index:5;background:#f3f2f2;border-right:2px solid {INK};box-shadow:8px 0 28px rgba(32,30,29,.28);display:flex;flex-direction:column">'
    s += settings_index(filter_box=True, width=258, footer=False).replace("border-right:2px solid rgba(32,30,29,.38);", "")
    s += f'<div style="padding:9px 14px;border-top:1px solid #d7d3d3;color:{INK2};font-size:11.5px">{k("j/k")} move · {k("enter")} open · {k("esc")} close</div></div>'
    return s


menu_btn = f'<span style="display:inline-flex;align-items:center;gap:6px;padding:4px 8px;border:1px solid rgba(32,30,29,.30);font-size:12px;color:{INK2};margin-right:10px;vertical-align:3px"><svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="{INK}" stroke-width="1.6"><line x1="2" y1="4" x2="14" y2="4"></line><line x1="2" y1="8" x2="14" y2="8"></line><line x1="2" y1="12" x2="14" y2="12"></line></svg>Settings</span>'
drawer_title = f'<h4 style="margin:0;font-size:20px;display:flex;align-items:center">{menu_btn}Display</h4>'

cards.append(card(nid(), "Medium", "<b>(a) Drawer, open.</b> <code>\\</code> or the <b>☰ Settings</b> button slides the index over the View, dimming it; <code>enter</code> or a click opens the page and closes the drawer, <code>esc</code> closes it. The filter box takes <code>/</code>.", 960, M_H,
    header("settings › display") + body(icon_rail("settings"), display_page("a960", title_html=drawer_title), dimmer(52)) + status(drawer_hints, SHORT_PATH), drawer_overlay()))
cards.append(card(nid(), "Medium", "<b>(a) Drawer, closed, at 720.</b> The page gets the whole View: 720 − 52 = 668px, the widest of the four. The index is one key away but out of sight, so the page list is not glanceable.", 720, M_H,
    header("settings › display") + body(icon_rail("settings"), display_page("a720", stack=True, title_html=drawer_title)) + status(drawer_hints, SHORT_PATH)))


def drill_index():
    rows = ""
    for icon, label, _, desc in PAGES:
        on = icon == "display"
        st = f"background:{INK};color:#f3f2f2;" if on else "border-bottom:1px solid #d7d3d3;"
        rows += f'<div style="display:flex;align-items:center;gap:12px;padding:10px 14px;{st}">{svg(icon, 16, "#f3f2f2" if on else INK)}<span style="width:140px;font-weight:800">{label}</span><span style="flex:1;color:{"#d7d3d3" if on else INK2}">{desc}</span><span style="color:{"#bab6b6" if on else INK3}">›</span></div>'
    return f'''
    <div style="flex:1;min-width:0;display:flex;flex-direction:column;padding:22px 28px;overflow:hidden">
     <div style="max-width:840px;width:100%">{page_title('<h4 style="margin:0;font-size:20px">Settings</h4>', "14 pages")}
      <div style="border-top:1px solid #d7d3d3">{rows}</div>
      <p style="margin-top:14px;font-size:11.5px;color:{INK3}">preferences sync · configuration local</p>
     </div>
    </div>'''


back = f'<div style="font-size:12px;color:{INK2};margin-bottom:10px"><span style="color:{INK};font-weight:800">‹ Settings</span> <span style="color:{INK3}">esc</span></div>'
crumb_link = f'<span style="color:{INK3};font-size:12px;white-space:nowrap"><span style="text-decoration:underline;text-underline-offset:2px">settings</span> › display</span>'
cards.append(card(nid(), "Medium", "<b>(b) Drill-down, the index page.</b> <code>g s</code> lands here. The index is a full page, so each row has room for a one-line description. <code>j/k</code> moves, <code>enter</code> or a click replaces this page with the chosen one.", 960, M_H,
    header("settings") + body(icon_rail("settings"), drill_index()) + status([("j/k", "page"), ("enter", "open"), ("q", "back to dashboard")], SHORT_PATH)))
cards.append(card(nid(), "Medium", "<b>(b) Drill-down, a page at 720.</b> The page gets the whole View. <code>esc</code>, the <b>‹ Settings</b> link or the <code>settings</code> breadcrumb segment goes back to the index with Display still selected. Moving between two pages costs two steps.", 720, M_H,
    header("", crumb_html=crumb_link) + body(icon_rail("settings"), display_page("b720", stack=True, above=back)) + status([("j/k", "section"), ("tab", "next field"), ("esc", "settings")], SHORT_PATH)))

picker_title = f'<h4 style="margin:0;font-size:20px;display:inline-flex;align-items:center;gap:8px;padding:2px 8px 2px 0">Display <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="{INK}" stroke-width="2"><polyline points="3,6 8,11 13,6"></polyline></svg></h4>'
picker_title_open = picker_title.replace("padding:2px 8px 2px 0", f"padding:2px 8px;margin-left:-8px;background:{INK};color:#f3f2f2").replace(f'stroke="{INK}" stroke-width="2"', 'stroke="#f3f2f2" stroke-width="2"')


def picker_menu(left):
    rows = ""
    for i, (icon, label, _, _) in enumerate(PAGES):
        on = icon == "display"
        st = f"background:{INK};color:#f3f2f2;font-weight:800;" if on else ""
        rows += f'<div style="display:flex;align-items:center;gap:9px;padding:6px 12px;{st}">{svg(icon, 14, "#f3f2f2" if on else INK)}<span style="flex:1">{label}</span></div>'
    return f'<div style="position:absolute;left:{left}px;top:116px;width:240px;z-index:5;background:#f3f2f2;border:2px solid {INK};box-shadow:0 12px 32px rgba(45,43,43,.30);padding:4px 0"><div style="margin:6px 10px 6px;height:28px;box-sizing:border-box;display:flex;align-items:center;font-size:12.5px;padding:0 9px;border:1px solid rgba(32,30,29,.30);color:{INK3}">type to filter</div>{rows}<div style="height:1px;background:#d7d3d3;margin-top:4px"></div><div style="padding:7px 12px 4px;font-size:11.5px;color:{INK2}">{k("enter")} open · {k("esc")} close</div></div>'


picker_right = f'<span style="display:inline-flex;gap:6px;align-items:center"><span style="padding:1px 6px;border:1px solid rgba(32,30,29,.28);color:{INK2}">{k("[")} General</span><span style="padding:1px 6px;border:1px solid rgba(32,30,29,.28);color:{INK2}">Units {k("]")}</span><span>2 of 14</span></span>'
cards.append(card(nid(), "Medium", "<b>(c) Header picker, open.</b> The page title is the menu: a click or <code>p</code> drops the page list under it, typing filters, <code>enter</code> opens. It floats over the page; nothing else moves.", 960, M_H,
    header("settings › display") + body(icon_rail("settings"), display_page("c960", title_html=picker_title_open, right=picker_right)) + status([("j/k", "section"), ("[ ]", "page"), ("p", "pages")], SHORT_PATH), picker_menu(52 + 28 - 8 + 2)))
cards.append(card(nid(), "Medium", "<b>(c) Header picker, closed, at 720.</b> The page keeps the whole View; <code>[</code> and <code>]</code> step to the previous and next page in index order, with their names on the chips. Good for stepping through neighbours, weak for jumping far (Display → About is twelve presses or the menu).", 720, M_H,
    header("settings › display") + body(icon_rail("settings"), display_page("c720", stack=True, title_html=picker_title, right=picker_right)) + status([("j/k", "section"), ("[ ]", "page"), ("p", "pages")], SHORT_PATH)))

cards.append(card(nid(), "Medium", "<b>(d) Docked but narrow.</b> A 120px index of icons and short labels (Sync, Backup, Logs) replaces the 232px one. Always visible, nothing to open or close, and <code>j/k</code> works in it as today. The page gets 960 − 52 − 120 = 788px, so the preview still sits beside the form.", 960, M_H,
    header("settings › display") + body(icon_rail("settings"), narrow_index(), display_page("d960")) + status(SETTINGS_HINTS, SHORT_PATH)))
cards.append(card(nid(), "Medium", "<b>(d) Docked but narrow, at 720.</b> The page gets 720 − 52 − 120 = 548px: enough for the form column, so the preview stacks. Two icon columns side by side are the cost.", 720, M_H,
    header("settings › display") + body(icon_rail("settings"), narrow_index(), display_page("d720", stack=True)) + status(SETTINGS_HINTS[:3], SHORT_PATH)))

turn("t3", "Medium · Settings navigation options",
     "Four ways to reach the Settings pages at Medium, each drawn at 960 (the reference) and 720 (the floor). The 960 frame shows the option while you navigate, the 720 frame shows it at rest on a page. Pick one; the others are dropped from the deck.", cards)

# ---- 4: Medium Chrome details ------------------------------------------------------------------
cards = []
long_crumb = f'<span style="color:{INK3};font-size:12px;white-space:nowrap;overflow:hidden;direction:rtl;text-overflow:ellipsis;min-width:0"><bdi>settings › institutions › ANZ › accounts › ANZ Everyday › edit</bdi></span>'
txn_rows = [("12 sep", "Woolworths Metro", "groceries", "−86.40"), ("10 sep", "Telstra", "phone", "−99.00"), ("09 sep", "Dept of Education", "salary", "+4,210.00"), ("08 sep", "Shell Coles Express", "fuel", "−71.15"), ("06 sep", "Origin Energy", "electricity", "−212.30"), ("05 sep", "Bunnings", "home", "−48.90")]


def simple_table(rows, width_note=""):
    r = "".join(f'<div style="display:flex;padding:8px 12px;border-bottom:1px solid #d7d3d3;font-variant-numeric:tabular-nums"><span style="width:70px">{d}</span><span style="flex:1">{p}</span><span style="width:110px;color:{INK2}">{c}</span><span style="width:90px;text-align:right">{a}</span></div>' for d, p, c, a in rows)
    return f'''
    <div style="flex:1;min-width:0;display:flex;flex-direction:column;padding:22px 28px;overflow:hidden">
      {page_title('<h4 style="margin:0;font-size:20px">ANZ Everyday</h4>', "aud · 312 transactions")}
      <div style="display:flex;padding:7px 12px;{LABEL};color:{INK2};border-bottom:1px solid #d7d3d3"><span style="width:70px">DATE</span><span style="flex:1">PAYEE</span><span style="width:110px">CATEGORY</span><span style="width:90px;text-align:right">AMOUNT</span></div>
      {r}
    </div>'''


toast = f'<div style="position:absolute;right:16px;bottom:40px;width:420px;z-index:5;display:flex;align-items:center;gap:10px;padding:10px 12px;background:{INK};color:#f3f2f2;box-shadow:0 8px 24px rgba(32,30,29,.30);font-size:12.5px"><span style="background:#2ecc71;color:{INK};font-weight:800;font-size:10px;letter-spacing:.1em;padding:2px 6px;flex:none">SAVED</span><span style="flex:1;min-width:0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis">Transaction saved to ANZ Everyday — Woolworths Metro, groceries, −86.40 on 12 September</span><span style="color:#bab6b6;flex:none">✕</span></div>'
long_hints = [("j/k", "row"), ("enter", "open"), ("a", "add"), ("e", "edit"), ("/", "filter"), (":", "command"), ("?", "help")]
cards.append(card(nid(), "Medium", "<b>Header, status line and toast at 720.</b> The breadcrumb truncates from the left, so the leaf (<code>… › ANZ Everyday › edit</code>) always shows. The status line keeps the mode chip and <code>…/teda.pldb</code> and drops key hints from the end until the rest fits (here <code>/ filter · : command · ? help</code> went). The toast stays 420px: the window-minus-16px rule only bites below 452px, which the 720 floor never reaches.", 720, M_H,
    header("", crumb_html=long_crumb) + body(icon_rail("transactions"), simple_table(txn_rows)) + status(long_hints[:4], SHORT_PATH), toast))


def palette(w):
    rows = [(":budget edit", "set limit or period", "b e"), (":budget list", "budgets vs actual", "g b"), (":budget new", "cap one category", "—"), (":budget period", "weekly · monthly · quarterly", "—"), (":report variance", "actual vs budget", "—")]
    body_rows = ""
    for i, (c, d, b) in enumerate(rows):
        if i == 0:
            body_rows += f'<div style="display:flex;align-items:center;gap:12px;padding:8px 16px;background:{INK};color:#f3f2f2"><span style="flex:1;font-weight:800;white-space:nowrap">{c}</span><span style="color:#d7d3d3;white-space:nowrap;overflow:hidden;text-overflow:ellipsis">{d}</span><span style="width:40px;text-align:right;color:#bab6b6;font-size:11.5px">{b}</span></div>'
        else:
            body_rows += f'<div style="display:flex;align-items:center;gap:12px;padding:8px 16px"><span style="flex:1;white-space:nowrap">{c}</span><span style="color:{INK2};white-space:nowrap;overflow:hidden;text-overflow:ellipsis">{d}</span><span style="width:40px;text-align:right;color:{INK3};font-size:11.5px">{b}</span></div>'
    return f'''<div style="position:absolute;left:16px;right:16px;top:96px;z-index:5;background:#f3f2f2;border:2px solid {INK};box-shadow:0 12px 32px rgba(45,43,43,.30)">
    <div style="display:flex;align-items:center;gap:10px;padding:12px 16px"><span style="font-weight:800;font-size:15px">&gt;</span><span style="font-size:15px;font-weight:800">bud</span><span style="width:8px;height:17px;background:#ec3013;display:inline-block"></span><div style="flex:1"></div><span style="font-size:11.5px;color:{INK2}">5 of 62</span></div>
    <div style="height:2px;background:{INK}"></div>{body_rows}
    <div style="height:1px;background:#d7d3d3"></div><div style="padding:8px 16px;font-size:11.5px;color:{INK2};white-space:nowrap;overflow:hidden;text-overflow:ellipsis">↑↓ select · tab complete · enter run · ^r history · esc close</div></div>'''


cards.append(card(nid(), "Medium", "<b>Command palette at 720.</b> 820px no longer fits, so the palette is the window minus 16px each side (688px), still 96px from the top. The description column truncates before the command does; the footer hint truncates from the end. At 960 the palette keeps its 820px.", 720, M_H,
    header("transactions") + body(icon_rail("transactions"), simple_table(txn_rows), dimmer(0)) + status([], SHORT_PATH).replace(f'background:{INK};color:#f3f2f2;font-weight:800;font-size:10px;letter-spacing:.1em;padding:2px 7px">NORMAL', 'background:#ec3013;color:#f3f2f2;font-weight:800;font-size:10px;letter-spacing:.1em;padding:2px 7px">COMMAND'), palette(688)))

# ---- horizontal scroll
cols = [("DATE", 96, "l"), ("ACCOUNT", 150, "l"), ("PAYEE", 210, "l"), ("CATEGORY", 150, "l"), ("TAGS", 170, "l"), ("MEMO", 240, "l"), ("AMOUNT", 110, "r"), ("BALANCE", 120, "r")]
wide = [("12 sep 2026", "ANZ Everyday", "Woolworths Metro", "groceries", "household", "weekly shop", "−86.40", "4,182.55"),
        ("10 sep 2026", "ANZ Everyday", "Telstra", "phone", "bills", "mobile, September", "−99.00", "4,268.95"),
        ("09 sep 2026", "ANZ Everyday", "Dept of Education", "salary", "income", "fortnightly pay", "+4,210.00", "4,367.95"),
        ("08 sep 2026", "ING Orange", "Shell Coles Express", "fuel", "car", "", "−71.15", "157.95"),
        ("06 sep 2026", "ANZ Everyday", "Origin Energy", "electricity", "bills · household", "quarter to 31 aug", "−212.30", "229.10"),
        ("05 sep 2026", "Amex", "Bunnings", "home", "garden", "hose fittings and mulch", "−48.90", "−1,402.60"),
        ("04 sep 2026", "ANZ Everyday", "Netflix", "subscriptions", "entertainment", "", "−18.99", "441.40"),
        ("03 sep 2026", "ING Orange", "Transfer from ANZ", "transfer", "", "to savings", "+500.00", "229.10")]
total = sum(c[1] for c in cols) + 24


def cell(v, w, a, hd=False):
    al = "text-align:right;" if a == "r" else ""
    return f'<span style="width:{w}px;flex:none;{al}white-space:nowrap;overflow:hidden;text-overflow:ellipsis">{v}</span>'


thead = f'<div style="display:flex;padding:7px 12px;{LABEL};color:{INK2};border-bottom:1px solid #d7d3d3;width:{total}px">' + "".join(cell(c[0], c[1], c[2]) for c in cols) + "</div>"
tbody = "".join(f'<div style="display:flex;padding:8px 12px;border-bottom:1px solid #d7d3d3;font-variant-numeric:tabular-nums;width:{total}px">' + "".join(cell(v, c[1], c[2]) for v, c in zip(r, cols)) + "</div>" for r in wide)
view_w = 960 - 52 - 56
thumb = round(view_w * view_w / total)
hscroll_view = f'''
    <div style="flex:1;min-width:0;display:flex;flex-direction:column;padding:22px 28px 0;overflow:hidden">
      {page_title('<h4 style="margin:0;font-size:20px">Transactions</h4>', f"all accounts · table {total}px wide, View {view_w}px")}
      <div style="flex:1;min-height:0;overflow:hidden;position:relative">
        <div style="width:{total}px">{thead}{tbody}</div>
        <div style="position:absolute;top:0;right:0;bottom:0;width:36px;background:linear-gradient(90deg,rgba(243,242,242,0),#f3f2f2)"></div>
      </div>
      <div style="height:14px;flex:none;display:flex;align-items:center;border-top:1px solid #d7d3d3;margin:0 0 6px"><div style="height:6px;width:{thumb}px;background:rgba(32,30,29,.45)"></div></div>
    </div>'''
cards.append(card(nid(), "Medium", f"<b>Horizontal scroll inside the View.</b> The Transactions table is {total}px wide and the View is {view_w}px. The View shows a horizontal scroll bar at its own bottom edge and scrolls sideways under the page title; the header, rails and status line never move. <code>h/l</code> or <code>shift</code>+wheel scroll it.", 960, M_H,
    header("transactions") + body(icon_rail("transactions"), hscroll_view) + status([("j/k", "row"), ("h/l", "scroll"), ("enter", "open"), ("a", "add")], SHORT_PATH)))

turn("t4", "Medium · Chrome details",
     "What the header, status line and overlays do when they run out of room, and how a View wider than its space scrolls.", cards)

# ---- 5: hybrid of (b) and (c), the user's pick to check ------------------------------------------
cards = []


def crumb_title(open_=False, page="Display"):
    caret_bg = f"background:{INK};" if open_ else "border:1px solid rgba(32,30,29,.30);"
    caret_stroke = "#f3f2f2" if open_ else INK
    return (f'<h4 style="margin:0;font-size:20px;display:flex;align-items:center;gap:8px;white-space:nowrap">'
            f'<span style="color:{INK2};font-weight:400;text-decoration:underline;text-decoration-color:rgba(32,30,29,.30);text-underline-offset:4px">Settings</span>'
            f'<span style="color:{INK3};font-weight:400">›</span><span>{page}</span>'
            f'<span style="width:24px;height:24px;display:inline-flex;align-items:center;justify-content:center;{caret_bg}"><svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="{caret_stroke}" stroke-width="2"><polyline points="3,6 8,11 13,6"></polyline></svg></span></h4>')


hybrid_hints = [("j/k", "section"), ("[/]", "pages"), ("p", "picker"), ("esc", "settings")]
cards.append(card(nid(), "Medium", "<b>(b + c) The index page.</b> <code>g s</code> lands on the drill-down index, as in (b). Its heading is plain <b>Settings</b>: no caret, because the index is already the full list. <code>enter</code> or a click opens a page.", 960, M_H,
    header("") + body(icon_rail("settings"), drill_index()) + status([("j/k", "page"), ("enter", "open"), ("q", "back to dashboard")], SHORT_PATH)))
cards.append(card(nid(), "Medium", "<b>(b + c) A page, picker open.</b> The page heading is a breadcrumb, <b>Settings › Display ▾</b>. A click on <b>Settings</b> (or <code>esc</code>) goes back to the index; the caret (or <code>p</code>, the keyboard way to open it) drops the page list under the page name, typing filters, <code>enter</code> jumps straight to another page without passing through the index.", 960, M_H,
    header("") + body(icon_rail("settings"), display_page("e960", title_html=crumb_title(True))) + status(hybrid_hints, SHORT_PATH), picker_menu(52 + 28 + 96)))
cards.append(card(nid(), "Medium", "<b>(b + c) A page at rest, at 720.</b> The page keeps the whole View (668px). <code>[</code> and <code>]</code> step to the previous and next page in index order; they are listed only in the status line hints, with no chips by the heading. Far jumps go through the caret, going up goes through <b>Settings</b>. At Medium the Chrome header drops its breadcrumb on Settings, so the page heading is the only trail.", 720, M_H,
    header("") + body(icon_rail("settings"), display_page("e720", stack=True, title_html=crumb_title())) + status(hybrid_hints, SHORT_PATH)))
turn("t5", "Medium · Settings navigation, drill-down + header picker (to confirm)",
     "The pick under test: (b)'s index page, with (c)'s picker folded into a breadcrumb page heading. <b>Settings</b> goes up to the index, the caret after the page name opens the picker, <code>[</code> and <code>]</code> step sideways.", cards)

print("".join(out))
