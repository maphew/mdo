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
2. Bump `version` in `Cargo.toml` (and let `Cargo.lock` update).
3. Tag and push: `git tag vX.Y.Z && git push origin vX.Y.Z`.
4. After the workflow publishes assets, refresh the package manifests
   (below) with the new version, URLs, and `SHA256SUMS` hashes.

The crates.io package is `mdo-cli` (the `mdo` crate name was taken); the
installed binary is `mdo`.

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
mdo, regenerates the checked-in HTML pages with `scripts/build-docs.py`, and
renders each `docs/adr/*.md` to HTML with the `docs/assets/site.css`
override.

Regenerate the checked-in pages locally with:

```bash
python scripts/build-docs.py
```

`README.html` at the repository root is a plain `mdo README.md` render kept
as an example of default output; regenerate it when the README changes.

## Architecture decision records

Design decisions live in `docs/adr/`:

- [ADR 0001 — Rename to mdo](adr/0001-rename-to-mdo.md)
- [ADR 0002 — Distribution strategy](adr/0002-distribution-strategy.md)
- [ADR 0003 — Keep Python metrics tooling](adr/0003-keep-python-metrics-tooling.md)
- [ADR 0004 — State-aware setup launcher](adr/0004-state-aware-setup-launcher.md)

## Public metrics

`scripts/collect-metrics.py` collects passive public metrics (GitHub release
download counts, stars, crates.io downloads) into `docs/metrics/`, published
at <https://maphew.github.io/mdo/metrics/>. It runs daily from
`.github/workflows/metrics.yml`.

The collector writes only aggregate or already-public information. It does
not read local user data, runtime mdo output, server logs, IP addresses, user
agents, cookies, or unique identifiers — mdo itself has no telemetry. See the
[metrics privacy note](https://maphew.github.io/mdo/metrics/privacy.html) and
[ADR 0003](adr/0003-keep-python-metrics-tooling.md) for why this tooling is
Python.
