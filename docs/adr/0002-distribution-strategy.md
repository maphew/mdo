# ADR 0002: Distribution strategy for `mdo`

- **Status**: Accepted
- **Date**: 2026-05-29
- **Deciders**: Matt
- **Supersedes**: none

## Context

ADR 0001 renamed the project to `mdo` and chose `mdo-cli` as the
crates.io package name while keeping `mdo` as the installed binary name.
The follow-up question was whether to also claim matching package names
across npm, PyPI, and other package indices.

The registry state checked on 2026-05-29 was:

| index | `mdo` | `mdo-cli` | note |
|---|---|---|---|
| crates.io | taken | free | `mdo` is a dormant monadic-do crate; `mdo-cli` is available |
| npm | taken | taken | `mdo-cli` is an old todo CLI and already exposes a `mdo` binary |
| PyPI | taken | free | `mdo-cli` is available but would need a real Python-facing package |
| Homebrew core | free | free | no formula present |
| WinGet | free | n/a | no matching package observed |
| Scoop main | free | n/a | no matching package observed |

The project is a Rust CLI. The canonical user experience is installing a
native binary, not importing a JavaScript or Python library.

## Decision

Keep **`mdo`** as the executable name.

Publish the canonical Rust package as **`mdo-cli`** on crates.io:

```bash
cargo install mdo-cli
```

Use GitHub Releases for the default user installation path: downloadable
native binaries and checksums. `cargo install mdo-cli` remains available
for Rust users and contributors, but it compiles from source and requires
the platform's native build tools.

Treat OS package managers as the next packaging layer:

- Homebrew tap formula named `mdo`
- WinGet package id such as `Maphew.Mdo`
- Scoop manifest named `mdo`

Do not publish placeholder packages to npm or PyPI just to reserve names.
Only publish there later if the packages provide real install value, such
as a maintained wrapper that installs the native binary and clearly points
back to the Rust project.

## Consequences

**Positive**

- The command users type remains the strongest name: `mdo`.
- The primary user install path does not require a Rust toolchain.
- The project avoids occupying unrelated ecosystems with empty packages.
- Native binary releases cover users who do not want a Rust toolchain.
- Homebrew, WinGet, and Scoop provide better CLI distribution paths than
  npm/PyPI for this project.

**Negative / accepted**

- The package name is not uniform across all indices.
- `cargo install mdo-cli` differs from the executable name `mdo`.
- npm cannot be used cleanly for `mdo` or `mdo-cli`; both names are already
  occupied, and `mdo-cli` already installs a `mdo` command.
- PyPI `mdo-cli` remains unclaimed unless a real Python package is created.

## Rejected options

**Rename to `mdoh`.** This appeared clean across crates.io, npm, and PyPI
on 2026-05-29, but it is a weaker name. `mdo` is shorter, easier to type,
and maps directly to the "Markdown open" positioning.

**Publish placeholder npm/PyPI packages.** This would reserve names but
would add maintenance burden and noise without improving the product. It
also risks looking like namespace squatting.

**Use npm or PyPI as first-class package channels.** These ecosystems are
not a natural fit for a Rust-native CLI unless the project later ships
well-maintained wrappers.
