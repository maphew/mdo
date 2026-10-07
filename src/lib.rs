//! Shared mdo rendering, watch-mode, temp-output, browser launch, page
//! assembly, and integration helpers.
//!
//! The crate is organized by its four real concerns (names per GLOSSARY.md):
//! [`render`] (Markdown to fragment plus the deep `convert` workflow),
//! [`page`] (assets, styles/scripts, document wrapping), [`temp_output`]
//! (per-source private output locations), and [`launch`] (browser openers).
//! Each keeps its internals private behind thin public exports re-exported
//! here, so caller paths do not change when the layout does.

use std::fs;
use std::io;
use std::path::PathBuf;

#[cfg(target_os = "android")]
mod android;
pub mod file_manager;
pub mod handler;
mod highlight;
pub mod launch;
pub mod page;
pub mod render;
pub mod temp_output;
pub mod watch;
#[cfg(target_os = "windows")]
pub mod windows_setup;

use temp_output::{ensure_private_dir, private_temp_root};

pub use launch::launch_browser;
pub use render::{
    convert, render_markdown_document, ConvertOutcome, ConvertRequest, RenderError, StageTimings,
};
pub use temp_output::temp_output_for;

const SETUP_SAMPLE_FILE_NAME: &str = "welcome-to-open-as-html-with-mdo.md";
const SETUP_SAMPLE_MARKDOWN: &str = "\
# Welcome to the world of Open as HTML with mdo

If you are reading this in your browser, mdo rendered Markdown as HTML and
opened it successfully.

Next, right-click any `.md` file and choose **Open as HTML** to read it this way.
If you make mdo your default Markdown app, a double-click does the same.
";

pub fn open_setup_sample() -> io::Result<()> {
    let source = setup_sample_input_path()?;
    fs::write(&source, SETUP_SAMPLE_MARKDOWN)?;

    let output = temp_output_for(&source)?;
    convert(ConvertRequest {
        input: &source,
        output: &output,
        bare: false,
        unsafe_html: false,
        private_output: true,
        css_override: None,
    })
    .map_err(io::Error::other)?;

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
