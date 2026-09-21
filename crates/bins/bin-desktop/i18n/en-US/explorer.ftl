## The file explorer dialog behind `:open` and `:new`: browsing real directories to a `.pldb`
## ledger file. Title, column headers, instructions, and button labels.

## The dialog title, mode-specific.

desktop-explorer-open = Open ledger file
desktop-explorer-new = New ledger file

## The table column headers, all caps.

desktop-explorer-column-name = NAME
desktop-explorer-column-size = SIZE
desktop-explorer-column-modified = MODIFIED

## The entry count in the path bar. `$count` is the number of items in the current directory.

desktop-explorer-item-count = { $count ->
    [one] { $count } item
   *[other] { $count } items
}

## The modified timestamp relative to now. `$value` is the time quantity and `$unit` is the unit.
## Rules: "just now", then the bucket (minute/hour/day/week/month/year) with pluralisation.

desktop-explorer-modified-just-now = just now
desktop-explorer-modified-minutes = { $value ->
    [one] { $value } minute ago
   *[other] { $value } minutes ago
}
desktop-explorer-modified-hours = { $value ->
    [one] { $value } hour ago
   *[other] { $value } hours ago
}
desktop-explorer-modified-days = { $value ->
    [one] { $value } day ago
   *[other] { $value } days ago
}
desktop-explorer-modified-weeks = { $value ->
    [one] { $value } week ago
   *[other] { $value } weeks ago
}
desktop-explorer-modified-months = { $value ->
    [one] { $value } month ago
   *[other] { $value } months ago
}
desktop-explorer-modified-years = { $value ->
    [one] { $value } year ago
   *[other] { $value } years ago
}

## The footer instructions: only `.pldb` files are selectable.

desktop-explorer-file-type-info-prefix = only
desktop-explorer-file-type-info-suffix = files can be opened

## The footer buttons.

desktop-explorer-cancel = Cancel
desktop-explorer-open-button = Open
desktop-explorer-new-button = New
