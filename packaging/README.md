# Package Manager Manifests

This directory contains packaging and store-submission material for the
desktop CLI and Android app.

## Release Used

The current manifests target `v0.6.0`:

| asset | sha256 |
|---|---|
| `mdo-universal-apple-darwin.tar.gz` | `45bdb4788084bee02390a37aa2d28030a7fb73a1c679f024a1c15bf48a39fb61` |
| `mdo-x86_64-unknown-linux-gnu.tar.gz` | `563458a5680c1ba473fa29ddc6d935805a0eb9bcdecbdde57a16d00b25f939ca` |
| `mdo-x86_64-pc-windows-msvc.zip` | `f723b6a1967445a26cab7f3530b383cb9c6b508e0ff02aed1132d88bf665f08f` |

Before submitting these manifests to a tap, the WinGet package repository, or a
Scoop bucket, publish the GitHub Release so the versioned URLs below are
publicly downloadable:

```text
https://github.com/maphew/mdo/releases/download/v0.6.0/<asset>
```

GitHub reported the `v0.6.0` release assets and hashes above after the release
workflow published the assets.

## Layout

- `homebrew/mdo.rb` - formula for a Homebrew tap
- `winget/Maphew.Mdo/0.6.0/` - WinGet package manifests
- `scoop/mdo.json` - Scoop bucket manifest
- `fdroid/` - proposed F-Droid build metadata and submission notes
- `google-play/` - Play Console checklist and data-safety answers

When cutting a new release, update the version, URLs, and hashes from that
release's `SHA256SUMS` asset.
