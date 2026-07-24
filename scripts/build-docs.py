#!/usr/bin/env python3
"""Render the docs site with mdo.

Every Markdown page under docs/ is rendered with mdo's out-of-the-box
settings — the same output users get on their own machine. The only
exception is the homepage, docs/index.md, which keeps the
docs/assets/site.css override so it can demo the faux browser window
(see docs/faux-browser-window.md).

Cross-page links in the Markdown sources point at .md files so they work
when browsing the sources on GitHub; after rendering, relative .md hrefs
are rewritten to .html so they also resolve on the published site.

Set MDO_BIN to reuse an already-built mdo binary; otherwise this script uses
`cargo run`.
"""

from __future__ import annotations

import os
import re
import subprocess
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
DOCS_DIR = REPO_ROOT / "docs"

# Relative .md link targets only: a colon in the href (http:, mailto:, ...)
# means an absolute URL, which is left alone.
MD_HREF = re.compile(r'href="([^":]+)\.md(#[^"]*)?"')


def rewrite_md_links(html_file: Path) -> None:
    html = html_file.read_text(encoding="utf-8")
    rewritten = MD_HREF.sub(
        lambda m: f'href="{m.group(1)}.html{m.group(2) or ""}"', html
    )
    if rewritten != html:
        html_file.write_text(rewritten, encoding="utf-8")


def run_mdo(*args: str) -> None:
    mdo_bin = os.environ.get("MDO_BIN")
    command = [mdo_bin, *args] if mdo_bin else ["cargo", "run", "--quiet", "--", *args]
    subprocess.run(command, cwd=REPO_ROOT, check=True)


def main() -> int:
    for source in sorted(DOCS_DIR.rglob("*.md")):
        relative = source.relative_to(REPO_ROOT).as_posix()
        output = source.with_suffix(".html").relative_to(REPO_ROOT).as_posix()
        if relative == "docs/index.md":
            run_mdo("--css", "docs/assets/site.css", "--output", output, relative)
        else:
            run_mdo("--output", output, relative)
        rewrite_md_links(REPO_ROOT / output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
