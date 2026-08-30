<!-- markdownlint-disable MD033 -->
# Personal Ledger

<!-- Improved compatibility of back to top link -->
<a name="readme-top"></a>

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
    <a href="https://github.com/IanTeda/personal-ledger">
        <img src="https://github.com/IanTeda/personal-ledger/raw/develop/docs/images/logo.png" alt="Logo" width="80" height="80">
    </a>
    <h3 align="center">Personal Ledger</h3>
    <p align="center">
        Track your personal expense, investments and assets to help you understand what you have and to make informed decisions..
    <br />
    <a href="https://ianteda.github.io/personal-ledger/">Explore the Docs</a>
    ·
    <a href="https://ianteda.github.io/personal-ledger/issues">Report a Bug</a>
    ·
    <a href="https://ianteda.github.io/personal-ledger/issues">Request a Feature</a>
  </p>
</div>

<!-- TABLE OF CONTENTS -->
<details>
  <summary>Table of Contents</summary>
  <ol>
    <li><a href="#about-the-project">About The Project</a></li>
    <li><a href="#includes">Project Includes</a></li>
    <li><a href="#getting-started">Getting Started</a></li>
    <li>
      <a href="#developing-with-this-repo">Developing With This Repo</a>
      <ul>
        <li><a href="#prerequisites">Prerequisites</a></li>
        <li><a href="#building-and-running">Building and Running</a></li>
        <li><a href="#testing-linting-and-formatting">Testing, Linting and Formatting</a></li>
        <li><a href="#documentation">Documentation</a></li>
        <li><a href="#agent-assisted-development">Agent-Assisted Development</a></li>
      </ul>
    </li>
    <li><a href="#usage">Usage</a></li>
    <li><a href="#roadmap">Roadmap</a></li>
    <li><a href="#contributing">Contributing</a></li>
    <li><a href="#license">License</a></li>
    <li><a href="#contact">Contact</a></li>
    <li><a href="#references">References</a></li>
    <li><a href="#similar-applications">Similar Applications</a></li>
  </ol>
</details>

<!-- ABOUT THE PROJECT -->
## About The Project

[![Personal Ledger Screenshot][product-screenshot]](https://github.com/IanTeda/personal-ledger)

**Personal Ledger** aims to provide its users with an intuitive and user-friendly interface that offers insights into their spending habits and investment position. By presenting information clearly and concisely, users can effectively monitor their overall financial health, make informed decisions, and ultimately achieve their personal financial goals.

Motivation for Personal Ledger:

* Applications in general are opinionated, and finance applications are no different. Looking at the finance applications available, none ticked my boxes or were monthly subscription models with high fees.
* I niavely thought I could do it better.
* Funsy project to hone my programming skills.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- PROJECT INCLUDES -->
## Includes

The Personal Ledger has a number of standalone applications and services that can be combined to suit your preference. They include the following:

* **Backend (Underdevement) :** A backend server that can be accessed by front ends through an rpc interface. The backend can be a standalone binary service or Docker deployable.
* **Web (Under Development) :** A web frontend app that uses the backend for data quering. The web frontend can be a standalone binary service or Docker deployable
* **Desktop (Planned) :** A desktop app that wraps the web front end that can access the bakend or be standalone.
* **TUI (Planned) :** A text interface frontend that can access the backend or be standalone.

<!-- PROJECT IS BUILT WITH -->
## Built With

I have tried to keep the tech stack with Rust, because that is what I would like to learn and improve with. The Rust stack being used includes the following:

* Sqlite:
* Tonic:
* Tuari:
* Ratui:

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- GETTING STARTED -->
## Getting Started

At the moment Personal Ledger is under development, so there is no end-user installation yet — the backend server only wires up a `Ping` utility RPC. If you want to build, run or contribute to the code, see [Developing With This Repo](#developing-with-this-repo) below.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- DEVELOPING WITH THIS REPO -->
## Developing With This Repo

Personal Ledger is a Rust Cargo workspace. This section covers what you need to build, test and run it locally.

### Prerequisites

Toolchain and dev-tool versions (Rust, protoc, mdBook, cargo-make, sqlx-cli, cargo-watch, cargo-audit) are pinned in `mise.toml` at the workspace root and managed by [mise](https://mise.jdx.dev/). Install mise, then install the pinned tools for this repo:

```sh
curl https://mise.run | sh
cd personal-ledger
mise trust
mise install
```

`mise`'s shell/dir activation will also install the pinned tools automatically when you `cd` into the repo. Without this step, tools like `protoc` or `cargo-make` won't be on your `PATH`.

### Building and Running

Most day-to-day work is plain `cargo` run against the workspace or a specific package:

```sh
# Build the whole workspace
cargo build

# Build just the server binary
cargo build --package server --bin server

# Run the server (reads config, initialises telemetry — currently does nothing else)
cargo run --package server
```

Building `lib-rpc` requires a system `protoc` (protobuf compiler), provided via `mise.toml`. `tonic_prost_build` regenerates `crates/libs/lib-rpc/src/generated/*.rs` from the `.proto` files on every build; the generated files are checked in but should be treated as build output, not hand-edited.

> **Note:** `crates/libs/lib-database` exists on disk but is *not* currently a workspace member — it's mid-development and not yet wired into `server`. Build/test it directly with `cargo build --package lib_database`, or add it back to `[workspace].members` if you're integrating it.

### Testing, Linting and Formatting

```sh
# Test the whole workspace
cargo test

# Test a single crate
cargo test --package lib_config

# Lint and format
cargo clippy
cargo fmt
```

### Documentation

Docs are built with mdBook and rustdoc via `cargo-make`:

```sh
cargo make docs-build     # docs-rustdoc + docs-mdbook
cargo make docs-serve     # serves mdBook on :8001
```

### Agent-Assisted Development

This repo is set up to work with AI coding agents (like [Claude Code](https://claude.com/claude-code)). Repository conventions for agents live in [`CLAUDE.md`](CLAUDE.md), and project-specific skills live under [`.claude/skills/`](.claude/skills/) (rustdoc conventions, tracing/log-level conventions, and unit-test patterns).

The agent development workflow (grilling out an idea, writing a spec, splitting it into tickets, implementing with TDD and code review) follows [Matt Pocock's Claude Code skills](https://github.com/mattpocock/skills) — worth a read as a reference for how contributions here are expected to move from idea to shipped code.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- USAGE -->
## Usage

This space will be used to show useful examples of how a project can be used. Additional screenshots, code examples and demos work.

Include any links to more resources.

_For more examples, please refer to the [Documentation](https://example.com)_

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- ROADMAP -->
## Roadmap

* [ ] Map out the development workflow
* [ ] Develop REST API server (backend) minimum viable product
* [ ] Develop web server (frontend) minimum viable product
* [ ] Develop deployment integration
* [ ] Work through feature set:
  * [ ] Feature 1
  * [ ] Feature 2

See the [Personal Ledger - Roadmap](https://github.com/users/IanTeda/projects/1) for a full list of proposed features.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- CONTRIBUTING -->
## Contributing

Contributions are what make the open source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

If you have a suggestion that would make this better, please fork the repo and create a pull request. You can also simply open an issue with the tag "enhancement".
Don't forget to give the project a star! Thanks again!

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- LICENSE -->
## License

Distributed under the GPL3.0 License. See `LICENSE.txt` for more information.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- CONTACT -->
## Contact

* Ian Teda - [ian@teda.id.au](mailto:ian@teda.id.au)
* Project Link: [https://github.com/IanTeda/personal-ledger](https://github.com/IanTeda/personal-ledger)

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- REFERENCES -->
## References

The following is a list of resources you find helpful and would like to give credit to:

* [Choose an Open Source License](https://choosealicense.com)
* [Img Shields](https://shields.io)
* [GitHub Pages](https://pages.github.com)
* [Best README Template](https://github.com/othneildrew/Best-README-Template/tree/master)

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- SIMILAR PROJECTS -->
## Similar Applications

Below is a list of similar applications that you might prefer to use:

* [GNU Cash](https://gnucash.org/)
* [KmyMoney](https://kmymoney.org/)
* [Money Manager EX](https://moneymanagerex.org/)
* [Firefly III](https://www.firefly-iii.org/)
* [You Need A Budget (YNAB)](https://www.ynab.com/)
* [Mint](https://mint.intuit.com/)
* [Beancount](https://github.com/beancount/beancount?tab=readme-ov-file)
* [Plain Text Accounting](https://plaintextaccounting.org/)
* [BudgE](https://github.com/linuxserver/budge)
* [Budget Zero](https://budgetzero.io/)
* [Buckets](https://www.budgetwithbuckets.com/)

<p align="right">(<a href="#readme-top">back to top</a>)</p>

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
[license-url]: https://github.com/IanTeda/personal-ledger/blob/master/LICENSE.txt
[product-screenshot]: https://github.com/IanTeda/personal-ledger/raw/develop/docs/images/screenshot.png
