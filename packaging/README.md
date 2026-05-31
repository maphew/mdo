# Package Manager Manifests

This directory contains starter manifests for OS package managers that install
the `mdo` executable from GitHub Release assets.

## Release Used

The current manifests target `v0.3.0`:

| asset | sha256 |
|---|---|
| `mdo-universal-apple-darwin.tar.gz` | `e3e8f4fd46b4bfed8b8816ebdcd6d462a56a8bb17dac6d7de7ae7f4447de9ad9` |
| `mdo-x86_64-unknown-linux-gnu.tar.gz` | `69d855334208b1b3021de240dbe591e733dd4425d951ab7dbedfd46ac0d8902d` |
| `mdo-x86_64-pc-windows-msvc.zip` | `f835abe99cef0951d50df7732ab6a2ef32adf41d83b8211329b82e046b1f771b` |

Before submitting these manifests to a tap, the WinGet package repository, or a
Scoop bucket, publish the GitHub Release so the versioned URLs below are
publicly downloadable:

```text
https://github.com/maphew/mdo/releases/download/v0.3.0/<asset>
```

At creation time, GitHub reported the `v0.3.0` release assets and hashes above,
but the release itself was still draft-only.

## Layout

- `homebrew/mdo.rb` - formula for a Homebrew tap
- `winget/Maphew.Mdo/0.3.0/` - WinGet package manifests
- `scoop/mdo.json` - Scoop bucket manifest

When cutting a new release, update the version, URLs, and hashes from that
release's `SHA256SUMS` asset.
