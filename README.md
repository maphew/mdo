# Open Markdown as HTML

`mdo` is a small, fast, self-contained command-line tool that turns a
Markdown file into a styled, standalone HTML5 page — because Markdown is
great for writing and diffing, but calmer to *read* in a browser. Pages
include embedded [simple.css](https://simplecss.org/) styling and automatic
light/dark mode with a manual toggle, and mdo makes no network calls at
runtime. With the optional file-manager integration, opening a `.md` file as
rendered HTML is as fast as opening it in a text editor. Linux, macOS, and
Windows.

Project site: <https://maphew.github.io/mdo/> ·
Public metrics: <https://maphew.github.io/mdo/metrics/>

## Install

Linux and macOS:

```bash
curl -fsSL https://maphew.github.io/mdo/install.sh | sh
```

Windows PowerShell:

```powershell
irm https://maphew.github.io/mdo/install.ps1 | iex
```

The hosted installers fetch the latest
[GitHub Release](https://github.com/maphew/mdo/releases), verify it against
`SHA256SUMS`, and install into a user-local bin directory
(`$HOME/.local/bin`, or `%LOCALAPPDATA%\mdo\bin` on Windows; set
`MDO_INSTALL_DIR` to change). Manual archives are on the Releases page.

Rust users can build from source instead with `cargo install mdo-cli`
(needs a working Rust toolchain, including the MSVC linker on Windows), or
`git clone` + `cargo build --release`.

### Android preview

The native Android app can open a Markdown document from the system picker or
handle it from a Files app, then renders it locally through mdo's shared Rust
engine. Android build and installation details are in
[`android/README.md`](android/README.md).

## Use

```bash
mdo FILE.md          # convert once; writes FILE.html next to the source
mdo --open FILE.md   # render to a temp page and open it in your browser
mdo --watch FILE.md  # re-render automatically on every save
```

`mdo --open` never leaves `.html` files beside your Markdown (unless you pass
`--output` to redirect it there yourself) — output goes to a stable per-file
temp path, and relative images and links still resolve.
Running `mdo` with no arguments prints a short landing page; `mdo --help`
shows the full reference.

Full CLI reference, output details, and examples: [docs/usage.md](docs/usage.md).

## File-manager integration

The core convenience: right-click or double-click a `.md` file and read it as
HTML immediately, with no generated files left beside the source.

```bash
mdo --setup                    # guided first-run setup (offers the install)
mdo --install-file-manager     # add "Open as HTML" for Markdown files
mdo --uninstall-file-manager   # remove everything the installer added
```

- **Windows** — right-click a `.md` file → **Open as HTML** (per-user
  registry entries only; no admin rights). Release ZIPs include a
  double-clickable `mdo-setup.exe` for guided setup.
- **Linux** — **Open With → Open as HTML** in XDG file managers; add
  `--set-default` to make it the default Markdown handler. Release archives
  include an `mdo-setup` launcher.
- **macOS** — a short Automator Quick Action recipe (no installer yet).

Everything is per-user, quiet, and reversible. Platform details:
[docs/file-manager-integration.md](docs/file-manager-integration.md).

## Security

Raw HTML inside Markdown is **sanitized by default** (scripts and
event-handler attributes removed); `--unsafe-html` disables that for trusted
input only. mdo makes no network calls at runtime, and the file-manager
integration is strictly per-user. Details and reporting:
[SECURITY.md](SECURITY.md).

## Project status — feedback wanted

mdo is young and honest about it: v0.5 is released, v0.6 is in progress. It
does its core job — double-click a `.md` file, get a rendered page — every
day on the author's Windows and Linux machines. macOS binaries are built and
released but are the least exercised, and the Finder integration is still a
manual recipe.

What we don't know yet is whether mdo is useful to anyone who isn't the
author. That's the question this phase of the project exists to answer, and
you can help:

- **Tried it?** [Tell us how it went](https://github.com/maphew/mdo/issues/new?template=experience_report.yml)
  — "worked, keeping it" and "gave up during setup" are equally valuable.
- **Something broke?** [File a bug](https://github.com/maphew/mdo/issues/new?template=bug_report.yml).
- **Just want to talk?** [Discussions](https://github.com/maphew/mdo/discussions) are open.

Critique is welcome at full strength, including "this shouldn't exist because
X already does it better." See [CONTRIBUTING.md](CONTRIBUTING.md) for more,
including a candid note on how this project is built.

## Documentation

- [Usage and CLI reference](docs/usage.md)
- [File-manager integration](docs/file-manager-integration.md)
- [Custom CSS](docs/custom-css.md)
- [Maintaining](docs/maintaining.md) — releases, packaging, docs site, metrics
- [Changelog](CHANGELOG.md)
- [Architecture decision records](docs/adr/)

## Credits and license

This project is a grateful fork of Hafiz Ali Raza's original
Markdown-to-HTML CLI. Hafiz remains credited as an author, and this fork
keeps that lineage explicit so future improvements can be offered back
upstream. The bundled [simple.css](https://simplecss.org/) is © 2020
[Kev Quirk](https://kevquirk.com/), MIT License — see
[`assets/simple.css.LICENSE`](assets/simple.css.LICENSE).

Dual-licensed under either the [MIT License](LICENSE-MIT) or the
[Apache License, Version 2.0](LICENSE-APACHE), at your option, matching the
upstream project.
