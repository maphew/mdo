# ADR 0001: Rename `md2htmlx` → `mdo`

- **Status**: Accepted
- **Date**: 2026-05-21
- **Deciders**: Matt
- **Supersedes**: none

## Context

The CLI ships as `md2htmlx`. The name is hard to type (number row plus a
trailing `x`), doesn't describe the tool's actual headline behaviour, and
echoes the half-dozen near-identical names already in the `md2html*` space.

The tool's differentiator isn't "another markdown → HTML converter" —
there are many of those, including the direct Rust peer
`magiclen/markdown2html-converter`. The differentiator is the
**open-and-view experience**: a single static binary that renders to a
self-contained HTML5 page styled with simple.css, with `--watch`,
`--open` to a temp dir, and Windows Explorer integration via a GUI-subsystem
wrapper that avoids the console-window flash on double-click. The honest
one-line pitch is closer to *"double-click your .md files and they just
render"* than *"convert markdown to HTML."*

### Questions

1. Is there an existing tool to contribute to instead of shipping another?
2. Is `md2h` (the natural short form of the current name) already in use?
3. What short names are clean across npm, PyPI, crates.io, and GitHub?
4. Of the clean names, which match the tool's headline feature?

### Research

**Namespace survey.** `md2h*` as a GitHub substring covers 674 repos,
almost all variations on `md2html`. The exact name `md2h` is unclaimed on
npm, PyPI, and crates.io; the three GitHub repos with that exact name are
personal repos with zero or one star and no published tooling. The
namespace is functionally clear but the *substring* is exhausted.

**Contribute-vs-fork.** The closest peer,
`magiclen/markdown2html-converter`, is a single-binary Rust CLI emitting
standalone HTML, but it bundles github-markdown-css + highlight.js +
MathJax by default. That kitchen-sink-with-JS posture is the opposite of
this tool's lean-static stance. Adding simple.css styling,
`--open`-to-temp behaviour, and Windows shell integration there would be
a rewrite, not a contribution. Server-based previewers (`grip`,
`markserv`, `mdview`) solve a different problem (they run a server). None
of the static-output tools surveyed ship Windows Explorer integration
with the no-flash GUI wrapper.

**Conclusion on positioning.** Shipping a separate tool is justified.
xkcd:927 doesn't apply once the README frames the tool around the
double-click-on-Windows experience rather than "markdown to HTML."

### Name candidates evaluated

All checked across npm / PyPI / crates.io / GitHub exact-name repos.

| candidate | npm | PyPI | crates.io | exact GH | verdict |
|---|---|---|---|---|---|
| `mdpeek` | free | free | free | 0 | clean; 6 chars |
| `mdoh` | free | free | free | 2 trivial | clean; "M-D, oh!" |
| `mdpop` | free | free | free | 2 | clean; 5 chars |
| `mdh5` | free | free | free | 1 | rejected — number-row reach |
| `mdo` | taken (dormant) | taken (abandoned) | taken (dormant 2016) | 8 | accepted with caveat |
| `mdv` | taken (active) | **taken (Axiros TMV)** | **taken (active 2026-05)** | – | rejected — xkcd:927 trap |
| `unmd` | taken | free | free | 4 | viable but less mnemonic |

**`mdv` rejected.** crates.io `mdv` is the Rust port of Axiros's
Terminal Markdown Viewer, v3.0.0, last updated 2026-05-16 — actively
maintained, in the same problem space. PyPI `mdv` is the same project's
Python lineage. Three letters + same use case = the exact xkcd:927
condition.

**`mdh5` rejected** on typing ergonomics. Number-row reaches are what
we're trying to escape from.

**`mdo` accepted.** Three letters, alternating hands on the home row
(`m` right index → `d` left middle → `o` right ring), no shifts, no
reaches. Mnemonic "md open" maps to the `--open` headline feature. All
three "taken" claims turn out to be unrelated and inactive:

- crates.io `mdo`: "Monadic do notation for rust using macro and duck
  typing", last touched 2016-06-23. A decade dormant, semantically
  unrelated to markdown.
- PyPI `mdo`: no summary, no homepage, 4 versions all from a two-day
  window in May 2020. Abandoned.
- npm `mdo`: "Markdown Object Parser", last touched 2022. Different
  problem (parsing AST, not rendering).

## Decision

Rename to **`mdo`**.

Publish on crates.io as **`mdo-cli`**, because the bare `mdo` crate
slot is held by the dormant monad library. The binary name stays
`mdo`. Users run `cargo install mdo-cli` once and `mdo file.md`
thereafter. The crate-name vs binary-name split is invisible after
install.

Optionally pursue a crates.io name transfer for the dormant `mdo`
crate in the background. If granted, republish under the bare name
with the same binary name; no user-facing change.

### Fallback

If `mdo-cli` becomes unworkable (transfer denied AND `mdo-cli` claimed
in the interim AND the `-cli` suffix feels wrong), fall back to
**`mdoh`** as both crate and binary name. `mdoh` is clean across all
four registries today and reads as the interjection "M-D, oh!" —
on-brand if less self-documenting.

## Consequences

**Positive**

- Three-letter binary, easy to type, no number-row reaches.
- Mnemonic ("md open") aligns with the marquee `--open` feature.
- No active collision in the markdown-tooling space.
- The shell-integration pitch (`.md` → `mdo` → browser) fits in one
  line.

**Negative / accepted**

- Crate name carries the `-cli` suffix until/unless the dormant `mdo`
  crate is transferred. Visible only in `cargo install` instructions.
- The `rust-md2html` crate on crates.io becomes a dead name. crates.io
  has no forwarding primitive. Acceptable: 0 stars on the repo, no
  known installed base.
- Windows Explorer registry keys installed by the previous version
  will orphan on any user machines that have them. Mitigation: ship a
  note in release notes pointing at the uninstall script. No automated
  migration — the install footprint is small and contained.
- GitHub repo rename will eventually be needed
  (`maphew/md2htmlx` → `maphew/mdo`); GitHub's automatic redirects
  cover inbound links for the repo itself but not for crates.io.

## Notes

This ADR captures the state at decision time. The crates.io name
transfer attempt, if pursued, gets its own follow-up ADR if the
outcome changes the public crate name.
