//! # mdo
//!
//! `mdo` is a small command-line tool that converts Markdown (`.md`) files to HTML.
//!
//! By default it produces a complete, HTML5-compliant document styled with
//! [simple.css](https://simplecss.org/) (vendored at build time, no network access at runtime).
//!
//! ## Usage
//!
//! ```sh
//! mdo [OPTIONS] <INPUT>
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
//! Forked with gratitude from Hafiz Ali Raza's original Markdown-to-HTML CLI.
//! Bundles [simple.css](https://simplecss.org/) (© 2020 Kev Quirk, MIT).

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
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
#[command(name = "mdo", author, version, about)]
struct Cli {
    /// Input Markdown file
    input: PathBuf,

    /// Output HTML file (defaults to <input>.html alongside the input,
    /// or to a temp directory when --open is used). Existing files are overwritten.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Watch the input file and re-render on every change
    #[arg(short, long)]
    watch: bool,

    /// Emit only the raw HTML fragment (no <html>, <head>, <body>, no CSS)
    #[arg(short, long)]
    bare: bool,

    /// Render to a temp directory and launch the system default browser.
    /// The source folder is left untouched unless --output is given.
    #[arg(long)]
    open: bool,
}

fn derive_output(input: &Path) -> PathBuf {
    input.with_extension("html")
}

/// Stable per-source-path location under the OS temp dir, e.g.
/// `%TEMP%\mdo\<hash>\<stem>.html`. Re-opening the same source
/// overwrites the same file rather than accumulating new ones.
fn temp_output_for(input: &Path) -> PathBuf {
    let canonical = fs::canonicalize(input).unwrap_or_else(|_| input.to_path_buf());
    let mut hasher = DefaultHasher::new();
    canonical.hash(&mut hasher);
    let hash = hasher.finish();

    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");

    let mut p = std::env::temp_dir();
    p.push("mdo");
    p.push(format!("{:016x}", hash));
    p.push(format!("{stem}.html"));
    p
}

/// Build a `file://` URL for a directory, suitable for use as `<base href>`.
/// Adds a trailing slash so relative refs resolve correctly. Performs minimal
/// URL-encoding (just spaces) which is enough for typical filesystem paths.
fn dir_to_file_url(dir: &Path) -> String {
    let s = dir.to_string_lossy().replace('\\', "/");
    // Strip Windows extended-length prefix (`\\?\C:\…` → `C:/…`) that
    // `fs::canonicalize` produces; browsers don't understand `file:////?/...`.
    let s = s.strip_prefix("//?/").unwrap_or(&s);
    let s = s.trim_end_matches('/').replace(' ', "%20");
    if s.starts_with('/') {
        // Unix absolute path: /home/x → file:///home/x/
        format!("file://{s}/")
    } else {
        // Windows drive path: A:/dev/x → file:///A:/dev/x/
        format!("file:///{s}/")
    }
}

/// Launch the platform's default handler for `path` (typically a web browser
/// for `.html`). Non-blocking — the spawned process runs independently.
fn launch_browser(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        // The empty "" is the title arg that `start` requires when the
        // first quoted arg would otherwise be interpreted as the title.
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn()?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open").arg(path).spawn()?;
    }
    Ok(())
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

fn wrap_html5(body: &str, title: &str, base_href: Option<&str>) -> String {
    // `<base href>` makes relative image/link refs in the rendered HTML resolve
    // against the *source* directory even when the HTML lives elsewhere
    // (e.g. when --open writes to %TEMP%). Must come before any element that
    // references a URL, so we put it first inside <head>.
    let base_tag = match base_href {
        Some(href) => format!("<base href=\"{}\">\n", html_escape(href)),
        None => String::new(),
    };
    format!(
        "<!DOCTYPE html>\n\
         <html lang=\"en\">\n\
         <head>\n\
         <meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         {base_tag}\
         <title>{title}</title>\n\
         <style>\n{css}\n</style>\n\
         {theme_toggle}\
         </head>\n\
         <body>\n\
         <main>\n{body}\n</main>\n\
         </body>\n\
         </html>\n",
        base_tag = base_tag,
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

fn convert(input: &Path, output: &Path, bare: bool) {
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

        // Only inject <base href> when the output lives in a different
        // directory from the source — otherwise relative refs already
        // resolve correctly and a base tag would just add noise.
        let base_href = match (
            fs::canonicalize(input)
                .ok()
                .and_then(|p| p.parent().map(Path::to_path_buf)),
            output.parent().and_then(|p| fs::canonicalize(p).ok()),
        ) {
            (Some(in_dir), Some(out_dir)) if in_dir != out_dir => Some(dir_to_file_url(&in_dir)),
            _ => None,
        };

        wrap_html5(&body, &title, base_href.as_deref())
    };

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("❌ Failed to create {:?}: {}", parent, e);
                return;
            }
        }
    }

    if let Err(e) = fs::write(output, final_html) {
        eprintln!("❌ Failed to write to {:?}: {}", output, e);
    } else {
        println!("✅ Converted {:?} → {:?}", input, output);
    }
}

fn main() -> notify::Result<()> {
    let args = Cli::parse();

    // Output precedence:
    //   1. explicit --output           (always wins)
    //   2. --open without --output     → temp dir (don't pollute the source folder)
    //   3. neither                     → next to the input
    let output = match (args.output.clone(), args.open) {
        (Some(p), _) => p,
        (None, true) => temp_output_for(&args.input),
        (None, false) => derive_output(&args.input),
    };

    convert(&args.input, &output, args.bare);

    if args.open {
        match launch_browser(&output) {
            Ok(()) => println!("🌐 Opened {:?} in default browser", output),
            Err(e) => eprintln!("⚠️  Failed to launch browser: {}", e),
        }
    }

    if !args.watch {
        return Ok(());
    }

    let (tx, rx) = channel();
    let mut watcher = recommended_watcher(tx)?;
    watcher.watch(&args.input, RecursiveMode::NonRecursive)?;

    println!(
        "👀 Watching {:?} for changes... (Ctrl+C to stop)",
        args.input
    );

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
