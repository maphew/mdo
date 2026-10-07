//! Page assembly: everything that turns an HTML fragment into the final
//! HTML document - styles, scripts, and document wrapping.

use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;
const SIMPLE_CSS: &str = include_str!("../assets/simple.min.css");
const APP_DISPLAY_NAME: &str = "mdo";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

// mdo keeps simple.css as the base stylesheet but softens the heading/body
// scale for generated documents. Users who prefer the unmodified vendored
// simple.css typography can pass assets/restore-simple-css.css with --css.
const MDO_DEFAULT_TYPOGRAPHY_CSS: &str = include_str!("../assets/mdo-default-typography.css");

// pulldown-cmark emits GFM alerts as blockquotes with a
// `markdown-alert-{kind}` class. Keep the presentation in the generated
// document so alerts also work for bare installs with no external assets.
// The plain `:root` and prefers-color-scheme rules keep alerts styled when
// JavaScript is disabled (the theme-toggle script never runs, so no
// data-theme attribute is ever set); the higher-specificity
// `:root[data-theme=...]` rules then follow the theme toggle rather than the
// OS preference once a reader has selected a manual theme.
const GFM_ALERTS_CSS: &str = r#"
:root{--alert-note:#0969da;--alert-tip:#1a7f37;--alert-important:#8250df;--alert-warning:#9a6700;--alert-caution:#cf222e}
@media (prefers-color-scheme: dark){:root{--alert-note:#58a6ff;--alert-tip:#3fb950;--alert-important:#a371f7;--alert-warning:#d29922;--alert-caution:#f85149}}
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

pub(crate) fn wrap_html5(
    body: &str,
    title: &str,
    base_href: Option<&str>,
    css_override: Option<&str>,
    highlight_css: Option<&str>,
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
    let highlight_css = highlight_css.unwrap_or_default();
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
         <style>\n{css}\n{gfm_alerts_css}\n{highlight_css}\n.mdo-source-meta{{margin-top:3rem;border:0;padding:1rem 0 1.5rem;font-size:.8rem;line-height:1.4;color:var(--text-light);text-align:center}}\n</style>\n\
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
        highlight_css = highlight_css,
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
pub(crate) fn source_modified_unix_secs(input: &Path) -> Option<u64> {
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
#[cfg(test)]
mod tests {
    use super::*;
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
        let html = wrap_html5("<p>hi</p>", "Title", None, None, None, Some(0));

        assert!(html.contains(&format!(
            "<meta name=\"generator\" content=\"mdo {APP_VERSION}\">"
        )));
        assert!(!html.contains("Generated by"));
        assert!(!html.contains("mdo-generated"));
    }

    #[test]
    fn wrap_html5_shows_source_modified_time_with_machine_datetime() {
        let html = wrap_html5("<p>hi</p>", "Title", None, None, None, Some(951_827_696));

        assert!(html.contains(
            "<footer class=\"mdo-source-meta\">Source modified: \
             <time datetime=\"2000-02-29T12:34:56Z\">February 29, 2000, 12:34 PM UTC</time></footer>"
        ));
    }

    #[test]
    fn wrap_html5_renders_without_source_modified_time() {
        let html = wrap_html5("<p>hi</p>", "Title", None, None, None, None);

        assert!(html.contains("<main>\n<p>hi</p>\n</main>"));
        assert!(!html.contains("Source modified"));
        assert!(!html.contains("<footer"));
    }

    #[test]
    fn wrapped_document_styles_alerts_in_light_and_dark_palettes() {
        let html = wrap_html5("<p>hi</p>", "Title", None, None, None, None);

        // No-JS defaults: plain :root plus an OS dark-mode media query, so
        // alerts stay styled when the theme-toggle script never runs.
        assert!(html.contains("\n:root{--alert-note:"));
        assert!(html.contains("@media (prefers-color-scheme: dark){:root{--alert-note:"));
        // Manual theme-toggle overrides.
        assert!(html.contains(":root[data-theme=\"light\"]{--alert-note:"));
        assert!(html.contains(":root[data-theme=\"dark\"]{--alert-note:"));
        for kind in ["note", "tip", "important", "warning", "caution"] {
            assert!(html.contains(&format!("blockquote.markdown-alert-{kind}")));
        }
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
}
