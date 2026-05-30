#!/usr/bin/env python3
"""Collect passive public metrics for mdo.

The collector writes only aggregate or already-public information under
docs/metrics. It does not read local user data, runtime mdo output, server
logs, IP addresses, user agents, cookies, or unique identifiers.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import urllib.error
import urllib.request
from dataclasses import dataclass
from datetime import UTC, datetime
from html import escape
from pathlib import Path
from typing import Any


OWNER = "maphew"
REPO = "mdo"
CRATE = "mdo-cli"
USER_AGENT = "mdo-metrics/1.0 (+https://github.com/maphew/mdo)"
MIN_PUBLIC_COUNT = 3

REPO_ROOT = Path(__file__).resolve().parents[1]
DOCS_DIR = REPO_ROOT / "docs"
METRICS_DIR = DOCS_DIR / "metrics"
HISTORY_DIR = METRICS_DIR / "history"


@dataclass
class FetchResult:
    ok: bool
    status: int | None
    data: Any
    error: str | None = None


def request_json(url: str, token: str | None = None) -> FetchResult:
    headers = {
        "Accept": "application/vnd.github+json"
        if "api.github.com" in url
        else "application/json",
        "User-Agent": USER_AGENT,
    }
    if token:
        headers["Authorization"] = f"Bearer {token}"
        headers["X-GitHub-Api-Version"] = "2022-11-28"

    request = urllib.request.Request(url, headers=headers)
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            status = response.status
            body = response.read().decode("utf-8")
            return FetchResult(True, status, json.loads(body) if body else None)
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        return FetchResult(False, exc.code, None, summarize_http_error(exc.code, body))
    except (urllib.error.URLError, TimeoutError, json.JSONDecodeError) as exc:
        return FetchResult(False, None, None, str(exc))


def summarize_http_error(status: int, body: str) -> str:
    try:
        parsed = json.loads(body)
    except json.JSONDecodeError:
        return f"HTTP {status}"
    message = parsed.get("message")
    return f"HTTP {status}: {message}" if message else f"HTTP {status}"


def source_status(result: FetchResult) -> dict[str, Any]:
    status: dict[str, Any] = {"ok": result.ok, "status": result.status}
    if result.error:
        status["error"] = result.error
    return status


def compact_repo(data: dict[str, Any]) -> dict[str, Any]:
    return {
        "full_name": data.get("full_name"),
        "html_url": data.get("html_url"),
        "description": data.get("description"),
        "stars": data.get("stargazers_count"),
        "forks": data.get("forks_count"),
        "watchers": data.get("subscribers_count"),
        "open_issues": data.get("open_issues_count"),
        "created_at": data.get("created_at"),
        "updated_at": data.get("updated_at"),
        "pushed_at": data.get("pushed_at"),
    }


def collect_github_repo(token: str | None) -> dict[str, Any]:
    url = f"https://api.github.com/repos/{OWNER}/{REPO}"
    result = request_json(url, token)
    payload: dict[str, Any] = {
        "source": url,
        "status": source_status(result),
    }
    if result.ok and isinstance(result.data, dict):
        payload["repository"] = compact_repo(result.data)
    return payload


def compact_asset(asset: dict[str, Any]) -> dict[str, Any]:
    return {
        "name": asset.get("name"),
        "download_count": asset.get("download_count"),
        "size": asset.get("size"),
        "updated_at": asset.get("updated_at"),
        "browser_download_url": asset.get("browser_download_url"),
    }


def collect_github_releases(token: str | None) -> dict[str, Any]:
    url = f"https://api.github.com/repos/{OWNER}/{REPO}/releases"
    result = request_json(url, token)
    payload: dict[str, Any] = {
        "source": url,
        "status": source_status(result),
        "total_asset_downloads": None,
        "releases": [],
    }
    if not result.ok or not isinstance(result.data, list):
        return payload

    total = 0
    releases = []
    for release in result.data:
        assets = [compact_asset(asset) for asset in release.get("assets", [])]
        release_total = sum(asset.get("download_count") or 0 for asset in assets)
        total += release_total
        releases.append(
            {
                "tag_name": release.get("tag_name"),
                "name": release.get("name"),
                "html_url": release.get("html_url"),
                "published_at": release.get("published_at"),
                "asset_downloads": release_total,
                "assets": assets,
            }
        )

    payload["total_asset_downloads"] = total
    payload["releases"] = releases
    return payload


def collect_crates_io() -> dict[str, Any]:
    url = f"https://crates.io/api/v1/crates/{CRATE}"
    result = request_json(url)
    payload: dict[str, Any] = {
        "source": url,
        "status": source_status(result),
    }
    if result.ok and isinstance(result.data, dict):
        crate = result.data.get("crate", {})
        payload["crate"] = {
            "id": crate.get("id"),
            "name": crate.get("name"),
            "html_url": f"https://crates.io/crates/{CRATE}",
            "downloads": crate.get("downloads"),
            "recent_downloads": crate.get("recent_downloads"),
            "max_version": crate.get("max_version"),
            "created_at": crate.get("created_at"),
            "updated_at": crate.get("updated_at"),
        }
    return payload


def totals_only(series: dict[str, Any]) -> dict[str, Any]:
    return {
        "count": series.get("count"),
        "uniques": series.get("uniques"),
        "days": [
            {
                "timestamp": item.get("timestamp"),
                "count": item.get("count"),
                "uniques": item.get("uniques"),
            }
            for item in series.get("views", series.get("clones", []))
        ],
    }


def threshold_entries(entries: list[dict[str, Any]], keys: tuple[str, ...]) -> list[dict[str, Any]]:
    public_entries = []
    for entry in entries:
        if (entry.get("count") or 0) < MIN_PUBLIC_COUNT:
            continue
        public_entries.append({key: entry.get(key) for key in keys})
    return public_entries


def collect_github_traffic(token: str | None) -> dict[str, Any]:
    payload: dict[str, Any] = {
        "note": "GitHub traffic endpoints expose only recent aggregate windows. Entries with counts below the public threshold are omitted.",
        "minimum_public_count": MIN_PUBLIC_COUNT,
        "available": bool(token),
        "views": None,
        "clones": None,
        "referrers": [],
        "paths": [],
        "sources": {},
    }
    if not token:
        payload["skip_reason"] = "Set METRICS_GITHUB_TOKEN or GITHUB_TOKEN to collect GitHub traffic aggregates."
        return payload

    endpoints = {
        "views": f"https://api.github.com/repos/{OWNER}/{REPO}/traffic/views",
        "clones": f"https://api.github.com/repos/{OWNER}/{REPO}/traffic/clones",
        "referrers": f"https://api.github.com/repos/{OWNER}/{REPO}/traffic/popular/referrers",
        "paths": f"https://api.github.com/repos/{OWNER}/{REPO}/traffic/popular/paths",
    }
    for name, url in endpoints.items():
        result = request_json(url, token)
        payload["sources"][name] = {"source": url, "status": source_status(result)}
        if not result.ok:
            continue
        if name in {"views", "clones"} and isinstance(result.data, dict):
            payload[name] = totals_only(result.data)
        elif name == "referrers" and isinstance(result.data, list):
            payload[name] = threshold_entries(result.data, ("referrer", "count", "uniques"))
        elif name == "paths" and isinstance(result.data, list):
            payload[name] = threshold_entries(result.data, ("path", "title", "count", "uniques"))
    payload["available"] = any(
        source["status"]["ok"] for source in payload["sources"].values()
    )
    return payload


def check_url_presence(name: str, ecosystem: str, url: str) -> dict[str, Any]:
    result = request_json(url)
    return {
        "name": name,
        "ecosystem": ecosystem,
        "source": url,
        "present": result.ok,
        "status": source_status(result),
    }


def check_aur_presence(package: str) -> dict[str, Any]:
    url = f"https://aur.archlinux.org/rpc/v5/info/{package}"
    result = request_json(url)
    present = False
    if result.ok and isinstance(result.data, dict):
        present = (result.data.get("resultcount") or 0) > 0
    return {
        "name": package,
        "ecosystem": "AUR",
        "source": url,
        "present": present,
        "status": source_status(result),
    }


def collect_package_presence() -> list[dict[str, Any]]:
    checks = [
        check_url_presence("mdo-cli", "crates.io", f"https://crates.io/api/v1/crates/{CRATE}"),
        check_url_presence("mdo", "Homebrew", "https://formulae.brew.sh/api/formula/mdo.json"),
        check_url_presence("mdo", "Scoop main", "https://raw.githubusercontent.com/ScoopInstaller/Main/master/bucket/mdo.json"),
        check_url_presence("Maphew.Mdo", "WinGet", "https://api.github.com/repos/microsoft/winget-pkgs/contents/manifests/m/Maphew/Mdo"),
        check_aur_presence("mdo"),
        check_aur_presence("mdo-bin"),
    ]
    return checks


def collect_metrics(now: datetime, token: str | None) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "project": {
            "name": "mdo",
            "repository": f"https://github.com/{OWNER}/{REPO}",
            "crate": f"https://crates.io/crates/{CRATE}",
            "site": "https://maphew.github.io/mdo/",
        },
        "collected_at": now.isoformat(timespec="seconds").replace("+00:00", "Z"),
        "privacy": {
            "summary": "Only aggregate or already-public adoption signals are collected. The mdo binary does not phone home.",
            "excluded": [
                "runtime telemetry",
                "IP addresses",
                "user agents",
                "cookies",
                "unique installation IDs",
                "raw server logs",
            ],
        },
        "github": {
            "repository": collect_github_repo(token),
            "releases": collect_github_releases(token),
            "traffic": collect_github_traffic(token),
        },
        "crates_io": collect_crates_io(),
        "package_presence": collect_package_presence(),
    }


def metric_number(value: Any) -> str:
    if isinstance(value, int):
        return f"{value:,}"
    return "n/a"


def maybe_metric_card(label: str, value: Any, note: str) -> str:
    return (
        '<div class="metric-card">'
        f"<span>{escape(label)}</span>"
        f"<strong>{escape(metric_number(value))}</strong>"
        f"<small>{escape(note)}</small>"
        "</div>"
    )


def render_release_rows(releases: list[dict[str, Any]]) -> str:
    rows = []
    for release in releases:
        assets = release.get("assets", [])
        if not assets:
            rows.append(
                "<tr>"
                f"<td>{escape(release.get('tag_name') or '')}</td>"
                "<td>No assets</td><td>0</td>"
                "</tr>"
            )
            continue
        for asset in assets:
            rows.append(
                "<tr>"
                f"<td>{escape(release.get('tag_name') or '')}</td>"
                f"<td>{escape(asset.get('name') or '')}</td>"
                f"<td>{metric_number(asset.get('download_count'))}</td>"
                "</tr>"
            )
    return "\n".join(rows) or '<tr><td colspan="3">No release data available.</td></tr>'


def render_presence_rows(entries: list[dict[str, Any]]) -> str:
    rows = []
    for entry in entries:
        state = "present" if entry.get("present") else "not found"
        rows.append(
            "<tr>"
            f"<td>{escape(entry.get('ecosystem') or '')}</td>"
            f"<td>{escape(entry.get('name') or '')}</td>"
            f'<td><a href="{escape(entry.get("source") or "")}">{escape(state)}</a></td>'
            "</tr>"
        )
    return "\n".join(rows)


def render_traffic(metrics: dict[str, Any]) -> str:
    traffic = metrics["github"]["traffic"]
    if not traffic.get("available"):
        return (
            "<p>GitHub traffic aggregates were not available for this snapshot. "
            "The workflow can collect them when an authorized token is configured.</p>"
        )
    views = traffic.get("views") or {}
    clones = traffic.get("clones") or {}
    referrers = traffic.get("referrers") or []
    referrer_items = "".join(
        f"<li>{escape(item.get('referrer') or '')}: {metric_number(item.get('count'))}</li>"
        for item in referrers
    )
    return (
        '<div class="metric-grid">'
        + maybe_metric_card("Views", views.get("count"), "recent GitHub traffic window")
        + maybe_metric_card("Clones", clones.get("count"), "recent GitHub traffic window")
        + "</div>"
        + "<h3>Top Referrers</h3>"
        + (f"<ul>{referrer_items}</ul>" if referrer_items else "<p>No referrers above the public threshold.</p>")
    )


def render_metrics_html(metrics: dict[str, Any]) -> str:
    repo = metrics["github"]["repository"].get("repository", {})
    releases = metrics["github"]["releases"]
    crate = metrics["crates_io"].get("crate", {})
    collected_at = metrics["collected_at"]
    cards = [
        maybe_metric_card("GitHub stars", repo.get("stars"), "public repository signal"),
        maybe_metric_card("GitHub forks", repo.get("forks"), "public repository signal"),
        maybe_metric_card("Release downloads", releases.get("total_asset_downloads"), "GitHub release assets"),
        maybe_metric_card("crates.io downloads", crate.get("downloads"), "crate total"),
        maybe_metric_card("crates.io recent", crate.get("recent_downloads"), "crate recent window"),
    ]
    return f"""<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>mdo public metrics</title>
  <meta name="description" content="Public, privacy-preserving adoption metrics for mdo.">
  <link rel="stylesheet" href="../assets/site.css">
</head>
<body>
  <header class="site-header">
    <nav class="nav" aria-label="Primary navigation">
      <a class="brand" href="../index.html">
        <span class="brand-mark" aria-hidden="true">&#9410;</span>
        <span>mdo</span>
      </a>
      <div class="nav-links">
        <a href="../index.html">Home</a>
        <a href="latest.json">JSON</a>
        <a href="privacy.html">Privacy</a>
        <a href="https://github.com/{OWNER}/{REPO}">GitHub</a>
      </div>
    </nav>
  </header>
  <main>
    <section class="section">
      <div class="section-inner">
        <div class="section-heading">
          <h1>Public Metrics</h1>
          <p>Passive adoption signals for mdo. Everything here is aggregate or already public, and the CLI does not send telemetry.</p>
        </div>
        <p class="source-note">Last collected: <code>{escape(collected_at)}</code></p>
        <div class="metric-grid">
          {''.join(cards)}
        </div>
      </div>
    </section>
    <section class="section">
      <div class="section-inner">
        <div class="section-heading">
          <h2>Release Downloads</h2>
          <p>Counts come from public GitHub release asset metadata.</p>
        </div>
        <div class="table-wrap">
          <table class="metrics-table">
            <thead><tr><th>Release</th><th>Asset</th><th>Downloads</th></tr></thead>
            <tbody>{render_release_rows(releases.get("releases", []))}</tbody>
          </table>
        </div>
      </div>
    </section>
    <section class="section">
      <div class="section-inner">
        <div class="section-heading">
          <h2>GitHub Traffic</h2>
          <p>GitHub only exposes recent aggregate traffic windows to authorized repository maintainers.</p>
        </div>
        {render_traffic(metrics)}
      </div>
    </section>
    <section class="section">
      <div class="section-inner">
        <div class="section-heading">
          <h2>Package Presence</h2>
          <p>Presence checks show where mdo appears in public package ecosystems. They are spread signals, not usage counts.</p>
        </div>
        <div class="table-wrap">
          <table class="metrics-table">
            <thead><tr><th>Ecosystem</th><th>Name</th><th>Status</th></tr></thead>
            <tbody>{render_presence_rows(metrics["package_presence"])}</tbody>
          </table>
        </div>
      </div>
    </section>
  </main>
  <footer class="site-footer">
    <p>Metrics are generated from <a href="latest.json">public JSON</a>. See the <a href="privacy.html">metrics privacy note</a>.</p>
  </footer>
</body>
</html>
"""


def write_outputs(metrics: dict[str, Any]) -> None:
    METRICS_DIR.mkdir(parents=True, exist_ok=True)
    HISTORY_DIR.mkdir(parents=True, exist_ok=True)

    json_text = json.dumps(metrics, indent=2, sort_keys=True) + "\n"
    (METRICS_DIR / "latest.json").write_text(json_text, encoding="utf-8")
    date = metrics["collected_at"][:10]
    (HISTORY_DIR / f"{date}.json").write_text(json_text, encoding="utf-8")
    (METRICS_DIR / "index.html").write_text(render_metrics_html(metrics), encoding="utf-8")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output-only",
        metavar="JSON",
        help="render output files from an existing metrics JSON file without network access",
    )
    args = parser.parse_args(argv)

    if args.output_only:
        metrics = json.loads(Path(args.output_only).read_text(encoding="utf-8"))
    else:
        token = os.getenv("METRICS_GITHUB_TOKEN") or os.getenv("GITHUB_TOKEN")
        metrics = collect_metrics(datetime.now(UTC), token)
    write_outputs(metrics)
    print(f"Wrote {METRICS_DIR / 'latest.json'}")
    print(f"Wrote {METRICS_DIR / 'index.html'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
