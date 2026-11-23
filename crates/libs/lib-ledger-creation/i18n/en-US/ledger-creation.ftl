## Why creating a Ledger wrote nothing: the `$reason` of `toast-ledger-create-failed`, read as
## "Couldn't create ledger teda.pldb: <reason>". A Client composes this layer into its Locale.

ledger-creation-exists = a file of that name is already there
ledger-creation-unwritable = it can't be written there

## The base Unit's refusals, the same rules as Add unit's. The Create ledger Dialog keeps Create off
## while a field is invalid, so these reach the person only from a Client that asks nothing.

ledger-creation-code-needed = its base unit needs a code
ledger-creation-name-needed = its base unit needs a name
ledger-creation-price-source-gone = its base unit's price source no longer exists
ledger-creation-quote-gone = its base unit can only be quoted in itself
ledger-creation-base-unit-invalid = its base unit isn't valid

## The Create ledger Dialog. Its fields are Add unit's, so their labels and placeholders are
## `lib-units`'.

ledger-creation-title = Create ledger
ledger-creation-submit = Create
ledger-creation-base-unit-help = The base unit is what your accounts and totals are kept in. It can't be changed once an account uses it.

## The Create ledger Dialog's two stages (#1344): Base unit, then Ledger, with Create on the last.
## Each stage's name heads its step in the strip under the title, `$number · $stage`, and the
## status line names the stage showing, `$stage ($number of $total)`, the name in bold.

ledger-creation-stage-base-unit = Base unit
ledger-creation-stage-ledger = Ledger
ledger-creation-step = { $number } · { $stage }
ledger-creation-stage-count = ({ $number } of { $total })
ledger-creation-next = Next
ledger-creation-back = Back

## The Ledger stage's fields, each with its line of help.

ledger-creation-ledger-help = Who and what this ledger is. All three can be changed later on Settings › Ledger.
ledger-creation-field-ledger-name = Ledger name
ledger-creation-ledger-name-help = Prefilled from the file name. Renaming one never renames the other.
ledger-creation-field-owner = Owner
ledger-creation-owner-help = Who the ledger belongs to. Free text, and it can be left empty.
ledger-creation-field-financial-year = Financial year starts
ledger-creation-financial-year-help = Bills and Documents count their years from the 1st of this month.
