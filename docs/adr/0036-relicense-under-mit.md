# Personal Ledger is relicensed from GPL-3.0 to MIT

Personal Ledger was distributed under GPL-3.0 and, from the relicensing commit `d5bcd50` onwards, is distributed under plain MIT. The question was raised in #263 and researched in `docs/research/project-licence.md`. Ian Teda is the sole copyright holder of every commit and no external contribution has been merged, so no one else's consent was needed.

We chose plain MIT over `MIT OR Apache-2.0`, the Rust convention the research recommended. Apache-2.0's express patent grant is not something this project needs, and one licence is simpler to state in every crate and on the Help screen. We chose a permissive licence over copyleft. Staying on GPL-3.0 would not have kept hosted forks of the Sync Server open anyway: GPL-3.0's copyleft applies on distribution, and only AGPL-3.0's §13 reaches software offered over a network.

Copies obtained before the change stay available under GPL-3.0 to whoever holds them, including any image or CI artefact built earlier. The change cannot recall those copies, so it applies from the relicensing commit onwards and nothing before it.

Whether any GPL-3.0 binary was published is not settled. On 2026-10-11 the repository had no releases and no tags, no retained CI artefacts, and three workflow runs in its history, none of them a publish. GHCR packages could not be listed, because the `gh` token lacks `read:packages`. The research records a release-triggered Sync Server publish on 2026-09-03 for `v0.0.1-sync-server-ci-test`, which would have pushed `ghcr.io/ianteda/personal-ledger-sync-server`, and manual Desktop and TUI runs on 2026-09-17. Those runs can no longer be retrieved, so that record stands unverified. Treat any image or artefact published before `d5bcd50` as GPL-3.0, and re-check GHCR with a token that has `read:packages`.

No dependency's licence blocks MIT, and the allow list in `deny.toml` is unchanged.

## Considered Options

- `MIT OR Apache-2.0`: rejected, because the patent grant adds a second licence to maintain for no benefit to this project.
- `AGPL-3.0-or-later`: rejected in favour of a permissive licence, although it is the only option that would keep hosted forks of the Sync Server open.
- Staying on GPL-3.0: rejected in favour of a permissive licence, and it would not have covered hosted forks of the Sync Server either.
