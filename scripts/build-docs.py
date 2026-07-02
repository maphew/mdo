#!/usr/bin/env python3
"""Regenerate checked-in docs pages with mdo's runtime CSS pipeline.

Set MDO_BIN to reuse an already-built mdo binary; otherwise this script uses
`cargo run`.
"""

from __future__ import annotations

import os
import subprocess
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]


def run_mdo(*args: str) -> None:
    mdo_bin = os.environ.get("MDO_BIN")
    command = [mdo_bin, *args] if mdo_bin else ["cargo", "run", "--quiet", "--", *args]
    subprocess.run(command, cwd=REPO_ROOT, check=True)


def main() -> int:
    run_mdo(
        "--css",
        "docs/assets/site.css",
        "--output",
        "docs/index.html",
        "docs/index.md",
    )
    run_mdo(
        "--css",
        "docs/assets/sample.css",
        "--output",
        "docs/assets/sample.html",
        "docs/assets/sample.md",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
