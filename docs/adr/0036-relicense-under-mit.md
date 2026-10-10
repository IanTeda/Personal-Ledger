# Personal Ledger is relicensed from GPL-3.0 to MIT

Personal Ledger was distributed under GPL-3.0 and, from the relicensing commit `d5bcd50` onwards, is distributed under plain MIT. The question was raised in #263 and researched in `docs/research/project-licence.md`. Ian Teda is the sole copyright holder of every commit and no external contribution has been merged, so no one else's consent was needed.

We chose plain MIT over `MIT OR Apache-2.0`, the Rust convention the research recommended. Apache-2.0's express patent grant is not something this project needs, and one licence is simpler to state in every crate and on the Help screen. We chose a permissive licence over copyleft. Staying on GPL-3.0 would not have kept hosted forks of the Sync Server open anyway: GPL-3.0's copyleft applies on distribution, and only AGPL-3.0's §13 reaches software offered over a network.

Copies obtained before the change stay available under GPL-3.0 to whoever holds them, including any image or CI artefact built earlier. The change cannot recall those copies, so it applies from the relicensing commit onwards and nothing before it.

No dependency's licence blocks MIT, and the allow list in `deny.toml` is unchanged.

## Considered Options

- `MIT OR Apache-2.0`: rejected, because the patent grant adds a second licence to maintain for no benefit to this project.
- `AGPL-3.0-or-later`: rejected in favour of a permissive licence, although it is the only option that would keep hosted forks of the Sync Server open.
- Staying on GPL-3.0: rejected in favour of a permissive licence, and it would not have covered hosted forks of the Sync Server either.
