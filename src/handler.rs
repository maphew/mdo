//! The Open-as-HTML handler invocation contract.
//!
//! Every platform surface that registers or forwards Open as HTML spells
//! the same two facts — how a handler launches mdo (`--open` plus the
//! file), and how mdo's binaries are found beside each other (siblings of
//! the running executable). This module owns both; the Linux desktop-entry
//! template, the Windows registry command builders, and the `mdo-open`
//! forwarding binary are thin adapters consuming them, so a change to the
//! invocation is made here once and every platform entry stays in step.
//!
//! The file argument itself is platform presentation: a `%f` placeholder in
//! a desktop entry, `%1` in a Windows registry command string, and the real
//! path when a binary is spawned. Only the argv prefix is the contract.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

/// The flags a handler command puts before the file argument when it
/// launches mdo directly. Spelled once: the desktop-entry template, the
/// Windows registry command, and the forward binary all consume this, so
/// the contract cannot drift between the six places that used to repeat it.
pub const OPEN_FLAGS: &[&str] = &["--open"];

/// The full open-mode argv a handler spawns mdo with: the contract flags
/// (see [`OPEN_FLAGS`]) followed by the files to open, in order.
pub fn open_argv(files: &[OsString]) -> Vec<OsString> {
    let mut argv = Vec::with_capacity(OPEN_FLAGS.len() + files.len());
    argv.extend(OPEN_FLAGS.iter().map(OsString::from));
    argv.extend(files.iter().cloned());
    argv
}

/// Locate a sibling binary beside the running one (or beside whichever
/// executable the caller anchors on). This is the one discovery function
/// and the one error mode for everything mdo ships: the binaries always
/// run from the same directory.
///
/// The error is a `NotFound` io error naming the anchor and the path that
/// was checked, so callers can print it as-is instead of inventing their
/// own "not found" story.
pub fn sibling_binary(exe: &Path, name: &str) -> io::Result<PathBuf> {
    let parent = exe.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "no parent directory for {} (looking for {name})",
                exe.display()
            ),
        )
    })?;
    let sibling = parent.join(name);
    if !sibling.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "no {name} found beside {} (looked at {})",
                exe.display(),
                sibling.display()
            ),
        ));
    }
    Ok(sibling)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture_dir(name: &str) -> PathBuf {
        let uuid = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after Unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "mdo-handler-test-{name}-{}-{uuid}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("failed to create temp fixture dir");
        dir
    }

    #[test]
    fn open_argv_prepends_the_contract_flags_before_the_files() {
        let files = vec![OsString::from("notes.md"), OsString::from("todo.md")];
        let argv = open_argv(&files);
        let argv: Vec<String> = argv
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(argv[0], OPEN_FLAGS[0], "the contract flag comes first");
        assert_eq!(argv[1..], ["notes.md", "todo.md"]);
    }

    #[test]
    fn sibling_binary_resolves_a_real_sibling() {
        let dir = fixture_dir("resolves");
        let anchor = dir.join("mdo");
        fs::write(&anchor, "binary").expect("failed to write anchor fixture");
        let sibling = dir.join("mdo-open");
        fs::write(&sibling, "binary").expect("failed to write sibling fixture");

        let found = sibling_binary(&anchor, "mdo-open")
            .expect("a real sibling beside the anchor should resolve");
        assert_eq!(found, sibling);
    }

    #[test]
    fn sibling_binary_error_names_anchor_and_looked_at_path() {
        let dir = fixture_dir("missing");
        let anchor = dir.join("mdo");
        fs::write(&anchor, "binary").expect("failed to write anchor fixture");

        let err = sibling_binary(&anchor, "mdo-open")
            .expect_err("a missing sibling is the one error mode");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        let message = err.to_string();
        assert!(
            message.contains(&anchor.display().to_string()),
            "error should name the anchor: {message}"
        );
        assert!(
            message.contains(&dir.join("mdo-open").display().to_string()),
            "error should name the path that was checked: {message}"
        );
    }

    #[test]
    fn sibling_binary_rejects_a_directory_named_like_a_binary() {
        let dir = fixture_dir("not-a-file");
        let anchor = dir.join("mdo");
        fs::write(&anchor, "binary").expect("failed to write anchor fixture");
        let dir_sibling = dir.join("mdo-open");
        fs::create_dir_all(&dir_sibling).expect("failed to create decoy dir");

        let err =
            sibling_binary(&anchor, "mdo-open").expect_err("a directory is not a runnable sibling");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }
}
