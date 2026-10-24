## The Payees module's Messages, read by its Presenters. Each Client composes this layer into its
## Locale; no Client catalogue keeps a copy. Column labels other screens share (Name, Actions) and
## the Payee Toasts come from the shared layer.

## The Settings page. `$count` is the active Payees, `$glyph` the Add button's leading plus sign and
## `$key` the key that opens Add.

payees-count = { $count ->
    [one] { $count } payee
   *[other] { $count } payees
}
payees-without-default = <strong>{ $count }</strong> without a default category
payees-status-without-default = { $count } without default category
payees-kicker = a–z
payees-add-button = { $glyph } Add payee
payees-empty = No payees yet. Press { $key } to add one.
payees-column-default-category = Default category
payees-column-match-rules = Match rules
payees-no-default-category = no default category
payees-inactive = inactive
payees-rule-count = { $count ->
    [one] { $count } rule
   *[other] { $count } rules
}
payees-row-edit = edit
payees-row-delete = delete
payees-footnote = Payees without a default category need one picked each time. Match rules drive renaming on import.

## The page's legend.

payees-hint-row = row
payees-hint-edit = edit
payees-hint-delete = delete
payees-hint-new = new

## The Add and Edit payee Dialogs (6b, 6c). `$glyph` is the add button's leading plus sign.

payees-add-title = Add payee
payees-add-submit = Add payee
payees-name-placeholder = e.g. Aussie Candle Co
payees-field-default-category = Default category (optional)
payees-category-none = — none, ask every time —
payees-field-match-rules = Match rules
payees-no-rules = no rules yet
payees-rule-placeholder = e.g. AUSSIE CANDLE
payees-rule-add = { $glyph } add
payees-rules-callout = Rules match case-insensitively against the raw statement description. Add one per format, e.g. WOOLWORTHS, WW SUPERMARKET.
payees-error-name-taken = { $name } already exists
payees-error-alias-taken = { $alias } is already a match rule on { $owner }
payees-refusal-gone = This payee has since been deleted.

## The Edit payee Dialog. `$name` is the Payee's stored name; `$count` the Splits carrying it.

payees-edit-title = Edit payee — { $name }
payees-edit-submit = Save
payees-edit-rule-placeholder = add another format…
payees-edit-usage-callout = { $count ->
    [one] Used by { $count } transaction.
   *[other] Used by { $count } transactions.
} Changing the default category only affects new transactions — existing ones keep whatever category they were assigned at the time.

## The Delete payee Dialog (6d). A Payee nothing names is deleted; one in use can only be
## deactivated, and an inactive one in use is offered reactivation. A Payee whose use can't be
## counted is offered what one in use is, with an `-unchecked` warning. `$name` is the Payee's name,
## `$count` the Splits carrying it, `$rules` its match rules.

payees-delete-title = Delete payee — { $name }
payees-delete-submit = Delete payee
payees-delete-warning = This payee isn't used by any transaction. Deleting it <strong>cannot be undone.</strong>
payees-delete-callout = { $rules ->
    [0] It has no match rules.
    [one] Deleting also removes its { $rules } match rule.
   *[other] Deleting also removes its { $rules } match rules.
}
payees-deactivate-title = Deactivate payee — { $name }
payees-deactivate-submit = Deactivate payee
payees-deactivate-warning = This payee is used by <strong>{ $count ->
    [one] { $count } transaction
   *[other] { $count } transactions
}</strong>, so it can't be deleted — only deactivated.
payees-deactivate-warning-unchecked = What uses this payee <strong>couldn't be checked</strong>, so it can't be deleted — only deactivated.
payees-deactivate-callout = It stays on the list and in the Transactions filter, dimmed, and its transactions keep it. Its match rules stop matching on import. You can reactivate it later.
payees-reactivate-title = Reactivate payee — { $name }
payees-reactivate-submit = Reactivate payee
payees-reactivate-warning = This payee is inactive and used by <strong>{ $count ->
    [one] { $count } transaction
   *[other] { $count } transactions
}</strong>.
payees-reactivate-warning-unchecked = This payee is inactive, and what uses it <strong>couldn't be checked</strong>.
payees-reactivate-callout = Reactivating counts it again and its match rules match on import again.
payees-delete-confirm-label = Type { $name } to confirm
