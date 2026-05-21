# Changelog

## Unreleased

- Rename the project from `md2htmlx` to `mdo`.
- Rename the Rust binary from `md2htmlx` to `mdo`.
- Rename the Windows no-flash wrapper from `md2htmlx-open` to `mdo-open`.
- Publish the crates.io package as `mdo-cli`; install with `cargo install mdo-cli`.
- Update Windows Explorer integration to use `mdo` registry entries, `%LOCALAPPDATA%\mdo\md.ico`, and the `Render with mdo` verb.
- Update `--open` temp output paths from `%TEMP%\md2htmlx\<hash>\` to `%TEMP%\mdo\<hash>\`.
- If you previously installed the Windows Explorer integration, run the old version's `scripts/uninstall-explorer.ps1` before upgrading. The new uninstaller also removes the legacy registry entries when present.
