use gpui::*;

mod views;
use views::MainWindow;

fn main() {
    Application::with_platform(gpui_platform::current_platform(false)).run(|cx: &mut App| {
        cx.open_window(WindowOptions::default(), |_window, cx| {
            cx.new(|_cx| MainWindow::new())
        })
        .unwrap();
        cx.activate(true);
    });
}
