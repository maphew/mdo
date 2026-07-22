#!/usr/bin/env python3
"""Render the docs site with mdo.

Every Markdown page under docs/ is rendered with mdo's out-of-the-box
settings — the same output users get on their own machine. The only
exception is the homepage, docs/index.md, which keeps the
docs/assets/site.css override so it can demo the faux browser window
(see docs/faux-browser-window.md).

Set MDO_BIN to reuse an already-built mdo binary; otherwise this script uses
`cargo run`.
"""

from __future__ import annotations

import os
import subprocess
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
DOCS_DIR = REPO_ROOT / "docs"


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
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
