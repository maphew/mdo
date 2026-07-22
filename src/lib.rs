//! Shared mdo rendering, temp-output, browser launch, and integration helpers.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
#[cfg(unix)]
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, UNIX_EPOCH};

#[cfg(unix)]
use std::fs::OpenOptions;
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};

use pulldown_cmark::{html, Options, Parser as MdParser};
use url::Url;

#[cfg(target_os = "android")]
mod android;
pub mod file_manager;
#[cfg(target_os = "windows")]
pub mod windows_setup;

const SIMPLE_CSS: &str = include_str!("../assets/simple.min.css");
const APP_DISPLAY_NAME: &str = "mdo";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SETUP_SAMPLE_FILE_NAME: &str = "welcome-to-open-as-html-with-mdo.md";
pub const SETUP_SAMPLE_MARKDOWN: &str = "\
# Welcome to the world of Open as HTML with mdo

If you are reading this in your browser, mdo rendered Markdown as HTML and
opened it successfully.

Next, right-click any `.md` file and choose **Open as HTML** to read it this way.
If you make mdo your default Markdown app, a double-click does the same.
";
const UNSAFE_TEMP_OUTPUT_STEM_CHARS: &[char] = &[
    '&', '^', '%', '(', ')', '!', '"', '\'', '<', '>', '|', ';', '`', '$', '\\', '/', ':',
];

// mdo keeps simple.css as the base stylesheet but softens the heading/body
// scale for generated documents. Users who prefer the unmodified vendored
// simple.css typography can pass assets/restore-simple-css.css with --css.
const MDO_DEFAULT_TYPOGRAPHY_CSS: &str = include_str!("../assets/mdo-default-typography.css");

// pulldown-cmark emits GFM alerts as blockquotes with a
// `markdown-alert-{kind}` class. Keep the presentation in the generated
// document so alerts also work for bare installs with no external assets.
// The explicit palette values follow the theme toggle rather than relying on
// the browser's OS preference after a reader has selected a manual theme.
const GFM_ALERTS_CSS: &str = r#"
:root[data-theme="light"]{--alert-note:#0969da;--alert-tip:#1a7f37;--alert-important:#8250df;--alert-warning:#9a6700;--alert-caution:#cf222e}
:root[data-theme="dark"]{--alert-note:#58a6ff;--alert-tip:#3fb950;--alert-important:#a371f7;--alert-warning:#d29922;--alert-caution:#f85149}
blockquote[class^="markdown-alert-"]{--alert-color:var(--accent);border-left:.3rem solid var(--alert-color);background:var(--accent-bg);padding:.75rem 1rem}
blockquote.markdown-alert-note{--alert-color:var(--alert-note)}blockquote.markdown-alert-tip{--alert-color:var(--alert-tip)}blockquote.markdown-alert-important{--alert-color:var(--alert-important)}blockquote.markdown-alert-warning{--alert-color:var(--alert-warning)}blockquote.markdown-alert-caution{--alert-color:var(--alert-caution)}
"#;

// ─── BEGIN THEME TOGGLE ────────────────────────────────────────────────
// Self-contained light/dark mode toggle. To remove this feature entirely:
//   1. Delete this block (down to "END THEME TOGGLE").
//   2. Delete the `{theme_toggle}` line and `theme_toggle = ...` arg in
//      `wrap_html5` below.
// Variable values mirror simple.css's @media (prefers-color-scheme: dark).
//
// Behavior notes:
// - OS theme is detected automatically and tracked live until the user makes
//   a manual choice; the manual choice persists via localStorage.
// - localStorage can throw on file:// pages in some browsers/configurations,
//   so every access is wrapped \u2014 the toggle still works for the current page
//   even when persistence is unavailable.
// - An in-memory `manual` flag (not the localStorage read) is the source of
//   truth for "the user made an explicit choice". It is initialized from the
//   saved value and set on every toggle click regardless of whether
//   localStorage.setItem succeeds, so a later prefers-color-scheme change
//   can never clobber an explicit choice just because persistence failed
//   (e.g. on file:// pages).
// - The button is a real <button> (keyboard focusable/activatable) with a
//   state-aware aria-label.
// - This <style> block is emitted before the --css override block, so custom
//   CSS can restyle or hide the toggle (e.g. `#theme-toggle{display:none}`)
//   and reclaim the reserved narrow-screen padding (`body{padding-top:0}`).
const THEME_TOGGLE: &str = r#"<style id="mdo-theme-toggle">
:root[data-theme="light"]{color-scheme:light;--bg:#fff;--accent-bg:#f5f7ff;--text:#212121;--text-light:#585858;--accent:#0d47a1;--accent-hover:#1266e2;--accent-text:var(--bg);--code:#d81b60;--preformatted:#444;--disabled:#efefef}
:root[data-theme="dark"]{color-scheme:dark;--bg:#212121;--accent-bg:#2b2b2b;--text:#dcdcdc;--text-light:#ababab;--accent:#ffb300;--accent-hover:#ffe099;--accent-text:var(--bg);--code:#f06292;--preformatted:#ccc;--disabled:#111}
:root[data-theme="dark"] img,:root[data-theme="dark"] video{opacity:.8}
#theme-toggle{position:fixed;top:.75rem;right:.75rem;z-index:1000;padding:.25rem .6rem;font-size:1rem;line-height:1;cursor:pointer;border-radius:var(--standard-border-radius);border:var(--border-width) solid var(--border);background:var(--accent-bg);color:var(--text)}
@media (max-width:55rem){body{padding-top:2.75rem}}
@media print{#theme-toggle{display:none}}
</style>
<script>
(function(){
  var read=function(){try{return localStorage.getItem('theme')}catch(e){return null}};
  var apply=function(t){document.documentElement.dataset.theme=t;};
  var mq=matchMedia('(prefers-color-scheme: dark)');
  var saved=read();
  var manual=saved!==null;
  apply(saved||(mq.matches?'dark':'light'));
  document.addEventListener('DOMContentLoaded',function(){
    var b=document.createElement('button');
    b.id='theme-toggle';b.type='button';
    var sync=function(){
      var dark=document.documentElement.dataset.theme==='dark';
      b.textContent=dark?'\u2600':'\u263E';
      var label=dark?'Switch to light theme':'Switch to dark theme';
      b.title=label;
      b.setAttribute('aria-label',label);
    };
    b.onclick=function(){
      var next=document.documentElement.dataset.theme==='dark'?'light':'dark';
      apply(next);
      manual=true;
      try{localStorage.setItem('theme',next)}catch(e){}
      sync();
    };
    if(mq.addEventListener){
      mq.addEventListener('change',function(e){
        if(!manual){apply(e.matches?'dark':'light');sync();}
      });
    }
    document.body.appendChild(b);
    sync();
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
///
/// Delegates to [`Url::from_directory_path`], which correctly
/// percent-encodes reserved and non-ASCII characters (spaces, `#`, `%`, `?`,
/// Unicode, …) and handles Windows drive letters, UNC paths, and the
/// `\\?\`/`\\?\UNC\` extended-length prefixes that `fs::canonicalize`
/// produces — a hand-rolled string transform got all of this subtly wrong
/// (e.g. a literal `#` in a directory name would truncate the URL when a
/// browser parsed it as a fragment). Always includes a trailing slash so
/// relative refs resolve correctly against the directory.
///
/// Returns `None` if `dir` can't be represented as a `file://` URL (notably:
/// relative paths). Callers should fall back to omitting the `<base href>`
/// tag entirely rather than emit a broken URL.
fn dir_to_file_url(dir: &Path) -> Option<String> {
    Url::from_directory_path(dir)
        .ok()
        .map(|url| url.to_string())
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
        // `open` returns promptly once Launch Services accepts the request,
        // so waiting for it doesn't block on the browser itself — and a
        // nonzero exit (no handler for the file) must surface as a launch
        // failure rather than being discarded with the child handle.
        let status = std::process::Command::new("open").arg(path).status()?;
        if !status.success() {
            return Err(io::Error::other(format!("open exited with {status}")));
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        launch_via_xdg_open(path)?;
    }
    Ok(())
}

/// Open `path` with the first available freedesktop opener. Falls back through
/// common launchers when `xdg-open` (xdg-utils) is not installed, and reaps the
/// launcher process in a detached thread so it does not linger as a zombie
/// during long `--watch` sessions.
///
/// Spawning an opener is not the same as launching a browser: `xdg-open` can
/// exist, start, and then exit nonzero because no browser handler is
/// configured. We therefore listen for each opener's exit for a short window
/// — a quick nonzero exit is an honest failure (and we fall through to the
/// next opener), while an opener still running after the window has almost
/// certainly handed off (or, in xdg-open's no-desktop fallback, IS the
/// browser), so we detach and call it success. We never block on the browser
/// itself beyond that window.
#[cfg(all(unix, not(target_os = "macos")))]
fn launch_via_xdg_open(path: &Path) -> std::io::Result<()> {
    const OPENERS: &[(&str, &[&str])] = &[
        ("xdg-open", &[]),
        ("gio", &["open"]),
        ("gnome-open", &[]),
        ("kde-open5", &[]),
        ("kde-open", &[]),
        ("wslview", &[]),
    ];
    const LAUNCH_FAILURE_WINDOW: Duration = Duration::from_secs(2);

    let mut last_failure: Option<String> = None;
    for (program, leading) in OPENERS {
        let mut command = std::process::Command::new(program);
        command.args(*leading).arg(path);
        let Ok(mut child) = command.spawn() else {
            continue; // not installed; try the next opener
        };

        // The waiting thread doubles as the reaper for the detached-success
        // case: it always waits the child to completion, we just stop
        // listening for the result after the failure window.
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(child.wait());
        });

        match rx.recv_timeout(LAUNCH_FAILURE_WINDOW) {
            Ok(Ok(status)) if status.success() => return Ok(()),
            Ok(Ok(status)) => {
                last_failure = Some(format!("{program} exited with {status}"));
            }
            Ok(Err(e)) => {
                last_failure = Some(format!("failed waiting on {program}: {e}"));
            }
            // Still running after the window: it handed off to (or is) the
            // browser. The waiting thread reaps it eventually.
            Err(_) => return Ok(()),
        }
    }

    match last_failure {
        Some(failure) => Err(io::Error::other(failure)),
        None => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "no desktop opener found (install xdg-utils)",
        )),
    }
}

fn markdown_to_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_GFM);
    options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);

    let parser = MdParser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
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

/// Render Markdown supplied in memory as a complete, self-contained HTML page.
///
/// This is the platform-neutral entry point used by the Android application.
/// Raw HTML is always sanitized, matching the safe default of the desktop CLI.
/// `fallback_title` is used only when the document has no top-level heading.
pub fn render_markdown_document(
    markdown: &str,
    fallback_title: &str,
    source_modified_unix_secs: Option<u64>,
) -> String {
    let body = sanitize_html(&markdown_to_html(markdown));
    let title = derive_title(markdown, fallback_title);
    wrap_html5(&body, &title, None, None, source_modified_unix_secs)
}

fn derive_title(markdown: &str, fallback: &str) -> String {
    if let Some(title) = front_matter_title(markdown) {
        return title;
    }

    // A valid metadata block is not document content. Exclude it from the
    // existing H1 search even when it has no usable title key.
    let markdown = yaml_front_matter(markdown)
        .map(|(_, document)| document)
        .unwrap_or(markdown);

    // Strip fenced code blocks first so a heading — whether an ATX `# ` line or
    // a raw <h1> — inside a ``` or ~~~ block is never mistaken for the title.
    let without_fences = strip_code_fences(markdown);
    for line in without_fences.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.trim_end().strip_prefix("# ") {
            return rest.trim().to_string();
        }
    }
    if let Some(title) = derive_raw_html_h1_title(&without_fences) {
        return title;
    }
    fallback.to_string()
}

/// Extract a simple scalar `title:` from a complete YAML-style metadata block
/// at the start of the document.
///
/// pulldown-cmark deliberately recognizes the block delimiters without
/// interpreting YAML. mdo follows that narrow behavior: it reads only an
/// unindented `title` key and does not add a general-purpose YAML parser (and
/// its dependency/security surface) merely to title the generated page.
fn front_matter_title(markdown: &str) -> Option<String> {
    let (metadata, _) = yaml_front_matter(markdown)?;
    let mut title = None;
    for line in metadata.lines() {
        if line.starts_with(char::is_whitespace) {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            if key == "title" {
                let value = unquote_front_matter_scalar(value.trim());
                if !value.is_empty() {
                    title = Some(value.to_string());
                }
            }
        }
    }

    title
}

/// Split a complete YAML-style metadata block from the document body while
/// following pulldown-cmark's delimiter rules closely enough for title
/// extraction. The returned metadata excludes delimiters.
fn yaml_front_matter(markdown: &str) -> Option<(&str, &str)> {
    let first_line_end = markdown.find('\n').unwrap_or(markdown.len());
    if markdown[..first_line_end].trim_end() != "---" || first_line_end == markdown.len() {
        return None;
    }

    let metadata_start = first_line_end + 1;
    let remainder = &markdown[metadata_start..];
    let mut consumed = 0;
    for segment in remainder.split_inclusive('\n') {
        let line = segment.trim_end_matches(['\r', '\n']).trim_end();
        if line == "---" || line == "..." {
            return Some((
                &remainder[..consumed],
                &remainder[consumed + segment.len()..],
            ));
        }
        consumed += segment.len();
    }

    // split_inclusive also yields a final segment without a newline.
    let final_line = remainder[consumed..].trim_end();
    if final_line == "---" || final_line == "..." {
        Some((&remainder[..consumed], ""))
    } else {
        None
    }
}

fn unquote_front_matter_scalar(value: &str) -> &str {
    if value.len() < 2 {
        return value;
    }

    let bytes = value.as_bytes();
    if matches!(
        (bytes[0], bytes[value.len() - 1]),
        (b'\'', b'\'') | (b'"', b'"')
    ) {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

/// Return `markdown` with fenced code blocks (``` or ~~~) removed, so title
/// detection never reads inside code samples.
fn strip_code_fences(markdown: &str) -> String {
    let mut out = String::with_capacity(markdown.len());
    let mut fence: Option<char> = None;
    for line in markdown.lines() {
        if let Some(marker) = code_fence_marker(line.trim_start()) {
            match fence {
                Some(open) if open == marker => fence = None,
                Some(_) => {}
                None => fence = Some(marker),
            }
            continue;
        }
        if fence.is_some() {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Detect a code-fence line, returning its marker char when the line starts
/// with at least three backticks or tildes. This is a pragmatic subset of
/// CommonMark (it does not bound the indent or require the closing fence to be
/// at least as long), which is sufficient for title detection.
fn code_fence_marker(trimmed_start: &str) -> Option<char> {
    if trimmed_start.starts_with("```") {
        Some('`')
    } else if trimmed_start.starts_with("~~~") {
        Some('~')
    } else {
        None
    }
}

fn derive_raw_html_h1_title(markdown: &str) -> Option<String> {
    let lower = markdown.to_ascii_lowercase();
    let mut search_start = 0;

    while let Some(offset) = lower[search_start..].find("<h1") {
        let start = search_start + offset;
        let after_tag_name = start + 3;
        let next = lower.as_bytes().get(after_tag_name).copied();
        if !matches!(
            next,
            Some(b'>') | Some(b' ') | Some(b'\t') | Some(b'\r') | Some(b'\n')
        ) {
            search_start = after_tag_name;
            continue;
        }

        let content_start = start + markdown[start..].find('>')? + 1;
        let content_end = content_start + lower[content_start..].find("</h1>")?;
        let title = strip_html_tags(&markdown[content_start..content_end])
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if !title.is_empty() {
            return Some(title);
        }

        search_start = content_end + 5;
    }

    None
}

fn strip_html_tags(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut in_tag = false;

    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }

    output
}

fn wrap_html5(
    body: &str,
    title: &str,
    base_href: Option<&str>,
    css_override: Option<&str>,
    source_modified_unix_secs: Option<u64>,
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
    let mdo_default_typography = format!(
        "<style id=\"mdo-default-typography\">\n{}\n</style>\n",
        escape_style_end_tags(MDO_DEFAULT_TYPOGRAPHY_CSS)
    );
    // Provenance lives only in <meta name="generator">; the visible page
    // carries no mdo branding, version, or render timing (v0.6 quiet output).
    let generator = format!("{APP_DISPLAY_NAME} {APP_VERSION}");
    // Restrained source-freshness footer: the source file's filesystem
    // modification time, rendered as UTC with a machine-readable datetime.
    // The tiny inline script re-formats it in the reader's locale/timezone
    // when JavaScript is available. Omitted entirely when the timestamp is
    // missing or unreadable.
    let source_meta = source_modified_unix_secs
        .map(|secs| {
            let machine = utc_datetime_from_unix_secs(secs);
            let human = human_utc_datetime_from_unix_secs(secs);
            format!(
                "<footer class=\"mdo-source-meta\">Source modified: <time datetime=\"{machine}\">{human}</time></footer>\n\
                 <script>\n\
                 (function(){{var t=document.querySelector('.mdo-source-meta time');if(!t)return;var d=new Date(t.getAttribute('datetime'));if(isNaN(d))return;try{{t.textContent=d.toLocaleString(undefined,{{year:'numeric',month:'long',day:'numeric',hour:'numeric',minute:'2-digit'}});}}catch(e){{}}}})();\n\
                 </script>\n",
                machine = html_escape(&machine),
                human = html_escape(&human),
            )
        })
        .unwrap_or_default();
    format!(
        "<!DOCTYPE html>\n\
         <html lang=\"en\">\n\
         <head>\n\
         <meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <meta name=\"generator\" content=\"{generator}\">\n\
         {base_tag}\
         <title>{title}</title>\n\
         <style>\n{css}\n{gfm_alerts_css}\n.mdo-source-meta{{margin-top:3rem;border:0;padding:1rem 0 1.5rem;font-size:.8rem;line-height:1.4;color:var(--text-light);text-align:center}}\n</style>\n\
         {theme_toggle}\
         {mdo_default_typography}\
         {css_override_block}\
         </head>\n\
         <body>\n\
         <main>\n{body}\n</main>\n\
         {source_meta}\
         </body>\n\
         </html>\n",
        generator = html_escape(&generator),
        base_tag = base_tag,
        title = html_escape(title),
        css = SIMPLE_CSS,
        gfm_alerts_css = GFM_ALERTS_CSS,
        theme_toggle = THEME_TOGGLE, // ← THEME TOGGLE injection point (delete this line to remove)
        mdo_default_typography = mdo_default_typography,
        css_override_block = css_override_block,
        body = body,
        source_meta = source_meta,
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

const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// The source file's mtime as seconds since the Unix epoch, or `None` when
/// the metadata is missing or unreadable (which must never block rendering).
fn source_modified_unix_secs(input: &Path) -> Option<u64> {
    fs::metadata(input)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|mtime| mtime.duration_since(UNIX_EPOCH).ok())
        .map(|elapsed| elapsed.as_secs())
}

/// ISO 8601 UTC datetime, e.g. `2026-07-14T17:42:05Z`, for `<time datetime>`.
fn utc_datetime_from_unix_secs(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let (year, month, day) = civil_from_days(days);
    let seconds_of_day = secs % 86_400;
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Readable UTC fallback text, e.g. `July 14, 2026, 5:42 PM UTC`, shown when
/// JavaScript cannot re-format the timestamp in the reader's locale.
fn human_utc_datetime_from_unix_secs(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let (year, month, day) = civil_from_days(days);
    let seconds_of_day = secs % 86_400;
    let hour24 = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let (hour12, meridiem) = match hour24 {
        0 => (12, "AM"),
        1..=11 => (hour24, "AM"),
        12 => (12, "PM"),
        _ => (hour24 - 12, "PM"),
    };
    let month_name = MONTH_NAMES[(month - 1) as usize];

    format!("{month_name} {day}, {year}, {hour12}:{minute:02} {meridiem} UTC")
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

/// Wall-clock time spent in each stage of one render workflow, reported to
/// stderr by `--verbose`. Stages that do not run for a given invocation
/// (`sanitize` under `--unsafe-html`, `assemble` under `--bare`) report zero.
#[derive(Default)]
struct StageTimings {
    read: Duration,
    markdown: Duration,
    sanitize: Duration,
    assemble: Duration,
    write: Duration,
}

/// Print per-stage and total timings for a completed render to STDERR only.
/// Performance diagnostics belong in the terminal: they never touch stdout
/// or the generated HTML (v0.6 quiet output). `total` is the wall-clock time
/// of the whole workflow, so it is always at least the sum of the stages.
fn report_render_timings(input: &Path, timings: &StageTimings, total: Duration) {
    let ms = |d: Duration| d.as_secs_f64() * 1000.0;
    eprintln!("⏱  Render workflow for {:?}:", input);
    eprintln!("   read     {:>10.3} ms", ms(timings.read));
    eprintln!("   markdown {:>10.3} ms", ms(timings.markdown));
    eprintln!("   sanitize {:>10.3} ms", ms(timings.sanitize));
    eprintln!("   assemble {:>10.3} ms", ms(timings.assemble));
    eprintln!("   write    {:>10.3} ms", ms(timings.write));
    eprintln!("   total    {:>10.3} ms", ms(total));
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
    convert_with_diagnostics(
        input,
        output,
        bare,
        unsafe_html,
        private_output,
        css_override,
        false,
    )
}

/// Full render workflow with optional `--verbose` timing diagnostics.
///
/// The generated HTML is byte-identical whether or not `verbose` is set:
/// timings are measured around the existing stages and reported to stderr
/// only after a fully successful render.
pub fn convert_with_diagnostics(
    input: &Path,
    output: &Path,
    bare: bool,
    unsafe_html: bool,
    private_output: bool,
    css_override: Option<&Path>,
    verbose: bool,
) -> bool {
    let workflow_start = Instant::now();
    let mut timings = StageTimings::default();

    let stage_start = Instant::now();
    let markdown = match fs::read_to_string(input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ Failed to read {:?}: {}", input, e);
            return false;
        }
    };
    timings.read = stage_start.elapsed();

    let css_override = if bare {
        None
    } else {
        match css_override {
            Some(path) => {
                let stage_start = Instant::now();
                let css = fs::read_to_string(path);
                timings.read += stage_start.elapsed();
                match css {
                    Ok(css) => Some(css),
                    Err(e) => {
                        eprintln!("❌ Failed to read CSS override {:?}: {}", path, e);
                        return false;
                    }
                }
            }
            None => None,
        }
    };

    let stage_start = Instant::now();
    let raw_body = markdown_to_html(&markdown);
    timings.markdown = stage_start.elapsed();

    let body = if unsafe_html {
        raw_body
    } else {
        let stage_start = Instant::now();
        let sanitized = sanitize_html(&raw_body);
        timings.sanitize = stage_start.elapsed();
        sanitized
    };

    let final_html = if bare {
        body
    } else {
        let stage_start = Instant::now();
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
            (Some(in_dir), Some(out_dir)) if in_dir != out_dir => dir_to_file_url(&in_dir),
            _ => None,
        };

        let wrapped = wrap_html5(
            &body,
            &title,
            base_href.as_deref(),
            css_override.as_deref(),
            source_modified_unix_secs(input),
        );
        timings.assemble = stage_start.elapsed();
        wrapped
    };

    let stage_start = Instant::now();
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
        timings.write = stage_start.elapsed();
        println!("✅ Converted {:?} → {:?}", input, output);
        if verbose {
            report_render_timings(input, &timings, workflow_start.elapsed());
        }
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

pub fn open_setup_sample() -> io::Result<()> {
    let source = setup_sample_input_path()?;
    fs::write(&source, SETUP_SAMPLE_MARKDOWN)?;

    let output = temp_output_for(&source)?;
    if !convert(&source, &output, false, false, true) {
        return Err(io::Error::other("sample render failed"));
    }

    launch_browser(&output)?;
    Ok(())
}

fn setup_sample_input_path() -> io::Result<PathBuf> {
    let root = private_temp_root();
    ensure_private_dir(&root)?;

    let dir = root.join("setup");
    ensure_private_dir(&dir)?;
    Ok(dir.join(SETUP_SAMPLE_FILE_NAME))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::SystemTime;

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
    fn formats_unix_timestamps_as_machine_utc_datetimes() {
        assert_eq!(utc_datetime_from_unix_secs(0), "1970-01-01T00:00:00Z");
        assert_eq!(
            utc_datetime_from_unix_secs(951_827_696),
            "2000-02-29T12:34:56Z"
        );
    }

    #[test]
    fn formats_unix_timestamps_as_readable_utc_datetimes() {
        // Midnight and noon exercise the 12-hour AM/PM edge cases.
        assert_eq!(
            human_utc_datetime_from_unix_secs(0),
            "January 1, 1970, 12:00 AM UTC"
        );
        assert_eq!(
            human_utc_datetime_from_unix_secs(951_827_696),
            "February 29, 2000, 12:34 PM UTC"
        );
        assert_eq!(
            human_utc_datetime_from_unix_secs(13 * 3_600 + 5 * 60),
            "January 1, 1970, 1:05 PM UTC"
        );
    }

    #[test]
    fn wrap_html5_keeps_generator_meta_without_visible_branding() {
        let html = wrap_html5("<p>hi</p>", "Title", None, None, Some(0));

        assert!(html.contains(&format!(
            "<meta name=\"generator\" content=\"mdo {APP_VERSION}\">"
        )));
        assert!(!html.contains("Generated by"));
        assert!(!html.contains("mdo-generated"));
    }

    #[test]
    fn wrap_html5_shows_source_modified_time_with_machine_datetime() {
        let html = wrap_html5("<p>hi</p>", "Title", None, None, Some(951_827_696));

        assert!(html.contains(
            "<footer class=\"mdo-source-meta\">Source modified: \
             <time datetime=\"2000-02-29T12:34:56Z\">February 29, 2000, 12:34 PM UTC</time></footer>"
        ));
    }

    #[test]
    fn wrap_html5_renders_without_source_modified_time() {
        let html = wrap_html5("<p>hi</p>", "Title", None, None, None);

        assert!(html.contains("<main>\n<p>hi</p>\n</main>"));
        assert!(!html.contains("Source modified"));
        assert!(!html.contains("<footer"));
    }

    #[test]
    fn in_memory_render_uses_shared_safe_document_pipeline() {
        let html = render_markdown_document(
            "# Phone note\n\nHello **Android**.\n\n<script>alert('no')</script>",
            "fallback.md",
            None,
        );

        assert!(html.contains("<title>Phone note</title>"));
        assert!(html.contains("<strong>Android</strong>"));
        assert!(html.contains("<meta name=\"generator\" content=\"mdo "));
        assert!(!html.contains("<script>alert('no')</script>"));
    }

    #[test]
    fn markdown_extensions_keep_their_existing_html() {
        let markdown = "~~gone~~\n\n\
                        | A | B |\n\
                        | - | - |\n\
                        | 1 | 2 |\n\n\
                        Footnote[^one].\n\n\
                        [^one]: Detail\n\n\
                        - [x] done\n\
                        - [ ] later\n";
        let expected = "<p><del>gone</del></p>\n\
                        <table><thead><tr><th>A</th><th>B</th></tr></thead><tbody>\n\
                        <tr><td>1</td><td>2</td></tr>\n\
                        </tbody></table>\n\
                        <p>Footnote<sup class=\"footnote-reference\"><a href=\"#one\">1</a></sup>.</p>\n\
                        <div class=\"footnote-definition\" id=\"one\"><sup class=\"footnote-definition-label\">1</sup>\n\
                        <p>Detail</p>\n\
                        </div>\n\
                        <ul>\n\
                        <li><input disabled=\"\" type=\"checkbox\" checked=\"\"/>\n\
                        done</li>\n\
                        <li><input disabled=\"\" type=\"checkbox\"/>\n\
                        later</li>\n\
                        </ul>\n";

        assert_eq!(markdown_to_html(markdown), expected);
    }

    #[test]
    fn gfm_alerts_render_all_supported_classes_and_survive_sanitizing() {
        for kind in ["note", "tip", "important", "warning", "caution"] {
            let markdown = format!("> [!{}]\n> Alert text\n", kind.to_uppercase());
            let raw = markdown_to_html(&markdown);
            let class = format!("class=\"markdown-alert-{kind}\"");

            assert!(raw.contains(&class), "{raw}");
            assert!(!raw.contains("[!"), "{raw}");
            assert!(sanitize_html(&raw).contains(&class), "{raw}");
        }
    }

    #[test]
    fn wrapped_document_styles_alerts_in_light_and_dark_palettes() {
        let html = wrap_html5("<p>hi</p>", "Title", None, None, None);

        assert!(html.contains(":root[data-theme=\"light\"]{--alert-note:"));
        assert!(html.contains(":root[data-theme=\"dark\"]{--alert-note:"));
        for kind in ["note", "tip", "important", "warning", "caution"] {
            assert!(html.contains(&format!("blockquote.markdown-alert-{kind}")));
        }
    }

    #[test]
    fn yaml_front_matter_is_hidden_and_supplies_document_title() {
        let markdown =
            "---\ntitle: 'Front Matter Title'\nauthor: Example\n...\n\n# Heading Title\n";
        let body = markdown_to_html(markdown);

        assert_eq!(body, "<h1>Heading Title</h1>\n");
        assert_eq!(derive_title(markdown, "fallback"), "Front Matter Title");

        let document = render_markdown_document(markdown, "fallback", None);
        assert!(document.contains("<title>Front Matter Title</title>"));
        assert!(!document.contains("author: Example"));
    }

    #[test]
    fn front_matter_title_supports_colons_and_double_quotes() {
        let markdown = "---\ntitle: \"Project: Alpha\"\n---\n\n# Heading\n";
        assert_eq!(derive_title(markdown, "fallback"), "Project: Alpha");
    }

    #[test]
    fn missing_empty_or_unclosed_front_matter_title_uses_existing_fallbacks() {
        assert_eq!(
            derive_title("---\nauthor: Example\n---\n\n# Heading\n", "fallback"),
            "Heading"
        );
        assert_eq!(
            derive_title("---\ntitle: \n---\n", "Fallback Title"),
            "Fallback Title"
        );
        assert_eq!(
            derive_title("---\ntitle: Incomplete\n# Real Heading\n", "fallback"),
            "Real Heading"
        );
        assert_eq!(
            derive_title(
                "---\ndescription: |\n# Metadata, not a heading\n---\n\n# Real Heading\n",
                "fallback"
            ),
            "Real Heading"
        );
    }

    #[test]
    fn smart_punctuation_is_not_enabled() {
        let html = markdown_to_html("-- --- ... \"straight\" 'quotes'\n");

        assert_eq!(html, "<p>-- --- ... \"straight\" 'quotes'</p>\n");
        assert!(!html.contains(['–', '—', '…', '“', '”', '‘', '’']));
    }

    #[test]
    fn title_can_come_from_raw_html_h1() {
        let title = derive_title(
            r#"<section><h1 class="hero-title">Project <span>Home</span></h1></section>"#,
            "fallback",
        );

        assert_eq!(title, "Project Home");
    }

    #[test]
    fn derive_title_ignores_headings_in_code_fences() {
        let markdown = "```sh\n# not a title\n```\n\n# Real Title\n";
        assert_eq!(derive_title(markdown, "fallback"), "Real Title");
    }

    #[test]
    fn derive_title_falls_back_when_only_heading_is_fenced() {
        let markdown = "~~~\n# fenced heading\n~~~\n";
        assert_eq!(derive_title(markdown, "Fallback Title"), "Fallback Title");
    }

    #[test]
    fn derive_title_ignores_raw_html_h1_inside_code_fence() {
        let markdown = "```html\n<h1>Not the title</h1>\n```\n\n# Real Title\n";
        assert_eq!(derive_title(markdown, "fallback"), "Real Title");
    }

    #[test]
    fn css_override_escapes_style_end_tags() {
        let escaped = escape_style_end_tags("h1{} </STYLE><script>alert(1)</script>");

        assert!(escaped.contains("<\\/style><script>"));
        assert!(!escaped.to_ascii_lowercase().contains("</style><script>"));
    }

    // Unix-rooted paths like `/home/user/...` are NOT absolute on Windows
    // (no drive or UNC prefix), so `Url::from_directory_path` rejects them
    // there — every test using such a path must be `#[cfg(unix)]`-gated or
    // it panics under the Windows CI test job. Windows path forms get their
    // own `#[cfg(windows)]` tests below.
    #[cfg(unix)]
    #[test]
    fn dir_to_file_url_adds_trailing_slash() {
        let url =
            dir_to_file_url(Path::new("/home/user/docs")).expect("absolute path should convert");
        assert!(url.ends_with('/'));
        assert_eq!(url, "file:///home/user/docs/");
    }

    #[cfg(unix)]
    #[test]
    fn dir_to_file_url_encodes_spaces() {
        let url = dir_to_file_url(Path::new("/home/user/my docs")).expect("should convert");
        assert_eq!(url, "file:///home/user/my%20docs/");
    }

    // `#` is a legal Windows filename character but is not exercised there
    // because these are pure-path conversions, not filesystem operations —
    // gating still keeps the Windows suite focused on Windows-specific forms
    // (drive letters, UNC, `\\?\`) covered below.
    #[cfg(unix)]
    #[test]
    fn dir_to_file_url_encodes_hash() {
        let url = dir_to_file_url(Path::new("/home/user/docs#1")).expect("should convert");
        assert_eq!(url, "file:///home/user/docs%231/");
    }

    #[cfg(unix)]
    #[test]
    fn dir_to_file_url_encodes_percent() {
        let url = dir_to_file_url(Path::new("/home/user/50%done")).expect("should convert");
        assert_eq!(url, "file:///home/user/50%25done/");
    }

    // `?` is invalid in Windows filenames, so this case can't arise there.
    #[cfg(unix)]
    #[test]
    fn dir_to_file_url_encodes_question_mark() {
        let url = dir_to_file_url(Path::new("/home/user/docs?draft")).expect("should convert");
        assert_eq!(url, "file:///home/user/docs%3Fdraft/");
    }

    #[cfg(unix)]
    #[test]
    fn dir_to_file_url_encodes_unicode_accented() {
        let url = dir_to_file_url(Path::new("/home/user/résumé")).expect("should convert");
        assert_eq!(url, "file:///home/user/r%C3%A9sum%C3%A9/");
    }

    #[cfg(unix)]
    #[test]
    fn dir_to_file_url_encodes_unicode_cjk() {
        let url = dir_to_file_url(Path::new("/home/user/日本語")).expect("should convert");
        assert_eq!(url, "file:///home/user/%E6%97%A5%E6%9C%AC%E8%AA%9E/");
    }

    #[test]
    fn dir_to_file_url_returns_none_for_relative_path() {
        assert_eq!(dir_to_file_url(Path::new("relative/dir")), None);
    }

    #[cfg(windows)]
    #[test]
    fn dir_to_file_url_windows_drive_letter() {
        let url = dir_to_file_url(Path::new(r"C:\foo\bar")).expect("should convert");
        assert_eq!(url, "file:///C:/foo/bar/");
    }

    #[cfg(windows)]
    #[test]
    fn dir_to_file_url_windows_drive_letter_with_space() {
        let url = dir_to_file_url(Path::new(r"C:\Program Files\mdo")).expect("should convert");
        assert_eq!(url, "file:///C:/Program%20Files/mdo/");
    }

    // `fs::canonicalize` on Windows produces the `\\?\` extended-length
    // prefix; browsers don't understand `file:////?/C:/...`, so this must
    // normalize to the same form as the plain drive-letter case.
    #[cfg(windows)]
    #[test]
    fn dir_to_file_url_windows_extended_length_prefix() {
        let url = dir_to_file_url(Path::new(r"\\?\C:\foo\bar")).expect("should convert");
        assert_eq!(url, "file:///C:/foo/bar/");
    }

    #[cfg(windows)]
    #[test]
    fn dir_to_file_url_windows_unc_path() {
        let url = dir_to_file_url(Path::new(r"\\server\share\dir")).expect("should convert");
        assert_eq!(url, "file://server/share/dir/");
    }

    #[test]
    fn setup_sample_contains_welcome_copy() {
        assert!(SETUP_SAMPLE_MARKDOWN.contains("# Welcome to the world of Open as HTML with mdo"));
        assert!(SETUP_SAMPLE_MARKDOWN.contains("opened it successfully"));
        assert_eq!(
            SETUP_SAMPLE_FILE_NAME,
            "welcome-to-open-as-html-with-mdo.md"
        );
    }
}
