# ADR 0004: State-aware desktop setup launcher

- **Status**: Proposed
- **Date**: 2026-07-10
- **Deciders**: Matt
- **Related issues**: `mdo-u79`, `mdo-gfw`

## Context

`mdo-setup` currently opens the first-run terminal tour. The tour always asks
whether to install file-manager integration, even when that integration is
already installed. Linux also ships no visible desktop entry for `mdo-setup`:
the only entry created by integration setup is the deliberately hidden
`mdo.desktop` file handler. Windows packages expose `mdo-setup.exe` as a
command, but do not consistently create a Start menu shortcut.

The setup surface should remain useful after onboarding. It should report the
current integration state, offer only actions that make sense in that state,
remove integration cleanly, and explain how to remove the binaries without
trying to delete the running executable.

## Recommendation

Build the state-aware setup flow, but treat it as a small integration manager,
not as a general uninstaller. The value is strongest on Linux, where a visible
application-menu entry fixes the current discoverability gap. The same state
model is worthwhile on Windows and prevents repeat runs from presenting a
misleading install prompt.

Keep the persistent setup launcher separate from the file-handler
registration. Removing “Open as HTML” must not remove the launcher. A full
cleanup action may remove both, then print the appropriate binary-removal
commands; it must not delete or replace running binaries.

## State model

Expose a platform-neutral report rather than only a three-value enum:

```text
IntegrationReport {
    availability: NotInstalled | OpenWith | Default,
    launcher: Missing | Installed | Stale,
    details: platform-specific observations and warnings,
}
```

`availability` drives the normal prompt. `launcher` and `details` preserve
partial, stale, and unqueryable states without pretending that they are a
healthy `OpenWith` installation. An I/O or platform-query failure is an error,
not `NotInstalled`.

In implementation, model handler health explicitly (`Healthy`, `Partial`, or
`Stale`) so a malformed registration can retain its observed availability
without being presented as ready to use.

The contextual flow remains one decision per run:

| Current state | Primary choices |
|---|---|
| `NotInstalled` | Install for Open With; install and request default; keep unchanged |
| `OpenWith` | Keep; request/set default; remove integration |
| `Default` | Keep; remove integration |
| Partial or stale | Repair; remove remnants; keep unchanged |

“Complete cleanup” is a secondary, explicitly named action. It removes the
file-handler registration, launcher registration, and mdo-owned icon, then
prints binary-removal guidance.

## Linux design

The handler is installed when
`$XDG_DATA_HOME/applications/mdo.desktop` exists, is a valid desktop entry,
and its `Exec` target is usable. A missing or stale target is a partial state,
not an installed state.

Query the defaults with:

```bash
xdg-mime query default text/markdown
xdg-mime query default text/x-markdown
```

Report `Default` only when `mdo.desktop` is the effective default for every
Markdown MIME type that resolves on the host. A split result is a partial
state and should offer repair. If `xdg-mime` is absent or fails, preserve the
known installed state but report that the default is unknown; do not silently
classify it as non-default. This matches the freedesktop lookup model, where
defaults can come from multiple desktop-specific and XDG configuration/data
locations rather than only the file currently edited by mdo. See the
[MIME Applications specification](https://specifications.freedesktop.org/mime-apps/latest-single/).

Install a second file,
`$XDG_DATA_HOME/applications/mdo-setup.desktop`, with `NoDisplay=false`,
`Terminal=false`, no `MimeType`, and an absolute, desktop-entry-escaped `Exec`
path to `mdo-setup`. The existing `mdo.desktop` remains `NoDisplay=true` and is
the only entry associated with Markdown MIME types.

Do not let normal integration removal break the persistent launcher's icon.
Either give the launcher a separately owned icon or retain the shared icon
until both handler and launcher registrations have been removed.

The launcher installation vehicle is deliberately layered:

1. `install.sh` installs or refreshes the per-user launcher after copying the
   binaries. This is the primary Linux release path and fixes discoverability
   immediately.
2. Homebrew installs a launcher template as package data. `mdo-setup` installs
   the per-user launcher when first run, because a formula should not mutate a
   particular user's home directory during package installation.
3. `cargo install` has no post-install hook. Document
   `mdo-setup --install-launcher`, and have any successful direct invocation of
   `mdo-setup` repair/register its launcher as an idempotent fallback.

The fallback cannot solve first-launch discovery for Cargo/Homebrew users; the
documentation must say that Linux users initially run `mdo-setup` from a shell
unless their installer created the application-menu entry. It must not imply
that double-clicking a bare extensionless binary is reliable.

## Windows design

Registration is `OpenWith` only when the `mdo.md` ProgID, its open command,
and the `.md\OpenWithProgids` value all exist and point to a usable mdo handler.
Missing pieces or a command that targets a missing executable are partial or
stale and should offer repair.

Determine `Default` by querying the effective `.md` association through the
Windows association API, not merely by checking the mdo-owned HKCU keys.
Windows merges machine and per-user class registrations, and the user's
effective default is distinct from an application's ProgID registration. The
read-only `UserChoice\ProgId` value may be a fallback observation, but mdo must
never write `UserChoice` or assume it can reproduce Windows' protected data.
The user must choose defaults through Windows UI; mdo should register itself as a
candidate and open the appropriate Default Apps/“Open with” UI rather than
writing `UserChoice`. This follows Microsoft's
[Default Programs guidance](https://learn.microsoft.com/en-us/windows/win32/shell/default-programs)
and [HKCR merge behavior](https://learn.microsoft.com/en-us/windows/win32/sysinfo/hkey-classes-root-key).

Create a per-user Start menu shortcut named “mdo Setup” that targets
`mdo-setup.exe`. Scoop should declare it with the manifest's `shortcuts` field.
Other portable install paths, including Cargo and the current WinGet portable
manifest, use an idempotent `mdo-setup --install-launcher` fallback; a successful
direct setup launch repairs the shortcut. If WinGet cannot express the shortcut
for the portable package, retain this fallback rather than changing installer
technology solely for the launcher.

The hosted Windows installer script should register the shortcut after copying
the binaries, just as the hosted Linux installer registers its desktop entry.

Uninstall removes only keys and values owned by mdo, the installed icon, and
the mdo Setup shortcut. It must not delete the `.md` mapping itself or overwrite
another application's default. Microsoft's file-type guidance likewise says
to remove owned ProgIDs while leaving file-type mappings alone when ownership
may have changed; see
[How to register a file type](https://learn.microsoft.com/en-us/windows/win32/shell/how-to-register-a-file-type-for-a-new-application).

## Binary removal guidance

Do not guess one definitive install method from the executable path. Prefer a
small install receipt written by packaging/setup when available. Without a
receipt, print labeled possibilities:

```text
cargo uninstall mdo-cli
brew uninstall mdo
winget uninstall Maphew.Mdo
scoop uninstall mdo
rm -f ~/.local/bin/mdo ~/.local/bin/mdo-open ~/.local/bin/mdo-setup
```

Only display commands relevant to the current OS and detected tools/path. The
setup process does not execute them.

Package-manager removal may bypass `mdo-setup` and leave per-user handler state
behind. Add uninstall hooks where the package format supports them; otherwise
show a prominent pre-uninstall cleanup command in package notes and tolerate
stale registrations if users remove binaries first.

## Implementation boundaries and tests

Keep status inspection and state transitions in `file_manager`; keep terminal,
dialog, and shortcut/desktop-launch behavior in `mdo-setup`. Launcher
registration should have explicit install, status, and remove functions so
integration removal cannot accidentally remove it.

Before shipping, cover:

- Linux missing, healthy, stale, split-default, and failed-`xdg-mime` states
  using isolated XDG directories and a fake command runner.
- Desktop entry quoting, `NoDisplay`, MIME ownership, idempotent repair, and
  removal that preserves unrelated `mimeapps.list` entries.
- Windows complete, partial, stale, and effective-default states using a
  registry/association-query abstraction; verify that no `UserChoice` value is
  written.
- Start menu shortcut install/repair/removal and paths containing spaces.
- Every UX state transition, including declining an action and complete cleanup.
- Packaging checks that `install.sh` creates the Linux launcher and Scoop
  declares the Windows shortcut.

## Consequences

The setup launcher becomes a durable control surface and Linux gains a real
application-menu entry. The handler and launcher lifecycles remain independent,
and default-app changes respect each platform's ownership rules.

The cost is a richer status result, platform query abstractions, and packaging
work in addition to the prompt change. Partial and unknown states must be
tested explicitly; collapsing them into the three happy-path labels would make
the UI simple at the expense of correctness.
