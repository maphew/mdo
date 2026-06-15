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
- ⚡ Fast and self-contained — single binary, no runtime assets
- 🧩 Built on `pulldown-cmark`, `clap`, and `notify`

### Why?

There are countless Markdown-to-HTML converters available, so why make another one, becoming yet another [xkcd:927 joke](https://xkcd.com/927/)?

I could not find a simple, fast, and self-contained solution. Everything I looked at wanted to be a full-featured editor, relied on node or python in PATH, or needed some other runtime dependency. `md2htmlx` was very close, but did not calm my primary itch: every day I read dozens to hundreds of md files. Markdown is pretty darn good for authoring, awesome for diffs relative to other formats, but they're not very nice for reading. HTML is richer and calmer, and I find I absorb and understand more deeply.

Mdo + file-manager integration creates html pages so quickly they are throw-away friendly. I don't have to create an HTML file for every long Markdown file I want to read, or add a "make this an html report" to an agent workflow, regularly saving thousands of tokens. 

---

## 📦 Installation

### From crates.io

```bash
cargo install mdo-cli
```

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
Usage: mdo [OPTIONS] <INPUT>

Arguments:
  <INPUT>  Input Markdown file

Options:
  -o, --output <OUTPUT>  Output HTML file (defaults to <input>.html alongside the input,
                         or to a temp directory when --open is used). Existing files are overwritten
  -w, --watch            Watch the input file and re-render on every change
  -b, --bare             Emit only the HTML fragment (no <html>, <head>, <body>, no CSS)
      --unsafe-html      Preserve raw HTML from the Markdown source instead of sanitizing it
      --open             Render to a temp directory and launch the system default browser.
                         The source folder is left untouched unless --output is given
  -h, --help             Print help
  -V, --version          Print version
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

This is the recommended setup for a Windows **Open as HTML** file
association — use the bundled `mdo-open.exe` wrapper and double-clicking
a `.md` file in Explorer will render to the platform temp directory and
open it without leaving any artifacts in the source folder.

---

## Linux file manager integration

The repo also ships Linux helpers under [`scripts/`](scripts) for per-user
file-manager integration:

```bash
# Add "Open as HTML" as an "Open With" handler for Markdown files
./scripts/install-linux-file-manager.sh

# Same, but also make it the default Markdown handler
./scripts/install-linux-file-manager.sh --set-default

# Undo everything the install script did
./scripts/uninstall-linux-file-manager.sh
```

The installer writes `~/.local/share/applications/mdo.desktop`, whose
command is `mdo --open %f`, plus a small `Ⓜ` SVG icon under
`~/.local/share/icons/hicolor/scalable/apps/mdo.svg`. The desktop entry is
named **Open as HTML**, so GNOME Files/Nautilus and other XDG file managers
show an action-oriented entry instead of a tool-name-only entry. Rerunning the
installer also removes older Nautilus Scripts entries named **Preview with
mdo** or **Render with mdo**.

Pass `--exe /path/to/mdo` if the binary is not on `PATH`. The script looks
for `mdo` on `PATH` first, then falls back to `target/release/mdo`
next to this repo after `cargo build --release`.

Result examples after install:

- Most XDG file managers: right-click a `.md` file → **Open With** →
  **Open as HTML**.
- With `--set-default`: double-clicking a Markdown file launches `mdo --open`.
- The rendered page opens in your default browser from a temp path such as
  `/tmp/mdo/<hash>/<name>.html`, and no `.html` file is left beside the source.

---

## 🪟 Windows Explorer integration

The repo ships two PowerShell helpers under [`scripts/`](scripts) that wire
mdo into Explorer for the current user only (no admin, no HKLM changes):

```powershell
# Add: an "Open as HTML" right-click verb and Open With app entry
powershell -ExecutionPolicy Bypass -File .\scripts\install-explorer.ps1

# Undo everything the install script did
powershell -ExecutionPolicy Bypass -File .\scripts\uninstall-explorer.ps1
```

The install script registers `mdo-open.exe`, a tiny windows-subsystem
wrapper built alongside `mdo.exe`. The wrapper exists for one reason:
when Explorer launches a normal console binary it briefly flashes a black
console window. `mdo-open.exe` runs as a GUI subsystem app and spawns
`mdo.exe --open` with `CREATE_NO_WINDOW`, so double-clicking a `.md`
file renders from the platform temp directory and opens straight in the
browser with no flash. The regular CLI is
unchanged — `mdo.exe` from a terminal still prints to stdout normally.

The install script auto-locates `mdo-open.exe` via `PATH`, falling
back to `target\release\mdo-open.exe` next to the repo. Pass
`-ExePath C:\path\to\mdo-open.exe` to override. `mdo.exe` must
sit next to `mdo-open.exe`; both are produced by `cargo build
--release` and `cargo install mdo-cli`.

It also generates a small `.ico` at `%LOCALAPPDATA%\mdo\md.ico` by
rendering a single Unicode character — by default Ⓜ (circled M) in a
mid-tone blue chosen so it stays legible in both light and dark Explorer
themes. Override either via parameters:

```powershell
.\scripts\install-explorer.ps1 -IconChar "📄" -IconColor "#E64A19"
```

`uninstall-explorer.ps1` removes the .ico (and its folder if empty)
along with all the registry keys. It also removes the old **Preview with mdo**
and **Render with mdo** verbs from earlier installs.

To make **Open as HTML** the *default* `.md` handler after running the install
script, right-click a `.md` file → **Open with → Choose another app** →
pick **Open as HTML** → tick **Always use this app**. Windows requires that
last step to be done interactively.

Result examples after install:

- Right-click any `.md` file → **Open as HTML**. On Windows 11, this may
  appear under **Show more options**.
- **Open with → Open as HTML** appears as an available app for Markdown files.
- If you make **Open as HTML** the default handler, double-clicking a `.md`
  file opens the rendered page in your browser with no console-window flash.
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
