#!/usr/bin/env python3
"""Assess Amp orb sizing from machine evidence in an exported thread."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any


SIZES = {
    "a0.tiny": {"cpus": 1, "memory_kb": 2 * 1024 * 1024},
    "a0.small": {"cpus": 2, "memory_kb": 4 * 1024 * 1024},
    "a0.medium": {"cpus": 8, "memory_kb": 16 * 1024 * 1024},
    "a0.large": {"cpus": 16, "memory_kb": 32 * 1024 * 1024},
}
SIZE_NAMES = list(SIZES)

HARD_PRESSURE_PATTERNS = {
    "out-of-memory failure": re.compile(
        r"\b(?:out of memory|oom(?:killed| kill)?|cannot allocate memory)\b", re.I
    ),
    "process killed with exit 137": re.compile(
        r"(?:exit(?:ed| code| status)?\s*(?:with\s*)?137|status\s*137)", re.I
    ),
    "process killed by signal 9": re.compile(
        r"(?:signal\s*9|sigkill|command terminated by signal 9)", re.I
    ),
}
TIME_RSS = re.compile(r"Maximum resident set size \(kbytes\):\s*(\d+)", re.I)
TIME_CPU = re.compile(r"Percent of CPU this job got:\s*(\d+(?:\.\d+)?)%", re.I)
TIME_ELAPSED = re.compile(
    r"Elapsed \(wall clock\) time.*?:\s*((?:\d+:)?\d+:\d+(?:\.\d+)?)", re.I
)


@dataclass
class Measurement:
    max_rss_kb: int
    cpu_percent: float | None
    elapsed_seconds: float | None


@dataclass
class Assessment:
    verdict: str
    current_size: str
    recommended_size: str
    confidence: str
    evidence: list[str]
    missing_evidence: list[str]
    measurements: list[Measurement]


@dataclass
class ShellResult:
    command: str
    output: str
    exit_code: int | None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Assess whether an Amp orb was under- or over-sized."
    )
    parser.add_argument("source", help="Amp thread ID/URL or exported thread JSON")
    parser.add_argument("--size", required=True, choices=SIZE_NAMES)
    parser.add_argument("--json", action="store_true", dest="as_json")
    return parser.parse_args()


def load_thread(source: str) -> dict[str, Any]:
    path = Path(source)
    if path.is_file():
        with path.open(encoding="utf-8") as handle:
            return json.load(handle)

    try:
        completed = subprocess.run(
            ["amp", "threads", "export", source],
            check=True,
            capture_output=True,
            text=True,
        )
    except FileNotFoundError as error:
        raise RuntimeError("Amp CLI not found; pass an exported JSON file instead") from error
    except subprocess.CalledProcessError as error:
        detail = error.stderr.strip() or "unknown Amp CLI error"
        raise RuntimeError(f"could not export thread: {detail}") from error
    return json.loads(completed.stdout)


def shell_results(thread: dict[str, Any]) -> list[ShellResult]:
    shell_commands = {
        block.get("id"): block.get("input", {}).get("command", "")
        for message in thread.get("messages", [])
        for block in message.get("content", [])
        if block.get("type") == "tool_use" and block.get("name") == "shell_command"
    }
    results: list[ShellResult] = []
    for message in thread.get("messages", []):
        for block in message.get("content", []):
            if (
                block.get("type") != "tool_result"
                or block.get("toolUseID") not in shell_commands
            ):
                continue
            command = shell_commands[block.get("toolUseID")]
            if not isinstance(command, str):
                command = ""
            run = block.get("run", {})
            result = run.get("result")
            if isinstance(result, str):
                results.append(ShellResult(command=command, output=result, exit_code=None))
            elif isinstance(result, dict):
                output = result.get("output", "")
                exit_code = result.get("exitCode")
                results.append(
                    ShellResult(
                        command=command,
                        output=output if isinstance(output, str) else json.dumps(output),
                        exit_code=exit_code if isinstance(exit_code, int) else None,
                    )
                )
    return results


def parse_elapsed(value: str) -> float | None:
    try:
        parts = [float(part) for part in value.split(":")]
    except ValueError:
        return None
    if len(parts) == 2:
        return parts[0] * 60 + parts[1]
    if len(parts) == 3:
        return parts[0] * 3600 + parts[1] * 60 + parts[2]
    return None


def measurements_from(texts: list[str]) -> list[Measurement]:
    measurements: list[Measurement] = []
    for text in texts:
        rss_matches = list(TIME_RSS.finditer(text))
        for index, rss_match in enumerate(rss_matches):
            end = rss_matches[index + 1].start() if index + 1 < len(rss_matches) else len(text)
            # GNU time usually prints RSS after CPU and elapsed, so inspect the preceding report too.
            start = rss_matches[index - 1].end() if index else 0
            report = text[start:end]
            cpu_matches = list(TIME_CPU.finditer(report))
            elapsed_matches = list(TIME_ELAPSED.finditer(report))
            measurements.append(
                Measurement(
                    max_rss_kb=int(rss_match.group(1)),
                    cpu_percent=float(cpu_matches[-1].group(1)) if cpu_matches else None,
                    elapsed_seconds=(
                        parse_elapsed(elapsed_matches[-1].group(1))
                        if elapsed_matches
                        else None
                    ),
                )
            )
    return measurements


def assess(thread: dict[str, Any], size_name: str) -> Assessment:
    results = shell_results(thread)
    texts = [result.output for result in results]
    failed_texts = [
        result.output
        for result in results
        if result.exit_code is not None and result.exit_code != 0
    ]
    evidence: list[str] = []
    pressure: list[str] = []
    for label, pattern in HARD_PRESSURE_PATTERNS.items():
        if any(pattern.search(text) for text in failed_texts):
            pressure.append(label)
    if any(result.exit_code == 137 for result in results):
        pressure.append("shell command exited with status 137")

    measurement_texts = [
        result.output
        for result in results
        if re.search(r"(?:^|[;&|()\s])/usr/bin/time\s+-v\b", result.command)
    ]
    measurements = measurements_from(measurement_texts)
    current = SIZES[size_name]
    current_index = SIZE_NAMES.index(size_name)
    next_size = SIZE_NAMES[min(current_index + 1, len(SIZE_NAMES) - 1)]

    if pressure:
        evidence.extend(pressure)
        return Assessment(
            verdict="under-sized",
            current_size=size_name,
            recommended_size=next_size,
            confidence="high",
            evidence=evidence,
            missing_evidence=[],
            measurements=measurements,
        )

    if measurements:
        peak_rss = max(item.max_rss_kb for item in measurements)
        memory_ratio = peak_rss / current["memory_kb"]
        evidence.append(
            f"peak measured RSS was {peak_rss / 1024:.0f} MiB "
            f"({memory_ratio:.0%} of {size_name} memory)"
        )

        if memory_ratio >= 0.85:
            return Assessment(
                verdict="under-sized",
                current_size=size_name,
                recommended_size=next_size,
                confidence="medium",
                evidence=evidence,
                missing_evidence=["no hard OOM signal was found"],
                measurements=measurements,
            )

        cpu_samples = [m.cpu_percent for m in measurements if m.cpu_percent is not None]
        representative = any(
            m.elapsed_seconds is not None and m.elapsed_seconds >= 30 for m in measurements
        )
        if not representative or not cpu_samples:
            missing = []
            if not representative:
                missing.append("no measured workload ran for at least 30 seconds")
            if not cpu_samples:
                missing.append("no GNU time CPU measurement was found")
            return Assessment(
                verdict="insufficient-evidence",
                current_size=size_name,
                recommended_size=size_name,
                confidence="low",
                evidence=evidence,
                missing_evidence=missing,
                measurements=measurements,
            )

        if current_index > 0:
            smaller_name = SIZE_NAMES[current_index - 1]
            smaller = SIZES[smaller_name]
            fits_smaller_memory = peak_rss <= smaller["memory_kb"] * 0.60
            fits_smaller_cpu = max(cpu_samples) <= smaller["cpus"] * 60
            if fits_smaller_memory and fits_smaller_cpu:
                evidence.append(
                    f"measured workload retains at least 40% CPU and memory headroom on {smaller_name}"
                )
                return Assessment(
                    verdict="over-sized",
                    current_size=size_name,
                    recommended_size=smaller_name,
                    confidence="medium" if len(measurements) == 1 else "high",
                    evidence=evidence,
                    missing_evidence=(
                        ["only one representative measurement was found"]
                        if len(measurements) == 1
                        else []
                    ),
                    measurements=measurements,
                )

        return Assessment(
            verdict="right-sized",
            current_size=size_name,
            recommended_size=size_name,
            confidence="medium",
            evidence=evidence,
            missing_evidence=["repeat representative workloads to increase confidence"],
            measurements=measurements,
        )

    return Assessment(
        verdict="insufficient-evidence",
        current_size=size_name,
        recommended_size=size_name,
        confidence="low",
        evidence=[],
        missing_evidence=[
            "no hard memory-pressure failure was found in tool results",
            "no GNU time maximum-RSS measurement was found in tool results",
            "thread exports do not contain historical CPU or memory telemetry",
        ],
        measurements=[],
    )


def print_human(assessment: Assessment) -> None:
    print(f"Verdict: {assessment.verdict}")
    print(f"Confidence: {assessment.confidence}")
    print(f"Current size: {assessment.current_size}")
    print(f"Recommended size: {assessment.recommended_size}")
    if assessment.evidence:
        print("Evidence:")
        for item in assessment.evidence:
            print(f"  - {item}")
    if assessment.missing_evidence:
        print("Missing evidence:")
        for item in assessment.missing_evidence:
            print(f"  - {item}")


def main() -> int:
    args = parse_args()
    try:
        thread = load_thread(args.source)
        assessment = assess(thread, args.size)
    except (json.JSONDecodeError, OSError, RuntimeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2

    if args.as_json:
        print(json.dumps(asdict(assessment), indent=2))
    else:
        print_human(assessment)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
