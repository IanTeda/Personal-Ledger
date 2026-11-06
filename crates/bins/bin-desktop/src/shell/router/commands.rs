//! The no Ledger open content's Command clicks of `Root`. An `impl Root` block for the concern,
//! not the command registry (`navigation::command`) or the command runner: the Shell's Presenter
//! runs a palette Command, its own effects itself and the Desktop's through the root Router's
//! `run_effect` (`root_router.rs`).

use gpui::Context;
use lib_shell::{NavInput, PaletteRun};

use super::Root;
use crate::shell::view::no_ledger::NoLedgerCommand;
use crate::{
    navigation::{command, nav::InputMode},
    shell::view::{page_clicks::PresenterPage, window_size::WindowSize},
};

/// A click on the no Ledger open content's own `:open`/`:new` text (issue #167), through
/// `PageClicks` (#1024): looks the command up in the registry and runs it exactly as the palette's
/// own `enter` key would -- entering `InputMode::Command` first, same as typing `:` would, so the
/// status line's `COMMAND` badge and `Esc` (which only acts outside `InputMode::Normal`) both
/// behave identically regardless of which entry point opened the dialog. Like every page's click,
/// it is one input (`crate::input::send`): with no Ledger open there is nothing to commit, but a
/// Notice the command queues is drained and the rail's counts read, as after any other click.
impl PresenterPage<NoLedgerCommand> for Root {
    fn send(&mut self, event: NoLedgerCommand, cx: &mut Context<'_, Self>) {
        let Some(effect) = command::find(event.name()) else {
            return;
        };
        // `open` and `new` read no Window Size: only the rail toggle does.
        self.send_with_router(WindowSize::Large, cx, |presenter, router| {
            presenter.navigate(NavInput::EnterMode(InputMode::Command));
            presenter.run_palette(
                PaletteRun {
                    name: event.name(),
                    effect: effect.palette_effect(),
                    argument: String::new(),
                },
                router,
            );
        });
        cx.notify();
    }
}
