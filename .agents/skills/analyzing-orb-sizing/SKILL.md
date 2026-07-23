---
name: analyzing-orb-sizing
description: Retrospectively assesses whether an Amp orb was under-sized, over-sized, or appropriately sized from thread tool output and measured workloads. Use when reviewing orb size, resource pressure, OOM failures, build performance, or orb cost efficiency.
compatibility: Requires Python 3.9+ and the Amp CLI when analyzing a thread ID.
argument-hint: <thread-id-or-export.json> --size <a0.tiny|a0.small|a0.medium|a0.large>
---

# Analyzing Orb Sizing

Assess orb sizing from evidence, not repository type alone. Run the bundled analyzer first, then use `read_thread` when semantic context is needed to decide whether observed workloads were representative.

## Run the analyzer

From the repository root:

```bash
python3 .agents/skills/analyzing-orb-sizing/scripts/analyze.py \
  T-xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx --size a0.small
```

It also accepts an existing export, which is useful offline:

```bash
amp threads export T-xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx > /tmp/thread.json
python3 .agents/skills/analyzing-orb-sizing/scripts/analyze.py \
  /tmp/thread.json --size a0.small --json
```

Always pass the size used by that thread. Project defaults are prospective and may not match an existing orb.

## Interpret the result

- **under-sized**: hard resource-pressure evidence such as OOM, exit 137, allocation failure, or measured usage that reaches the orb limit. Move up one size, then repeat the representative workload.
- **over-sized**: representative measured workloads fit comfortably on the next smaller size with CPU and memory headroom. Move down one size and verify there.
- **right-sized**: representative measurements use meaningful capacity without pressure and do not safely fit the next smaller size.
- **insufficient-evidence**: the transcript lacks trustworthy resource measurements or hard pressure signals. Do not interpret an uneventful thread as proof of over-sizing.

The analyzer only treats shell/tool results as machine evidence. User and assistant prose can describe the task but must not be counted as telemetry.

## Add evidence to a future thread

Run the slowest representative build or test under GNU `time` inside the orb:

```bash
/usr/bin/time -v cargo test
/usr/bin/time -v ./gradlew test
```

For parallel builds, also record the orb identity and capacity:

```bash
printf 'orb-capacity cpus=%s memory_kb=%s\n' "$(nproc)" "$(awk '/MemTotal/ {print $2}' /proc/meminfo)"
```

Use a clean and an incremental build when both matter. One trivial command, setup/install work, model thinking time, network waits, and idle thread duration are not representative sizing measurements.

## Apply judgment after the script

1. Use `read_thread` to identify what the measured command did and whether it represents normal project work.
2. Separate compute delay from dependency downloads, network services, lock contention, and test sleeps.
3. Prefer multiple representative samples. A hard OOM is decisive; an over-sizing recommendation needs measured headroom.
4. Mention confidence, evidence, and missing evidence in the final recommendation.
5. Disk is 40 GB for every documented size, so changing size does not solve disk pressure.

## Size reference

| Size | CPUs | Memory | Hourly price |
|---|---:|---:|---:|
| `a0.tiny` | 1 | 2 GB | $0.10 |
| `a0.small` | 2 | 4 GB | $0.21 |
| `a0.medium` | 8 | 16 GB | $0.83 |
| `a0.large` | 16 | 32 GB | $1.66 |

Treat prices as a dated reference and verify current Amp pricing before making a cost forecast.
