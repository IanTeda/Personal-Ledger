## The file explorer dialog behind `:open` and `:new`: browsing real directories to a `.pldb`
## ledger file, and behind `:data`, picking the Data folder. Title, column headers, instructions,
## and button labels.

## The dialog title, mode-specific.

desktop-explorer-open = Open ledger file
desktop-explorer-new = New ledger file
desktop-explorer-pick = Choose data folder
# Beside the folder picker's title (1e·pick): the Data folder now. `$folder` is its path, home as ~.
desktop-explorer-pick-now = now { $folder }

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

## The footer filter checkboxes. The second has a dimmer trailing clause.

desktop-explorer-filter-pldb-only = show only .pldb files
desktop-explorer-filter-hide-hidden = hide hidden files
desktop-explorer-filter-hide-hidden-detail = — dot files and folders

## The footer buttons.

desktop-explorer-cancel = Cancel
desktop-explorer-open-button = Open
desktop-explorer-new-button = Create
# The folder picker's primary button, named for the folder the path bar shows. `$folder` is its name.
desktop-explorer-use-button = Use { $folder }
# The folder picker's footer note (1e·pick).
desktop-explorer-pick-note = Picks the folder you're in. Files show what's there; nothing is moved.

## New mode's File name field (1e·new): its label, and the inline refusal of a name already in the
## folder browsed to. `$name` is the file name with `.pldb` added, `$folder` the folder's name.

desktop-explorer-file-name = File name
desktop-explorer-name-taken = { $name } is already in { $folder }. Pick another name, or :open it.
