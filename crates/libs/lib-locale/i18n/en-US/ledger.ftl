## The Ledger's own Preferences, shared by Settings › Ledger in both Clients and the Create ledger
## Dialog (ADR-0061).

## Why a Ledger name can't be saved: it is blank once trimmed, or over the limit. `$max` is the most
## characters a name holds.
ledger-name-error-blank = A Ledger needs a name
ledger-name-error-too-long = { $max ->
    [one] A ledger name is at most { $max } character
   *[other] A ledger name is at most { $max } characters
}

## The name a new Ledger starts with when its file name gives none.
ledger-name-fallback = My ledger
