use std::sync::OnceLock;

use pulldown_cmark::{html, CodeBlockKind, CowStr, Event, Options, Parser, Tag, TagEnd};
use syntect::highlighting::ThemeSet;
use syntect::html::{css_for_theme_with_class_style, ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

const CLASS_STYLE: ClassStyle = ClassStyle::SpacedPrefixed {
    prefix: "mdo-syntect-",
};

struct Replacement {
    token: String,
    html: String,
}

pub(crate) struct RenderedMarkdown {
    raw_html: String,
    replacements: Vec<Replacement>,
}

impl RenderedMarkdown {
    pub(crate) fn raw_html(&self) -> &str {
        &self.raw_html
    }

    /// Splice generated markup only when every placeholder survived the
    /// caller's sanitizer exactly once. Validation happens for the complete
    /// set before the first replacement so a collision can never cause a
    /// partial or attacker-positioned splice.
    pub(crate) fn finish(&self, mut body: String) -> Option<String> {
        if self
            .replacements
            .iter()
            .any(|replacement| body.matches(&replacement.token).count() != 1)
        {
            return None;
        }

        for replacement in &self.replacements {
            body = body.replacen(&replacement.token, &replacement.html, 1);
        }
        Some(body)
    }

    pub(crate) fn css(&self) -> Option<&'static str> {
        (!self.replacements.is_empty()).then(highlight_css)
    }
}

pub(crate) fn render(
    markdown: &str,
    options: Options,
    placeholder_discriminator: u64,
) -> RenderedMarkdown {
    let syntax_set = syntax_set();
    let placeholder_prefix = placeholder_prefix(markdown, placeholder_discriminator);
    let mut parser = Parser::new_ext(markdown, options);
    let mut events = Vec::new();
    let mut replacements = Vec::new();

    while let Some(event) = parser.next() {
        let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) = event else {
            events.push(event);
            continue;
        };

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
        let highlighted = syntax_set
            .find_syntax_by_token(language)
            .and_then(|syntax| highlight_code(&code, syntax, syntax_set).ok());

        if let Some(highlighted) = highlighted {
            let token = format!("{placeholder_prefix}{}__", replacements.len());
            let language = escape_html_attribute(language);
            let generated = format!(
                "<pre class=\"mdo-highlight mdo-syntect-code\"><code class=\"language-{language}\">{highlighted}</code></pre>\n"
            );
            events.push(Event::Html(CowStr::Boxed(token.clone().into_boxed_str())));
            replacements.push(Replacement {
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
    }
}

fn syntax_set() -> &'static SyntaxSet {
    static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn highlight_code(
    code: &str,
    syntax: &syntect::parsing::SyntaxReference,
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

fn scope_theme_css(css: &str, theme_selector: &str) -> String {
    let mut scoped = String::with_capacity(css.len() + 256);
    for line in css.lines() {
        if line.trim_start().starts_with('.') {
            scoped.push_str(theme_selector);
            scoped.push(' ');
        }
        scoped.push_str(line);
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
        for selector in css
            .lines()
            .filter(|line| line.contains(".mdo-syntect-") && line.trim_end().ends_with('{'))
        {
            assert!(
                selector.starts_with(":root[data-theme=\"light\"] ")
                    || selector.starts_with(":root[data-theme=\"dark\"] "),
                "unscoped selector: {selector}"
            );
        }
    }
}
