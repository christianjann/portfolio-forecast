extern crate gpui_mobile;

use gpui::{prelude::*, App, WindowOptions};
use portfolio_forecast_common::views::portfolio_screen::PortfolioScreen;

struct NsLogLogger;

impl log::Log for NsLogLogger {
    fn enabled(&self, _: &log::Metadata) -> bool { true }
    fn log(&self, record: &log::Record) {
        let msg = format!("[{}] {}: {}", record.level(), record.target(), record.args());
        nslog(&msg);
    }
    fn flush(&self) {}
}

fn nslog(msg: &str) {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};
    unsafe {
        extern "C" { fn NSLog(fmt: *mut AnyObject, ...); }
        let c_msg = std::ffi::CString::new(msg).unwrap_or_default();
        let ns_msg: *mut AnyObject = msg_send![class!(NSString), alloc];
        let ns_msg: *mut AnyObject = msg_send![ns_msg, initWithUTF8String: c_msg.as_ptr()];
        let c_fmt = std::ffi::CString::new("%@").unwrap_or_default();
        let ns_fmt: *mut AnyObject = msg_send![class!(NSString), alloc];
        let ns_fmt: *mut AnyObject = msg_send![ns_fmt, initWithUTF8String: c_fmt.as_ptr()];
        NSLog(ns_fmt, ns_msg);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn gpui_ios_register_app() {
    let _ = log::set_logger(&NsLogLogger).map(|()| log::set_max_level(log::LevelFilter::Info));
    std::panic::set_hook(Box::new(|info| { nslog(&format!("GPUI PANIC: {info}")); }));
    gpui_mobile::ios::ffi::set_app_callback(Box::new(|cx: &mut App| {
        open_main_window(cx);
    }));
}

pub fn ios_main() {
    gpui_ios_register_app();
    gpui_mobile::ios::ffi::run_app();
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
