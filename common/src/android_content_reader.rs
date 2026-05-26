//! JNI bridge to `GpuiContentReader.readAllBytes()`.
//!
//! On Android, the file picker returns a `content://` URI which cannot be
//! opened with `std::fs`. This module calls back into Java to read the bytes
//! via `ContentResolver.openInputStream()` and returns them as a `Vec<u8>`.

use gpui_mobile::android::jni as jni_helpers;
use jni::objects::{JByteArray, JValue};

/// Read the full contents of a file identified by a content URI.
///
/// Returns `Err` if the JNI call fails; returns `Ok(None)` if Java returned
/// null (e.g. the URI was empty or unreadable); returns `Ok(Some(bytes))` on
/// success.
pub fn read_content_uri(uri: &str) -> Result<Option<Vec<u8>>, String> {
    jni_helpers::with_env(|env| {
        let activity = jni_helpers::activity(env)?;
        let cls = jni_helpers::find_app_class(env, "dev.gpui.mobile.GpuiContentReader")?;

        let j_uri = env.new_string(uri).map_err(|e| e.to_string())?;

        let result = env
            .call_static_method(
                &cls,
                jni::jni_str!("readAllBytes"),
                jni::jni_sig!(
                    "(Landroid/app/Activity;Ljava/lang/String;)[B"
                ),
                &[JValue::Object(&activity), JValue::Object(&j_uri)],
            )
            .and_then(|v| v.l())
            .map_err(|e| {
                env.exception_clear();
                e.to_string()
            })?;

        if result.is_null() {
            return Ok(None);
        }

        // Cast the JObject to a byte array and copy into Vec<u8>.
        let byte_array: JByteArray<'_> = unsafe { JByteArray::from_raw(env, result.as_raw()) };
        let bytes = env
            .convert_byte_array(&byte_array)
            .map_err(|e| e.to_string())?;
        Ok(Some(bytes))
    })
}
