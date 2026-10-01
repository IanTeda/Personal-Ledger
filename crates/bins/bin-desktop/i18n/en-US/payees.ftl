## The Payees page. Column labels other screens share (Name, Actions) come from the shared layer.

## The active payee count, used by the page summary and the status line.

desktop-payees-count = { $count ->
    [one] { $count } payee
   *[other] { $count } payees
}

## The page summary's second figure, the count in bold, and the status line's plain version.

desktop-payees-without-default = <strong>{ $count }</strong> without a default category
desktop-payees-status-without-default = { $count } without default category

## The page. `$glyph` is the Add button's leading plus sign, `$key` the key token that opens Add.

desktop-payees-add-button = { $glyph } Add payee
desktop-payees-empty = No payees yet. Press { $key } to add one.
desktop-payees-column-default-category = Default category
desktop-payees-column-match-rules = Match rules
desktop-payees-column-transactions = Transactions
desktop-payees-column-total = Total
desktop-payees-no-default-category = no default category
desktop-payees-inactive = inactive
desktop-payees-rule-count = { $count ->
    [one] { $count } rule
   *[other] { $count } rules
}
desktop-payees-row-edit = edit
desktop-payees-row-delete = delete
desktop-payees-footnote = Payees without a default category need one picked each time. Match rules drive renaming on import.

## The hint strip.

desktop-hint-view-transactions = view transactions

## The Add and Edit payee dialogs. `$glyph` is the add button's leading plus sign.

desktop-payees-add-title = Add payee
desktop-payees-add-submit = Add payee
desktop-payees-name-placeholder = e.g. Aussie Candle Co
desktop-payees-field-default-category = Default category
desktop-payees-category-none = — none, ask every time —
desktop-payees-field-match-rules = Match rules
desktop-payees-no-rules = no rules yet
desktop-payees-rule-placeholder = e.g. AUSSIE CANDLE
desktop-payees-rule-add = { $glyph } add
desktop-payees-rules-callout = Rules match case-insensitively against the raw statement description. Add one per format, e.g. WOOLWORTHS, WW SUPERMARKET.
desktop-payees-error-name-taken = { $name } already exists
desktop-payees-error-alias-taken = { $alias } is already a match rule on { $owner }
desktop-hint-confirm = confirm

## The Edit payee dialog. `$name` is the Payee's stored name; `$count` the Splits carrying it.

desktop-payees-edit-title = Edit payee — { $name }
desktop-payees-edit-submit = Save
desktop-payees-edit-rule-placeholder = add another format…
desktop-payees-edit-usage-callout = { $count ->
    [one] { $count } transaction uses this payee.
   *[other] { $count } transactions use this payee.
} Changing the default category only affects new transactions — existing ones keep whatever category they were assigned at the time.

## The Delete payee dialog (6d). A Payee no Split carries is deleted; one in use can only be
## deactivated, and an inactive one in use is offered reactivation. `$name` is the Payee's name,
## `$count` the Splits carrying it, `$rules` its match rules.

desktop-payees-delete-title = Delete payee — { $name }
desktop-payees-delete-submit = Delete payee
desktop-payees-delete-warning = This payee isn't used by any transaction. Deleting it <strong>cannot be undone.</strong>
desktop-payees-delete-callout = { $rules ->
    [0] It has no match rules.
    [one] Its { $rules } match rule is also removed.
   *[other] Its { $rules } match rules are also removed.
}
desktop-payees-deactivate-title = Deactivate payee — { $name }
desktop-payees-deactivate-submit = Deactivate payee
desktop-payees-deactivate-warning = This payee is used by <strong>{ $count ->
    [one] { $count } transaction
   *[other] { $count } transactions
}</strong>, so it can't be deleted — only deactivated.
desktop-payees-deactivate-callout = It stays on the list and in the Transactions filter, dimmed, and its transactions keep it. Its match rules stop matching on import. You can reactivate it later.
desktop-payees-reactivate-title = Reactivate payee — { $name }
desktop-payees-reactivate-submit = Reactivate payee
desktop-payees-reactivate-warning = This payee is inactive and used by <strong>{ $count ->
    [one] { $count } transaction
   *[other] { $count } transactions
}</strong>.
desktop-payees-reactivate-callout = Reactivating counts it again and its match rules match on import again.
desktop-payees-delete-confirm-label = Type { $name } to confirm

## The Settings page: its kicker over the A–Z list. The heading's meta, Add button, row actions and
## footnote are the Payees page's own messages.

desktop-settings-payees-kicker = a–z
