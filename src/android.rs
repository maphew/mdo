//! JNI bridge for the Android application.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use jni::objects::{JClass, JString};
use jni::sys::jstring;
use jni::JNIEnv;

use crate::render_markdown_document;

/// Render a Markdown string for `io.github.maphew.mdo.NativeRenderer`.
///
/// JNI failures are surfaced as Java `RuntimeException`s. Panics are caught so
/// they can never unwind across the FFI boundary into Android's runtime.
#[no_mangle]
pub extern "system" fn Java_io_github_maphew_mdo_NativeRenderer_renderMarkdown(
    mut env: JNIEnv,
    _class: JClass,
    markdown: JString,
    fallback_title: JString,
    source_modified_unix_secs: jni::sys::jlong,
) -> jstring {
    let result = (|| -> Result<String, String> {
        let markdown: String = env
            .get_string(&markdown)
            .map_err(|error| format!("could not read Markdown: {error}"))?
            .into();
        let fallback_title: String = env
            .get_string(&fallback_title)
            .map_err(|error| format!("could not read document title: {error}"))?
            .into();
        let modified = u64::try_from(source_modified_unix_secs).ok();

        catch_unwind(AssertUnwindSafe(|| {
            render_markdown_document(&markdown, &fallback_title, modified)
        }))
        .map_err(|_| "mdo renderer panicked".to_string())
    })();

    match result {
        Ok(html) => match env.new_string(html) {
            Ok(value) => value.into_raw(),
            Err(error) => {
                let _ = env.throw_new(
                    "java/lang/RuntimeException",
                    format!("could not return rendered HTML: {error}"),
                );
                ptr::null_mut()
            }
        },
        Err(message) => {
            let _ = env.throw_new("java/lang/RuntimeException", message);
            ptr::null_mut()
        }
    }
}
