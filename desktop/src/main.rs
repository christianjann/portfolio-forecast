use gpui::*;

mod views;
use views::MainWindow;

fn main() {
    Application::with_platform(gpui_platform::current_platform(false)).run(|cx: &mut App| {
        let default_size = Size::new(px(800.), px(600.));
        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(default_size, cx)),
            ..Default::default()
        };

        cx.open_window(window_options, |_window, cx| {
            cx.new(|_cx| MainWindow::new())
        })
        .unwrap();
        cx.activate(true);
    });
}
