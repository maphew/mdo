# Changelog

## Unreleased

- Move shared rendering, temp-output, browser-launch, tour-sample, and file-manager helpers into `src/lib.rs`.
- Add `mdo-setup.exe`, a Windows GUI-subsystem onboarding/setup entrypoint that can install Explorer integration without opening a terminal.
- Teach `mdo-open.exe` to launch `mdo-setup.exe` when opened directly with no file arguments.

## 0.2.0 - 2026-05-29

- Rename the project from `md2htmlx` to `mdo`.
- Rename the Rust binary from `md2htmlx` to `mdo`.
- Rename the Windows no-flash wrapper from `md2htmlx-open` to `mdo-open`.
- Publish the crates.io package as `mdo-cli`; install with `cargo install mdo-cli`.
- Update Windows Explorer integration to use `mdo` registry entries, `%LOCALAPPDATA%\mdo\md.ico`, and the `Open as HTML` verb.
- Update Linux file-manager integration to use the `Open as HTML` label and remove duplicate Nautilus Scripts entries.
- Update `--open` temp output paths from `%TEMP%\md2htmlx\<hash>\` to `%TEMP%\mdo\<hash>\`.
- If you previously installed the Windows Explorer integration, run the old version's `scripts/uninstall-explorer.ps1` before upgrading. The new uninstaller also removes the legacy registry entries when present.
- Add explicit MIT and Apache-2.0 license files.
- Add release-preflight CI for formatting, linting, tests, and crate packaging.
- Add tag-driven GitHub Release automation for native binary archives and checksums.
- Add a GitHub Pages site under `docs/` with install instructions, usage examples, and migration notes.
- Add GitHub Pages deployment automation for the static `docs/` site.
- Set `mdo` as the default binary for `cargo run`.
