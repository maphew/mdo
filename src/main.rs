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
//! - `-b, --bare`           Emit only the HTML fragment (no `<html>`, `<head>`, `<body>`, no CSS)
//! - `--unsafe-html`        Preserve raw HTML from the Markdown source
//!
//! Without `--watch`, the tool converts once and exits.
//!
//! ## Credits
//!
//! Forked with gratitude from Hafiz Ali Raza's original Markdown-to-HTML CLI.
//! Bundles [simple.css](https://simplecss.org/) (© 2020 Kev Quirk, MIT).

use std::collections::hash_map::DefaultHasher;
use std::fs::{self, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use clap::Parser;
use notify::{recommended_watcher, EventKind, RecursiveMode, Watcher};
use pulldown_cmark::{html, Options, Parser as MdParser};

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};

/// Embedded simple.css (https://simplecss.org/), vendored from
/// https://unpkg.com/simpledotcss/simple.min.css
const SIMPLE_CSS: &str = include_str!("../assets/simple.min.css");
const APP_DISPLAY_NAME: &str = "mdo";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_HOMEPAGE: &str = env!("CARGO_PKG_HOMEPAGE");
const UNSAFE_TEMP_OUTPUT_STEM_CHARS: &[char] = &[
    '&', '^', '%', '(', ')', '!', '"', '\'', '<', '>', '|', ';', '`', '$', '\\', '/', ':',
];

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

    /// Emit only the HTML fragment (no <html>, <head>, <body>, no CSS)
    #[arg(short, long)]
    bare: bool,

    /// Preserve raw HTML from the Markdown source instead of sanitizing it
    #[arg(long)]
    unsafe_html: bool,

    /// Render to a temp directory and launch the system default browser.
    /// The source folder is left untouched unless --output is given.
    #[arg(long)]
    open: bool,
}

fn derive_output(input: &Path) -> PathBuf {
    input.with_extension("html")
}

/// Stable per-source-path location under a private temp/cache dir, e.g.
/// `%TEMP%\mdo-<uid>\<hash>\<stem>.html`. Re-opening the same source
/// overwrites the same file rather than accumulating new ones.
fn temp_output_for(input: &Path) -> io::Result<PathBuf> {
    let canonical = fs::canonicalize(input).unwrap_or_else(|_| input.to_path_buf());
    let mut hasher = DefaultHasher::new();
    canonical.hash(&mut hasher);
    let hash = hasher.finish();

    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");
    let stem = sanitize_temp_output_stem(stem);

    let root = private_temp_root();
    ensure_private_dir(&root)?;

    let source_dir = root.join(format!("{:016x}", hash));
    ensure_private_dir(&source_dir)?;

    Ok(source_dir.join(format!("{stem}.html")))
}

fn sanitize_temp_output_stem(stem: &str) -> String {
    let mut sanitized = String::with_capacity(stem.len());

    for ch in stem.chars() {
        if ch.is_control() || UNSAFE_TEMP_OUTPUT_STEM_CHARS.contains(&ch) {
            sanitized.push('_');
        } else {
            sanitized.push(ch);
        }
    }

    if sanitized.chars().any(|ch| ch != '_') {
        sanitized
    } else {
        "document".to_string()
    }
}

fn private_temp_root() -> PathBuf {
    let mut p = std::env::temp_dir();
    #[cfg(unix)]
    p.push(format!("mdo-{}", unsafe { libc::geteuid() }));
    #[cfg(not(unix))]
    p.push("mdo");
    p
}

#[cfg(unix)]
fn ensure_private_dir(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            let file_type = metadata.file_type();
            if file_type.is_symlink() || !file_type.is_dir() {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("{path:?} exists but is not a directory"),
                ));
            }

            let uid = unsafe { libc::geteuid() };
            if metadata.uid() != uid {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("{path:?} is not owned by the current user"),
                ));
            }

            let mode = metadata.permissions().mode();
            if mode & 0o077 != 0 {
                fs::set_permissions(path, fs::Permissions::from_mode(mode & !0o077))?;
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let mut builder = fs::DirBuilder::new();
            builder.mode(0o700);
            builder.create(path)?;
        }
        Err(e) => return Err(e),
    }

    Ok(())
}

#[cfg(not(unix))]
fn ensure_private_dir(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)
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
        // SECURITY: open via the Win32 ShellExecuteW API (through the `opener`
        // crate) rather than `cmd /C start`. `start` runs the path through
        // cmd.exe's command-line parser, so cmd metacharacters (`&`, `^`, …) —
        // which are legal in Windows filenames and reach us via the input
        // file's stem in temp_output_for — would be interpreted as commands
        // (e.g. opening `a&calc&.md` would launch calc.exe). ShellExecuteW
        // takes the path as a single typed argument, so those characters stay
        // inert. It is non-blocking, preserving the fire-and-forget behavior.
        opener::open(path).map_err(io::Error::other)?;
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

fn render_markdown(markdown: &str, unsafe_html: bool) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = MdParser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    if unsafe_html {
        return html_output;
    }

    sanitize_html(&html_output)
}

fn sanitize_html(html: &str) -> String {
    ammonia::Builder::default()
        .add_generic_attributes(&["class", "id"])
        .add_tags(&["input"])
        .add_tag_attributes("input", &["checked", "type"])
        .add_tag_attribute_values("input", "checked", &[""])
        .add_tag_attribute_values("input", "type", &["checkbox"])
        .set_tag_attribute_value("input", "disabled", "")
        .clean(html)
        .to_string()
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

fn wrap_html5(
    body: &str,
    title: &str,
    base_href: Option<&str>,
    render_duration: Duration,
    generated_date: &str,
) -> String {
    // `<base href>` makes relative image/link refs in the rendered HTML resolve
    // against the *source* directory even when the HTML lives elsewhere
    // (e.g. when --open writes to %TEMP%). Must come before any element that
    // references a URL, so we put it first inside <head>.
    let base_tag = match base_href {
        Some(href) => format!("<base href=\"{}\">\n", html_escape(href)),
        None => String::new(),
    };
    let generator = format!("{APP_DISPLAY_NAME} {APP_VERSION}");
    let rendered_in = format_duration(render_duration);
    format!(
        "<!DOCTYPE html>\n\
         <html lang=\"en\">\n\
         <head>\n\
         <meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <meta name=\"generator\" content=\"{generator}\">\n\
         {base_tag}\
         <title>{title}</title>\n\
         <style>\n{css}\n.mdo-generated{{margin-top:3rem;border:0;padding:1rem 0 1.5rem;font-size:.8rem;line-height:1.4;color:var(--text-light);text-align:center}}\n.mdo-generated a{{color:inherit}}\n</style>\n\
         {theme_toggle}\
         </head>\n\
         <body>\n\
         <main>\n{body}\n</main>\n\
         <footer class=\"mdo-generated\">Generated by <a href=\"{homepage}\">{app}</a> {version} in {rendered_in} on <time datetime=\"{generated_date}\">{generated_date}</time>.</footer>\n\
         </body>\n\
         </html>\n",
        generator = html_escape(&generator),
        base_tag = base_tag,
        title = html_escape(title),
        css = SIMPLE_CSS,
        theme_toggle = THEME_TOGGLE, // ← THEME TOGGLE injection point (delete this line to remove)
        body = body,
        homepage = html_escape(APP_HOMEPAGE),
        app = html_escape(APP_DISPLAY_NAME),
        version = html_escape(APP_VERSION),
        rendered_in = html_escape(&rendered_in),
        generated_date = html_escape(generated_date),
    )
}

fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs_f64();
    if seconds > 0.0 && seconds < 0.001 {
        "0.001s".to_string()
    } else {
        format!("{seconds:.3}s")
    }
}

fn utc_date_now() -> String {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    utc_date_from_unix_secs(elapsed.as_secs())
}

fn utc_date_from_unix_secs(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let (year, month, day) = civil_from_days(days);

    format!("{year:04}-{month:02}-{day:02}")
}

fn civil_from_days(days_since_unix_epoch: i64) -> (i64, i64, i64) {
    // Howard Hinnant's civil-from-days algorithm, shifted for Unix epoch days.
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };

    (year, month, day)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn convert(
    input: &Path,
    output: &Path,
    bare: bool,
    unsafe_html: bool,
    private_output: bool,
) -> bool {
    let markdown = match fs::read_to_string(input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ Failed to read {:?}: {}", input, e);
            return false;
        }
    };

    let render_started = Instant::now();
    let body = render_markdown(&markdown, unsafe_html);
    let render_duration = render_started.elapsed();
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

        wrap_html5(
            &body,
            &title,
            base_href.as_deref(),
            render_duration,
            &utc_date_now(),
        )
    };

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("❌ Failed to create {:?}: {}", parent, e);
                return false;
            }
        }
    }

    if let Err(e) = write_output_file(output, &final_html, private_output) {
        eprintln!("❌ Failed to write to {:?}: {}", output, e);
        false
    } else {
        println!("✅ Converted {:?} → {:?}", input, output);
        true
    }
}

#[cfg(unix)]
fn write_output_file(output: &Path, contents: &str, private_output: bool) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options
        .write(true)
        .create(true)
        .truncate(true)
        .custom_flags(libc::O_NOFOLLOW);

    if private_output {
        options.mode(0o600);
    }

    let mut file = options.open(output)?;
    file.write_all(contents.as_bytes())
}

#[cfg(not(unix))]
fn write_output_file(output: &Path, contents: &str, _private_output: bool) -> io::Result<()> {
    fs::write(output, contents)
}

fn main() -> notify::Result<()> {
    let args = Cli::parse();

    // Output precedence:
    //   1. explicit --output           (always wins)
    //   2. --open without --output     → temp dir (don't pollute the source folder)
    //   3. neither                     → next to the input
    let (output, private_output) = match (args.output.clone(), args.open) {
        (Some(p), _) => (p, false),
        (None, true) => match temp_output_for(&args.input) {
            Ok(path) => (path, true),
            Err(e) => {
                eprintln!("❌ Failed to prepare temp output directory: {}", e);
                return Ok(());
            }
        },
        (None, false) => (derive_output(&args.input), false),
    };

    let converted = convert(
        &args.input,
        &output,
        args.bare,
        args.unsafe_html,
        private_output,
    );

    if args.open && converted {
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
                    convert(
                        &args.input,
                        &output,
                        args.bare,
                        args.unsafe_html,
                        private_output,
                    );
                    last_render = Instant::now();
                }
            }
            Ok(Err(e)) => eprintln!("⚠️  Watcher error: {}", e),
            Err(_) => {} // timeout
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn temp_output_stem_replaces_shell_metacharacters() {
        let input = unique_temp_input("a&calc&.md");
        let output = temp_output_for(&input).expect("temp path should be built");
        let file_name = output
            .file_name()
            .and_then(|s| s.to_str())
            .expect("temp path should end with unicode filename");

        assert_eq!(file_name, "a_calc_.html");
        assert!(!file_name.contains('&'));

        cleanup_temp_fixture(&input, &output);
    }

    #[test]
    fn temp_output_stem_keeps_readable_normal_names() {
        let input = unique_temp_input("résumé-draft_2026.md");
        let output = temp_output_for(&input).expect("temp path should be built");
        let file_name = output
            .file_name()
            .and_then(|s| s.to_str())
            .expect("temp path should end with unicode filename");

        assert_eq!(file_name, "résumé-draft_2026.html");

        cleanup_temp_fixture(&input, &output);
    }

    fn unique_temp_input(file_name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let counter = NEXT_TEMP_FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "mdo-unit-test-{}-{nonce}-{counter}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("test temp dir should be created");
        let input = dir.join(file_name);
        fs::write(&input, "# Test\n").expect("test input should be written");
        input
    }

    fn cleanup_temp_fixture(input: &Path, output: &Path) {
        if let Some(parent) = output.parent() {
            let _ = fs::remove_dir_all(parent);
        }
        if let Some(parent) = input.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn formats_unix_timestamps_as_utc_dates() {
        assert_eq!(utc_date_from_unix_secs(0), "1970-01-01");
        assert_eq!(utc_date_from_unix_secs(951_827_696), "2000-02-29");
    }

    #[test]
    fn formats_render_duration_with_readable_floor() {
        assert_eq!(format_duration(Duration::ZERO), "0.000s");
        assert_eq!(format_duration(Duration::from_nanos(1)), "0.001s");
        assert_eq!(format_duration(Duration::from_millis(340)), "0.340s");
    }
}
