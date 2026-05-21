# 📝 md2htmlx — Markdown to HTML5 Converter (with optional live watch)

`md2htmlx` is a small, fast command-line tool written in Rust that converts
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
cargo install md2htmlx
```

### Build from source

```bash
git clone https://github.com/maphew/md2htmlx.git
cd md2htmlx
cargo build --release
./target/release/md2htmlx input.md
```

---

## 📦 Usage

```text
Usage: md2htmlx [OPTIONS] <INPUT>

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
`%TEMP%\md2htmlx\<hash>\<name>.html` on Windows) so the source folder stays
clean. Re-opening the same file overwrites the same temp output. A
`<base href="file:///…">` tag pointing at the source folder is automatically
injected whenever the output lives elsewhere, so relative images and links in
the Markdown still resolve correctly.

### Examples

Convert once and exit (default — produces a styled, standalone HTML5 page next to the input). The README.html in this repo is generated like this:

```bash
md2htmlx input.md                    # writes input.html
md2htmlx input.md -o docs/out.html   # writes docs/out.html
```

Emit a bare HTML fragment (useful for embedding in another template):

```bash
md2htmlx --bare input.md
```

Watch for changes and re-render on every save:

```bash
md2htmlx --watch input.md
```

Render to a temp file and open it in your default browser (does **not**
write next to the source):

```bash
md2htmlx --open input.md
```

This is the recommended setup for a Windows "Open with md2htmlx" file
association — point the verb at `md2htmlx.exe --open "%1"` and double-clicking
a `.md` file in Explorer will render and open it without leaving any artifacts
in the source folder.

---

## Linux file manager integration

The repo also ships Linux helpers under [`scripts/`](scripts) for per-user
file-manager integration:

```bash
# Add md2htmlx as an "Open With" Markdown handler and, on GNOME Files/Nautilus,
# add a right-click Scripts entry named "Render with md2htmlx"
./scripts/install-linux-file-manager.sh

# Same, but also make md2htmlx the default Markdown handler
./scripts/install-linux-file-manager.sh --set-default

# Undo everything the install script did
./scripts/uninstall-linux-file-manager.sh
```

The installer writes `~/.local/share/applications/md2htmlx.desktop`, whose
command is `md2htmlx --open %f`, plus a small `Ⓜ` SVG icon under
`~/.local/share/icons/hicolor/scalable/apps/md2htmlx.svg`. On GNOME
Files/Nautilus it also writes
`~/.local/share/nautilus/scripts/Render with md2htmlx`; use it from
right-click → **Scripts** → **Render with md2htmlx**.

Pass `--exe /path/to/md2htmlx` if the binary is not on `PATH`. The script looks
for `md2htmlx` on `PATH` first, then falls back to `target/release/md2htmlx`
next to this repo after `cargo build --release`.

---

## 🪟 Windows Explorer integration

The repo ships two PowerShell helpers under [`scripts/`](scripts) that wire
md2htmlx into Explorer for the current user only (no admin, no HKLM changes):

```powershell
# Add: an "Open with → md2htmlx" entry and a
#      "Render with md2htmlx" right-click verb on .md files
powershell -ExecutionPolicy Bypass -File .\scripts\install-explorer.ps1

# Undo everything the install script did
powershell -ExecutionPolicy Bypass -File .\scripts\uninstall-explorer.ps1
```

The install script registers `md2htmlx-open.exe`, a tiny windows-subsystem
wrapper built alongside `md2htmlx.exe`. The wrapper exists for one reason:
when Explorer launches a normal console binary it briefly flashes a black
console window. `md2htmlx-open.exe` runs as a GUI subsystem app and spawns
`md2htmlx.exe --open` with `CREATE_NO_WINDOW`, so double-clicking a `.md`
file opens straight in the browser with no flash. The regular CLI is
unchanged — `md2htmlx.exe` from a terminal still prints to stdout normally.

The install script auto-locates `md2htmlx-open.exe` via `PATH`, falling
back to `target\release\md2htmlx-open.exe` next to the repo. Pass
`-ExePath C:\path\to\md2htmlx-open.exe` to override. `md2htmlx.exe` must
sit next to `md2htmlx-open.exe`; both are produced by `cargo build
--release` and `cargo install md2htmlx`.

It also generates a small `.ico` at `%LOCALAPPDATA%\md2htmlx\md.ico` by
rendering a single Unicode character — by default Ⓜ (circled M) in a
mid-tone blue chosen so it stays legible in both light and dark Explorer
themes. Override either via parameters:

```powershell
.\scripts\install-explorer.ps1 -IconChar "📄" -IconColor "#E64A19"
```

`uninstall-explorer.ps1` removes the .ico (and its folder if empty)
along with all the registry keys.

To make md2htmlx the *default* `.md` handler after running the install
script, right-click a `.md` file → **Open with → Choose another app** →
pick **md2htmlx** → tick **Always use this app**. Windows requires that
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

This project is a grateful fork of
**[rust-md2html](https://github.com/haffizaliraza/rust-md2html)** by
[Hafiz Ali Raza](https://github.com/haffizaliraza), which provided the original
Markdown-to-HTML CLI and watch-mode skeleton. Thank you for the head start!

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
