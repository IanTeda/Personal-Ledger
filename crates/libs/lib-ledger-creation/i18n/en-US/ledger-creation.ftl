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
## `lib-units`' and the file name is the one the person chose in the file explorer.

ledger-creation-title = Create ledger
ledger-creation-submit = Create
ledger-creation-field-file = File
ledger-creation-base-unit-help = The base unit is what your accounts and totals are kept in. It can't be changed once an account uses it.
