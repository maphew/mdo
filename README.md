# 📝 mdo — Markdown to HTML5 Converter (with optional live watch)

`mdo` is a small, fast command-line tool written in Rust that converts
Markdown files into HTML.

By default it produces a complete, **HTML5-compliant** document styled with
[simple.css](https://simplecss.org/) (vendored at build time — no network
access at runtime). An optional **watch mode** keeps re-rendering the output
whenever the Markdown source is edited.

---

## 🚀 Features

- ✅ Converts `.md` files to standalone HTML5 documents
- 🎨 Pretty default styling via embedded [simple.css](https://simplecss.org/)
- 🌓 Automatic light/dark mode (follows OS) plus a manual toggle button
- 📄 `--bare` flag emits a raw HTML fragment (no `<html>`/`<head>`/`<body>`/CSS)
- 👀 `--watch` flag enables auto-rerender on file change (with debouncing)
- 🌐 `--open` flag renders to a temp dir and launches the system default browser
- ⚡ Fast and self-contained — single binary, no runtime assets
- 🧩 Built on `pulldown-cmark`, `clap`, and `notify`

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
  -b, --bare             Emit only the raw HTML fragment (no <html>, <head>, <body>, no CSS)
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
`%TEMP%\mdo\<hash>\<name>.html` on Windows) so the source folder stays
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

Watch for changes and re-render on every save:

```bash
mdo --watch input.md
```

Render to a temp file and open it in your default browser (does **not**
write next to the source):

```bash
mdo --open input.md
```

This is the recommended setup for a Windows "Open with mdo" file
association — use the bundled `mdo-open.exe` wrapper and double-clicking
a `.md` file in Explorer will render to the platform temp directory and
open it without leaving any artifacts in the source folder.

---

## Linux file manager integration

The repo also ships Linux helpers under [`scripts/`](scripts) for per-user
file-manager integration:

```bash
# Add mdo as an "Open With" Markdown handler and, on GNOME Files/Nautilus,
# add a right-click Scripts entry named "Preview with mdo"
./scripts/install-linux-file-manager.sh

# Same, but also make mdo the default Markdown handler
./scripts/install-linux-file-manager.sh --set-default

# Undo everything the install script did
./scripts/uninstall-linux-file-manager.sh
```

The installer writes `~/.local/share/applications/mdo.desktop`, whose
command is `mdo --open %f`, plus a small `Ⓜ` SVG icon under
`~/.local/share/icons/hicolor/scalable/apps/mdo.svg`. On GNOME
Files/Nautilus it also writes
`~/.local/share/nautilus/scripts/Preview with mdo`; use it from
right-click → **Scripts** → **Preview with mdo**. Older installs may still
show **Render with mdo** until you rerun the installer.

Pass `--exe /path/to/mdo` if the binary is not on `PATH`. The script looks
for `mdo` on `PATH` first, then falls back to `target/release/mdo`
next to this repo after `cargo build --release`.

---

## 🪟 Windows Explorer integration

The repo ships two PowerShell helpers under [`scripts/`](scripts) that wire
mdo into Explorer for the current user only (no admin, no HKLM changes):

```powershell
# Add: an "Open with → mdo" entry and a
#      "Preview with mdo" right-click verb on .md files
powershell -ExecutionPolicy Bypass -File .\scripts\install-explorer.ps1

# Undo everything the install script did
powershell -ExecutionPolicy Bypass -File .\scripts\uninstall-explorer.ps1
```

The install script registers `mdo-open.exe`, a tiny windows-subsystem
wrapper built alongside `mdo.exe`. The wrapper exists for one reason:
when Explorer launches a normal console binary it briefly flashes a black
console window. `mdo-open.exe` runs as a GUI subsystem app and spawns
`mdo.exe --open` with `CREATE_NO_WINDOW`, so double-clicking a `.md`
file previews from the platform temp directory and opens straight in the
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
along with all the registry keys. It also removes the old **Render with mdo**
verb from earlier installs.

To make mdo the *default* `.md` handler after running the install
script, right-click a `.md` file → **Open with → Choose another app** →
pick **mdo** → tick **Always use this app**. Windows requires that
last step to be done interactively.

---

## 🎨 Default output

The default (non-`--bare`) output is a complete HTML5 document:

- `<!DOCTYPE html>` + `<html lang="en">`
- UTF-8 charset and responsive viewport meta
- `<title>` derived from the first `# Heading` in the source (falls back to the
  input filename)
- An inlined copy of [simple.css](https://simplecss.org/) inside `<style>`,
  giving you sensible typography and automatic light/dark mode out of the box
- A small floating ☀/☾ button (top-right) for manually overriding the theme;
  the choice is remembered in `localStorage`
- Body content wrapped in `<main>`

Markdown extensions enabled: tables, footnotes, task lists, strikethrough.

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
- An `--open` flag that renders to a temp directory and launches the system
  default browser (with auto-injected `<base href>` so relative refs resolve)
- Light/dark theme toggle button overlaid on the rendered page
- Title auto-derived from the first heading
- Debounced file-change events (no more duplicate renders per save)
- Surfaced watcher errors instead of swallowing them
- Markdown extensions: tables, footnotes, task lists (in addition to strikethrough)

The bundled [simple.css](https://simplecss.org/) is © 2020
[Kev Quirk](https://kevquirk.com/) and distributed under the MIT License — see
[`assets/simple.css.LICENSE`](assets/simple.css.LICENSE).

---

## 📄 License

This project is dual-licensed under either of:

- MIT License ([LICENSE-MIT](https://opensource.org/licenses/MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](https://www.apache.org/licenses/LICENSE-2.0.html))

at your option, matching the licensing of the upstream project.
