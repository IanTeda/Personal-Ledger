<!-- markdownlint-disable MD033 -->
# Personal Ledger

<!-- The extra return after center is needed for it to render the markdown links center -->
<div align="center">

[![License][license-shield]][license-url]
[![Issues][issues-shield]][issues-url]
[![Contributors][contributors-shield]][contributors-url]
[![Forks][forks-shield]][forks-url]
[![Stargazers][stars-shield]][stars-url]

</div>

<!-- PROJECT HEADER -->
<br />
<div align="center">
    <!-- Absolute URL: the README renders both on GitHub (repo root) and in the mdBook build (outside `src = "docs"`), so no relative path resolves in both -->
    <a href="https://github.com/IanTeda/Personal-Ledger">
        <img src="https://raw.githubusercontent.com/IanTeda/Personal-Ledger/main/docs/images/personal-ledger-logo01.png" alt="Personal Ledger logo" width="120" height="120">
    </a>
    <h3 align="center">Personal Ledger</h3>
    <p align="center">
        Keep track of your spending, investments and assets, so you know where you stand and can make better decisions.
    <br />
    <a href="https://ianteda.github.io/personal-ledger/">Read the Docs</a>
    ·
    <a href="https://github.com/IanTeda/Personal-Ledger/issues">Report a Bug</a>
    ·
    <a href="https://github.com/IanTeda/Personal-Ledger/issues">Request a Feature</a>
  </p>
</div>

## About

![Personal Ledger Desktop Client screenshot][screenshot-desktop]

_The Desktop Client_

![Personal Ledger TUI Client screenshot][screenshot-tui]

_The TUI Client_

Personal Ledger is a personal finance app. The goal is simple: show you clearly where your money goes and what you own, so you can keep an eye on your finances and plan ahead.

Why build another one?

- Every finance app has opinions, and none of the ones I tried matched mine. The rest wanted a hefty monthly subscription.
- I (perhaps naively) thought I could do better.
- It's a fun way to sharpen my programming skills.

## What's in the box

Personal Ledger is made of a few pieces you can mix and match:

- **Desktop app (in development):** a native desktop app.
- **TUI app (in development):** the same ledger in your terminal.
- **Sync Server (in development):** an optional server that keeps your ledger in sync across devices. Runs as a standalone binary or in Docker.

Right now the apps are built UI-first, so most screens show sample data. Saving your own data comes next.

## Built with

It's Rust all the way down, because that's the language I want to get better at:

- [SQLite](https://sqlite.org/) with [SQLx](https://github.com/launchbadge/sqlx) for storage
- [GPUI](https://www.gpui.rs/) and [gpui-component](https://github.com/longbridge/gpui-component) for the desktop app
- [Ratatui](https://ratatui.rs/) for the terminal app
- [Tonic](https://github.com/hyperium/tonic) (gRPC) and [Axum](https://github.com/tokio-rs/axum) for the Sync Server

## Getting started

There's nothing to install yet. If you'd like to build it yourself or help out, read on.

### Set up your tools

Tool versions (Rust, protoc, mdBook, sqlx-cli and friends) are pinned in `mise.toml` and managed by [mise](https://mise.jdx.dev/). Install mise, then let it install everything else:

```sh
curl https://mise.run | sh
cd personal-ledger
mise trust
mise install
```

### Set up the dev database

The database crate checks its SQL queries at compile time, so it needs a database to check against. Create a `.env` file in the repo root:

```sh
DATABASE_URL=sqlite:/absolute/path/to/Personal-Ledger/.personal-ledger-dev.db
```

Then build the database:

```sh
mise run db
```

### Build and run

```sh
cargo build                          # build everything
mise run watch-desktop               # run the desktop app, reloading on save
mise run watch-tui                   # run the terminal app, reloading on save
cargo run --package bin_sync_server  # run the Sync Server
```

### Test and lint

```sh
cargo test       # run all the tests
mise run lint    # the same checks CI runs (rustfmt + clippy)
mise run lint-fix
```

Run `mise run install-hooks` once and a pre-push hook will run the lint checks for you.

Run `mise tasks` to see every available task, including docs (`docs-serve`) and database helpers (`db-*`).

### Working with AI agents

The repo is set up for AI coding agents like [Claude Code](https://claude.com/claude-code). Agent conventions live in [`CLAUDE.md`](CLAUDE.md) and project skills in [`.claude/skills/`](.claude/skills/). The workflow (idea → spec → tickets → TDD → review) follows [Matt Pocock's skills](https://github.com/mattpocock/skills).

## Roadmap

- [ ] Finish the desktop and TUI screens
- [ ] Save real data to the local ledger
- [ ] Sync between devices through the Sync Server
- [ ] Packaging and releases

See the [project board](https://github.com/users/IanTeda/projects/1) for the full list.

## Contributing

Contributions are welcome. Fork the repo, make your change on a branch and open a pull request. Got an idea but no code? [Open an issue](https://github.com/IanTeda/Personal-Ledger/issues).

## License

GPL-3.0. See [`LICENSE`](LICENSE) for details.

## Contact

Ian Teda — [ian@teda.id.au](mailto:ian@teda.id.au)

## Similar apps

Not quite what you're after? These might suit you better:

- [GnuCash](https://gnucash.org/)
- [KMyMoney](https://kmymoney.org/)
- [Money Manager EX](https://moneymanagerex.org/)
- [Firefly III](https://www.firefly-iii.org/)
- [You Need A Budget (YNAB)](https://www.ynab.com/)
- [Beancount](https://github.com/beancount/beancount)
- [Plain Text Accounting](https://plaintextaccounting.org/)
- [BudgE](https://github.com/linuxserver/budge)
- [Budget Zero](https://budgetzero.io/)
- [Buckets](https://www.budgetwithbuckets.com/)

<!-- MARKDOWN LINKS & IMAGES -->
<!-- https://www.markdownguide.org/basic-syntax/#reference-style-links -->
[contributors-shield]: https://img.shields.io/github/contributors/IanTeda/personal-ledger.svg?style=for-the-badge
[contributors-url]: https://github.com/IanTeda/personal-ledger/graphs/contributors
[forks-shield]: https://img.shields.io/github/forks/IanTeda/personal-ledger.svg?style=for-the-badge
[forks-url]: https://github.com/IanTeda/personal-ledger/network/members
[stars-shield]: https://img.shields.io/github/stars/IanTeda/personal-ledger.svg?style=for-the-badge
[stars-url]: https://github.com/IanTeda/personal-ledger/stargazers
[issues-shield]: https://img.shields.io/github/issues/IanTeda/personal-ledger.svg?style=for-the-badge
[issues-url]: https://github.com/IanTeda/personal-ledger/issues
[license-shield]: https://img.shields.io/github/license/IanTeda/personal-ledger.svg?style=for-the-badge
[license-url]: https://github.com/IanTeda/personal-ledger/blob/main/LICENSE
<!-- Absolute URLs for the same reason as the logo: the README is rendered by both GitHub and mdBook -->
[screenshot-desktop]: https://raw.githubusercontent.com/IanTeda/Personal-Ledger/main/docs/images/screenshot-desktop.png
[screenshot-tui]: https://raw.githubusercontent.com/IanTeda/Personal-Ledger/main/docs/images/screenshot-tui.png
