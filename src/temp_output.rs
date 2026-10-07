//! Temp output: the per-source private location under the OS temp
//! directory where rendered pages are written for opening. Stable per
//! source path; never written next to the source.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};

const UNSAFE_TEMP_OUTPUT_STEM_CHARS: &[char] = &[
    '&', '^', '%', '(', ')', '!', '"', '\'', '<', '>', '|', ';', '`', '$', '\\', '/', ':',
];

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

pub(crate) fn private_temp_root() -> PathBuf {
    let mut p = std::env::temp_dir();
    #[cfg(unix)]
    p.push(format!("mdo-{}", unsafe { libc::geteuid() }));
    #[cfg(not(unix))]
    p.push("mdo");
    p
}

#[cfg(unix)]
pub(crate) fn ensure_private_dir(path: &Path) -> io::Result<()> {
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
pub(crate) fn ensure_private_dir(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

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
}
