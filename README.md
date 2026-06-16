# 📝 mdo — Markdown to HTML5 Converter (with optional live watch)

`mdo` is a small, fast, self-contained command-line tool that converts Markdown
files into HTML on Linux, macOS, and Windows.

Optional file-manager integration turns 2x-click of any .md file into rendered HTML in default browser with the same speed as opening the .md in a text editor.

By default it produces a complete, **HTML5-compliant** document styled with
[simple.css](https://simplecss.org/) (vendored at build time, no network
access at runtime). An optional **watch mode** keeps re-rendering the output
whenever the Markdown source is edited.

Project site: <https://maphew.github.io/mdo/>  
Public metrics: <https://maphew.github.io/mdo/metrics/>

## 🚀 Features

- ✅ Converts `.md` files to standalone HTML5 documents
- 🎨 Pretty default styling via embedded [simple.css](https://simplecss.org/)
- 🌓 Automatic light/dark mode (follows OS) plus a manual toggle button
- 📄 `--bare` flag emits a sanitized HTML fragment (no `<html>`/`<head>`/`<body>`/CSS)
- 🔒 Raw Markdown HTML is sanitized by default; use `--unsafe-html` to preserve it for trusted input
- 👀 `--watch` flag enables auto-rerender on file change (with debouncing)
- 🌐 `--open` flag renders to a temp dir and launches the system default browser
- 🧑‍🚀 `--tour` / no-arg interactive first-run guide, plus native Windows/Linux `mdo-setup` onboarding
- 🧭 Built-in `--install-file-manager` / `--uninstall-file-manager` integration (no separate install scripts needed)
- ⚡ Fast and self-contained — no runtime assets; `mdo.exe` remains the core CLI
- 🧩 Built on `pulldown-cmark`, `clap`, and `notify`

### Why?

There are countless Markdown-to-HTML converters available, so why make another one, becoming yet another [xkcd:927 joke](https://xkcd.com/927/)?

I could not find a simple, fast, and self-contained solution. Everything I looked at wanted to be a full-featured editor, relied on node or python in PATH, or needed some other runtime dependency. `md2htmlx` was very close, but did not calm my primary itch: every day I read dozens to hundreds of md files. Markdown is pretty darn good for authoring, awesome for diffs relative to other formats, but they're not very nice for reading. HTML is richer and calmer, and I find I absorb and understand more deeply.

Mdo + file-manager integration creates html pages so quickly they are throw-away friendly. I don't have to create an HTML file for every long Markdown file I want to read, or add a "make this an html report" to an agent workflow, regularly saving thousands of tokens. 

---

## 📦 Installation

### Native downloads

Download native archives for Linux, macOS, and Windows from
[GitHub Releases](https://github.com/maphew/mdo/releases), then verify them
with the accompanying `SHA256SUMS` file. The Linux archive includes `mdo`,
`mdo-open`, and `mdo-setup`; the Windows ZIP includes `mdo.exe`,
`mdo-open.exe`, and `mdo-setup.exe`, so you can use `mdo` without installing
Rust, Visual Studio, or the MSVC build tools. `mdo` stays the normal
command-line tool; double-click `mdo-setup` / `mdo-setup.exe` for a native
first-run window that can install file-manager integration without opening a
terminal. On Linux, `mdo-setup` uses `zenity`, `kdialog`, or `yad` if one is
available.

### From crates.io

```bash
cargo install mdo-cli
```

`cargo install` builds from source and therefore needs a working Rust build
toolchain. On Windows with the default `*-pc-windows-msvc` Rust target, install
**Visual Studio Build Tools** with **Desktop development with C++** (or the
equivalent Visual Studio workload) so `link.exe` is available. If Cargo reports
`linker link.exe not found`, use the GitHub Release ZIP above or install the
MSVC linker before retrying.

### Build from source

```bash
git clone https://github.com/maphew/mdo.git
cd mdo
cargo build --release
./target/release/mdo input.md
```

### Maintainer releases

GitHub releases are published from this fork by `.github/workflows/release.yml`.
Push a version tag such as `v0.2.0` to build Linux, macOS, and Windows archives
and publish them to a GitHub Release. The workflow can also be run manually with
an existing tag via **Actions -> Release -> Run workflow**.

The release workflow keeps repository access read-only for build jobs and grants
`contents: write` only to the final release-publishing job. GitHub Actions are
pinned to commit SHAs, with Dependabot configured to propose updates.

---

## 📦 Usage

```text
Usage: mdo [OPTIONS] [INPUT]

Arguments:
  [INPUT]  Input Markdown file

Options:
  -o, --output <OUTPUT>     Output HTML file (defaults to <input>.html alongside the input,
                            or to a temp directory when --open is used). Existing files are overwritten
  -w, --watch               Watch the input file and re-render on every change
  -b, --bare                Emit only the HTML fragment (no <html>, <head>, <body>, no CSS)
      --unsafe-html         Preserve raw HTML from the Markdown source instead of sanitizing it
      --open                Render to a temp directory and launch the system default browser.
                            The source folder is left untouched unless --output is given
      --tour                Show a first-run tour with safe next steps for new users
      --install-file-manager
                            Install per-user file-manager integration for Markdown files
      --uninstall-file-manager
                            Remove per-user file-manager integration installed by mdo
      --set-default         With --install-file-manager on Linux, make Open as HTML the default
                            Markdown handler. Windows default selection remains interactive
  -h, --help                Print help
  -V, --version             Print version
```

If `--output` is omitted, the output is written next to the input with the
extension changed to `.html` (e.g. `foo.md` → `foo.html`). Existing files are
overwritten without prompting.

When `--open` is used without `--output`, the rendered HTML goes to a stable
location under your OS temp directory (e.g.
`%TEMP%\mdo\<hash>\<name>.html` on Windows, or `/tmp/mdo-<uid>/<hash>/<name>.html`
on Unix-like systems) so the source folder stays
clean. Re-opening the same file overwrites the same temp output. A
`<base href="file:///…">` tag pointing at the source folder is automatically
injected whenever the output lives elsewhere, so relative images and links in
the Markdown still resolve correctly.

### Examples

Convert once and exit (default — produces a styled, standalone HTML5 page next to the input). The README.html in this repo is generated like this:

```bash
mdo input.md                    # writes input.html
mdo input.md -o docs/out.html   # writes docs/out.html
```

Emit a bare HTML fragment (useful for embedding in another template):

```bash
mdo --bare input.md
```

Raw HTML from the Markdown source is sanitized by default. Preserve it only
when the source is trusted:

```bash
mdo --unsafe-html input.md
```

Watch for changes and re-render on every save:

```bash
mdo --watch input.md
```

Render to a temp file and open it in your default browser (does **not**
write next to the source):

```bash
mdo --open input.md
```

This same render-to-temp path is what the built-in file-manager integration
registers, so double-click/right-click opens never leave `.html` artifacts
beside the source file.

### First-run tour

Running `mdo --tour` prints a short new-user path:

```bash
mdo --tour
```

When run interactively with no arguments, `mdo` shows the same terminal tour
and, on Windows and Linux, offers to install the reversible per-user **Open as
HTML** file-manager integration. The prompt defaults to **Yes**, but it still
does not change your default Markdown app; choose **No** to skip or run the
installer again later. After you press Enter to close the tour, mdo renders and
opens a short welcome sample so you can immediately verify the browser-opening
flow. In scripts or other non-interactive contexts, no-argument `mdo` still
exits with a usage error and suggests `mdo --tour`.

On Windows and Linux release builds, double-click `mdo-setup` /
`mdo-setup.exe` for the same onboarding path in native desktop dialogs. This
preserves `mdo` as the normal CLI while giving file-manager users a no-terminal
setup flow.

---

## Linux file manager integration

`mdo` can install or remove its own per-user XDG file-manager integration;
no companion install script is required:

```bash
# Native first-run window, including optional file-manager integration install
mdo-setup

# Add "Open as HTML" as an "Open With" handler for Markdown files
mdo --install-file-manager

# Same, but also make it the default Markdown handler
mdo --install-file-manager --set-default

# Undo everything the installer did
mdo --uninstall-file-manager
```

The installer writes `~/.local/share/applications/mdo.desktop`, whose
command is the current binary plus `--open %f`, and a small `Ⓜ` SVG icon under
`~/.local/share/icons/hicolor/scalable/apps/mdo.svg`. The desktop entry is
named **Open as HTML**, so GNOME Files/Nautilus and other XDG file managers
show an action-oriented entry instead of a tool-name-only entry. Rerunning the
installer also removes older Nautilus Scripts entries named **Preview with
mdo** or **Render with mdo**.

Linux release archives also include `mdo-setup`, which shows the same onboarding
path in desktop dialogs when `zenity`, `kdialog`, or `yad` is available. If you
launch `mdo-open` directly with no file, it opens `mdo-setup` when the setup
helper is present.

Result examples after install:

- Most XDG file managers: right-click a `.md` file → **Open With** →
  **Open as HTML**.
- With `--set-default`: double-clicking a Markdown file launches `mdo --open`.
- The rendered page opens in your default browser from a temp path such as
  `/tmp/mdo-<uid>/<hash>/<name>.html`, and no `.html` file is left beside the source.

---

## 🪟 Windows Explorer integration

`mdo.exe` can install or remove its own per-user Explorer integration (no
admin rights and no HKLM changes). Windows release ZIPs also include
`mdo-setup.exe`, a native no-terminal setup window for the same first-run flow:

```powershell
# Native first-run window, including optional Explorer integration install
.\mdo-setup.exe

# CLI install: add an "Open as HTML" right-click verb and Open With app entry
.\mdo.exe --install-file-manager

# Undo everything the installer did
.\mdo.exe --uninstall-file-manager
```

The installer registers **Open as HTML** for `.md` files. If `mdo-open.exe`
is present next to `mdo.exe`, it is used as the Explorer handler to avoid the
brief black console-window flash that Windows shows for normal console
binaries. If only `mdo.exe` is present, the installer still works and registers
`mdo.exe --open "%1"` directly, so a single downloaded executable is enough
for file-manager integration. The Windows binaries embed the mdo icon; the
installer also writes that icon to a per-user path and registers both `mdo.exe`
and `mdo-open.exe` with the friendly app name **Open as HTML**, so Windows
"Open with" surfaces do not need to expose the wrapper binary name. If you
launch `mdo-open.exe` directly with no file, it opens `mdo-setup.exe` when the
setup helper is present.

To make **Open as HTML** the *default* `.md` handler after running the install
command, right-click a `.md` file → **Open with → Choose another app** →
pick **Open as HTML** → tick **Always use this app**. Windows requires that
last step to be done interactively.

Result examples after install:

- Right-click any `.md` file → **Open as HTML**. On Windows 11, this may
  appear under **Show more options**.
- **Open with → Open as HTML** appears as an available app for Markdown files.
- If you make **Open as HTML** the default handler, double-clicking a `.md`
  file opens the rendered page in your browser.
- The rendered page opens from `%TEMP%\mdo\<hash>\<name>.html`, and the source
  folder stays unchanged.

---

## macOS Finder quick action

mdo does not ship a macOS installer script yet, but Finder can run `mdo --open`
through a per-user Automator Quick Action. Apple documents Quick Action
workflows and shell-script actions in the Automator User Guide:

- <https://support.apple.com/en-by/guide/automator/use-quick-action-workflows-aut73234890a/2.10/mac/15.0>
- <https://support.apple.com/guide/automator/use-scripts-aut4bb6b2b4f/mac>

Create a Quick Action in Automator, set it to receive files in Finder, add
**Run Shell Script**, set **Pass input** to **as arguments**, and use the
absolute path to `mdo`:

```bash
for file in "$@"; do
  /path/to/mdo --open "$file"
done
```

Save the workflow as **Open as HTML**. The result is:

- Finder shows **Quick Actions → Open as HTML** for selected Markdown files.
- Running the Quick Action opens the rendered page in your browser.
- The rendered page opens from a temp path under `$TMPDIR/mdo/<hash>/<name>.html`,
  and no `.html` file is left beside the source.

---

## 🎨 Default output

The default (non-`--bare`) output is a complete HTML5 document:

- `<!DOCTYPE html>` + `<html lang="en">`
- UTF-8 charset and responsive viewport meta
- A `<meta name="generator" content="mdo <version>">` marker
- `<title>` derived from the first `# Heading` in the source (falls back to the
  input filename)
- An inlined copy of [simple.css](https://simplecss.org/) inside `<style>`,
  giving you sensible typography and automatic light/dark mode out of the box
- A small floating ☀/☾ button (top-right) for manually overriding the theme;
  the choice is remembered in `localStorage`
- Body content wrapped in `<main>`
- A tiny footer showing the mdo version, render duration, and UTC generation date

Markdown extensions enabled: tables, footnotes, task lists, strikethrough.
Rendered Markdown HTML is sanitized by default to remove active content such as
scripts and event-handler attributes.

---

## 🙏 Credits

This project is a grateful fork of Hafiz Ali Raza's original
Markdown-to-HTML CLI. Hafiz remains credited as an author, and this fork keeps
that lineage explicit so future improvements can be offered back upstream.

This fork adds:

- Convert-and-exit as the default; watch mode is now opt-in (`--watch`)
- Optional `--output` (defaults to `<input>.html` next to the source)
- Standalone HTML5 output with embedded [simple.css](https://simplecss.org/)
- A `--bare` flag that preserves the original fragment-only behavior
- Default sanitization for rendered Markdown HTML, with `--unsafe-html` for
  trusted sources that need raw HTML preserved
- An `--open` flag that renders to a temp directory and launches the system
  default browser (with auto-injected `<base href>` so relative refs resolve)
- Light/dark theme toggle button overlaid on the rendered page
- Title auto-derived from the first heading
- Passive generated-page attribution via generator metadata and a tiny
  dated footer; no network calls or identifiers are added
- Debounced file-change events (no more duplicate renders per save)
- Surfaced watcher errors instead of swallowing them
- Markdown extensions: tables, footnotes, task lists (in addition to strikethrough)

The bundled [simple.css](https://simplecss.org/) is © 2020
[Kev Quirk](https://kevquirk.com/) and distributed under the MIT License — see
[`assets/simple.css.LICENSE`](assets/simple.css.LICENSE).

---

## 📄 License

This project is dual-licensed under either of:

- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option, matching the licensing of the upstream project.
