import importlib.util
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
SCRIPT = REPO_ROOT / "scripts" / "collect-metrics.py"
SPEC = importlib.util.spec_from_file_location("collect_metrics", SCRIPT)
collect_metrics = importlib.util.module_from_spec(SPEC)
sys.modules["collect_metrics"] = collect_metrics
SPEC.loader.exec_module(collect_metrics)


class CollectMetricsTests(unittest.TestCase):
    def test_threshold_entries_omits_low_counts(self):
        entries = [
            {"referrer": "example.com", "count": 2, "uniques": 1},
            {"referrer": "github.com", "count": 3, "uniques": 2},
        ]

        public_entries = collect_metrics.threshold_entries(
            entries, ("referrer", "count", "uniques")
        )

        self.assertEqual(public_entries, [{"referrer": "github.com", "count": 3, "uniques": 2}])

    def test_rendered_html_links_to_json_and_privacy_note(self):
        metrics = {
            "collected_at": "2026-05-30T12:00:00Z",
            "github": {
                "repository": {
                    "repository": {
                        "stars": 4,
                        "forks": 1,
                    }
                },
                "releases": {
                    "total_asset_downloads": 9,
                    "releases": [
                        {
                            "tag_name": "v0.3.0",
                            "assets": [
                                {
                                    "name": "mdo-x86_64-unknown-linux-gnu.tar.gz",
                                    "download_count": 9,
                                }
                            ],
                        }
                    ],
                },
                "traffic": {"available": False},
            },
            "crates_io": {
                "crate": {
                    "downloads": 100,
                    "recent_downloads": 12,
                }
            },
            "package_presence": [
                {
                    "ecosystem": "crates.io",
                    "name": "mdo-cli",
                    "source": "https://crates.io/crates/mdo-cli",
                    "present": True,
                }
            ],
        }

        html = collect_metrics.render_metrics_html(metrics)

        self.assertIn("Public Metrics", html)
        self.assertIn("latest.json", html)
        self.assertIn("privacy.html", html)
        self.assertIn("mdo-x86_64-unknown-linux-gnu.tar.gz", html)

    def test_output_only_writes_latest_history_and_index(self):
        metrics = {
            "schema_version": 1,
            "project": {"name": "mdo"},
            "collected_at": "2026-05-30T12:00:00Z",
            "github": {
                "repository": {"repository": {}},
                "releases": {"total_asset_downloads": 0, "releases": []},
                "traffic": {"available": False},
            },
            "crates_io": {"crate": {}},
            "package_presence": [],
        }

        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            source = tmp_path / "metrics.json"
            source.write_text(json.dumps(metrics), encoding="utf-8")
            original_metrics_dir = collect_metrics.METRICS_DIR
            original_history_dir = collect_metrics.HISTORY_DIR
            collect_metrics.METRICS_DIR = tmp_path / "metrics"
            collect_metrics.HISTORY_DIR = collect_metrics.METRICS_DIR / "history"
            try:
                with redirect_stdout(StringIO()):
                    self.assertEqual(collect_metrics.main(["--output-only", str(source)]), 0)
                self.assertTrue((collect_metrics.METRICS_DIR / "latest.json").exists())
                self.assertTrue((collect_metrics.METRICS_DIR / "index.html").exists())
                self.assertTrue((collect_metrics.HISTORY_DIR / "2026-05-30.json").exists())
            finally:
                collect_metrics.METRICS_DIR = original_metrics_dir
                collect_metrics.HISTORY_DIR = original_history_dir


if __name__ == "__main__":
    unittest.main()
