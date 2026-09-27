# Personal Ledger — logo handoff

Mark: **Totals rule** (direction 2a). Two entry bars closed by a red double underline, the accountant's mark for a balanced total. The corners are square on purpose: don't round the tile or apply a squircle mask.

## Folders

- `light/`: assets for light backgrounds and light mode
- `dark/`: assets for dark backgrounds and dark mode

Both folders use the same file names, so you can swap between them by path.

```
<theme>/
  favicon.ico              16, 32, 48 (PNG-in-ICO)
  app-icon.ico             16–256, Windows / Linux desktop
  app-icon.icns            16–1024 incl. @2x, macOS bundle
  server-icon.ico          16–256, sync server (tray, admin UI)
  svg/
    app-icon.svg           master, 64-unit grid, use ≥ 64px
    app-icon-48.svg        pixel-hinted for 48px
    app-icon-32.svg        pixel-hinted for 32px
    app-icon-16.svg        pixel-hinted for 16px
    server-icon*.svg       same set, sync server variant
    mark.svg               glyph only, transparent, no tile
    lockup.svg             icon + "Personal Ledger" wordmark (live text, see note)
  png/
    app-icon-{16,32,48,64,128,256,512,1024}.png
    server-icon-{16,32,48,64,128,256,512,1024}.png
    mark-{32,64,128,256,512}.png          transparent
    lockup-{32,64,128}.png                transparent, height in px
    social-preview-1280x640.png           GitHub repo social preview
```

## Which file to use

- **Desktop app bundle:** `app-icon.icns` (macOS) and `app-icon.ico` (Windows). On Linux, install `png/app-icon-{16…512}.png` into the hicolor theme.
- **Sync server:** the `server-icon.*` files for the tray, the favicon of its web/admin UI and container labels. The accent tile tells it apart from the desktop app.
- **Web favicon:** `favicon.ico`, plus `svg/app-icon-32.svg` as `<link rel="icon" type="image/svg+xml">`.
- **README header:** `png/lockup-64.png` (use the 128 file at `height="64"` for retina). Use `<picture>` to switch themes on GitHub:

```html
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/logo/dark/png/lockup-128.png">
  <img src="docs/logo/light/png/lockup-128.png" height="64" alt="Personal Ledger">
</picture>
```

- **GitHub social preview:** upload `png/social-preview-1280x640.png` under Settings → Social preview.
- **In-app header / about dialog:** `svg/mark.svg` or `svg/app-icon.svg`.

## Sizes and hinting

The master is drawn on a 64-unit grid. Below 64px, use the hinted 48, 32 and 16 drawings. Every edge in those falls on a whole pixel. Don't scale the master down to those sizes, because it will blur.

## Geometry (64-unit master)

| Element | x | y | w | h | Colour role |
|---|---|---|---|---|---|
| Tile | 0 | 0 | 64 | 64 | tile |
| Entry bar 1 | 12 | 12 | 40 | 8 | bars |
| Entry bar 2 | 12 | 25 | 26 | 8 | bars |
| Total rule 1 | 12 | 40 | 40 | 4 | rules |
| Total rule 2 | 12 | 48 | 40 | 4 | rules |

Margins are 12 units on all sides. The rules are half the weight of the entry bars. The widest gap sits just above the total.

## Colours

| Role | Light | Dark | Server (both) |
|---|---|---|---|
| tile | Ink `#201e1d` | Ground `#f3f2f2` | Accent `#ec3013` |
| bars | Ground `#f3f2f2` | Ink `#201e1d` | Ground `#f3f2f2` |
| rules | Accent `#ec3013` | Accent `#ec3013` | Ink `#201e1d` |
| mark bars (no tile) | Ink | Ground | — |
| wordmark | Ink | Ground | — |

These values come from the Modernist design system tokens (`--color-text`, `--color-bg`, `--color-accent`).

## Wordmark

Archivo ExtraBold (800), tracking −0.01em, set at 0.625 × the tile height. The gap between tile and text is 0.3125 × the tile height. It is vertically centred on cap height.

`lockup.svg` uses live `<text>`. It falls back to a generic sans font anywhere Archivo isn't installed, and that includes GitHub `<img>` tags. Use the PNG lockups there, or convert the text to outlines first (for example `inkscape --export-text-to-path`).
