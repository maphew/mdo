use std::collections::BTreeSet;
use std::sync::OnceLock;

use pulldown_cmark::{html, CodeBlockKind, CowStr, Event, Options, Parser, Tag, TagEnd};
use syntect::highlighting::ThemeSet;
use syntect::html::{css_for_theme_with_class_style, ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

const CLASS_STYLE: ClassStyle = ClassStyle::SpacedPrefixed {
    prefix: "mdo-syntect-",
};

/// Fenced blocks larger than this render as plain escaped code instead of
/// being highlighted. syntect drives fancy-regex grammars whose worst cases
/// are superlinear in input size, so an unbounded block could stall
/// rendering; 200 KB is far beyond anything a human reads highlighted.
const MAX_HIGHLIGHT_BYTES: usize = 200 * 1024;

/// Common fence-info aliases that `SyntaxSet::find_syntax_by_token` does not
/// resolve by itself, mapped to tokens that do resolve in the bundled
/// syntax set. The unit test `common_fence_aliases_resolve_to_bundled_syntaxes`
/// asserts every entry (alias and target) actually resolves.
const LANGUAGE_ALIASES: &[(&str, &str)] = &[
    ("sh", "bash"),
    ("shell", "bash"),
    ("zsh", "bash"),
    ("yml", "yaml"),
    ("csharp", "C#"),
    ("cs", "C#"),
    ("c++", "cpp"),
    ("jsonc", "json"),
    // The bundled syntax set has no TypeScript grammar; JavaScript is the
    // closest match and far better than an unhighlighted block.
    ("ts", "js"),
    ("tsx", "js"),
    ("typescript", "js"),
];

struct Replacement {
    fence_index: usize,
    token: String,
    html: String,
}

/// Why a rendered document could not be spliced after sanitization.
#[derive(Debug)]
pub(crate) enum SpliceError {
    /// At least one placeholder appeared more than once: document text
    /// normalized into a colliding copy of the token. Retrying the render
    /// with a fresh discriminator can produce a collision-free set.
    Collision,
    /// These fences' placeholders were destroyed outright (zero occurrences),
    /// for example swallowed as rawtext by a user-authored unclosed
    /// `<style>` or `<script>`. No discriminator survives that context, so
    /// the caller must re-render with these fences as plain code blocks.
    Destroyed(BTreeSet<usize>),
}

pub(crate) struct RenderedMarkdown {
    raw_html: String,
    replacements: Vec<Replacement>,
    fence_count: usize,
}

impl RenderedMarkdown {
    pub(crate) fn raw_html(&self) -> &str {
        &self.raw_html
    }

    /// Total number of fenced code blocks seen by the renderer, whether or
    /// not they were highlighted. Used by callers as the index space for the
    /// plain-fence fallback set.
    pub(crate) fn fence_count(&self) -> usize {
        self.fence_count
    }

    /// Splice generated markup only when every placeholder survived the
    /// caller's sanitizer exactly once. Validation happens for the complete
    /// set before the first replacement so a collision can never cause a
    /// partial or attacker-positioned splice.
    pub(crate) fn finish(&self, mut body: String) -> Result<String, SpliceError> {
        let mut destroyed = BTreeSet::new();
        let mut collision = false;
        for replacement in &self.replacements {
            match body.matches(&replacement.token).count() {
                1 => {}
                0 => {
                    destroyed.insert(replacement.fence_index);
                }
                _ => collision = true,
            }
        }
        // Destroyed placeholders take priority: retrying with a new
        // discriminator can never revive them, so report the fences that
        // must fall back to plain blocks before collision retries resume.
        if !destroyed.is_empty() {
            return Err(SpliceError::Destroyed(destroyed));
        }
        if collision {
            return Err(SpliceError::Collision);
        }

        for replacement in &self.replacements {
            body = body.replacen(&replacement.token, &replacement.html, 1);
        }
        Ok(body)
    }

    pub(crate) fn css(&self) -> Option<&'static str> {
        (!self.replacements.is_empty()).then(highlight_css)
    }
}

/// Render Markdown to HTML, replacing highlightable fenced code blocks with
/// unique placeholder tokens. Fences whose index appears in `plain_fences`
/// (or that exceed [`MAX_HIGHLIGHT_BYTES`]) are emitted as ordinary escaped
/// code blocks instead.
pub(crate) fn render(
    markdown: &str,
    options: Options,
    placeholder_discriminator: u64,
    plain_fences: &BTreeSet<usize>,
) -> RenderedMarkdown {
    let syntax_set = syntax_set();
    let placeholder_prefix = placeholder_prefix(markdown, placeholder_discriminator);
    let mut parser = Parser::new_ext(markdown, options);
    let mut events = Vec::new();
    let mut replacements = Vec::new();
    let mut fence_count = 0_usize;

    while let Some(event) = parser.next() {
        let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) = event else {
            events.push(event);
            continue;
        };

        let fence_index = fence_count;
        fence_count += 1;

        let start = Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info.clone())));
        let mut code_events = Vec::new();
        let mut code = String::new();
        let mut end = None;
        for code_event in parser.by_ref() {
            if matches!(code_event, Event::End(TagEnd::CodeBlock)) {
                end = Some(code_event);
                break;
            }
            if let Event::Text(text) = &code_event {
                code.push_str(text);
            }
            code_events.push(code_event);
        }

        let language = info.split_whitespace().next().unwrap_or_default();
        let highlighted = if plain_fences.contains(&fence_index) || code.len() > MAX_HIGHLIGHT_BYTES
        {
            None
        } else {
            resolve_language(syntax_set, language)
                .and_then(|syntax| highlight_code(&code, syntax, syntax_set).ok())
        };

        if let Some(highlighted) = highlighted {
            let token = format!("{placeholder_prefix}{fence_index}__");
            let language = escape_html_attribute(language);
            let generated = format!(
                "<pre class=\"mdo-highlight mdo-syntect-code\"><code class=\"language-{language}\">{highlighted}</code></pre>\n"
            );
            events.push(Event::Html(CowStr::Boxed(token.clone().into_boxed_str())));
            replacements.push(Replacement {
                fence_index,
                token,
                html: generated,
            });
        } else {
            events.push(start);
            events.extend(code_events);
            if let Some(end) = end {
                events.push(end);
            }
        }
    }

    let mut raw_html = String::new();
    html::push_html(&mut raw_html, events.into_iter());
    RenderedMarkdown {
        raw_html,
        replacements,
        fence_count,
    }
}

fn syntax_set() -> &'static SyntaxSet {
    static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn resolve_language<'a>(syntax_set: &'a SyntaxSet, token: &str) -> Option<&'a SyntaxReference> {
    let canonical = LANGUAGE_ALIASES
        .iter()
        .find(|(alias, _)| token.eq_ignore_ascii_case(alias))
        .map_or(token, |(_, canonical)| canonical);
    syntax_set.find_syntax_by_token(canonical)
}

fn highlight_code(
    code: &str,
    syntax: &SyntaxReference,
    syntax_set: &SyntaxSet,
) -> Result<String, syntect::Error> {
    let mut generator = ClassedHTMLGenerator::new_with_class_style(syntax, syntax_set, CLASS_STYLE);
    for line in LinesWithEndings::from(code) {
        generator.parse_html_for_line_which_includes_newline(line)?;
    }
    Ok(generator.finalize())
}

fn placeholder_prefix(markdown: &str, first_discriminator: u64) -> String {
    for discriminator in first_discriminator.. {
        let candidate = format!("__MDO_HIGHLIGHT_PLACEHOLDER_{discriminator}_");
        if !markdown.contains(&candidate) {
            return candidate;
        }
    }
    unreachable!("u64 placeholder discriminator space exhausted")
}

fn escape_html_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn highlight_css() -> &'static str {
    static CSS: OnceLock<String> = OnceLock::new();
    CSS.get_or_init(|| {
        let themes = ThemeSet::load_defaults();
        let light = css_for_theme_with_class_style(&themes.themes["InspiredGitHub"], CLASS_STYLE)
            .expect("bundled light theme should generate CSS");
        let dark = css_for_theme_with_class_style(&themes.themes["base16-ocean.dark"], CLASS_STYLE)
            .expect("bundled dark theme should generate CSS");

        format!(
            "/* Syntax highlighting generated by syntect. */\n{}\n{}",
            scope_theme_css(&light, ":root[data-theme=\"light\"]"),
            scope_theme_css(&dark, ":root[data-theme=\"dark\"]")
        )
    })
}

/// Prefix every selector in every rule with the theme scope. syntect emits
/// comma-joined selector lists on a single line, so each comma-separated
/// selector must be scoped individually or every selector after the first
/// comma would apply in both themes.
fn scope_theme_css(css: &str, theme_selector: &str) -> String {
    let mut scoped = String::with_capacity(css.len() + 1024);
    for line in css.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with('.') {
            scoped.push_str(line);
            scoped.push('\n');
            continue;
        }

        let (selectors, tail) = match trimmed.split_once('{') {
            Some((selectors, tail)) => (selectors, Some(tail)),
            None => (trimmed, None),
        };
        let mut first = true;
        for selector in selectors.split(',') {
            if !first {
                scoped.push_str(", ");
            }
            first = false;
            scoped.push_str(theme_selector);
            scoped.push(' ');
            scoped.push_str(selector.trim());
        }
        if let Some(tail) = tail {
            scoped.push_str(" {");
            scoped.push_str(tail);
        }
        scoped.push('\n');
    }
    scoped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_prefix_avoids_literal_source_collisions() {
        let markdown = "__MDO_HIGHLIGHT_PLACEHOLDER_0_0__";
        assert_eq!(
            placeholder_prefix(markdown, 0),
            "__MDO_HIGHLIGHT_PLACEHOLDER_1_"
        );
    }

    #[test]
    fn generated_theme_css_is_scoped_to_both_manual_palettes() {
        let css = highlight_css();
        assert!(css.contains(":root[data-theme=\"light\"] .mdo-syntect-code"));
        assert!(css.contains(":root[data-theme=\"dark\"] .mdo-syntect-code"));

        // A selector reachable only after a comma must never be emitted
        // without a theme scope, or dark colors would leak into light mode.
        assert!(
            !css.contains(", .mdo-syntect-"),
            "found comma-joined selector without a theme scope"
        );

        for line in css.lines().filter(|line| line.contains(".mdo-syntect-")) {
            let selector_list = match line.split_once('{') {
                Some((selectors, _)) => selectors,
                None => continue, // property line such as `color: #ddd;`
            };
            for selector in selector_list.split(',') {
                let selector = selector.trim();
                assert!(
                    selector.starts_with(":root[data-theme=\"light\"] .mdo-syntect-")
                        || selector.starts_with(":root[data-theme=\"dark\"] .mdo-syntect-"),
                    "unscoped selector: {selector}"
                );
            }
        }
    }

    #[test]
    fn common_fence_aliases_resolve_to_bundled_syntaxes() {
        let syntax_set = syntax_set();
        for (alias, target) in LANGUAGE_ALIASES {
            assert!(
                syntax_set.find_syntax_by_token(target).is_some(),
                "alias target {target:?} does not resolve in the bundled syntax set"
            );
            assert!(
                resolve_language(syntax_set, alias).is_some(),
                "alias {alias:?} does not resolve"
            );
        }
        // Unknown languages still fall back to plain blocks.
        assert!(resolve_language(syntax_set, "not-a-real-language").is_none());
    }

    #[test]
    fn oversized_fences_fall_back_to_plain_blocks() {
        let line = "let value = 1;\n";
        let code = line.repeat(MAX_HIGHLIGHT_BYTES / line.len() + 1);
        let markdown = format!("```rust\n{code}```\n");
        let rendered = render(&markdown, Options::empty(), 0, &BTreeSet::new());

        assert!(rendered.replacements.is_empty());
        assert!(rendered.raw_html().contains("let value = 1;"));
        assert!(!rendered.raw_html().contains("mdo-syntect"));

        // A block just under the cap still highlights.
        let small = render(
            "```rust\nlet value = 1;\n```\n",
            Options::empty(),
            0,
            &BTreeSet::new(),
        );
        assert_eq!(small.replacements.len(), 1);
    }

    #[test]
    fn plain_fences_are_skipped_by_index() {
        let markdown = "```rust\nlet one = 1;\n```\n\n```rust\nlet two = 2;\n```\n";
        let plain = BTreeSet::from([0]);
        let rendered = render(markdown, Options::empty(), 0, &plain);

        assert_eq!(rendered.fence_count(), 2);
        assert_eq!(rendered.replacements.len(), 1);
        assert_eq!(rendered.replacements[0].fence_index, 1);
        assert!(rendered.raw_html().contains("let one = 1;"));
        assert!(!rendered.raw_html().contains("let two = 2;"));
    }
}
