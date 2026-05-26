extern crate gpui_mobile;

use gpui::{prelude::*, App, WindowOptions};
use gpui::Application;
use portfolio_forecast_common::views::portfolio_screen::PortfolioScreen;

use gpui_mobile::android::jni;

#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("portfolio-forecast"),
    );

    jni::install_panic_hook();
    log::info!("android_main: entered");

    let _platform = jni::init_platform(&app);
    log::info!("android_main: platform initialised");

    let shared = match jni::shared_platform() {
        Some(s) => s,
        None => {
            log::error!("android_main: shared_platform() returned None — aborting");
            return;
        }
    };

    log::info!("android_main: creating GPUI Application");

    Application::with_platform(shared.into_rc()).run(|cx: &mut App| {
        log::info!("Application::run callback — opening PortfolioScreen");
        open_main_window(cx);
    });

    log::info!("android_main: Application.run returned");
}

fn open_main_window(cx: &mut App) {
    match cx.open_window(
        WindowOptions {
            window_bounds: None,
            ..Default::default()
        },
        |_, cx| cx.new(|_| PortfolioScreen::new()),
    ) {
        Ok(_) => log::info!("PortfolioScreen window opened"),
        Err(e) => log::error!("open_window failed: {e:#}"),
    }
    cx.activate(true);
}
