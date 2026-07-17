# Contributing to mdo

Thanks for looking under the hood. Right now the most valuable contribution
is not code — it's **honest feedback from someone who isn't the author**.

## The contribution we want most: your experience

mdo was built to scratch one person's daily itch (reading many Markdown files
as rendered HTML). The open question is whether it's useful to anyone else.
You can answer that question better than any amount of code can.

- **It worked / it didn't work** — either way, tell us.
  [Open an experience report](https://github.com/maphew/mdo/issues/new?template=experience_report.yml)
  or start a [Discussion](https://github.com/maphew/mdo/discussions).
- **Harsh critique is explicitly welcome**, including "this tool shouldn't
  exist because X already does it better." If X really is better, we'd rather
  know than not.
- Small observations count: a confusing sentence in `--setup`, a right-click
  menu entry that didn't appear, a page that rendered ugly.

## Bug reports

[Open a bug report](https://github.com/maphew/mdo/issues/new?template=bug_report.yml).
The template asks for your OS, how you installed mdo, and the exact command or
file-manager action — those three answer most questions up front.

## Code contributions

PRs are welcome, but for anything beyond a small fix please open an issue or
discussion first so we don't waste your effort on something out of scope.
Current scope and non-goals live in the v0.6 planning notes; in short: mdo
wants to stay a small, fast, self-contained reader's tool. No GUI, no embedded
web server, no template marketplace.

### Build and test

```bash
git clone https://github.com/maphew/mdo.git
cd mdo
cargo build
cargo test --all-targets
```

Quality gates that CI enforces (run them before pushing):

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets --locked
```

Platform notes: the file-manager integration code is `cfg`-gated per platform,
so Windows-specific code only compiles on Windows and likewise for Linux.
CI covers Linux, macOS, and Windows.

### Issue tracking

Day-to-day planning happens in a [beads](https://github.com/gastownhall/beads)
database inside the repo (`.beads/`). You don't need to care about it —
GitHub Issues are the front door for outside contributors, and we'll wire
accepted work into the internal tracker ourselves.

## A note on how this project is built

Much of mdo's code is written with AI coding agents, directed and reviewed by
the maintainer, with cross-model reviews on substantial changes (details in PR
descriptions). If that affects your willingness to use or contribute to the
project, that's a fair position — we mention it so you can make that call with
full information rather than discover it in the commit log.

## License

By contributing you agree that your contributions are dual-licensed under
MIT OR Apache-2.0, matching the project.
