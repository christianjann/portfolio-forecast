//! Helpers to request Activity orientation changes via JNI on Android.

#[cfg(target_os = "android")]
mod imp {
    use gpui_mobile::android::jni as jni_helpers;
    use jni::objects::JValue;

    pub fn set_orientation(landscape: bool) -> Result<(), String> {
        jni_helpers::with_env(|env| {
            let activity = jni_helpers::activity(env)?;

            // ActivityInfo constants
            let ai_cls = jni_helpers::find_app_class(env, "android/content/pm/ActivityInfo")?;

            let orient_val = if landscape {
                env.get_static_field(&ai_cls, jni::jni_str!("SCREEN_ORIENTATION_SENSOR_LANDSCAPE"), jni::jni_sig!("I"))
            } else {
                env.get_static_field(&ai_cls, jni::jni_str!("SCREEN_ORIENTATION_SENSOR_PORTRAIT"), jni::jni_sig!("I"))
            }
            .map_err(|e| { env.exception_clear(); e.to_string() })?
            .i()
            .map_err(|e| { env.exception_clear(); e.to_string() })?;

            env.call_method(&activity, jni::jni_str!("setRequestedOrientation"), jni::jni_sig!("(I)V"), &[JValue::Int(orient_val)])
                .map_err(|e| { env.exception_clear(); e.to_string() })?;

            Ok(())
        })
    }

    pub fn is_landscape() -> Result<bool, String> {
        jni_helpers::with_env(|env| {
            let activity = jni_helpers::activity(env)?;

            let resources = env
                .call_method(&activity, jni::jni_str!("getResources"), jni::jni_sig!("()Landroid/content/res/Resources;"), &[])
                .and_then(|v| v.l())
                .map_err(|e| { env.exception_clear(); e.to_string() })?;

            let config = env
                .call_method(&resources, jni::jni_str!("getConfiguration"), jni::jni_sig!("()Landroid/content/res/Configuration;"), &[])
                .and_then(|v| v.l())
                .map_err(|e| { env.exception_clear(); e.to_string() })?;

            let orientation = env
                .get_field(&config, jni::jni_str!("orientation"), jni::jni_sig!("I"))
                .map_err(|e| { env.exception_clear(); e.to_string() })?
                .i()
                .map_err(|e| { env.exception_clear(); e.to_string() })?;

            let cfg_cls = jni_helpers::find_app_class(env, "android/content/res/Configuration")?;
            let orient_land = env
                .get_static_field(&cfg_cls, jni::jni_str!("ORIENTATION_LANDSCAPE"), jni::jni_sig!("I"))
                .map_err(|e| { env.exception_clear(); e.to_string() })?
                .i()
                .map_err(|e| { env.exception_clear(); e.to_string() })?;

            Ok(orientation == orient_land)
        })
    }
}

#[cfg(not(target_os = "android"))]
mod imp {
    pub fn set_orientation(_landscape: bool) -> Result<(), String> {
        Ok(())
    }
}

pub use imp::set_orientation;
#[cfg(target_os = "android")]
pub use imp::is_landscape;
#[cfg(not(target_os = "android"))]
pub fn is_landscape() -> Result<bool, String> { Ok(true) }
