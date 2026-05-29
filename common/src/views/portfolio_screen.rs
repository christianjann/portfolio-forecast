//! Portfolio screen — chart + 10-year forecast.
//!
//! Ported from `pp-viewer/src/views/main_window.rs` and adapted for mobile:
//!
//! - File opening uses `gpui_mobile::packages::file_selector` (no `rfd`).
//! - On Android the selected content URI is read via `GpuiContentReader`
//!   (see `android_content_reader.rs`) and passed to `crate::load_from_bytes`.
//! - Mouse hover crosshair is replaced by a touch-tap crosshair.
//! - Layout uses larger touch targets and is portrait-optimised.

use chrono::Datelike as _;
use chrono::Months;
use chrono::NaiveDate;
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use crate::analysis::NavPoint;

// ── State ─────────────────────────────────────────────────────────────────

pub struct PortfolioScreen {
    nav_series: Vec<NavPoint>,
    client: Option<crate::PClient>,
    file_name: Option<String>,
    error: Option<String>,
    /// Canvas-local X coordinate of the last tap/touch for the crosshair.
    cursor_x: Option<f32>,
    show_forecast: bool,
    forecast_series: Vec<(f64, f64)>,
    forecast_twr: f64,
    forecast_avg_monthly: f64,
    forecast_months_used: u32,
    /// Current orientation: true = landscape, false = portrait
    is_landscape: bool,
}

struct ChartData {
    points: Vec<(f64, f64)>,
    forecast_points: Vec<(f64, f64)>,
    cursor_x: Option<f32>,
    /// Forecast legend metadata: (annual_twr, avg_monthly, months_used)
    forecast_legend: Option<(f64, f64, u32)>,
}

fn decode_percent_encoded(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = from_hex(bytes[i + 1]);
            let lo = from_hex(bytes[i + 2]);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                out.push((hi << 4) | lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }

    String::from_utf8(out).unwrap_or_else(|_| input.to_string())
}

fn from_hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(10 + (b - b'a')),
        b'A'..=b'F' => Some(10 + (b - b'A')),
        _ => None,
    }
}

impl PortfolioScreen {
    pub fn new() -> Self {
        Self {
            nav_series: Vec::new(),
            client: None,
            file_name: None,
            error: None,
            cursor_x: None,
            show_forecast: false,
            forecast_series: Vec::new(),
            forecast_twr: 0.0,
            forecast_avg_monthly: 0.0,
            forecast_months_used: 0,
                is_landscape: {
                    #[cfg(target_os = "android")]
                    {
                        match crate::android_orientation::is_landscape() {
                            Ok(v) => v,
                            Err(_) => true,
                        }
                    }
                    #[cfg(not(target_os = "android"))]
                    {
                        true
                    }
                },
        }
    }

    fn load_bytes(&mut self, bytes: Vec<u8>, name: String, cx: &mut Context<'_, Self>) {
        match crate::load_from_bytes(&bytes) {
            Ok(client) => {
                self.nav_series = crate::analysis::compute_nav_series(&client);
                self.file_name = Some(name);
                self.client = Some(client);
                self.error = None;
                self.cursor_x = None;
                self.show_forecast = false;
                self.forecast_series.clear();
                self.forecast_twr = 0.0;
                self.forecast_avg_monthly = 0.0;
                self.forecast_months_used = 0;
            }
            Err(e) => {
                self.error = Some(e.to_string());
                self.nav_series.clear();
                self.client = None;
                self.file_name = None;
                self.cursor_x = None;
                self.show_forecast = false;
                self.forecast_series.clear();
                self.forecast_twr = 0.0;
                self.forecast_avg_monthly = 0.0;
                self.forecast_months_used = 0;
            }
        }
        cx.notify();
    }
}

// ── Render ────────────────────────────────────────────────────────────────

impl Render for PortfolioScreen {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let nav_data: Vec<(f64, f64)> = self
            .nav_series
            .iter()
            .map(|p| (p.date.num_days_from_ce() as f64, p.nav))
            .collect();

        let status = match (&self.error, &self.file_name) {
            (Some(e), _) => format!("Error: {}", e),
            (None, Some(name)) => name.clone(),
            (None, None) => "Tap Open, then select your .portfolio file".to_string(),
        };

        let cursor_x = self.cursor_x;
        let show_forecast = self.show_forecast;
        let forecast_data: Vec<(f64, f64)> = self.forecast_series.clone();
        let has_data = !self.nav_series.is_empty();
        let forecast_legend = if show_forecast {
            Some((self.forecast_twr, self.forecast_avg_monthly, self.forecast_months_used))
        } else {
            None
        };

        let toolbar = if self.is_landscape {
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .px_3()
                .py_3()
                .bg(hsla(0.0, 0.0, 0.14, 1.0))
                .child(
                    div()
                        .id("open-btn")
                        .px_5()
                        .py_3()
                        .bg(hsla(0.6, 0.7, 0.45, 1.0))
                        .rounded_lg()
                        .cursor_pointer()
                        .text_color(hsla(0.0, 0.0, 1.0, 1.0))
                        .child("Open")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _event, _window, cx| {
                                pick_and_load(this, cx);
                            }),
                        ),
                )
                .when(has_data, |row| {
                    row.child(
                        div()
                            .id("forecast-btn")
                            .px_4()
                            .py_3()
                            .bg(hsla(0.08, 0.7, 0.45, 1.0))
                            .rounded_lg()
                            .cursor_pointer()
                            .text_color(hsla(0.0, 0.0, 1.0, 1.0))
                            .child(if show_forecast { "Hide Forecast" } else { "Forecast" })
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _event, _window, cx| {
                                    this.show_forecast = !this.show_forecast;
                                    if this.show_forecast {
                                        if let Some(client) = &this.client {
                                            let res = compute_forecast(client, &this.nav_series);
                                            this.forecast_series      = res.series;
                                            this.forecast_twr         = res.annual_twr;
                                            this.forecast_avg_monthly = res.avg_monthly;
                                            this.forecast_months_used = res.months_used;
                                        }
                                    } else {
                                        this.forecast_series.clear();
                                        this.forecast_twr         = 0.0;
                                        this.forecast_avg_monthly = 0.0;
                                        this.forecast_months_used = 0;
                                    }
                                    cx.notify();
                                }),
                            ),
                    )
                })
                .child(
                    div()
                        .id("orient-btn")
                        .px_3()
                        .py_3()
                        .bg(hsla(0.55, 0.7, 0.45, 1.0))
                        .rounded_lg()
                        .cursor_pointer()
                        .text_color(hsla(0.0, 0.0, 1.0, 1.0))
                        .child(if self.is_landscape { "Portrait" } else { "Landscape" })
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _event, _window, cx| {
                                this.is_landscape = !this.is_landscape;
                                this.cursor_x = None;
                                #[cfg(target_os = "android")]
                                {
                                    let _ = crate::android_orientation::set_orientation(this.is_landscape);
                                }
                                cx.notify();
                            }),
                        ),
                )
                .child(
                    div()
                        .flex_1()
                        .text_color(hsla(0.0, 0.0, 0.6, 1.0))
                        .text_sm()
                        .overflow_hidden()
                        .child(status),
                )
        } else {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .px_3()
                .py_3()
                .bg(hsla(0.0, 0.0, 0.14, 1.0))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .id("open-btn")
                                .px_5()
                                .py_3()
                                .bg(hsla(0.6, 0.7, 0.45, 1.0))
                                .rounded_lg()
                                .cursor_pointer()
                                .text_color(hsla(0.0, 0.0, 1.0, 1.0))
                                .child("Open")
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _event, _window, cx| {
                                        pick_and_load(this, cx);
                                    }),
                                ),
                        )
                        .when(has_data, |row| {
                            row.child(
                                div()
                                    .id("forecast-btn")
                                    .px_4()
                                    .py_3()
                                    .bg(hsla(0.08, 0.7, 0.45, 1.0))
                                    .rounded_lg()
                                    .cursor_pointer()
                                    .text_color(hsla(0.0, 0.0, 1.0, 1.0))
                                    .child(if show_forecast { "Hide Forecast" } else { "Forecast" })
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _event, _window, cx| {
                                            this.show_forecast = !this.show_forecast;
                                            if this.show_forecast {
                                                if let Some(client) = &this.client {
                                                    let res = compute_forecast(client, &this.nav_series);
                                                    this.forecast_series      = res.series;
                                                    this.forecast_twr         = res.annual_twr;
                                                    this.forecast_avg_monthly = res.avg_monthly;
                                                    this.forecast_months_used = res.months_used;
                                                }
                                            } else {
                                                this.forecast_series.clear();
                                                this.forecast_twr         = 0.0;
                                                this.forecast_avg_monthly = 0.0;
                                                this.forecast_months_used = 0;
                                            }
                                            cx.notify();
                                        }),
                                    ),
                            )
                        })
                        .child(
                            div()
                                .id("orient-btn")
                                .px_3()
                                .py_3()
                                .bg(hsla(0.55, 0.7, 0.45, 1.0))
                                .rounded_lg()
                                .cursor_pointer()
                                .text_color(hsla(0.0, 0.0, 1.0, 1.0))
                                .child(if self.is_landscape { "Portrait" } else { "Landscape" })
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _event, _window, cx| {
                                        this.is_landscape = !this.is_landscape;
                                        this.cursor_x = None;
                                        #[cfg(target_os = "android")]
                                        {
                                            let _ = crate::android_orientation::set_orientation(this.is_landscape);
                                        }
                                        cx.notify();
                                    }),
                                ),
                        ),
                )
                .child(
                    div()
                        .w_full()
                        .text_color(hsla(0.0, 0.0, 0.6, 1.0))
                        .text_sm()
                        .overflow_hidden()
                        .child(status),
                )
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(hsla(0.0, 0.0, 0.08, 1.0))
            // ── Chart area ───────────────────────────────────────────────
            .child(
                div()
                    .id("chart-area")
                    .flex_1()
                    .w_full()
                    // Tap to move the crosshair
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, _window, cx| {
                            let x = f32::from(event.position.x);
                            this.cursor_x = Some(x);
                            cx.notify();
                        }),
                    )
                    .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _window, cx| {
                        let x = f32::from(event.position.x);
                        if this.cursor_x != Some(x) {
                            this.cursor_x = Some(x);
                            cx.notify();
                        }
                    }))
                    .child(
                        canvas(
                            move |bounds, _window, _cx| {
                                let ox = f32::from(bounds.origin.x);
                                ChartData {
                                    points: nav_data,
                                    forecast_points: forecast_data,
                                    cursor_x: cursor_x.map(|cx| cx - ox),
                                    forecast_legend,
                                }
                            },
                            |bounds, data, window, cx| {
                                paint_nav_chart(bounds, data, window, cx);
                            },
                        )
                        .size_full(),
                    ),
            )
            // ── Toolbar ─────────────────────────────────────────────────
            .child(toolbar)
    }
}

// ── File picker ───────────────────────────────────────────────────────────

fn pick_and_load(_this: &mut PortfolioScreen, cx: &mut Context<'_, PortfolioScreen>) {
    use gpui_mobile::packages::file_selector::{open_file, OpenFileOptions};

    log::info!("Open button tapped; invoking file selector");

    let opts = OpenFileOptions {
        // Android's SAF doesn't reliably handle custom extensions such as
        // `.portfolio` through MIME filtering; use */* so the picker opens.
        accept_type_groups: vec![],
        initial_directory: None,
    };

    // IMPORTANT: `open_file` blocks the calling thread waiting for the user
    // to interact with the system picker.  On Android, the android-activity
    // native thread (where all GPUI callbacks run) must remain unblocked so
    // that it can process the `onPause` lifecycle event that Android fires
    // when the picker activity comes to the foreground.  Blocking the native
    // thread before that event is processed causes a deadlock / ANR.
    //
    // Solution: run the blocking picker + content-read on a background
    // executor thread, then deliver the result back to the foreground.
    cx.spawn(async |this, cx| {
        // Run the blocking picker (and content reading) on a background thread.
        let outcome: Result<Option<(Vec<u8>, String)>, String> =
            cx.background_executor()
                .spawn(async move {
                    let selected = match open_file(&opts) {
                        Ok(Some(s)) => s,
                        Ok(None) => {
                            log::info!("File selector cancelled or returned no file");
                            return Ok(None);
                        }
                        Err(e) => {
                            log::error!("File selector failed: {e}");
                            return Err(format!("File picker error: {e}"));
                        }
                    };

                    log::info!("File selector returned: {}", selected.path);
                    let name = decode_percent_encoded(&selected.name);
                    let uri = selected.path.clone();

                    // On Android the path is a content:// URI; read bytes via JNI.
                    #[cfg(target_os = "android")]
                    {
                        match crate::android_content_reader::read_content_uri(&uri) {
                            Ok(Some(bytes)) => Ok(Some((bytes, name))),
                            Ok(None) => {
                                log::warn!("Content reader returned no bytes for URI: {uri}");
                                Err(format!("Could not read file: {uri}"))
                            }
                            Err(e) => {
                                log::error!("JNI content reader error: {e}");
                                Err(format!("JNI error reading file: {e}"))
                            }
                        }
                    }

                    // On iOS the path is a real file-system path.
                    #[cfg(target_os = "ios")]
                    {
                        match std::fs::read(&uri) {
                            Ok(bytes) => Ok(Some((bytes, name))),
                            Err(e) => Err(format!("Could not read file: {e}")),
                        }
                    }

                    // Fallback for host builds (cargo check on Linux/macOS).
                    #[cfg(not(any(target_os = "android", target_os = "ios")))]
                    {
                        let _ = (uri, name);
                        Err("File selection not supported on this platform".into())
                    }
                })
                .await;

        // Back on the foreground thread — update the model.
        let _ = this.update(cx, |this, cx| match outcome {
            Ok(Some((bytes, name))) => this.load_bytes(bytes, name, cx),
            Ok(None) => {}
            Err(e) => {
                this.error = Some(e);
                cx.notify();
            }
        });
    })
    .detach();
}

// ── Chart layout constants ────────────────────────────────────────────────

/// Left margin (Y-axis labels).
const ML: f64 = 72.0;
/// Right margin.
const MR: f64 = 12.0;
/// Top margin. Reserve extra space on mobile devices with display cutouts / notches.
#[cfg(any(target_os = "android", target_os = "ios"))]
const MT: f64 = 44.0;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
const MT: f64 = 12.0;
/// Bottom margin (X-axis labels).
const MB: f64 = 28.0;

// ── Paint ─────────────────────────────────────────────────────────────────

fn paint_nav_chart(bounds: Bounds<Pixels>, data: ChartData, window: &mut Window, cx: &mut App) {
    window.paint_quad(fill(bounds, hsla(0.0, 0.0, 0.05, 1.0)));

    let points = &data.points;
    if points.len() < 2 {
        return;
    }

    let ox = f64::from(bounds.origin.x);
    let oy = f64::from(bounds.origin.y);
    let w  = f64::from(bounds.size.width);
    let h  = f64::from(bounds.size.height);

    let cl = ox + ML;
    let ct = oy + MT;
    let cw = (w - ML - MR).max(1.0);
    let ch = (h - MT - MB).max(1.0);

    let forecast_pts = &data.forecast_points;
    let has_forecast  = !forecast_pts.is_empty();

    let min_nav      = points.iter().map(|p| p.1).fold(f64::INFINITY,     f64::min);
    let max_nav_hist = points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
    let max_nav = if has_forecast {
        forecast_pts.iter().map(|p| p.1).fold(max_nav_hist, f64::max)
    } else {
        max_nav_hist
    };
    let min_day      = points.first().unwrap().0;
    let max_day_hist = points.last().unwrap().0;
    let max_day = if has_forecast {
        forecast_pts.last().unwrap().0
    } else {
        max_day_hist
    };

    let nav_range = (max_nav - min_nav).max(1.0);
    let day_range = (max_day - min_day).max(1.0);

    let xd = |day: f64| -> f32 { (cl + cw * (day - min_day) / day_range) as f32 };
    let yn = |nav: f64| -> f32 { (ct + ch - ch * (nav - min_nav) / nav_range) as f32 };

    let y_ticks = nice_ticks(min_nav, max_nav, 6);
    let x_ticks = time_ticks(min_day, max_day, 6);

    // ── Grid lines ──────────────────────────────────────────────────────
    let grid_col = hsla(0.0, 0.0, 0.22, 1.0);
    for &t in &y_ticks {
        let y = yn(t);
        stroke_line(window, cl as f32, y, (cl + cw) as f32, y, 1.0, grid_col);
    }
    for &t in &x_ticks {
        let x = xd(t);
        stroke_line(window, x, ct as f32, x, (ct + ch) as f32, 1.0, grid_col);
    }

    // ── Area fill ────────────────────────────────────────────────────────
    let n = points.len();
    let bottom_y = (ct + ch) as f32;
    {
        let mut pb = PathBuilder::fill();
        pb.move_to(point(px(xd(points[0].0)), px(bottom_y)));
        pb.line_to(point(px(xd(points[0].0)), px(yn(points[0].1))));
        let mut prev_y = yn(points[0].1);
        for p in &points[1..] {
            pb.line_to(point(px(xd(p.0)), px(prev_y)));   // horizontal step
            pb.line_to(point(px(xd(p.0)), px(yn(p.1)))); // vertical jump
            prev_y = yn(p.1);
        }
        pb.line_to(point(px(xd(points[n - 1].0)), px(bottom_y)));
        pb.close();
        if let Ok(path) = pb.build() {
            window.paint_path(path, hsla(0.55, 0.7, 0.5, 0.3));
        }
    }

    // ── NAV line ─────────────────────────────────────────────────────────
    {
        let mut pb = PathBuilder::stroke(px(1.5));
        pb.move_to(point(px(xd(points[0].0)), px(yn(points[0].1))));
        let mut prev_y = yn(points[0].1);
        for p in &points[1..] {
            pb.line_to(point(px(xd(p.0)), px(prev_y)));   // horizontal step
            pb.line_to(point(px(xd(p.0)), px(yn(p.1)))); // vertical jump
            prev_y = yn(p.1);
        }
        if let Ok(path) = pb.build() {
            window.paint_path(path, hsla(0.55, 0.8, 0.62, 1.0));
        }
    }

    // ── Forecast area + line ─────────────────────────────────────────────
    if has_forecast {
        let fn_       = forecast_pts.len();
        let last_hist = points.last().unwrap();
        {
            let mut pb = PathBuilder::fill();
            pb.move_to(point(px(xd(last_hist.0)), px(bottom_y)));
            pb.line_to(point(px(xd(last_hist.0)), px(yn(last_hist.1))));
            for p in forecast_pts.iter() {
                pb.line_to(point(px(xd(p.0)), px(yn(p.1))));
            }
            pb.line_to(point(px(xd(forecast_pts[fn_ - 1].0)), px(bottom_y)));
            pb.close();
            if let Ok(path) = pb.build() {
                window.paint_path(path, hsla(0.08, 0.75, 0.55, 0.20));
            }
        }
        {
            let mut pb = PathBuilder::stroke(px(1.5));
            pb.move_to(point(px(xd(last_hist.0)), px(yn(last_hist.1))));
            for p in forecast_pts.iter() {
                pb.line_to(point(px(xd(p.0)), px(yn(p.1))));
            }
            if let Ok(path) = pb.build() {
                window.paint_path(path, hsla(0.08, 0.90, 0.65, 1.0));
            }
        }
        // "Today" divider
        stroke_line(
            window, xd(max_day_hist), ct as f32, xd(max_day_hist), (ct + ch) as f32,
            1.0, hsla(0.0, 0.0, 0.55, 0.6),
        );

        // ── Milestones ────────────────────────────────────────────────────
        const MILESTONES: &[f64] = &[
            10_000.0, 100_000.0, 500_000.0,
            1_000_000.0, 2_000_000.0, 5_000_000.0, 10_000_000.0,
        ];
        let current_nav = points.last().unwrap().1;
        let m_col = hsla(0.08, 0.90, 0.65, 1.0);
        let ts_m = window.text_system().clone();
        for &milestone in MILESTONES {
            if current_nav >= milestone { continue; }
            if milestone > max_nav { continue; }
            if let Some(crossing_day) = find_forecast_crossing(forecast_pts, milestone) {
                let mx = xd(crossing_day);
                let my = yn(milestone);
                let r  = px(4.5f32);
                let mx_px = px(mx);
                let my_px = px(my);
                // Filled circle
                let mut pb = PathBuilder::fill();
                pb.move_to(point(mx_px + r, my_px));
                pb.arc_to(point(r, r), px(0.), false, false, point(mx_px - r, my_px));
                pb.arc_to(point(r, r), px(0.), false, false, point(mx_px + r, my_px));
                pb.close();
                if let Ok(path) = pb.build() {
                    window.paint_path(path, m_col);
                }
                // Label
                let lbl_date = fmt_date(crossing_day);
                let lbl_val  = fmt_nav(milestone);
                let lbl_w = 52.0f32;
                let lbl_h = 30.0f32;
                let lbl_x = if mx + lbl_w + 6.0 < (cl + cw) as f32 {
                    mx + 6.0
                } else {
                    mx - lbl_w - 6.0
                };
                let lbl_y = my - lbl_h / 2.0;
                window.paint_quad(fill(
                    Bounds {
                        origin: point(px(lbl_x), px(lbl_y)),
                        size:   size(px(lbl_w), px(lbl_h)),
                    },
                    hsla(0.0, 0.0, 0.10, 0.85),
                ));
                let s1 = ts_m.shape_line(
                    lbl_date.clone().into(), px(9.0),
                    &[TextRun { len: lbl_date.len(), font: font(".SystemUIFont"),
                        color: m_col, background_color: None,
                        underline: None, strikethrough: None }],
                    None,
                );
                s1.paint(point(px(lbl_x + 4.0), px(lbl_y + 3.0)), px(11.0), TextAlign::Left, None, window, cx).ok();
                let s2 = ts_m.shape_line(
                    lbl_val.clone().into(), px(9.0),
                    &[TextRun { len: lbl_val.len(), font: font(".SystemUIFont"),
                        color: m_col, background_color: None,
                        underline: None, strikethrough: None }],
                    None,
                );
                s2.paint(point(px(lbl_x + 4.0), px(lbl_y + 16.0)), px(11.0), TextAlign::Left, None, window, cx).ok();
            }
        }
    }

    // ── Forecast legend ───────────────────────────────────────────────────
    if let Some((twr, avg_m, months)) = data.forecast_legend {
        // Place legend at bottom of forecast area to avoid overlapping the
        // forecast curve and milestone markers on small touch screens.
        let leg_w = 182.0f32;
        let leg_h = 58.0f32;
        let leg_col = hsla(0.08, 0.9, 0.65, 1.0);

        if has_forecast {
            let f_start = xd(max_day_hist);
            let f_end = xd(max_day);
            let f_width = f_end - f_start;

            let leg_x = if f_width > leg_w + 12.0 {
                f_end - leg_w - 6.0
            } else {
                f_start + (f_width - leg_w) / 2.0
            };
            let leg_y = (ct + ch - leg_h as f64 - 8.0) as f32;

            window.paint_quad(fill(
                Bounds {
                    origin: point(px(leg_x), px(leg_y)),
                    size:   size(px(leg_w), px(leg_h)),
                },
                hsla(0.0, 0.0, 0.14, 0.92),
            ));

            let sign = |v: f64| if v >= 0.0 { "+" } else { "-" };
            let leg_lines = [
                format!("TTWROR    {:+.1} % p.a.", twr * 100.0),
                format!("Monthly   {}{}", sign(avg_m), fmt_nav(avg_m.abs())),
                format!("Period    {} months", months),
            ];
            let ts_leg = window.text_system().clone();
            for (i, line) in leg_lines.iter().enumerate() {
                let shaped = ts_leg.shape_line(
                    line.clone().into(),
                    px(10.5),
                    &[TextRun {
                        len: line.len(),
                        font: font(".SystemUIFont"),
                        color: leg_col,
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    }],
                    None,
                );
                let ly = leg_y + 8.0 + i as f32 * 17.0;
                shaped.paint(point(px(leg_x + 8.0), px(ly)), px(13.0), TextAlign::Left, None, window, cx).ok();
            }
        } else {
            let leg_x = (cl + cw - leg_w as f64 - 6.0) as f32;
            let leg_y = (ct + 8.0) as f32;

            window.paint_quad(fill(
                Bounds {
                    origin: point(px(leg_x), px(leg_y)),
                    size:   size(px(leg_w), px(leg_h)),
                },
                hsla(0.0, 0.0, 0.14, 0.92),
            ));

            let sign = |v: f64| if v >= 0.0 { "+" } else { "-" };
            let leg_lines = [
                format!("TTWROR    {:+.1} % p.a.", twr * 100.0),
                format!("Monthly   {}{}", sign(avg_m), fmt_nav(avg_m.abs())),
                format!("Period    {} months", months),
            ];
            let ts_leg = window.text_system().clone();
            for (i, line) in leg_lines.iter().enumerate() {
                let shaped = ts_leg.shape_line(
                    line.clone().into(),
                    px(10.5),
                    &[TextRun {
                        len: line.len(),
                        font: font(".SystemUIFont"),
                        color: leg_col,
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    }],
                    None,
                );
                let ly = leg_y + 8.0 + i as f32 * 17.0;
                shaped.paint(point(px(leg_x + 8.0), px(ly)), px(13.0), TextAlign::Left, None, window, cx).ok();
            }
        }
    }

    // ── Y-axis labels ────────────────────────────────────────────────────
    let ts = window.text_system().clone();
    let label_col = hsla(0.0, 0.0, 0.52, 1.0);

    for &t in &y_ticks {
        let label = fmt_nav(t);
        let shaped = ts.shape_line(
            label.clone().into(),
            px(10.5),
            &[TextRun {
                len: label.len(),
                font: font(".SystemUIFont"),
                color: label_col,
                background_color: None,
                underline: None,
                strikethrough: None,
            }],
            None,
        );
        shaped
            .paint(
                point(px((ox + 2.0) as f32), px(yn(t) - 6.0)),
                px(13.0),
                TextAlign::Left,
                None,
                window,
                cx,
            )
            .ok();
    }

    // ── X-axis labels ────────────────────────────────────────────────────
    for &t in &x_ticks {
        let label = fmt_date(t);
        let shaped = ts.shape_line(
            label.clone().into(),
            px(10.0),
            &[TextRun {
                len: label.len(),
                font: font(".SystemUIFont"),
                color: label_col,
                background_color: None,
                underline: None,
                strikethrough: None,
            }],
            None,
        );
        let x = xd(t) - 22.0;
        let y = (oy + h - MB + 7.0) as f32;
        shaped
            .paint(point(px(x), px(y)), px(13.0), TextAlign::Left, None, window, cx)
            .ok();
    }

    // ── Crosshair + tooltip ───────────────────────────────────────────────
    if let Some(cx_local) = data.cursor_x {
        if (cx_local as f64) >= ML && (cx_local as f64) <= ML + cw {
            let t_day       = min_day + (cx_local as f64 - ML) / cw * day_range;
            let in_forecast = has_forecast && t_day > max_day_hist;
            let (nav_cur, day_cur) = if !in_forecast {
                let i1 = points.partition_point(|p| p.0 <= t_day).min(n - 1);
                if i1 == 0 {
                    (points[0].1, points[0].0)
                } else {
                    let i0   = i1 - 1;
                    let span = points[i1].0 - points[i0].0;
                    let f    = if span > 0.0 { (t_day - points[i0].0) / span } else { 0.0 };
                    (points[i0].1 * (1.0 - f) + points[i1].1 * f, t_day)
                }
            } else {
                let fn_  = forecast_pts.len();
                let i1   = forecast_pts.partition_point(|p| p.0 <= t_day).min(fn_ - 1);
                if i1 == 0 {
                    (forecast_pts[0].1, forecast_pts[0].0)
                } else {
                    let i0   = i1 - 1;
                    let span = forecast_pts[i1].0 - forecast_pts[i0].0;
                    let f    = if span > 0.0 { (t_day - forecast_pts[i0].0) / span } else { 0.0 };
                    (forecast_pts[i0].1 * (1.0 - f) + forecast_pts[i1].1 * f, t_day)
                }
            };
            let cx_raw  = ox as f32 + cx_local;
            let cy      = yn(nav_cur);
            let cur_col = hsla(0.0, 0.0, 0.8, 0.7);

            stroke_line(window, cx_raw, ct as f32, cx_raw, (ct + ch) as f32, 1.0, cur_col);
            stroke_line(window, cl as f32, cy, (cl + cw) as f32, cy, 1.0, cur_col);

            let dot     = 7.0f32; // slightly larger for touch
            let dot_col = if in_forecast { hsla(0.08, 0.9, 0.65, 1.0) } else { hsla(0.0, 0.0, 0.95, 1.0) };
            window.paint_quad(fill(
                Bounds {
                    origin: point(px(cx_raw - dot / 2.0), px(cy - dot / 2.0)),
                    size:   size(px(dot), px(dot)),
                },
                dot_col,
            ));

            let tip_text = format!("{}   {}", fmt_date(day_cur), fmt_nav(nav_cur));
            let shaped = ts.shape_line(
                tip_text.clone().into(),
                px(11.0),
                &[TextRun {
                    len: tip_text.len(),
                    font: font(".SystemUIFont"),
                    color: hsla(0.0, 0.0, 0.92, 1.0),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                }],
                None,
            );
            let tip_w = 148.0f32;
            let tip_h = 22.0f32;
            let tip_x = if cx_raw + tip_w + 10.0 < (cl + cw) as f32 {
                cx_raw + 10.0
            } else {
                cx_raw - tip_w - 10.0
            };
            let tip_y = (ct + 10.0) as f32;
            window.paint_quad(fill(
                Bounds {
                    origin: point(px(tip_x), px(tip_y)),
                    size:   size(px(tip_w), px(tip_h)),
                },
                hsla(0.0, 0.0, 0.18, 0.92),
            ));
            shaped
                .paint(
                    point(px(tip_x + 6.0), px(tip_y + 4.0)),
                    px(14.0),
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                )
                .ok();
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn stroke_line(window: &mut Window, x0: f32, y0: f32, x1: f32, y1: f32, w: f32, color: Hsla) {
    let mut pb = PathBuilder::stroke(px(w));
    pb.move_to(point(px(x0), px(y0)));
    pb.line_to(point(px(x1), px(y1)));
    if let Ok(path) = pb.build() {
        window.paint_path(path, color);
    }
}

fn find_forecast_crossing(pts: &[(f64, f64)], target: f64) -> Option<f64> {
    let i1 = pts.iter().position(|p| p.1 >= target)?;
    if i1 == 0 {
        return Some(pts[0].0);
    }
    let (d0, v0) = pts[i1 - 1];
    let (d1, v1) = pts[i1];
    let span = v1 - v0;
    if span.abs() < 1e-10 {
        return Some(d0);
    }
    Some(d0 + (target - v0) / span * (d1 - d0))
}

fn nice_ticks(min: f64, max: f64, target: usize) -> Vec<f64> {
    let range = (max - min).max(1.0);
    let rough = range / target as f64;
    let mag   = 10f64.powf(rough.log10().floor());
    let step  = if rough / mag <= 1.0      { mag }
                else if rough / mag <= 2.0 { 2.0 * mag }
                else if rough / mag <= 5.0 { 5.0 * mag }
                else                       { 10.0 * mag };
    let start = (min / step).ceil() * step;
    let mut ticks = Vec::new();
    let mut t = start;
    while t <= max + step * 0.01 {
        ticks.push(t);
        t += step;
    }
    ticks
}

fn time_ticks(min_day: f64, max_day: f64, target: usize) -> Vec<f64> {
    let d_start = NaiveDate::from_num_days_from_ce_opt(min_day as i32)
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());
    let d_end = NaiveDate::from_num_days_from_ce_opt(max_day as i32)
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

    let total_months = (d_end.year() - d_start.year()) * 12
        + d_end.month() as i32
        - d_start.month() as i32;

    let month_step: i32 = {
        let rough = (total_months / target as i32).max(1);
        if rough <= 1       { 1 }
        else if rough <= 3  { 3 }
        else if rough <= 6  { 6 }
        else if rough <= 12 { 12 }
        else if rough <= 24 { 24 }
        else if rough <= 60 { 60 }
        else                { 120 }
    };

    let mut y = d_start.year();
    let mut m = d_start.month() as i32;
    m = ((m - 1 + month_step) / month_step) * month_step + 1 - month_step;
    loop {
        while m > 12 { m -= 12; y += 1; }
        if let Some(d) = NaiveDate::from_ymd_opt(y, m as u32, 1) {
            if d >= d_start { break; }
        }
        m += month_step;
    }

    let mut ticks = Vec::new();
    loop {
        while m > 12 { m -= 12; y += 1; }
        match NaiveDate::from_ymd_opt(y, m as u32, 1) {
            Some(d) if d <= d_end => {
                ticks.push(d.num_days_from_ce() as f64);
                m += month_step;
            }
            _ => break,
        }
    }
    ticks
}

fn fmt_nav(v: f64) -> String {
    if v.abs() >= 1_000_000.0 {
        format!("{:.2}M", v / 1_000_000.0)
    } else if v.abs() >= 10_000.0 {
        format!("{:.0}k", v / 1_000.0)
    } else if v.abs() >= 1_000.0 {
        format!("{:.1}k", v / 1_000.0)
    } else {
        format!("{:.1}", v)
    }
}

fn fmt_date(days: f64) -> String {
    NaiveDate::from_num_days_from_ce_opt(days as i32)
        .map(|d| d.format("%Y-%m").to_string())
        .unwrap_or_default()
}

// ── Forecast ──────────────────────────────────────────────────────────────

struct ForecastResult {
    series:       Vec<(f64, f64)>,
    annual_twr:   f64,
    avg_monthly:  f64,
    months_used:  u32,
}

fn compute_forecast(client: &crate::PClient, nav_series: &[NavPoint]) -> ForecastResult {
    if nav_series.is_empty() {
        return ForecastResult { series: Vec::new(), annual_twr: 0.0, avg_monthly: 0.0, months_used: 0 };
    }

    let last      = nav_series.last().unwrap();
    let last_nav  = last.nav;
    let last_date = last.date;
    let last_secs = date_to_unix_secs(last_date);

    // Dynamic cutoff: last 5 years (60 months) if history is long enough, else all available months.
    let first_date   = nav_series.first().unwrap().date;
    let total_months = ((last_date.year() - first_date.year()) * 12
        + last_date.month() as i32 - first_date.month() as i32).max(1) as u32;
    let months_used  = total_months.min(60);
    let cutoff_secs  = last_date
        .checked_sub_months(Months::new(months_used))
        .map(date_to_unix_secs)
        .unwrap_or(i64::MIN);

    // Average monthly net capital addition over the window period.
    // Inflows:  DEPOSIT (6), DIVIDEND (8), INTEREST (9), INBOUND_DELIVERY (2)
    // Outflows: REMOVAL (7), OUTBOUND_DELIVERY (3)
    let mut inflow  = 0.0_f64;
    let mut outflow = 0.0_f64;
    for t in &client.transactions {
        let t_secs = match &t.date { Some(ts) => ts.seconds, None => continue };
        if t_secs > last_secs || t_secs < cutoff_secs { continue; }
        let amount = t.amount as f64 / 100.0;
        match t.r#type {
            6 | 8 | 9 | 2 => inflow  += amount,
            7 | 3          => outflow += amount,
            _ => {}
        }
    }
    let avg_monthly = (inflow - outflow) / months_used as f64;

    let r = compute_twr_rate(client).clamp(-0.50, 1.50);
    let r_monthly = (1.0 + r).powf(1.0 / 12.0) - 1.0;
    let mut result = Vec::with_capacity(120);
    let mut date   = last_date;

    for k in 1_u32..=120 {
        date = match date.checked_add_months(Months::new(1)) {
            Some(d) => d,
            None    => break,
        };
        let growth  = (1.0 + r).powf(k as f64 / 12.0);
        let annuity = if r_monthly.abs() > 1e-10 {
            avg_monthly * ((1.0 + r_monthly).powi(k as i32) - 1.0) / r_monthly
        } else {
            avg_monthly * k as f64
        };
        result.push((date.num_days_from_ce() as f64, last_nav * growth + annuity));
    }

    ForecastResult { series: result, annual_twr: r, avg_monthly, months_used }
}

fn compute_twr_rate(client: &crate::PClient) -> f64 {
    use std::collections::HashMap;

    let price_map = build_price_map(client);

    let mut sorted: Vec<_> = client.transactions.iter().collect();
    sorted.sort_by_key(|t| t.date.as_ref().map(|d| d.seconds).unwrap_or(0));

    if sorted.len() < 2 {
        return 0.07;
    }

    let first_secs = sorted.first().unwrap().date.as_ref().map(|d| d.seconds).unwrap_or(0);
    let last_tx_secs = sorted.last().unwrap().date.as_ref().map(|d| d.seconds).unwrap_or(0);
    // Use the furthest valuation horizon available, not just the last transaction.
    // This mirrors Portfolio Performance behavior better when prices continue after
    // the final cash-flow event.
    let last_price_day = price_map
        .values()
        .filter_map(|prices| prices.last().map(|(day, _)| *day))
        .max()
        .unwrap_or(last_tx_secs / 86_400);
    let end_secs = last_tx_secs.max(last_price_day * 86_400);
    let total_years = ((end_secs - first_secs) as f64 / 86_400.0 / 365.25).max(0.5);

    let mut cash:      HashMap<String, f64> = HashMap::new();
    let mut positions: HashMap<String, HashMap<String, f64>> = HashMap::new();

    let mut twr_product      = 1.0_f64;
    let mut period_start_val: Option<f64> = None;
    let mut sub_periods      = 0u32;

    for t in &sorted {
        let t_secs    = match &t.date { Some(ts) => ts.seconds, None => continue };
        let epoch_day = t_secs / 86_400;
        let amount    = t.amount as f64 / 100.0;
        let shares    = t.shares.unwrap_or(0) as f64 / 1e8;

        // Sub-period boundary at every external cash flow:
        // DEPOSIT (6), REMOVAL (7) — cash in/out
        // INBOUND_DELIVERY (2), OUTBOUND_DELIVERY (3) — securities in/out without cash
        if matches!(t.r#type, 2 | 3 | 6 | 7) {
            let nav_before = cash.values().sum::<f64>()
                + sec_val_at(&positions, &price_map, epoch_day);
            if let Some(psv) = period_start_val {
                if psv > 0.0 {
                    twr_product *= nav_before / psv;
                    sub_periods += 1;
                }
            }
        }

        match t.r#type {
            6 | 8 | 9 | 12 | 14 => {
                if let Some(acc) = &t.account {
                    *cash.entry(acc.clone()).or_default() += amount;
                }
            }
            7 | 10 | 11 | 13 => {
                if let Some(acc) = &t.account {
                    *cash.entry(acc.clone()).or_default() -= amount;
                }
            }
            0 => {
                if let Some(acc) = &t.account {
                    *cash.entry(acc.clone()).or_default() -= amount;
                }
                if let (Some(port), Some(sec)) = (&t.portfolio, &t.security) {
                    *positions.entry(port.clone()).or_default()
                        .entry(sec.clone()).or_default() += shares;
                }
            }
            1 => {
                if let Some(acc) = &t.account {
                    *cash.entry(acc.clone()).or_default() += amount;
                }
                if let (Some(port), Some(sec)) = (&t.portfolio, &t.security) {
                    *positions.entry(port.clone()).or_default()
                        .entry(sec.clone()).or_default() -= shares;
                }
            }
            2 => {
                if let (Some(port), Some(sec)) = (&t.portfolio, &t.security) {
                    *positions.entry(port.clone()).or_default()
                        .entry(sec.clone()).or_default() += shares;
                }
            }
            3 => {
                if let (Some(port), Some(sec)) = (&t.portfolio, &t.security) {
                    *positions.entry(port.clone()).or_default()
                        .entry(sec.clone()).or_default() -= shares;
                }
            }
            4 => {
                if let (Some(port), Some(sec)) = (&t.portfolio, &t.security) {
                    *positions.entry(port.clone()).or_default()
                        .entry(sec.clone()).or_default() -= shares;
                }
                if let (Some(op), Some(sec)) = (&t.other_portfolio, &t.security) {
                    *positions.entry(op.clone()).or_default()
                        .entry(sec.clone()).or_default() += shares;
                }
            }
            5 => {
                if let Some(acc) = &t.account {
                    *cash.entry(acc.clone()).or_default() -= amount;
                }
                if let Some(oth) = &t.other_account {
                    *cash.entry(oth.clone()).or_default() += amount;
                }
            }
            _ => {}
        }

        if matches!(t.r#type, 2 | 3 | 6 | 7) {
            let nav_after = cash.values().sum::<f64>()
                + sec_val_at(&positions, &price_map, epoch_day);
            period_start_val = Some(nav_after);
        }
    }

    {
        let last_epoch  = end_secs / 86_400;
        let current_nav = cash.values().sum::<f64>()
            + sec_val_at(&positions, &price_map, last_epoch);
        if let Some(psv) = period_start_val {
            if psv > 0.0 {
                twr_product *= current_nav / psv;
                sub_periods += 1;
            }
        }
    }

    log::debug!("[TWR debug] transactions={} sub_periods={} twr_product={:.6} total_years={:.2}",
        sorted.len(), sub_periods, twr_product, total_years);

    if sub_periods == 0 || twr_product <= 0.0 {
        log::debug!("[TWR debug] → fallback 7% (sub_periods={sub_periods}, twr_product={twr_product:.6})");
        return 0.07;
    }

    let twr_total = twr_product - 1.0;
    let rate = ((1.0 + twr_total).powf(1.0 / total_years) - 1.0).clamp(-0.50, 1.50);
    log::debug!("[TWR debug] → twr_total={:.4} annualised={:.4} ({:.2}% p.a.)",
        twr_total, rate, rate * 100.0);
    rate
}

fn sec_val_at(
    positions: &std::collections::HashMap<String, std::collections::HashMap<String, f64>>,
    price_map: &std::collections::HashMap<String, Vec<(i64, f64)>>,
    epoch_day: i64,
) -> f64 {
    positions
        .values()
        .flat_map(|p| p.iter())
        .filter_map(|(uuid, &shs)| {
            price_at(price_map.get(uuid.as_str())?, epoch_day).map(|p| shs * p)
        })
        .sum()
}

fn build_price_map(
    client: &crate::PClient,
) -> std::collections::HashMap<String, Vec<(i64, f64)>> {
    let mut map: std::collections::HashMap<String, Vec<(i64, f64)>> =
        std::collections::HashMap::new();
    for sec in &client.securities {
        let mut prices: Vec<(i64, f64)> = sec
            .prices
            .iter()
            .map(|p| (p.date, p.close as f64 / 1e8))
            .collect();
        if let Some(latest) = &sec.latest {
            prices.push((latest.date, latest.close as f64 / 1e8));
        }
        prices.sort_unstable_by_key(|p| p.0);
        prices.dedup_by_key(|p| p.0);
        map.insert(sec.uuid.clone(), prices);
    }
    map
}

fn price_at(prices: &[(i64, f64)], epoch_day: i64) -> Option<f64> {
    if prices.is_empty() { return None; }
    let idx = prices.partition_point(|p| p.0 <= epoch_day);
    Some(if idx == 0 { prices[0].1 } else { prices[idx - 1].1 })
}

fn date_to_unix_secs(d: NaiveDate) -> i64 {
    use chrono::NaiveTime;
    chrono::NaiveDateTime::new(d, NaiveTime::MIN)
        .and_utc()
        .timestamp()
}
