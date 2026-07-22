# Maintaining mdo

Maintainer-facing procedures: releases, packaging, the docs site, and public
metrics. Users never need this page.

## Releases

GitHub releases are published from this repository by
`.github/workflows/release.yml`. Push a version tag such as `v0.5.0` to build
Linux, macOS, and Windows archives and publish them to a GitHub Release. The
workflow can also be run manually with an existing tag via
**Actions → Release → Run workflow**.

The release workflow keeps repository access read-only for build jobs and
grants `contents: write` only to the final release-publishing job. GitHub
Actions are pinned to commit SHAs, with Dependabot configured to propose
updates.

Release checklist:

1. Update `CHANGELOG.md`: move **Unreleased** items under the new version
   heading with the release date.
2. Bump `version` in `Cargo.toml` (and let `Cargo.lock` update by building,
   e.g. `cargo check`).
3. Set Android's literal `versionName` and monotonically increasing
   `versionCode` in `android/app/build.gradle`. Confirm both Cargo and Android
   versions match the `vX.Y.Z` tag you're about to create.
4. Run the quality gates (`cargo test`, `cargo clippy`, and
   `cd android && ./gradlew assembleDebug assembleRelease bundleRelease
   lintDebug lintRelease`).
5. Verify the crate payload: `cargo publish --locked --dry-run`.
6. Commit the release changes (`CHANGELOG.md`, `Cargo.toml`, `Cargo.lock`,
   `android/app/build.gradle`, and the Android changelog entry)
   and push the commit, confirming `git status` is clean before tagging — a
   tag records whatever commit `HEAD` points to, so uncommitted or unpushed
   edits are silently left out of the release.
7. Tag and push: `git tag vX.Y.Z && git push origin vX.Y.Z`.
8. Publish the crate from that clean release commit: `cargo publish --locked`.
9. After the workflow publishes assets, refresh the package manifests
   (below) with the new version, URLs, and `SHA256SUMS` hashes.

The crates.io package is `mdo-cli` (the `mdo` crate name was taken); the
installed binary is `mdo`. Crates.io publishing is authenticated separately
and is not performed by the GitHub Release workflow.

### Android release signing

The release workflow can add a signed ARM64 APK and signed Android App Bundle
to each GitHub Release. It remains safely disabled until these repository
secrets exist:

- `ANDROID_KEYSTORE_BASE64`
- `ANDROID_KEYSTORE_PASSWORD`
- `ANDROID_KEY_ALIAS`
- `ANDROID_KEY_PASSWORD`

After the secrets are configured, set the repository variable
`ANDROID_RELEASE_SIGNING_ENABLED` to `true`. The workflow reconstructs the
keystore only in the runner's temporary directory, verifies both signatures,
and publishes `mdo-android-arm64.apk` and `mdo-android-arm64.aab` alongside
the desktop archives. Never commit the keystore or its passwords.

Use a long-lived key (at least 25 years), keep an offline backup, and use Play
App Signing with this key as the upload key. The APK published on GitHub must
keep using the same signing key for Android to accept in-place updates.

Detailed store preparation and the current human-owned steps are in
[`packaging/google-play/README.md`](https://github.com/maphew/mdo/blob/main/packaging/google-play/README.md) and
[`packaging/fdroid/README.md`](https://github.com/maphew/mdo/blob/main/packaging/fdroid/README.md).

## Package-manager manifests

`packaging/` contains starter manifests that install the `mdo` executable
from GitHub Release assets:

- `packaging/homebrew/mdo.rb` — formula for a Homebrew tap
- `packaging/winget/` — WinGet package manifests
- `packaging/scoop/mdo.json` — Scoop bucket manifest

See [`packaging/README.md`](https://github.com/maphew/mdo/blob/main/packaging/README.md)
for the currently targeted release and hashes. When cutting a new release,
update the version, URLs, and hashes from that release's `SHA256SUMS` asset
before submitting to a tap, bucket, or the WinGet repository.

## Docs site

The GitHub Pages site is the `docs/` directory, deployed by
`.github/workflows/pages.yml` on every push to `main`. The workflow builds
mdo, then renders every `docs/**/*.md` page to HTML with
`scripts/build-docs.py` using mdo's out-of-the-box settings — so the site
shows the same output users get on their own machines. The one exception is
the homepage `docs/index.md`, rendered with the `--css docs/assets/site.css`
override to demo the [faux browser window](faux-browser-window.html).

Regenerate the pages locally with:

```bash
python scripts/build-docs.py
```

Generated pages are not checked in — they are gitignored and built fresh by
the Pages workflow on every deploy. The only tracked HTML under `docs/` is
the hand-written metrics pages (`docs/metrics/*.html`).

`README.html` at the repository root is a plain `mdo README.md` render kept
as an example of default output; `scripts/build-docs.py` does not touch it.
Regenerate it separately when the README changes, e.g.
`cargo run --quiet -- README.md`.

## Architecture decision records

Design decisions live in `docs/adr/`:

- [ADR 0001 — Rename to mdo](adr/0001-rename-to-mdo.html)
- [ADR 0002 — Distribution strategy](adr/0002-distribution-strategy.html)
- [ADR 0003 — Keep Python metrics tooling](adr/0003-keep-python-metrics-tooling.html)
- [ADR 0004 — State-aware setup launcher](adr/0004-state-aware-setup-launcher.html)

## Public metrics

`scripts/collect-metrics.py` collects passive public metrics (GitHub release
download counts, stars, crates.io downloads) into `docs/metrics/`, published
at <https://maphew.github.io/mdo/metrics/>. It runs daily from
`.github/workflows/metrics.yml`.

The collector writes only aggregate or already-public information. It does
not read local user data, runtime mdo output, server logs, IP addresses, user
agents, cookies, or unique identifiers — mdo itself has no telemetry. See the
[metrics privacy note](https://maphew.github.io/mdo/metrics/privacy.html) and
[ADR 0003](adr/0003-keep-python-metrics-tooling.html) for why this tooling is
Python.
