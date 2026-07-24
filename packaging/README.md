# Package Manager Manifests

This directory contains packaging and store-submission material for the
desktop CLI and Android app.

## Release Used

The current manifests target `v0.6.1`:

| asset | sha256 |
|---|---|
| `mdo-android-arm64.aab` | `2414b20c45e65bd5efb8db9adbdb4d22fa9d55c655ee012e808e281981fe5725` |
| `mdo-android-arm64.apk` | `addd2bca8315c4248f20f344b316b7988fe543a5c97a003f24afc00d2685c31b` |
| `mdo-universal-apple-darwin.tar.gz` | `f36cd3cf7153520e9d9875bf4e3e4a6ca164d1ab4fd948b3f91b930bd0c49a65` |
| `mdo-x86_64-unknown-linux-gnu.tar.gz` | `8848a297e47812751b84986f47c1719ae7600c7d4de33417eb227528c421f227` |
| `mdo-x86_64-pc-windows-msvc.zip` | `4cca084920edf562ee1fdc8512d3c372be383d4a95f3d51dc2280d4e5e64abc7` |

Before submitting these manifests to a tap, the WinGet package repository, or a
Scoop bucket, publish the GitHub Release so the versioned URLs below are
publicly downloadable:

```text
https://github.com/maphew/mdo/releases/download/v0.6.1/<asset>
```

GitHub reported the `v0.6.1` release assets and hashes above after the release
workflow published the assets.

## Layout

- `homebrew/mdo.rb` - formula for a Homebrew tap
- `winget/Maphew.Mdo/0.6.1/` - WinGet package manifests
- `scoop/mdo.json` - Scoop bucket manifest
- `fdroid/` - proposed F-Droid build metadata and submission notes
- `google-play/` - Play Console checklist and data-safety answers

When cutting a new release, update the version, URLs, and hashes from that
release's `SHA256SUMS` asset.
