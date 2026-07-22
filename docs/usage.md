# mdo usage

Complete CLI reference and output details for `mdo`. For a quick start, see
the [README](https://github.com/maphew/mdo#readme); for desktop integration,
see [File-manager integration](file-manager-integration.html).

## CLI reference

```text
Usage: mdo [OPTIONS] [INPUT]

Arguments:
  [INPUT]  Input Markdown file

Options:
  -o, --output <OUTPUT>     Output HTML file (defaults to <input>.html alongside the input,
                            or to a temp directory when --open is used). Existing files are overwritten
  -w, --watch               Watch the input file and re-render on every change
  -b, --bare                Emit only the HTML fragment (no <html>, <head>, <body>, no CSS)
      --css <FILE>          Append a custom CSS file after mdo's default styling
      --unsafe-html         Preserve raw HTML from the Markdown source instead of sanitizing it
  -v, --verbose             Report timing diagnostics for the render workflow (read, markdown,
                            sanitize, assemble, write, total) on stderr. The generated HTML is unchanged
      --open                Render to a temp directory and launch the system default browser.
                            The source folder is left untouched unless --output is given
      --setup               Show a first-run setup with safe next steps for new users
      --install-file-manager
                            Install per-user file-manager integration for Markdown files
      --uninstall-file-manager
                            Remove per-user file-manager integration installed by mdo
      --set-default         When installing on Linux, make Open as HTML the default Markdown
                            handler. Windows still requires choosing the default app interactively
  -h, --help                Print help (see more with '--help')
  -V, --version             Print version
```

Running `mdo` with no arguments prints a short **Open Markdown as HTML**
landing page and exits successfully. `mdo` exits non-zero when a one-shot
render fails, so scripts can detect errors; `--watch` keeps running through
render errors.

## Output location

If `--output` is omitted, the output is written next to the input with the
extension changed to `.html` (e.g. `foo.md` → `foo.html`). Existing files are
overwritten without prompting.

When `--open` is used without `--output`, the rendered HTML goes to a stable
location under your OS temp directory so the source folder stays clean:

```text
Windows  %TEMP%\mdo\<hash>\<name>.html
Linux    /tmp/mdo-<uid>/<hash>/<name>.html
macOS    $TMPDIR/mdo-<uid>/<hash>/<name>.html
```

Re-opening the same file overwrites the same temp output. A
`<base href="file:///…">` tag pointing at the source folder is automatically
injected whenever the output lives elsewhere, so relative images and links in
the Markdown still resolve correctly.

## Examples

Convert once and exit (default — produces a styled, standalone HTML5 page
next to the input):

```bash
mdo input.md                    # writes input.html
mdo input.md -o docs/out.html   # writes docs/out.html
```

Render to a temp file and open it in your default browser (does **not**
write next to the source). This is the same path the file-manager
integration uses:

```bash
mdo --open input.md
```

Watch for changes and re-render on every save:

```bash
mdo --watch input.md
```

Emit a bare HTML fragment (useful for embedding in another template):

```bash
mdo --bare input.md
```

Append custom CSS after the built-in styles (see
[Custom CSS](custom-css.html) for details):

```bash
mdo --css my-overrides.css input.md
```

Raw HTML from the Markdown source is sanitized by default. Preserve it only
when the source is trusted:

```bash
mdo --unsafe-html input.md
```

Show timing diagnostics on stderr (the generated page is unchanged —
performance information stays in the terminal):

```bash
mdo --verbose input.md
```

## First-run setup

```bash
mdo --setup
```

The guided setup explains the render-and-open workflow and offers to install
the reversible per-user **Open as HTML** file-manager integration on Windows
and Linux. The prompt defaults to **Yes**, but it does not change your default
Markdown app; choose **No** to skip or run the installer again later. After
setup finishes, mdo renders and opens a short welcome sample so you can
immediately verify the browser-opening flow.

Windows and Linux release archives also include a double-clickable
`mdo-setup` launcher that opens the same guide without a terminal already
open — see [File-manager integration](file-manager-integration.html).

## Default output

The default (non-`--bare`) output is a complete HTML5 document:

- `<!DOCTYPE html>` + `<html lang="en">`
- UTF-8 charset and responsive viewport meta
- A `<meta name="generator" content="mdo <version>">` marker — the only
  provenance mdo adds; there is no visible branding, network call, or
  identifier in the page
- `<title>` derived from the first `# Heading` in the source (falls back to
  the input filename; headings inside fenced code blocks are ignored)
- An inlined copy of [simple.css](https://simplecss.org/) inside `<style>`,
  followed by mdo's calmer default typography (`body` 1rem, `h1` 2.4rem,
  `h2` 2rem, `h3` 1.4rem) sourced from
  [`assets/mdo-default-typography.css`](https://github.com/maphew/mdo/blob/main/assets/mdo-default-typography.css)
- Optional custom CSS from `--css <FILE>`, appended after the built-in styles
  so your rules can override them
- A small floating ☀/☾ button (top-right) for manually overriding the
  light/dark theme; it is keyboard accessible with a state-aware
  `aria-label`, follows OS theme changes until a manual choice is made, and
  remembers that choice in `localStorage` when available (it still works on
  `file://` pages where storage is blocked)
- Body content wrapped in `<main>`
- A restrained footer showing the source file's modification time
  (`Source modified: …`) with a machine-readable `<time datetime>`; it is
  omitted when the timestamp is unavailable

Markdown extensions enabled: tables, footnotes, task lists, strikethrough.
Rendered Markdown HTML is sanitized by default to remove active content such
as scripts and event-handler attributes.
