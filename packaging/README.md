# Package Manager Manifests

This directory contains starter manifests for OS package managers that install
the `mdo` executable from GitHub Release assets.

## Release Used

The current manifests target `v0.4.0`:

| asset | sha256 |
|---|---|
| `mdo-universal-apple-darwin.tar.gz` | `11e029412635767ca2328410f02ea962238274392a61352012694eb7eae4e6f0` |
| `mdo-x86_64-unknown-linux-gnu.tar.gz` | `a8e5c8037056f94ed560e7a146579167d16fcb7efa54b05b6ab2911e297f7b6c` |
| `mdo-x86_64-pc-windows-msvc.zip` | `258477f2ee60622c6a871c306893ad23c8f554abb4c9abac7872c7eb64b920bf` |

Before submitting these manifests to a tap, the WinGet package repository, or a
Scoop bucket, publish the GitHub Release so the versioned URLs below are
publicly downloadable:

```text
https://github.com/maphew/mdo/releases/download/v0.4.0/<asset>
```

GitHub reported the `v0.4.0` release assets and hashes above after the release
workflow published the assets.

## Layout

- `homebrew/mdo.rb` - formula for a Homebrew tap
- `winget/Maphew.Mdo/0.4.0/` - WinGet package manifests
- `scoop/mdo.json` - Scoop bucket manifest

When cutting a new release, update the version, URLs, and hashes from that
release's `SHA256SUMS` asset.
