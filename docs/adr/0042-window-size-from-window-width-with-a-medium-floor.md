# Window Size comes from the window's width, with Medium as the floor

The Desktop Client was drawn and built for one 1280 × 800 window, but under a tiling window manager it is often half a screen or less. Decided in [#519](https://github.com/IanTeda/Personal-Ledger/issues/519): the Chrome and each View arrange themselves by a Window Size picked from the window's width alone, and the window cannot be made narrower than the smallest Window Size.

## Decision

- Three Window Sizes, by window width in logical pixels:

| Window Size | Width | Reference frame for mockups |
| --- | --- | --- |
| Medium | 720 – 1199 | 960 × 1040 (half of a 1920 × 1080 screen, tiled) |
| Large | 1200 – 1599 | 1280 × 800 |
| X-Large | ≥ 1600 | 1920 × 1080 |

- The window's minimum size is 720 × 480. There is no Compact Window Size, and none is designed.
- Height never picks a Window Size: a short window scrolls.
- A View whose content is wider than the space the Chrome leaves it, such as a wide table at Medium, shows a horizontal scroll bar and scrolls inside the View. The Chrome itself never scrolls sideways.
- The Chrome is designed Medium-first. Existing View designs stay at Large until each is redrawn.
- Nothing stores the Window Size. GPUI redraws on every resize, so the Chrome and each View read it from the viewport during render.
- A rail toggled with `b` keeps that state only until the window crosses a Breakpoint, which resets the rail to that Window Size's default.
- At X-Large a View may dock a detail pane. Forms and Settings pages cap their content width at every Window Size.

## Consequences

- Every View's layout has to work from 720px wide. A View whose layout differs at Medium covers it in its `_keyboard` and `_mouse` UI tests by resizing the test window.
- Mockup decks draw each View at the three reference frames as the View is built or redrawn, starting with Settings.

## Considered Options

- **Pick by the monitor's size.** Rejected: a tiled window on a large monitor would get a layout too wide for it.
- **Add a Compact Window Size below 720.** Rejected: a personal-finance Desktop Client is not used at phone width, and a floor keeps every layout to three cases.
- **Store the Window Size and update it from a resize observer.** Rejected: render already re-runs on resize, so a stored copy could only go stale.
