//! # md2htmlx
//!
//! `md2htmlx` is a small command-line tool that converts Markdown (`.md`) files to HTML.
//!
//! By default it produces a complete, HTML5-compliant document styled with
//! [simple.css](https://simplecss.org/) (vendored at build time, no network access at runtime).
//!
//! ## Usage
//!
//! ```sh
//! md2htmlx [OPTIONS] <INPUT>
//! ```
//!
//! If no output path is given, the output is written next to the input with
//! the extension changed to `.html` (e.g. `foo.md` → `foo.html`). Existing
//! files are overwritten.
//!
//! Options:
//! - `-o, --output <FILE>`  Write to `<FILE>` instead of the derived name
//! - `-w, --watch`          Keep running and re-render on file changes
//! - `-b, --bare`           Emit only the raw HTML fragment (no `<html>`, `<head>`, `<body>`, no CSS)
//!
//! Without `--watch`, the tool converts once and exits.
//!
//! ## Credits
//!
//! Forked with gratitude from
//! [rust-md2html](https://github.com/haffizaliraza/rust-md2html) by Hafiz Ali Raza.
//! Bundles [simple.css](https://simplecss.org/) (© 2020 Kev Quirk, MIT).

use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

use clap::Parser;
use notify::{recommended_watcher, EventKind, RecursiveMode, Watcher};
use pulldown_cmark::{html, Options, Parser as MdParser};

/// Embedded simple.css (https://simplecss.org/), vendored from
/// https://unpkg.com/simpledotcss/simple.min.css
const SIMPLE_CSS: &str = include_str!("../assets/simple.min.css");

// ─── BEGIN THEME TOGGLE ────────────────────────────────────────────────
// Self-contained light/dark mode toggle. To remove this feature entirely:
//   1. Delete this block (down to "END THEME TOGGLE").
//   2. Delete the `{theme_toggle}` line and `theme_toggle = ...` arg in
//      `wrap_html5` below.
// Variable values mirror simple.css's @media (prefers-color-scheme: dark).
const THEME_TOGGLE: &str = r#"<style>
:root[data-theme="light"]{color-scheme:light;--bg:#fff;--accent-bg:#f5f7ff;--text:#212121;--text-light:#585858;--accent:#0d47a1;--accent-hover:#1266e2;--accent-text:var(--bg);--code:#d81b60;--preformatted:#444;--disabled:#efefef}
:root[data-theme="dark"]{color-scheme:dark;--bg:#212121;--accent-bg:#2b2b2b;--text:#dcdcdc;--text-light:#ababab;--accent:#ffb300;--accent-hover:#ffe099;--accent-text:var(--bg);--code:#f06292;--preformatted:#ccc;--disabled:#111}
:root[data-theme="dark"] img,:root[data-theme="dark"] video{opacity:.8}
#theme-toggle{position:fixed;top:.75rem;right:.75rem;z-index:1000;padding:.25rem .6rem;font-size:1rem;line-height:1;cursor:pointer;border-radius:var(--standard-border-radius);border:var(--border-width) solid var(--border);background:var(--accent-bg);color:var(--text)}
</style>
<script>
(function(){
  var saved=localStorage.getItem('theme');
  var sys=matchMedia('(prefers-color-scheme: dark)').matches?'dark':'light';
  document.documentElement.dataset.theme=saved||sys;
  document.addEventListener('DOMContentLoaded',function(){
    var b=document.createElement('button');
    b.id='theme-toggle';b.type='button';b.title='Toggle light/dark';
    var sync=function(){b.textContent=document.documentElement.dataset.theme==='dark'?'\u2600':'\u263E';};
    sync();
    b.onclick=function(){
      var next=document.documentElement.dataset.theme==='dark'?'light':'dark';
      document.documentElement.dataset.theme=next;
      localStorage.setItem('theme',next);
      sync();
    };
    document.body.appendChild(b);
  });
})();
</script>
"#;
// ─── END THEME TOGGLE ──────────────────────────────────────────────────

/// Markdown to HTML converter. Converts once by default; pass --watch to keep watching.
#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// Input Markdown file
    input: PathBuf,

    /// Output HTML file (defaults to <input>.html alongside the input).
    /// Existing files are overwritten.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Watch the input file and re-render on every change
    #[arg(short, long)]
    watch: bool,

    /// Emit only the raw HTML fragment (no <html>, <head>, <body>, no CSS)
    #[arg(short, long)]
    bare: bool,
}

fn derive_output(input: &PathBuf) -> PathBuf {
    input.with_extension("html")
}

fn render_markdown(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = MdParser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

fn derive_title(markdown: &str, fallback: &str) -> String {
    for line in markdown.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("# ") {
            return rest.trim().to_string();
        }
    }
    fallback.to_string()
}

fn wrap_html5(body: &str, title: &str) -> String {
    format!(
        "<!DOCTYPE html>\n\
         <html lang=\"en\">\n\
         <head>\n\
         <meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{title}</title>\n\
         <style>\n{css}\n</style>\n\
         {theme_toggle}\
         </head>\n\
         <body>\n\
         <main>\n{body}\n</main>\n\
         </body>\n\
         </html>\n",
        title = html_escape(title),
        css = SIMPLE_CSS,
        theme_toggle = THEME_TOGGLE, // ← THEME TOGGLE injection point (delete this line to remove)
        body = body,
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn convert(input: &PathBuf, output: &PathBuf, bare: bool) {
    let markdown = match fs::read_to_string(input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ Failed to read {:?}: {}", input, e);
            return;
        }
    };

    let body = render_markdown(&markdown);
    let final_html = if bare {
        body
    } else {
        let fallback = input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Document");
        let title = derive_title(&markdown, fallback);
        wrap_html5(&body, &title)
    };

    if let Err(e) = fs::write(output, final_html) {
        eprintln!("❌ Failed to write to {:?}: {}", output, e);
    } else {
        println!("✅ Converted {:?} → {:?}", input, output);
    }
}

fn main() -> notify::Result<()> {
    let args = Cli::parse();
    let output = args.output.unwrap_or_else(|| derive_output(&args.input));

    convert(&args.input, &output, args.bare);

    if !args.watch {
        return Ok(());
    }

    let (tx, rx) = channel();
    let mut watcher = recommended_watcher(tx)?;
    watcher.watch(&args.input, RecursiveMode::NonRecursive)?;

    println!("👀 Watching {:?} for changes... (Ctrl+C to stop)", args.input);

    // Simple debounce: ignore events that fire within DEBOUNCE_MS of the last render.
    const DEBOUNCE: Duration = Duration::from_millis(200);
    let mut last_render = Instant::now() - DEBOUNCE;

    loop {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(Ok(event)) => {
                if matches!(event.kind, EventKind::Modify(_)) {
                    if last_render.elapsed() < DEBOUNCE {
                        continue;
                    }
                    println!("🔁 File changed, re-rendering...");
                    convert(&args.input, &output, args.bare);
                    last_render = Instant::now();
                }
            }
            Ok(Err(e)) => eprintln!("⚠️  Watcher error: {}", e),
            Err(_) => {} // timeout
        }
    }
}
