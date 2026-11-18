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

## The status line once `u` or `ctrl+r` has stepped through History, `$label` the entry's label.
status-undid = Undid { $label }
status-redid = Redid { $label }
