# File-manager integration

mdo's core convenience: open a `.md` file from your file manager and read it
as rendered HTML immediately. Every integration launches the same
`mdo --open` render-and-open path, so no `.html` file is ever left beside the
source — output goes to a stable per-file temp path:

```text
Windows  %TEMP%\mdo\<hash>\<name>.html
Linux    /tmp/mdo-<uid>/<hash>/<name>.html
macOS    $TMPDIR/mdo-<uid>/<hash>/<name>.html
```

The file-handler registration is per-user and reversible with
`mdo --uninstall-file-manager`. (On Linux, the separate **mdo Setup**
application-menu launcher and the installed binaries are untouched by that
command; see the implementation details below.)

## Windows Explorer

`mdo.exe` installs or removes its own per-user Explorer integration (no
admin rights and no HKLM changes). Windows release ZIPs also include
`mdo-setup.exe`, a double-clickable guided setup:

```powershell
# Open the first-run setup (offers the optional Explorer integration install)
.\mdo-setup.exe

# CLI install: add an "Open as HTML" right-click verb and Open With app entry
.\mdo.exe --install-file-manager

# Undo everything the installer did
.\mdo.exe --uninstall-file-manager
```

Result after install:

- Right-click any `.md` file → **Open as HTML**. On Windows 11, this may
  appear under **Show more options**.
- **Open with → Open as HTML** appears as an available app for Markdown files.
- If you make **Open as HTML** the default handler, double-clicking a `.md`
  file opens the rendered page in your browser.

To make **Open as HTML** the *default* `.md` handler after installing,
right-click a `.md` file → **Open with → Choose another app** → pick
**Open as HTML** → tick **Always use this app**. Windows requires that last
step to be done interactively.

Rerunning the installer is safe: it detects its own existing registration and
keeps setup idempotent.

### Windows implementation details

- The installer registers **Open as HTML** for `.md` files using per-user
  (HKCU) registry entries only.
- If `mdo-open.exe` is present next to `mdo.exe`, it is used as the Explorer
  handler to avoid the brief black console-window flash that Windows shows
  for normal console binaries. If only `mdo.exe` is present, the installer
  registers `mdo.exe --open "%1"` directly, so a single downloaded executable
  is enough.
- The Windows binaries embed the mdo icon; the installer also writes that
  icon to a per-user path and registers both `mdo.exe` and `mdo-open.exe`
  with the friendly app name **Open as HTML**, so Windows "Open with"
  surfaces never expose the wrapper binary name.
- `mdo-setup.exe` — and `mdo-open.exe` launched directly with no file —
  opens the guided setup in a fresh Windows Terminal (`wt`) window using the
  **One Half Light** color scheme, centered on the active display, falling
  back to a plain new console when `wt` cannot be started.

## Linux file managers

`mdo` installs or removes its own per-user XDG integration; no companion
install script is required:

```bash
# Open the first-run setup in a terminal (offers the optional integration install)
mdo-setup

# Add "Open as HTML" as an "Open With" handler for Markdown files
mdo --install-file-manager

# Same, but also make it the default Markdown handler
mdo --install-file-manager --set-default

# Undo everything the installer did
mdo --uninstall-file-manager
```

Result after install:

- Most XDG file managers (GNOME Files/Nautilus, and others): right-click a
  `.md` file → **Open With** → **Open as HTML**.
- With `--set-default`: double-clicking a Markdown file launches `mdo --open`.
- The rendered page opens in your default browser from the temp path above.

### Linux implementation details

- The installer writes `~/.local/share/applications/mdo.desktop`, whose
  command is the current binary plus `--open %f`, and a small `Ⓜ` SVG icon
  under `~/.local/share/icons/hicolor/scalable/apps/mdo.svg`. The desktop
  entry is named **Open as HTML**, so file managers show an action-oriented
  entry instead of a tool-name-only entry.
- Rerunning the installer also removes older Nautilus Scripts entries named
  **Preview with mdo** or **Render with mdo**.
- The hosted Linux installer also writes the separate, visible
  `~/.local/share/applications/mdo-setup.desktop` launcher (**mdo Setup** in
  application menus). It has no MIME types and remains installed if you run
  `mdo --uninstall-file-manager`; that command only removes the hidden
  **Open as HTML** file-handler integration.
- `mdo-setup` opens the first-run setup in a terminal window: your
  `$TERMINAL`, or a known terminal emulator (`gnome-terminal`, `konsole`,
  `xterm`, and others). If none is found it shows a
  `zenity`/`kdialog`/`yad` notice pointing you to `mdo --setup`. Running
  `mdo-setup` repairs its per-user application-menu entry if needed — useful
  after a Cargo or Homebrew install.
- Launching `mdo-open` directly with no file opens `mdo-setup` when the
  setup helper is present.

## macOS Finder quick action

mdo does not ship a macOS installer yet, but Finder can run `mdo --open`
through a per-user Automator Quick Action. Apple documents Quick Action
workflows and shell-script actions in the Automator User Guide:

- [Quick Action workflows](https://support.apple.com/en-by/guide/automator/use-quick-action-workflows-aut73234890a/2.10/mac/15.0)
- [Run Shell Script](https://support.apple.com/guide/automator/use-scripts-aut4bb6b2b4f/mac)

Create a Quick Action in Automator, set it to receive files in Finder, add
**Run Shell Script**, set **Pass input** to **as arguments**, and use the
absolute path to `mdo`:

```bash
for file in "$@"; do
  /path/to/mdo --open "$file"
done
```

Save the workflow as **Open as HTML**. Finder then shows
**Quick Actions → Open as HTML** for selected Markdown files, and running it
opens the rendered page in your browser from the temp path above.
