//! Draws the counter into a 1280x720 BGRA frame at 30 fps (ADR-0002, ADR-0006, ADR-0014).
//! Pure: no OS calls here, so the whole renderer is unit-testable with injected fonts.

use ab_glyph::{point, Font, PxScale, ScaleFont};
use serde::Deserialize;
use tiny_skia::{Color, FillRule, Paint, Path, PathBuilder, Pixmap, Transform};

use crate::camera::fonts::Fonts;
use crate::camera::format::{format_duration, format_money, format_money_compact};
use crate::camera::frame::{normalize_size, Frame, FRAME_HEIGHT, FRAME_WIDTH};
use crate::camera::layout::{counter_layout, fit_cost, Rect};
use crate::camera::theme::{self, FontChoice, Layout, Overflow, Theme};

/// Translated strings supplied by the web UI (`packages/core/src/i18n.ts`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Labels {
    pub total_cost: String,
    pub elapsed: String,
    pub cost_per_minute: String,
    pub starts_in: String,
}

impl Default for Labels {
    fn default() -> Self {
        Labels {
            total_cost: "Total cost".into(),
            elapsed: "Elapsed".into(),
            cost_per_minute: "Cost per minute".into(),
            starts_in: "Starts in".into(),
        }
    }
}

/// Everything the renderer needs; comes from `packages/core` through `camera_update`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CounterInput {
    /// Epoch ms.
    #[serde(default)]
    pub start_at: i64,
    /// Epoch ms; set when the meeting was stopped.
    #[serde(default)]
    pub end_at: Option<i64>,
    /// Total cost of every participant, per millisecond.
    #[serde(default)]
    pub rate_per_ms: f64,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(default = "default_locale")]
    pub locale: String,
    #[serde(default)]
    pub labels: Labels,
    #[serde(default)]
    pub display: DisplayConfig,
    /// Output size in pixels. Only `SUPPORTED_SIZES` are produced (ADR-0015).
    #[serde(default = "default_width")]
    pub width: u32,
    #[serde(default = "default_height")]
    pub height: u32,
    /// Physical webcam to composite the counter card over (M7, ADR-0021). `None` = counter
    /// only. Ignored unless the layout is `widget`, whose card keeps the text readable.
    #[serde(default)]
    pub source: Option<String>,
}

impl CounterInput {
    /// Supported output size; unsupported values fall back to 1280x720.
    pub fn size(&self) -> (u32, u32) {
        normalize_size(self.width, self.height)
    }
}

fn default_width() -> u32 {
    FRAME_WIDTH
}

fn default_height() -> u32 {
    FRAME_HEIGHT
}

fn default_currency() -> String {
    "EUR".into()
}

fn default_locale() -> String {
    "es".into()
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(from = "DisplayConfigWire")]
pub struct DisplayConfig {
    pub theme: String,
    pub layout: Layout,
    pub cost_font: FontChoice,
    pub time_font: FontChoice,
    pub cost_scale: f32,
    pub time_scale: f32,
    pub overflow: Overflow,
}

/// Untrusted shape from the UI: strings for every enum, scales clamped (ADR-0006).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DisplayConfigWire {
    pub theme: String,
    pub layout: String,
    pub cost_font: String,
    pub time_font: String,
    pub cost_scale: f32,
    pub time_scale: f32,
    pub overflow: String,
}

impl Default for DisplayConfigWire {
    fn default() -> Self {
        let display = DisplayConfig::default();
        DisplayConfigWire {
            theme: display.theme,
            layout: "fullscreen".into(),
            cost_font: "theme".into(),
            time_font: "theme".into(),
            cost_scale: display.cost_scale,
            time_scale: display.time_scale,
            overflow: "compact".into(),
        }
    }
}

impl From<DisplayConfigWire> for DisplayConfig {
    fn from(wire: DisplayConfigWire) -> Self {
        DisplayConfig {
            theme: if wire.theme.is_empty() {
                "classic".into()
            } else {
                wire.theme
            },
            layout: theme::parse_layout(&wire.layout),
            cost_font: theme::parse_font_choice(&wire.cost_font),
            time_font: theme::parse_font_choice(&wire.time_font),
            cost_scale: clamp_scale(wire.cost_scale),
            time_scale: clamp_scale(wire.time_scale),
            overflow: theme::parse_overflow(&wire.overflow),
        }
    }
}

fn clamp_scale(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.5, 2.5)
    } else {
        1.0
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        DisplayConfig {
            theme: "classic".into(),
            layout: Layout::Fullscreen,
            cost_font: FontChoice::Theme,
            time_font: FontChoice::Theme,
            cost_scale: 1.0,
            time_scale: 1.0,
            overflow: Overflow::Compact,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CounterValues {
    pub elapsed_ms: i64,
    pub cost: f64,
    pub cost_per_minute: f64,
    pub starts_in_ms: i64,
}

/// Same rules as `meetingSnapshot` in `packages/core/src/cost.ts`.
pub fn counter_values(input: &CounterInput, now_ms: i64) -> CounterValues {
    let rate_per_ms = input.rate_per_ms.max(0.0);
    let end = input.end_at.unwrap_or(now_ms).min(now_ms);
    let elapsed_ms = (end - input.start_at).max(0);
    CounterValues {
        elapsed_ms,
        cost: rate_per_ms * elapsed_ms as f64,
        cost_per_minute: rate_per_ms * 60_000.0,
        starts_in_ms: (input.start_at - now_ms).max(0),
    }
}

/// The text drawn for the cost, after applying the compact/shrink rule.
#[derive(Clone, Debug, PartialEq)]
pub struct CostText {
    pub text: String,
    pub compact: bool,
    pub scale: f32,
}

/// Fraction of the available width the cost should occupy by default.
pub const COST_FILL_RATIO: f32 = 0.9;
/// Largest em size for the cost, as a fraction of its box height. Money uses digits and
/// the currency symbol, whose ink is about 0.7 em tall, so 1.0 still fits the box.
pub const COST_MAX_HEIGHT_RATIO: f32 = 1.0;

/// Font size whose glyphs span `target_width`, capped at `max_px` so the text stays
/// inside its box. This is what makes the cost fill most of the width (ADR-0015).
pub fn fit_font_px(text: &str, font: &ab_glyph::FontVec, target_width: f32, max_px: f32) -> f32 {
    const REFERENCE_PX: f32 = 100.0;
    let measured = measure(text, font, REFERENCE_PX);
    if measured <= 0.0 || target_width <= 0.0 {
        return max_px.clamp(8.0, REFERENCE_PX.max(8.0));
    }
    (REFERENCE_PX * target_width / measured).clamp(8.0, max_px.max(8.0))
}

/// Chooses between the full and the abbreviated cost so it fits in `available_px`.
pub fn cost_text(
    input: &CounterInput,
    fonts: &Fonts,
    now_ms: i64,
    available_px: f32,
    font_px: f32,
) -> CostText {
    let theme = theme::theme(&input.display.theme, now_ms);
    let font = fonts.get(input.display.cost_font.resolve(&theme));
    let cost = counter_values(input, now_ms).cost;
    let full = format_money(cost, &input.currency, &input.locale);
    let compact = format_money_compact(cost, &input.currency, &input.locale);
    let fit = fit_cost(
        available_px,
        measure(&full, font, font_px),
        measure(&compact, font, font_px),
        input.display.overflow,
    );
    CostText {
        text: if fit.compact { compact } else { full },
        compact: fit.compact,
        scale: fit.scale,
    }
}

/// Renders one frame. `background` is the captured webcam frame in overlay mode;
/// `None` fills the frame with the theme background (counter-only mode).
pub fn render(
    input: &CounterInput,
    fonts: &Fonts,
    now_ms: i64,
    background: Option<&Frame>,
) -> Frame {
    let theme = theme::theme(&input.display.theme, now_ms);
    let (width, height) = input.size();
    let layout = counter_layout(width as f32, height as f32, input.display.layout);
    let mut pixmap = match background {
        Some(frame) => pixmap_from_frame(frame),
        None => {
            let mut pixmap = Pixmap::new(width, height).expect("frame size");
            pixmap.fill(color(theme.background));
            pixmap
        }
    };
    if let Some(card) = layout.card {
        fill_card(&mut pixmap, card, &theme);
    }

    let values = counter_values(input, now_ms);
    let cost_font = input.display.cost_font.resolve(&theme);
    let time_font = input.display.time_font.resolve(&theme);

    let cost_font_ref = fonts.get(cost_font);
    let cost_full = format_money(values.cost, &input.currency, &input.locale);
    let base_px = fit_font_px(
        &cost_full,
        cost_font_ref,
        layout.cost.w * COST_FILL_RATIO,
        layout.cost.h * COST_MAX_HEIGHT_RATIO,
    );
    let cost_px = (base_px * input.display.cost_scale).max(12.0);
    let cost = cost_text(input, fonts, now_ms, layout.cost.w, cost_px);
    let cost_value = if values.starts_in_ms > 0 {
        format_money(0.0, &input.currency, &input.locale)
    } else {
        cost.text
    };
    draw_centered(
        &mut pixmap,
        &cost_value,
        cost_font_ref,
        cost_px * cost.scale,
        color(theme.text),
        layout.cost,
    );

    let time_px = (layout.time.h * 0.55 * input.display.time_scale).max(14.0);
    let time_value = if values.starts_in_ms > 0 {
        format!(
            "{} {}",
            input.labels.starts_in,
            format_duration(values.starts_in_ms)
        )
    } else {
        format_duration(values.elapsed_ms)
    };
    draw_centered(
        &mut pixmap,
        &time_value,
        fonts.get(time_font),
        time_px,
        color(theme.muted),
        layout.time,
    );

    let footer_px = (layout.footer.h * 0.5).clamp(12.0, 30.0);
    let footer = format!(
        "{} {}",
        input.labels.cost_per_minute,
        format_money(values.cost_per_minute, &input.currency, &input.locale)
    );
    draw_centered(
        &mut pixmap,
        &footer,
        fonts.get(time_font),
        footer_px,
        color(theme.muted),
        layout.footer,
    );

    draw_decoration(&mut pixmap, &theme, fonts, layout.content);

    frame_from_pixmap(pixmap)
}

fn color(rgb: (u8, u8, u8)) -> Color {
    Color::from_rgba8(rgb.0, rgb.1, rgb.2, 255)
}

fn fill_card(pixmap: &mut Pixmap, card: Rect, theme: &Theme) {
    let Some(rect) = tiny_skia::Rect::from_xywh(card.x, card.y, card.w, card.h) else {
        return;
    };
    let Some(path) = rounded_rect(rect, theme.radius) else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color(color(theme.background));
    pixmap.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

/// tiny-skia 0.11 has no rounded-rect helper: build it with quadrants (kappa curves).
fn rounded_rect(rect: tiny_skia::Rect, radius: f32) -> Option<Path> {
    let radius = radius
        .min(rect.width() / 2.0)
        .min(rect.height() / 2.0)
        .max(0.0);
    if radius <= 0.0 {
        return Some(PathBuilder::from_rect(rect));
    }
    let (x, y, w, h) = (rect.x(), rect.y(), rect.width(), rect.height());
    let kappa = 0.552_284_8 * radius;
    let mut builder = PathBuilder::new();
    builder.move_to(x + radius, y);
    builder.line_to(x + w - radius, y);
    builder.cubic_to(
        x + w - radius + kappa,
        y,
        x + w,
        y + radius - kappa,
        x + w,
        y + radius,
    );
    builder.line_to(x + w, y + h - radius);
    builder.cubic_to(
        x + w,
        y + h - radius + kappa,
        x + w - radius + kappa,
        y + h,
        x + w - radius,
        y + h,
    );
    builder.line_to(x + radius, y + h);
    builder.cubic_to(
        x + radius - kappa,
        y + h,
        x,
        y + h - radius + kappa,
        x,
        y + h - radius,
    );
    builder.line_to(x, y + radius);
    builder.cubic_to(x, y + radius - kappa, x + radius - kappa, y, x + radius, y);
    builder.close();
    builder.finish()
}

/// Emoji decorations need a face that has the glyph: skip them instead of drawing tofu.
fn draw_decoration(pixmap: &mut Pixmap, theme: &Theme, fonts: &Fonts, area: Rect) {
    if theme.decoration.is_empty() {
        return;
    }
    let font = fonts.get(theme.font);
    let supported: String = theme
        .decoration
        .chars()
        .filter(|c| c.is_whitespace() || font.glyph_id(*c).0 != 0)
        .collect();
    if supported.trim().is_empty() {
        return;
    }
    let px = (area.h * 0.04).clamp(16.0, 36.0);
    let width = measure(&supported, font, px);
    let style = TextStyle {
        font,
        px,
        color: color(theme.muted),
        clip: area,
    };
    draw_text(
        pixmap,
        &supported,
        &style,
        area.x + area.w - width,
        area.y + px,
    );
}

/// Font, size, colour and the box glyphs are clipped to (never paint over the webcam).
struct TextStyle<'a> {
    font: &'a ab_glyph::FontVec,
    px: f32,
    color: Color,
    clip: Rect,
}

fn draw_centered(
    pixmap: &mut Pixmap,
    text: &str,
    font: &ab_glyph::FontVec,
    px: f32,
    color: Color,
    rect: Rect,
) {
    let width = measure(text, font, px);
    let scaled = font.as_scaled(PxScale::from(px));
    let x = rect.x + (rect.w - width) / 2.0;
    let baseline = rect.y + rect.h / 2.0 + (scaled.ascent() + scaled.descent()) / 2.0;
    let style = TextStyle {
        font,
        px,
        color,
        clip: rect,
    };
    draw_text(pixmap, text, &style, x, baseline);
}

fn draw_text(pixmap: &mut Pixmap, text: &str, style: &TextStyle<'_>, start_x: f32, baseline: f32) {
    let scaled = style.font.as_scaled(PxScale::from(style.px));
    let mut pen_x = start_x;
    let mut previous = None;
    for ch in text.chars() {
        let id = scaled.glyph_id(ch);
        if let Some(previous_id) = previous {
            pen_x += scaled.kern(previous_id, id);
        }
        let glyph = id.with_scale_and_position(style.px, point(pen_x, baseline));
        if let Some(outlined) = scaled.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|x, y, coverage| {
                let target_x = bounds.min.x as i32 + x as i32;
                let target_y = bounds.min.y as i32 + y as i32;
                if target_x >= 0
                    && target_y >= 0
                    && style.clip.contains(target_x as f32, target_y as f32)
                {
                    blend(pixmap, target_x, target_y, style.color, coverage);
                }
            });
        }
        pen_x += scaled.h_advance(id);
        previous = Some(id);
    }
}

fn blend(pixmap: &mut Pixmap, x: i32, y: i32, color: Color, coverage: f32) {
    let (width, height) = (pixmap.width() as i32, pixmap.height() as i32);
    if x < 0 || y < 0 || x >= width || y >= height {
        return;
    }
    let alpha = coverage.clamp(0.0, 1.0);
    if alpha <= 0.0 {
        return;
    }
    let index = ((y as u32 * pixmap.width() + x as u32) * 4) as usize;
    let source = [color.red(), color.green(), color.blue()];
    let data = pixmap.data_mut();
    for (channel, source_value) in source.iter().enumerate() {
        let destination = data[index + channel] as f32 / 255.0;
        let out = destination * (1.0 - alpha) + source_value * alpha;
        data[index + channel] = (out * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    data[index + 3] = 255;
}

fn measure(text: &str, font: &ab_glyph::FontVec, px: f32) -> f32 {
    let scaled = font.as_scaled(PxScale::from(px));
    let mut width = 0.0;
    let mut previous = None;
    for ch in text.chars() {
        let id = scaled.glyph_id(ch);
        if let Some(previous_id) = previous {
            width += scaled.kern(previous_id, id);
        }
        width += scaled.h_advance(id);
        previous = Some(id);
    }
    width
}

fn pixmap_from_frame(frame: &Frame) -> Pixmap {
    let mut pixmap = Pixmap::new(frame.width(), frame.height()).expect("frame size");
    for (source, destination) in frame
        .pixels()
        .as_chunks::<4>()
        .0
        .iter()
        .zip(pixmap.data_mut().as_chunks_mut::<4>().0.iter_mut())
    {
        *destination = [source[2], source[1], source[0], 0xff];
    }
    pixmap
}

fn frame_from_pixmap(pixmap: Pixmap) -> Frame {
    let mut frame = Frame::new(pixmap.width(), pixmap.height());
    for (source, destination) in pixmap
        .data()
        .as_chunks::<4>()
        .0
        .iter()
        .zip(frame.pixels_mut().as_chunks_mut::<4>().0.iter_mut())
    {
        *destination = [source[2], source[1], source[0], 0xff];
    }
    frame
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> CounterInput {
        CounterInput {
            start_at: 1_000_000,
            end_at: None,
            // 0.01 per ms => 600 per minute.
            rate_per_ms: 0.01,
            currency: "EUR".into(),
            locale: "es".into(),
            labels: Labels::default(),
            display: DisplayConfig::default(),
            width: FRAME_WIDTH,
            height: FRAME_HEIGHT,
            source: None,
        }
    }

    fn fonts() -> Fonts {
        Fonts::load_system().expect("a system font is available")
    }

    fn ink_outside(frame: &Frame, rect: Rect, background: (u8, u8, u8)) -> usize {
        let mut count = 0;
        for y in 0..frame.height() {
            for x in 0..frame.width() {
                if !rect.contains(x as f32, y as f32) && frame.pixel(x, y) != background {
                    count += 1;
                }
            }
        }
        count
    }

    fn ink_inside(frame: &Frame, rect: Rect, background: (u8, u8, u8)) -> usize {
        let mut count = 0;
        for y in rect.y as u32..(rect.y + rect.h) as u32 {
            for x in rect.x as u32..(rect.x + rect.w) as u32 {
                if frame.pixel(x, y) != background {
                    count += 1;
                }
            }
        }
        count
    }

    /// Left and right edge of the painted pixels inside `rect`.
    fn ink_bounds(frame: &Frame, rect: Rect, background: (u8, u8, u8)) -> Option<(u32, u32)> {
        let mut min = u32::MAX;
        let mut max = 0;
        for y in rect.y as u32..(rect.y + rect.h) as u32 {
            for x in rect.x as u32..(rect.x + rect.w) as u32 {
                if frame.pixel(x, y) != background {
                    min = min.min(x);
                    max = max.max(x);
                }
            }
        }
        (max > min).then_some((min, max))
    }

    #[test]
    fn elapsed_cost_follows_the_timestamps() {
        let values = counter_values(&input(), 1_060_000);
        assert_eq!(values.elapsed_ms, 60_000);
        assert_eq!(values.cost, 600.0);
        assert_eq!(values.cost_per_minute, 600.0);
        assert_eq!(values.starts_in_ms, 0);
    }

    #[test]
    fn a_finished_meeting_stops_counting_at_end_at() {
        let mut input = input();
        input.end_at = Some(1_030_000);
        assert_eq!(counter_values(&input, 9_999_999).elapsed_ms, 30_000);
        assert_eq!(counter_values(&input, 9_999_999).cost, 300.0);
    }

    #[test]
    fn an_idle_window_publishes_zero_and_never_counts() {
        // Before the meeting starts the web sends start == end == now (ADR-0018), so the
        // preview in the meeting app shows 00:00:00 / 0,00 while the app is open.
        let idle = CounterInput {
            start_at: 5_000,
            end_at: Some(5_000),
            rate_per_ms: 0.4,
            ..input()
        };
        let values = counter_values(&idle, 5_500_000);
        assert_eq!(values.elapsed_ms, 0);
        assert_eq!(values.cost, 0.0);
        assert_eq!(values.starts_in_ms, 0);
    }

    #[test]
    fn a_future_start_reports_starts_in_and_zero_cost() {
        let values = counter_values(&input(), 994_000);
        assert_eq!(values.elapsed_ms, 0);
        assert_eq!(values.cost, 0.0);
        assert_eq!(values.starts_in_ms, 6_000);
    }

    #[test]
    fn cost_text_abbreviates_or_shrinks_like_the_web() {
        let mut input = input();
        input.rate_per_ms = 1.0;
        let fonts = fonts();
        let compact = cost_text(&input, &fonts, 1_060_000, 200.0, 120.0);
        assert!(compact.compact, "60 000 € does not fit at 120 px");
        assert!(compact.text.contains('K'), "abbreviated: {}", compact.text);
        assert!(
            compact.scale > 0.0 && compact.scale < 1.0,
            "shrunk to fit: {}",
            compact.scale
        );

        input.display.overflow = Overflow::Shrink;
        let shrunk = cost_text(&input, &fonts, 1_060_000, 200.0, 120.0);
        assert!(!shrunk.compact);
        assert!(shrunk.scale < 1.0);
        assert!(!shrunk.text.contains('K') && !shrunk.text.contains('M'));
    }

    #[test]
    fn renders_a_fullscreen_frame_in_the_theme_colours() {
        let frame = render(&input(), &fonts(), 1_060_000, None);
        let background = (0x0f, 0x11, 0x15);
        assert_eq!(frame.width(), FRAME_WIDTH);
        assert_eq!(frame.height(), FRAME_HEIGHT);
        assert_eq!(frame.pixel(0, 0), background);
        assert_eq!(frame.pixel(FRAME_WIDTH - 1, FRAME_HEIGHT - 1), background);
        let layout = counter_layout(FRAME_WIDTH as f32, FRAME_HEIGHT as f32, Layout::Fullscreen);
        assert!(
            ink_inside(&frame, layout.cost, background) > 0,
            "the cost is drawn"
        );
        assert!(
            ink_inside(&frame, layout.time, background) > 0,
            "the elapsed time is drawn"
        );
    }

    #[test]
    fn widget_mode_keeps_the_webcam_visible_outside_the_card() {
        let mut input = input();
        input.display.layout = Layout::Widget;
        let webcam = (0x22, 0x44, 0x66);
        let background = Frame::filled(FRAME_WIDTH, FRAME_HEIGHT, webcam);
        let frame = render(&input, &fonts(), 1_060_000, Some(&background));
        assert_eq!(frame.pixel(0, 0), webcam, "background is preserved");
        let layout = counter_layout(FRAME_WIDTH as f32, FRAME_HEIGHT as f32, Layout::Widget);
        let card = layout.card.expect("card");
        let card_center_x = (card.x + card.w / 2.0) as u32;
        assert_eq!(
            frame.pixel(card_center_x, card.y as u32 + 2),
            (0x0f, 0x11, 0x15)
        );
        let outside = ink_outside(&frame, card, webcam);
        assert_eq!(outside, 0, "{outside} pixels are drawn over the webcam");
        assert!(
            ink_inside(&frame, card, (0x0f, 0x11, 0x15)) > 0,
            "the counter is drawn inside the card"
        );
    }

    #[test]
    fn a_future_start_renders_without_panicking() {
        let frame = render(&input(), &fonts(), 990_000, None);
        let background = (0x0f, 0x11, 0x15);
        let layout = counter_layout(FRAME_WIDTH as f32, FRAME_HEIGHT as f32, Layout::Fullscreen);
        assert!(ink_inside(&frame, layout.cost, background) > 0);
        assert!(ink_inside(&frame, layout.time, background) > 0);
    }

    #[test]
    fn every_theme_renders() {
        let fonts = fonts();
        for (id, _) in crate::camera::themes_data::THEMES {
            let mut input = input();
            input.display.theme = (*id).into();
            let frame = render(&input, &fonts, 1_060_000, None);
            assert_eq!(frame.width(), FRAME_WIDTH);
        }
    }

    #[test]
    fn the_cost_font_fills_most_of_the_width_by_default() {
        let fonts = fonts();
        let font = fonts.get(crate::camera::theme::FontId::System);
        let px = fit_font_px("1.234,56 €", font, 900.0, 500.0);
        let width = measure("1.234,56 €", font, px);
        assert!((width - 900.0).abs() < 9.0, "{width} px wide at {px} px");
        assert!(px <= 500.0, "never taller than the box");
        assert_eq!(
            fit_font_px("0", font, 900.0, 60.0),
            60.0,
            "short text hits the height cap"
        );
    }

    #[test]
    fn the_rendered_cost_spans_most_of_the_content_width() {
        let frame = render(&input(), &fonts(), 1_060_000, None);
        let background = (0x0f, 0x11, 0x15);
        let layout = counter_layout(FRAME_WIDTH as f32, FRAME_HEIGHT as f32, Layout::Fullscreen);
        let (min, max) = ink_bounds(&frame, layout.cost, background).expect("the cost is drawn");
        let ink = (max - min) as f32;
        assert!(
            ink >= layout.cost.w * 0.8,
            "cost ink is {ink} px of {} available",
            layout.cost.w
        );
    }

    #[test]
    fn renders_the_selected_resolution() {
        let mut input = input();
        input.width = 640;
        input.height = 360;
        let frame = render(&input, &fonts(), 1_060_000, None);
        assert_eq!((frame.width(), frame.height()), (640, 360), "640x360 mode");
        let layout = counter_layout(640.0, 360.0, Layout::Fullscreen);
        assert!(ink_inside(&frame, layout.cost, (0x0f, 0x11, 0x15)) > 0);

        input.width = 1920;
        input.height = 1080;
        let frame = render(&input, &fonts(), 1_060_000, None);
        assert_eq!((frame.width(), frame.height()), (1920, 1080), "1080p mode");
    }

    #[test]
    fn unsupported_resolutions_fall_back_to_1280x720() {
        let mut input = input();
        input.width = 800;
        input.height = 600;
        assert_eq!(input.size(), (FRAME_WIDTH, FRAME_HEIGHT));
        let frame = render(&input, &fonts(), 1_060_000, None);
        assert_eq!((frame.width(), frame.height()), (FRAME_WIDTH, FRAME_HEIGHT));
    }

    #[test]
    fn portrait_and_square_frames_render() {
        let fonts = fonts();
        for (width, height) in [(720u32, 1280u32), (1080, 1080), (360, 640)] {
            let mut input = input();
            input.width = width;
            input.height = height;
            let frame = render(&input, &fonts, 1_060_000, None);
            assert_eq!((frame.width(), frame.height()), (width, height));
            let layout = counter_layout(width as f32, height as f32, Layout::Fullscreen);
            assert!(
                ink_inside(&frame, layout.cost, (0x0f, 0x11, 0x15)) > 0,
                "the cost is drawn on {width}x{height}"
            );
        }
    }

    #[test]
    fn the_widget_stays_inside_a_portrait_frame() {
        let mut input = input();
        input.width = 720;
        input.height = 1280;
        input.display.layout = Layout::Widget;
        let webcam = (0x22, 0x44, 0x66);
        let background = Frame::filled(720, 1280, webcam);
        let frame = render(&input, &fonts(), 1_060_000, Some(&background));
        let layout = counter_layout(720.0, 1280.0, Layout::Widget);
        let card = layout.card.expect("card");
        let outside = ink_outside(&frame, card, webcam);
        assert_eq!(outside, 0, "{outside} pixels are drawn over the webcam");
    }

    #[test]
    fn parses_the_output_size_from_the_ui() {
        let input: CounterInput =
            serde_json::from_str(r#"{"width":1920,"height":1080}"#).expect("size parses");
        assert_eq!(input.size(), (1920, 1080));

        let default: CounterInput = serde_json::from_str(r#"{}"#).expect("defaults parse");
        assert_eq!(default.size(), (FRAME_WIDTH, FRAME_HEIGHT));
    }

    #[test]
    fn parses_a_partial_config_from_the_ui() {
        let input: CounterInput = serde_json::from_str(
            r#"{"startAt":1000,"ratePerMs":0.5,"currency":"USD","locale":"en-US","labels":{"startsIn":"Starts in 5"}}"#,
        )
        .expect("partial config parses");
        assert_eq!(input.start_at, 1000);
        assert_eq!(input.rate_per_ms, 0.5);
        assert_eq!(input.currency, "USD");
        assert_eq!(input.labels.starts_in, "Starts in 5");
        assert_eq!(
            input.labels.total_cost, "Total cost",
            "missing labels keep their default"
        );
        assert_eq!(input.display, DisplayConfig::default());
        assert_eq!(input.end_at, None);
        assert_eq!(
            input.size(),
            (FRAME_WIDTH, FRAME_HEIGHT),
            "1280x720 by default"
        );
    }

    #[test]
    fn normalises_untrusted_display_settings() {
        let input: CounterInput = serde_json::from_str(
            r#"{"display":{"theme":"nope","layout":"widget","costFont":"mono","timeFont":"nonsense","costScale":9,"timeScale":0.1,"overflow":"shrink"}}"#,
        )
        .expect("display parses");
        assert_eq!(
            input.display.theme, "nope",
            "unknown themes fall back when rendering"
        );
        assert_eq!(input.display.layout, Layout::Widget);
        assert_eq!(
            input.display.cost_font,
            FontChoice::Font(crate::camera::theme::FontId::Mono)
        );
        assert_eq!(input.display.time_font, FontChoice::Theme);
        assert_eq!(input.display.cost_scale, 2.5);
        assert_eq!(input.display.time_scale, 0.5);
        assert_eq!(input.display.overflow, Overflow::Shrink);
    }
}
