# ADR 0003: Keep metrics tooling in Python

- **Status**: Accepted
- **Date**: 2026-05-31
- **Deciders**: Matt
- **Supersedes**: none

## Context

`mdo` is a Rust CLI. The shipped user-facing binary and its core behavior
should stay Rust-first.

The repository also includes passive project metrics tooling in
`scripts/collect-metrics.py`, with focused tests in
`tests/test_collect_metrics.py`. This tooling collects package and GitHub
metadata and renders static docs artifacts. It is operational repository
tooling, not code shipped to users as part of the `mdo` command.

The question is whether to port this metrics tooling to Rust so the
repository remains Rust-only, or keep the small Python tool and avoid
rewriting working glue code for language uniformity alone.

## Decision

Keep the metrics collector in Python.

Treat Rust as the implementation language for the shipped CLI and any
user-facing helper binaries. Treat Python as acceptable for isolated
repository automation when it is small, tested, and does not affect the
installed `mdo` runtime.

Do not port the metrics collector to Rust solely to make the repository
monolingual.

## Consequences

**Positive**

- Avoids a rewrite that would not improve the CLI or user install story.
- Keeps API/file/HTML generation glue in a language well-suited to that
  job.
- Preserves the existing tests and CI coverage for metrics behavior.
- Keeps the Rust codebase focused on product behavior rather than
  repository reporting chores.

**Negative / accepted**

- Contributors need Python available when running the full CI-equivalent
  local checks.
- The repository is not strictly Rust-only.
- Metrics tooling has a separate dependency/runtime surface from the CLI.

## Reconsider when

- Python availability becomes a real contributor or CI maintenance problem.
- Metrics logic starts sharing domain behavior with the Rust CLI.
- The collector becomes a supported binary or user-facing feature.
- The script grows large enough that Rust's type system and crate ecosystem
  would materially reduce defects.
- More non-Rust tooling accumulates and creates fragmented maintenance
  patterns.

## Rejected options

**Port now for language uniformity.** This would make the repo feel cleaner
on paper, but it would spend engineering time without improving shipped
behavior. Language monoculture is not a goal by itself.

**Declare all repository tooling must be Rust.** This is too strict for a
small project. It would discourage pragmatic automation and force incidental
tooling into the product language even when another language is simpler.

**Move metrics into the `mdo` binary.** Metrics collection is project
reporting, not Markdown rendering. Folding it into the CLI would broaden
the product surface for no user benefit.
