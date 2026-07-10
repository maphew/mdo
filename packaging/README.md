# Package Manager Manifests

This directory contains starter manifests for OS package managers that install
the `mdo` executable from GitHub Release assets.

## Release Used

The current manifests target `v0.5.0`:

| asset | sha256 |
|---|---|
| `mdo-universal-apple-darwin.tar.gz` | `3c9f4bd72e67b43abe9a4c597d9fedcd08174a52385e61f6cff2c9c0cfe25b1d` |
| `mdo-x86_64-unknown-linux-gnu.tar.gz` | `b9292b3a98dca7dac401f100c02100101f8c738fa457bf4f774b67a8e5a3c4e7` |
| `mdo-x86_64-pc-windows-msvc.zip` | `65f02f756cd7009cc574f13c8fe9bdcb5d7e1a61bd6a9aa32bccc038a9cee9f8` |

Before submitting these manifests to a tap, the WinGet package repository, or a
Scoop bucket, publish the GitHub Release so the versioned URLs below are
publicly downloadable:

```text
https://github.com/maphew/mdo/releases/download/v0.5.0/<asset>
```

GitHub reported the `v0.5.0` release assets and hashes above after the release
workflow published the assets.

## Layout

- `homebrew/mdo.rb` - formula for a Homebrew tap
- `winget/Maphew.Mdo/0.5.0/` - WinGet package manifests
- `scoop/mdo.json` - Scoop bucket manifest

When cutting a new release, update the version, URLs, and hashes from that
release's `SHA256SUMS` asset.
