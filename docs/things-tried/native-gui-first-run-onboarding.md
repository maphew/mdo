# Native GUI first-run onboarding

Status: tried, retired from the active product.
Date: 2026-06-17
Related bead: mdo-rp0

## Decision

Keep the supported first-run experience in the terminal:

- `mdo --tour` prints the non-interactive tour.
- Running `mdo` with no arguments in an interactive terminal shows the same tour, offers the reversible file-manager integration, then opens the welcome sample.
- File-manager integration remains available through explicit CLI commands: `mdo --install-file-manager` and `mdo --uninstall-file-manager`.

Do not ship a native `mdo-setup` GUI companion for now.

## What was tried

### Windows `mdo-setup.exe`

A Windows GUI-subsystem binary was added as a no-terminal first-run setup window. It used `MessageBoxW` prompts to:

1. introduce mdo,
2. offer to install Explorer integration,
3. report success, and
4. optionally open the welcome sample.

A later iteration collapsed this to one install/skip prompt and opened the welcome sample automatically.

### Linux `mdo-setup`

A Linux companion used desktop dialog helpers (`zenity`, `kdialog`, or `yad`) to offer the same setup path outside a terminal.

### `mdo-open` no-file handoff

The `mdo-open` wrapper was taught to launch `mdo-setup` / `mdo-setup.exe` when opened with no file arguments, so users who double-clicked the wrapper could find onboarding.

### Windows registry API writes

To avoid command-window flashes while a GUI-subsystem setup program installed Explorer integration, the Windows registry writes were changed from spawning `reg.exe` to using Win32 registry APIs directly.

## Why it was retired

The native-dialog path felt clunky compared with the terminal tour:

- Desktop message boxes made the flow feel heavier than the single terminal prompt.
- Success or informational OK-only dialogs interrupted the path.
- Windows command windows could flash when setup work spawned console helpers.
- The GUI path duplicated onboarding copy and behavior that already worked in the terminal.
- The extra binary increased release, package-manager, and documentation surface area.

The terminal tour is simpler and clearer. It also preserves normal CLI stdout/stderr behavior because `mdo.exe` remains a console-subsystem program.

## What remains useful

If native onboarding is revisited, keep these lessons:

- Prefer one decision point over a sequence of dialogs.
- Avoid OK-only confirmation dialogs unless reporting an error.
- GUI-subsystem launchers must not spawn console-subsystem helpers in a way that creates visible console windows.
- Any native path should share copy and behavior with the terminal tour rather than becoming a parallel onboarding flow.
- Registry/API work may still be useful if the installer itself becomes GUI-native again.
