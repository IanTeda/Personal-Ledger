## The Units module's Messages, read by its Presenters. Each Client composes this layer into its
## Locale; no Client catalogue keeps a copy.

## The Units Settings page: both tables, the row menu, the legend and ACTIVITY.

units-scope = { $count ->
    [one] { $count } unit
   *[other] { $count } units
} · synced
units-price-sources-heading = Price sources
units-column-flags = Flags
units-column-price-source = Price source
units-column-source = Source
units-column-last-updated = Last updated
units-not-fetched-yet = Not fetched yet
units-none = —
units-flag-base = base
units-legend-base = base unit
units-inactive = inactive
units-menu-edit-unit = Edit unit…
units-menu-delete-unit = Delete unit…
units-menu-reactivate-unit = Reactivate unit
units-menu-test-source = Test price source
units-menu-edit-source = Edit price source…
units-menu-delete-source = Delete price source…
units-hint-unit = unit
units-hint-price-source = price source
units-hint-edit = edit
units-hint-delete = delete
units-hint-new = new
units-hint-test-source = test source
units-hint-test = test
units-hint-units = units
units-hint-price-sources = price sources
units-status-test-not-yet-built = test price source — not yet built
units-activity-saved = Saved unit { $code }
units-activity-deleted = Deleted unit { $code }
units-activity-deactivated = Deactivated unit { $code }
units-activity-reactivated = Reactivated unit { $code }
units-activity-base-set = Made { $code } the base unit
units-activity-source-saved = Saved price source { $name }
units-activity-source-deleted = Deleted price source { $name }

## The detail pane docked at X-Large.

units-pane-unit = Unit
units-pane-used-by = Used by
units-pane-used-by-value = { $accounts ->
    [one] { $accounts } account
   *[other] { $accounts } accounts
} · { $transactions ->
    [one] { $transactions } transaction
   *[other] { $transactions } transactions
}
units-pane-accounts = Accounts
units-pane-accounts-total = Total
units-pane-accounts-none = No accounts use this unit.
units-pane-title = { $code } · { $name }
units-pane-meta = { $kind } · valued in { $base }
units-pane-edit-unit = Edit unit
units-pane-delete-unit = Delete unit
units-pane-reactivate-unit = Reactivate unit
units-pane-test = Test
units-pane-edit-source = Edit source
units-pane-delete-source = Delete source
units-pane-add-source = Add price source
units-pane-url = URL
units-pane-last-price = Last price
units-pane-source-meta = { $origin } · prices { $kinds } · { $key }
units-pane-origin-built-in = built-in
units-pane-origin-custom = custom
units-pane-every-kind = every kind
units-pane-key-needed = key needed
units-pane-key-not-needed = key not needed
units-pane-source-custom = { $name } · custom
units-pane-manual-note = Prices you enter by hand.
units-pane-values = Values
units-pane-values-none = No units use this source.
units-pane-symbol = symbol
units-pane-quoted-in = quoted in { $code }
units-pane-values-note = Each unit sets its own symbol in Edit unit.
units-pane-values-note-custom = Each unit sets its own symbol in Edit unit. Deleting this source moves these units to Manual entry.
units-pane-activity = Activity
units-pane-activity-note = This session's Units outcomes, newest first. Errors still toast.

## The Toasts an outcome raises while the detail pane isn't docked.

units-toast-deleted = Deleted unit { $code }
units-toast-deactivated = Deactivated unit { $code }
units-toast-reactivated = Reactivated unit { $code }
units-toast-source-saved = Saved price source { $name }
units-toast-source-deleted = Deleted price source { $name }{ $moved ->
    [0] {""}
    [one] {" · "}{ $moved } unit moved to Manual entry
   *[other] {" · "}{ $moved } units moved to Manual entry
}

## How long ago a source last fetched, in LAST UPDATED. `$value` is a count of the unit named.

units-ago-just-now = just now
units-ago-minutes = { $value ->
    [one] { $value } minute ago
   *[other] { $value } minutes ago
}
units-ago-hours = { $value ->
    [one] { $value } hour ago
   *[other] { $value } hours ago
}
units-ago-days = { $value ->
    [one] { $value } day ago
   *[other] { $value } days ago
}
units-ago-weeks = { $value ->
    [one] { $value } week ago
   *[other] { $value } weeks ago
}
units-ago-months = { $value ->
    [one] { $value } month ago
   *[other] { $value } months ago
}
units-ago-years = { $value ->
    [one] { $value } year ago
   *[other] { $value } years ago
}

## The Add and Edit unit Dialogs.

units-add =
    .title = Add unit
    .submit = Add
units-edit =
    .title = Edit unit — { $code }
    .submit = Save
units-code-placeholder = e.g. usd
units-name-placeholder = e.g. US Dollar
units-group-source = Price source
units-field-source = Source
units-source-help = From the Price Sources table. + on that table adds another.
units-field-symbol = Symbol
units-symbol-placeholder = e.g. bitcoin
units-symbol-help = { $source }'s id for this unit.
units-field-quote = Quoted in
units-quote-base = { $code } (base unit)
units-test-price = Test price

## The Delete unit Dialog, and the Deactivate it turns into for a unit in use.

units-delete =
    .title = Delete unit — { $code }
    .submit = Delete unit
units-delete-confirm-label = Type { $code } to confirm
units-delete-new-base-label = New base unit
units-delete-new-base-help = { $code } is the base unit. Choose the unit that takes over before it is deleted.
units-deactivate =
    .title = Deactivate unit — { $code }
    .submit = Deactivate unit
units-in-use = { $code } is used by { $uses }, so it can't be deleted.
units-in-use-accounts = { $count ->
    [one] { $count } account
   *[other] { $count } accounts
}
units-in-use-bill-plans = { $count ->
    [one] { $count } bill plan
   *[other] { $count } bill plans
}
units-in-use-budgets = { $count ->
    [one] { $count } budget
   *[other] { $count } budgets
}
units-in-use-help = Deactivating it keeps it on those and hides it from the unit pickers. You can reactivate it later.
units-in-use-base = It is also the base unit, so it stays active until another unit takes over as base.
units-base-unit-option = { $code } — { $name }

## The Set base unit Dialog.

units-set-base =
    .title = Set base unit
    .submit = Set base unit
units-set-base-label = Base unit

## Refusals: an Interactor said no, shown in the open Dialog.

units-refusal-unit-gone = This unit was removed while the dialog was open.
units-refusal-price-source-gone = That price source was removed while the dialog was open.
units-refusal-only-unit = { $code } is the only unit, so it can't be deleted.
units-refusal-base-unit-needed = { $code } is the base unit, and no active unit can take over as base.
units-refusal-new-base-gone = The unit chosen as the new base unit is gone.
units-refusal-inactive-base = A deactivated unit can't be the base unit.
units-refusal-quoted = { $count ->
    [one] { $code } is the quote unit of { $count } other unit, so it can't be deleted. Change that unit's quote first.
   *[other] { $code } is the quote unit of { $count } other units, so it can't be deleted. Change their quotes first.
}
units-refusal-manual-entry = Manual entry can't be edited or deleted.
units-refusal-built-in-endpoint = A built-in price source's address isn't edited here.
units-refusal-built-in-delete = A built-in price source can't be deleted.

## The Add, Edit and Delete price source Dialogs (16z·custom, 16z·edit, 16z·del).

units-price-source-add =
    .title = Add price source
    .submit = Add price source
units-price-source-edit =
    .title = Edit price source — { $name }
    .submit = Save
units-price-source-delete =
    .title = Delete price source — { $name }
    .submit = Delete price source
units-price-source-field-updates = Updates
units-price-source-field-address = Address
units-price-source-field-request = Request
units-price-source-field-price-path = Price path
units-price-source-field-auth = Authentication
units-price-source-field-key-param = Key parameter
units-price-source-field-api-key = API key
units-price-source-name-placeholder = e.g. Alpha Vantage
units-price-source-address-placeholder = https://
units-price-source-request-placeholder = /query?symbol={ "{" }symbol{ "}" }
units-price-source-price-path-placeholder = /price
units-price-source-key-param-placeholder = apikey
units-price-source-name-help = Shown in the Price Sources table and the unit picker.
units-price-source-request-help = { "{" }symbol{ "}" } is the unit's symbol and { "{" }quote{ "}" } the unit it's quoted in.
units-price-source-price-path-help = A JSON pointer to the price in the response.
units-price-source-key-note = API keys aren't saved yet: storing them encrypted is still to be explored.
units-price-source-updates-every-15-minutes = Every 15 min
units-price-source-updates-hourly = Hourly
units-price-source-updates-daily = Daily
units-price-source-updates-manual-only = Manual only
units-price-source-auth-none = None
units-price-source-auth-api-key = API key
units-price-source-test-with-symbol = Test with symbol
units-price-source-test = Test price source
units-price-source-test-not-built = Testing isn't built yet.
units-price-source-built-in = { $name } · built-in
units-price-source-built-in-help = Built-in sources update with the app, so their address isn't edited here.
units-price-source-usage = { $count ->
    [0] No units use this source.
    [one] Values { $count } unit: { $codes }.
   *[other] Values { $count } units: { $codes }.
}
units-price-source-delete-warning = This price source values { $count ->
    [one] { $count } unit
   *[other] { $count } units
}. Deleting it cannot be undone.
units-price-source-delete-fallback = { $count ->
    [one] { $codes } falls back to Manual entry and keeps its last price.
   *[other] { $codes } fall back to Manual entry and keep their last prices.
}
units-price-source-delete-confirm-label = Type { $name } to confirm
