//! Shared mdo rendering, temp-output, browser launch, and integration helpers.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
#[cfg(unix)]
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::fs::OpenOptions;
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};

use pulldown_cmark::{html, Options, Parser as MdParser};

pub mod file_manager;

const SIMPLE_CSS: &str = include_str!("../assets/simple.min.css");
const APP_DISPLAY_NAME: &str = "mdo";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_HOMEPAGE: &str = env!("CARGO_PKG_HOMEPAGE");
pub const TOUR_SAMPLE_FILE_NAME: &str = "welcome-to-open-as-html-with-mdo.md";
pub const TOUR_SAMPLE_MARKDOWN: &str = "\
# Welcome to the world of Open as HTML with mdo

If you are reading this in your browser, mdo rendered Markdown as HTML and
opened it successfully.

Next, try **Open as HTML** (double-click) on any `.md` file you want to read.
";
const UNSAFE_TEMP_OUTPUT_STEM_CHARS: &[char] = &[
    '&', '^', '%', '(', ')', '!', '"', '\'', '<', '>', '|', ';', '`', '$', '\\', '/', ':',
];

// mdo keeps simple.css as the base stylesheet but softens the heading/body
// scale for generated documents. Users who prefer the unmodified vendored
// simple.css typography can pass assets/restore-simple-css.css with --css.
const MDO_DEFAULT_TYPOGRAPHY: &str = r#"<style id="mdo-default-typography">
body{font-size:1rem}
h1{font-size:2.4rem}
h2{font-size:2rem}
h3{font-size:1.4rem}
</style>
"#;

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

pub fn derive_output(input: &Path) -> PathBuf {
    input.with_extension("html")
}

/// Stable per-source-path location under a private temp/cache dir, e.g.
/// `%TEMP%\mdo-<uid>\<hash>\<stem>.html`. Re-opening the same source
/// overwrites the same file rather than accumulating new ones.
pub fn temp_output_for(input: &Path) -> io::Result<PathBuf> {
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
            if let Err(create_error) = builder.create(path) {
                if create_error.kind() == io::ErrorKind::AlreadyExists {
                    return ensure_private_dir(path);
                }

                return Err(create_error);
            }
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
pub fn launch_browser(path: &Path) -> std::io::Result<()> {
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
    css_override: Option<&str>,
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
    let css_override_block = css_override
        .filter(|css| !css.trim().is_empty())
        .map(|css| {
            format!(
                "<style id=\"mdo-css-override\">\n/* Custom CSS from --css */\n{}\n</style>\n",
                escape_style_end_tags(css)
            )
        })
        .unwrap_or_default();
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
         {mdo_default_typography}\
         {css_override_block}\
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
        mdo_default_typography = MDO_DEFAULT_TYPOGRAPHY,
        css_override_block = css_override_block,
        body = body,
        homepage = html_escape(APP_HOMEPAGE),
        app = html_escape(APP_DISPLAY_NAME),
        version = html_escape(APP_VERSION),
        rendered_in = html_escape(&rendered_in),
        generated_date = html_escape(generated_date),
    )
}

fn escape_style_end_tags(css: &str) -> String {
    const STYLE_END_PREFIX: &[u8] = b"</style";

    let mut escaped = String::with_capacity(css.len());
    let bytes = css.as_bytes();
    let mut i = 0;

    while i < css.len() {
        if i + STYLE_END_PREFIX.len() <= css.len()
            && bytes[i..i + STYLE_END_PREFIX.len()].eq_ignore_ascii_case(STYLE_END_PREFIX)
        {
            escaped.push_str("<\\/style");
            i += STYLE_END_PREFIX.len();
        } else {
            let ch = css[i..]
                .chars()
                .next()
                .expect("index should always point at a char boundary");
            escaped.push(ch);
            i += ch.len_utf8();
        }
    }

    escaped
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

pub fn convert(
    input: &Path,
    output: &Path,
    bare: bool,
    unsafe_html: bool,
    private_output: bool,
) -> bool {
    convert_with_css_override(input, output, bare, unsafe_html, private_output, None)
}

pub fn convert_with_css_override(
    input: &Path,
    output: &Path,
    bare: bool,
    unsafe_html: bool,
    private_output: bool,
    css_override: Option<&Path>,
) -> bool {
    let markdown = match fs::read_to_string(input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ Failed to read {:?}: {}", input, e);
            return false;
        }
    };

    let css_override = if bare {
        None
    } else {
        match css_override {
            Some(path) => match fs::read_to_string(path) {
                Ok(css) => Some(css),
                Err(e) => {
                    eprintln!("❌ Failed to read CSS override {:?}: {}", path, e);
                    return false;
                }
            },
            None => None,
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
            css_override.as_deref(),
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

pub fn open_tour_sample() -> io::Result<()> {
    let source = tour_sample_input_path()?;
    fs::write(&source, TOUR_SAMPLE_MARKDOWN)?;

    let output = temp_output_for(&source)?;
    if !convert(&source, &output, false, false, true) {
        return Err(io::Error::other("sample render failed"));
    }

    launch_browser(&output)?;
    Ok(())
}

fn tour_sample_input_path() -> io::Result<PathBuf> {
    let root = private_temp_root();
    ensure_private_dir(&root)?;

    let dir = root.join("tour");
    ensure_private_dir(&dir)?;
    Ok(dir.join(TOUR_SAMPLE_FILE_NAME))
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

    #[test]
    fn css_override_escapes_style_end_tags() {
        let escaped = escape_style_end_tags("h1{} </STYLE><script>alert(1)</script>");

        assert!(escaped.contains("<\\/style><script>"));
        assert!(!escaped.to_ascii_lowercase().contains("</style><script>"));
    }

    #[test]
    fn tour_sample_contains_welcome_copy() {
        assert!(TOUR_SAMPLE_MARKDOWN.contains("# Welcome to the world of Open as HTML with mdo"));
        assert!(TOUR_SAMPLE_MARKDOWN.contains("opened it successfully"));
        assert_eq!(TOUR_SAMPLE_FILE_NAME, "welcome-to-open-as-html-with-mdo.md");
    }
}
