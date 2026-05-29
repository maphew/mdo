use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_dir(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("mdo-test-{name}-{}-{stamp}", std::process::id()));
    fs::create_dir_all(&dir).expect("failed to create temp fixture dir");
    dir
}

#[test]
fn converts_markdown_to_styled_html5_document() {
    let dir = fixture_dir("html5");
    let input = dir.join("sample.md");
    fs::write(
        &input,
        "# Sample Title\n\nA paragraph with **strong** text.\n\n| A | B |\n|---|---|\n| 1 | 2 |\n",
    )
    .expect("failed to write markdown fixture");

    let output = Command::new(env!("CARGO_BIN_EXE_mdo"))
        .arg(&input)
        .output()
        .expect("failed to run mdo");

    assert!(output.status.success(), "mdo failed: {output:?}");

    let html = fs::read_to_string(dir.join("sample.html")).expect("failed to read html output");
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("<title>Sample Title</title>"));
    assert!(html.contains("<h1>Sample Title</h1>"));
    assert!(html.contains("<strong>strong</strong>"));
    assert!(html.contains("<table>"));
    assert!(html.contains("#theme-toggle"));

    fs::remove_dir_all(dir).expect("failed to clean up temp fixture dir");
}

#[test]
fn bare_output_omits_document_shell() {
    let dir = fixture_dir("bare");
    let input = dir.join("fragment.md");
    let output_path = dir.join("fragment.html");
    fs::write(&input, "**bare** fragment").expect("failed to write markdown fixture");

    let output = Command::new(env!("CARGO_BIN_EXE_mdo"))
        .args(["--bare", "--output"])
        .arg(&output_path)
        .arg(&input)
        .output()
        .expect("failed to run mdo");

    assert!(output.status.success(), "mdo failed: {output:?}");

    let html = fs::read_to_string(output_path).expect("failed to read html output");
    assert!(html.contains("<p><strong>bare</strong> fragment</p>"));
    assert!(!html.contains("<!DOCTYPE html>"));
    assert!(!html.contains("<html"));
    assert!(!html.contains("<style>"));

    fs::remove_dir_all(dir).expect("failed to clean up temp fixture dir");
}
