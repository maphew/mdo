# Security Policy

## Reporting a vulnerability

Please report suspected vulnerabilities privately via
[GitHub private vulnerability reporting](https://github.com/maphew/mdo/security/advisories/new)
rather than opening a public issue. You should get a first response within a
week; this is a spare-time project, so please allow some slack.

## Scope notes

- mdo renders local Markdown files to local HTML. It makes **no network
  calls at runtime** — styling is embedded at build time.
- Raw HTML inside Markdown is **sanitized by default** (scripts and
  event-handler attributes removed, via [ammonia](https://crates.io/crates/ammonia)).
  The `--unsafe-html` flag disables that sanitization and is documented as
  trusted-input-only; output produced with `--unsafe-html` on hostile input is
  outside the threat model.
- File-manager integration is strictly **per-user** (no admin rights, no
  HKLM writes on Windows) and reversible via `--uninstall-file-manager`.
- Browser launches on Windows use the `ShellExecuteW` API directly rather
  than `cmd /C start`, so shell metacharacters in file paths are never
  interpreted as commands.

Supported version: the latest release. There are no security backports to
older versions.
