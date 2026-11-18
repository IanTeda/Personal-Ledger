## The Shell's own status-line hints (`lib_shell::ShellViewModel`). Each label sits beside its key,
## which the Client styles.

hint-command = command
hint-search = search
hint-help = help
hint-toggle-sidebar = toggle sidebar

## The `?` Help card's shortcut cheat-sheet (`lib_shell::HelpPresenter`).
help-shortcut-palette = Command palette
help-shortcut-help = This help
help-shortcut-focus = Move focus
help-shortcut-confirm = Open / confirm
help-shortcut-close = Close overlay
help-shortcut-undo = Undo
help-shortcut-redo = Redo

## The status line when `u` or `ctrl+r` finds an empty History (`lib_shell::ShellPresenter`).
status-nothing-to-undo = Nothing to undo
status-nothing-to-redo = Nothing to redo

## The status line once `u`, `ctrl+r` or a click in the header's History list has stepped through
## History, `$count` entries, `$label` the last entry stepped.
status-undid = { $count ->
    [1] Undid { $label }
   *[other] Undid { $count } changes, back to before { $label }
}
status-redid = { $count ->
    [1] Redid { $label }
   *[other] Redid { $count } changes, through { $label }
}

## When a History entry was recorded, back from now, in the header's Undo list.
history-just-now = just now
history-minutes-ago = { $minutes ->
   *[other] { $minutes } min ago
}
history-hours-ago = { $hours ->
   *[other] { $hours } h ago
}
history-days-ago = { $days ->
   *[other] { $days } d ago
}
