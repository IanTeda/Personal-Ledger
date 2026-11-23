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
units-column-last = Last
units-not-fetched-yet = Not fetched yet
units-none = —
units-flag-base = base
units-legend-base = base unit
units-inactive = inactive
# Two parts of one line, as a Unit row's kind and flags: `Currency · base`.
units-joined = { $head } · { $tail }
units-menu-prices = Prices
units-menu-edit-unit = Edit unit…
units-menu-delete-unit = Delete unit…
units-menu-reactivate-unit = Reactivate unit
units-menu-set-price = Set price…
units-menu-fetch = Fetch price now
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
units-hint-set-price = set price
units-hint-fetch = fetch price
units-hint-prices = prices
units-hint-units = units
units-hint-price-sources = price sources
units-activity-saved = Saved unit { $code }
units-activity-deleted = Deleted unit { $code }
units-activity-deactivated = Deactivated unit { $code }
units-activity-reactivated = Reactivated unit { $code }
units-activity-base-set = Made { $code } the base unit
units-activity-price-set = Set the price of { $code } for the week of { $week }
units-activity-source-saved = Saved price source { $name }
units-activity-source-deleted = Deleted price source { $name }

## What History calls each change, for Undo, Redo and the header's list (#1247).
units-changed-added = Add unit { $code }
units-changed-edited = Edit unit { $code }
units-changed-deleted = Delete unit { $code }
units-changed-deactivated = Deactivate unit { $code }
units-changed-reactivated = Reactivate unit { $code }
units-changed-base-set = Make { $code } the base unit
units-changed-price-set = Set the price of { $code } for the week of { $week }
units-changed-source-added = Add price source { $name }
units-changed-source-edited = Edit price source { $name }
units-changed-source-deleted = Delete price source { $name }

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
units-pane-price = Price
units-pane-price-none = No weekly prices yet.
units-pane-accounts = Accounts
units-pane-accounts-total = Total
units-pane-accounts-none = No accounts use this unit.
units-pane-title = { $code } · { $name }
units-pane-meta = { $kind } · valued in { $base }
units-pane-prices = Prices
units-pane-edit-unit = Edit unit
units-pane-delete-unit = Delete unit
units-pane-reactivate-unit = Reactivate unit
units-pane-set-price = Set price
units-pane-fetch = Fetch now
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
units-pane-key-optional = key optional
units-pane-api-key = API key
units-pane-api-key-saved = Saved
units-pane-api-key-not-set = Not set
units-pane-api-key-unreadable = Saved, but can't be read on this device
units-pane-powered-by-coingecko = Powered by CoinGecko
units-pane-yahoo-unofficial = Yahoo Finance's API is unofficial and may stop working.
units-pane-source-custom = { $name } · custom
units-pane-manual-note = Prices you enter by hand.
units-pane-values = Values
units-pane-values-none = No units use this source.
units-pane-symbol = symbol
units-pane-symbol-value = { $label } { $symbol }
units-pane-quoted-in = quoted in { $code }
units-pane-values-note = Each unit sets its own symbol in Edit unit.
units-pane-values-note-custom = Each unit sets its own symbol in Edit unit. Deleting this source moves these units to Manual entry.
units-pane-activity = Activity
units-pane-activity-note = This session's Units outcomes, newest first. Errors still toast.

## The Toasts an outcome raises while the detail pane isn't docked.

units-toast-deleted = Deleted unit { $code }
units-toast-deactivated = Deactivated unit { $code }
units-toast-reactivated = Reactivated unit { $code }
units-toast-price-fetched = Fetched { $code } at { $price }
units-toast-price-set = Set the price of { $code } for the week of { $week }
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
units-field-precision = Precision
units-precision-placeholder = 0 to { $max }
units-precision-help = Decimal places its amounts, and prices quoted in it, show at.
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
units-in-use-locations = { $count ->
    [one] { $count } location
   *[other] { $count } locations
}
units-in-use-institutions = { $count ->
    [one] { $count } institution as its usual unit
   *[other] { $count } institutions as their usual unit
}
units-in-use-help = Deactivating it keeps it on those and hides it from the unit pickers. You can reactivate it later.
units-in-use-base = It is also the base unit, so it stays active until another unit takes over as base.
units-edit-usage = Used by { $uses }.
units-edit-usage-none = Nothing uses this unit yet.
units-base-unit-option = { $code } — { $name }

## The Set base unit Dialog.

units-set-base =
    .title = Set base unit
    .submit = Set base unit
units-set-base-label = Base unit

## The Set price Dialog, for a unit on Manual entry. `$date` is the week's Monday.

units-set-price =
    .title = Set price — { $code }
    .submit = Set price
units-set-price-field-week = Week
units-set-price-week-of = Week of { $date }
units-set-price-field-close = Close
units-set-price-close-help = In { $code }, to { $places ->
    [one] { $places } place
   *[other] { $places } places
}.

## The Unit prices view: one Unit's weekly close chart over its weekly prices table, in a
## 12-month window. `$from` and `$to` are the window's first and last weeks' Mondays.

units-prices-title = { $code } · { $name } · prices
units-prices-last-price = Last price
units-prices-last-price-value = { $price } · { $age }
units-prices-week-change = Week change
units-prices-week-change-value = { $amount } ({ $percent }%)
units-prices-range = 52w range
units-prices-range-value = { $low } – { $high } { $code }
units-prices-symbol = Symbol · priced in
units-prices-symbol-value = { $symbol } · { $code }
units-prices-chart-heading = Weekly close
units-prices-table-heading = Weekly prices
units-prices-window = { $from } – { $to }
units-prices-column-week = Week
units-prices-column-open = Open
units-prices-column-high = High
units-prices-column-low = Low
units-prices-column-close = Close
units-prices-column-change = Change
units-prices-change = { $percent }%
units-prices-empty = { $code } has no weekly prices yet.
units-prices-window-empty = No weekly prices in this window.
units-prices-hint-week = week
units-prices-hint-earlier = earlier
units-prices-hint-later = later
units-prices-hint-back = back

## PRICE HISTORY: the selected Unit's newest weekly prices on the Units page, under PRICE SOURCES
## or in the docked pane. `$chord` is the key that opens the prices view; `$shown` and `$weeks`
## count weeks.

units-history-heading = Price history
units-history-subtitle = { $code } · { $source }
units-history-flag-provisional = this week · provisional
units-history-provisional = provisional
units-history-hint-open = { $chord } all prices
units-history-count = Newest week first · { $shown } of { $weeks ->
    [one] { $weeks } week
   *[other] { $weeks } weeks
}
units-history-pane-hint = { $chord } opens all { $weeks ->
    [one] { $weeks } week
   *[other] { $weeks } weeks
}
units-history-base = { $code } is the base unit: everything is valued in it, so it has no price.
units-history-none = { $code } has no prices yet. A fetch or backfill will fill this.

## A backfill in place under PRICE HISTORY's heading. `$reason` is a fetch failure's name and `$retry`
## when it tries again, `tries again in 8 minutes`.

units-history-backfilling = Fetching history…
units-history-backfill-failed = History failed · { $reason } · { $retry }

## Refusals: an Interactor said no, shown in the open Dialog.

units-refusal-unit-gone = This unit was removed while the dialog was open.
units-refusal-price-source-gone = That price source was removed while the dialog was open.
units-refusal-keys-locked = The key wasn't saved: this Ledger's API keys were saved with another device's API key secret.
units-refusal-key-not-saved = The key wasn't saved: no API key secret could be kept in this device's configuration.
units-refusal-quote-gone = The unit it's quoted in was removed while the dialog was open.
units-refusal-only-unit = { $code } is the only unit, so it can't be deleted.
units-refusal-base-unit-needed = { $code } is the base unit, and no active unit can take over as base.
units-refusal-new-base-gone = The unit chosen as the new base unit is gone.
units-refusal-inactive-base = A deactivated unit can't be the base unit.
units-refusal-quoted = { $count ->
    [one] { $code } is the quote unit of { $count } other unit, so it can't be deleted. Change that unit's quote first.
   *[other] { $code } is the quote unit of { $count } other units, so it can't be deleted. Change their quotes first.
}
units-refusal-priced-by-source = { $code } is priced by its price source, so it takes no price by hand.
units-refusal-future-week = The week of { $date } hasn't started, so it can't be priced yet.
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
units-price-source-field-headers = Headers
units-price-source-field-api-key = API key
units-price-source-name-placeholder = e.g. Alpha Vantage
units-price-source-address-placeholder = https://
units-price-source-request-placeholder = /query?symbol={ "{" }symbol{ "}" }
units-price-source-price-path-placeholder = /price
units-price-source-key-param-placeholder = apikey
units-price-source-headers-placeholder = Accept: application/json
units-price-source-name-help = Shown in the Price Sources table and the unit picker.
units-price-source-request-help = { "{" }symbol{ "}" } is the unit's symbol and { "{" }quote{ "}" } the unit it's quoted in; { "{" }symbol_lower{ "}" } and { "{" }quote_lower{ "}" } are the same in lower case.
units-price-source-headers-help = Optional. Name: value, with | between headers. Sent with every request and shown in the clear, so never put a key here.
units-price-source-price-path-help = A JSON pointer to the price in the response.
units-price-source-key-note = Saved encrypted in the Ledger, which syncs it. Each device reads it with its own copy of the API key secret.
units-price-source-key-optional-note = Optional. Without a key { $name } is asked keyless, at a lower call limit.
units-price-source-key-saved-placeholder = Key saved · leave blank to keep it
units-price-source-key-locked = This Ledger's API keys were saved with another device's API key secret. Copy that secret into this device's settings to change keys here.
units-price-source-remove-key = Remove key
units-price-source-updates-every-15-minutes = Every 15 min
units-price-source-updates-hourly = Hourly
units-price-source-updates-daily = Daily
units-price-source-updates-manual-only = Manual only
units-price-source-auth-none = None
units-price-source-auth-api-key = API key
units-price-source-test-with-symbol = Test with symbol
units-price-source-test = Test price source
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
}.
units-price-source-delete-fallback = { $count ->
    [one] { $count } unit falls back to Manual entry and keeps its last price: { $codes }.
   *[other] { $count } units fall back to Manual entry and keep their last prices: { $codes }.
}
units-price-source-delete-confirm-label = Type { $name } to confirm

## The command palette's descriptions of the Commands the keymap lists, in lower case as the palette shows them.

units-command-new = add a Unit
units-command-edit = edit the selected Unit
units-command-delete = delete the selected Unit
units-command-reactivate = reactivate the selected Unit
units-command-test-source = test the selected price source
units-command-next-table = move between the Units and the price sources
units-command-set-base = choose the base Unit
units-command-set-price = set a price for the selected Unit
units-command-fetch = fetch the selected Unit's or Price Source's prices now
units-command-fetch-all = fetch every Unit's price now

## Fetch failures (#1387): held by each Client in memory, never in the Ledger. A reason's name shows
## in LAST UPDATED and Last fetch, its detail after it. `$source` is a Price Source's name, `$symbol`
## and `$quote` a Unit's symbol and quote Unit code.

units-flag-fetch-failed = fetch failed
units-fetch-failure-offline = Offline
units-fetch-failure-rate-limited = Rate limited
units-fetch-failure-key-missing = Key missing
units-fetch-failure-key-rejected = Key rejected
units-fetch-failure-source-error = Source error
units-fetch-failure-not-found = Not found
units-fetch-failure-no-price = No price
units-fetch-detail-offline = couldn't reach { $source }
units-fetch-detail-rate-limited = { $source } asked to be called less often
units-fetch-detail-key-missing = { $source } needs an API key this device doesn't have
units-fetch-detail-key-rejected = { $source } refused the API key
units-fetch-detail-source-error = { $source } answered with an error
units-fetch-detail-not-found = { $source } has no { $symbol } in { $quote }
units-fetch-detail-no-price = { $source } gave no price for { $symbol } in { $quote }

## The Last fetch fact in a Unit's or a Price Source's PRICE SOURCE group. `$ago` is how long ago,
## `5 minutes ago`; `$reason` a reason's name and `$detail` its detail; `$retry` when it tries again.

units-pane-last-fetch = Last fetch
units-last-fetch-fetched = Fetched { $ago }
units-last-fetch-failed = Failed { $ago } · { $reason }: { $detail } · { $retry }
units-retry-next-check = tries again at the next check
units-retry-minutes = { $value ->
    [one] tries again in { $value } minute
   *[other] tries again in { $value } minutes
}
units-retry-hours = { $value ->
    [one] tries again in { $value } hour
   *[other] tries again in { $value } hours
}

## PRICE HISTORY's warning beside its heading: `$text` is what applies, `Out of date · fetch failed`.

units-history-warning = ⚠ { $text }
units-history-out-of-date = Out of date

## Fetch now's Warning Toasts. `$reason` is a round failure's phrase, `$detail` a Unit failure's
## detail; `$fetched` counts the prices written, `$failed` the Units that weren't.

units-fetch-round-offline = offline, try again later
units-fetch-round-rate-limited = rate limited, try again later
units-fetch-round-key-missing = no API key on this device
units-fetch-round-key-rejected = the API key was refused
units-fetch-round-source-error = it answered with an error
units-toast-fetch-round-failed = Couldn't fetch from { $source }: { $reason }
units-toast-fetch-unit-failed = Couldn't fetch { $code }: { $detail }
units-toast-fetch-some-failed = Fetched { $fetched ->
    [one] { $fetched } price
   *[other] { $fetched } prices
} from { $source }; { $failed } { $what }
units-toast-fetch-round-stopped = Fetched { $fetched ->
    [one] { $fetched } price
   *[other] { $fetched } prices
} from { $source }; then { $reason }
units-fetch-some-not-found = not found
units-fetch-some-no-price = with no price
units-fetch-some-failed = failed

## Fetch now on a Price Source or every Unit: a Toast as each source's round starts, the status line
## naming the sources still out, rows reading `Fetching…`, and one outcome Toast per round.
## `$sources` is the sources' names joined with commas.

units-fetching = Fetching…
units-toast-fetching = Fetching prices from { $source }…
units-toast-already-fetching = Already fetching prices from { $source }
units-status-fetching = Fetching prices from { $sources }…
units-status-fetching-history = Fetching price history for { $code } from { $source }…
units-status-fetch-no-units = No active Unit uses { $source }
# A call a person asked for waiting out the source's `Retry-After`, counting `$seconds` down.
units-status-fetch-waiting = Waiting for { $source }, { $seconds } s…
units-toast-fetch-round-fetched = Fetched { $fetched ->
    [one] { $fetched } price
   *[other] { $fetched } prices
} from { $source }

## Testing a Price Source (#1393): one symbol's price asked and shown, writing nothing. In the form,
## `$quote` and `$code` are Unit codes, `$field` a field's label, `$ms` the milliseconds the answer
## took, `$time` when it came (`14:36`) and `$status` its HTTP status. `$reason` is a fetch
## failure's name and `$detail` its detail.

units-price-source-field-symbol = Symbol
units-price-source-symbol-placeholder = e.g. VAS.AX
units-price-source-test-quoted = Quoted in { $quote }, the base unit.
units-price-source-test-first-unit = Prices { $code }, the first unit it values.
units-price-source-test-no-units = { $source } values no unit, so there is nothing to test.
units-price-source-test-needs-field = Fill in { $field } to test.
units-price-source-test-fix-field = Fix { $field } to test.
units-price-source-test-needs-symbol = Type a symbol to test.
units-price-source-test-needs-key = Type an API key to test.
units-price-source-testing = Testing…
units-price-source-test-found = Price found · { $ms } ms
units-price-source-test-found-detail = { $symbol } · { $price } · tested { $time }
units-price-source-test-unsaved = with unsaved changes
units-price-source-test-response = Response · HTTP { $status }
units-price-source-test-writes-nothing = A test writes nothing.
units-toast-test-found = { $source } works: { $symbol } · { $price } · { $ms } ms
units-toast-test-failed = { $source } test: { $reason } · { $detail }
units-toast-already-testing = Already testing { $source }
