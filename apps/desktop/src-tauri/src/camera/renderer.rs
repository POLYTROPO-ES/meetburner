//! Draws the counter into a 1280x720 BGRA frame at 30 fps (ADR-0002, ADR-0006, ADR-0014).
//! Pure: no OS calls here, so the whole renderer is unit-testable with injected fonts.

use ab_glyph::{point, Font, PxScale, ScaleFont};
use serde::Deserialize;
use tiny_skia::{
    Color, FillRule, FilterQuality, GradientStop, LinearGradient, Paint, Path, PathBuilder, Pixmap,
    PixmapPaint, Point, SpreadMode, Transform,
};

use crate::camera::fonts::Fonts;
use crate::camera::format::{format_duration, format_money, format_money_compact};
use crate::camera::frame::{normalize_size, Frame, FRAME_HEIGHT, FRAME_WIDTH};
#[cfg(test)]
use crate::camera::layout::counter_layout;
use crate::camera::layout::{counter_layout_with_label, fit_cost, label_font_px, Rect};

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
    pub overflow: Overflow,
    /// Font, size, colour and visibility per counter line (ADR-0029).
    pub elements: Elements,
    /// Card paint: solid or gradient, opacity and radius (ADR-0029).
    pub background: BackgroundStyle,
    /// Clockwise turn of the counter layer in degrees, `0..=360`. The webcam never
    /// turns: only the card, its background and its text are rotated.
    pub rotation: f32,
    /// Per-layer mirroring; the webcam and the counter are independent (ADR-0022).
    pub mirror: Mirror,
    /// Where the widget card goes, as fractions of the frame (ADR-0023).
    #[serde(default)]
    pub overlay: Option<Overlay>,
}

/// Custom widget card rectangle from the UI, as fractions of the frame.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Overlay {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Default for Overlay {
    fn default() -> Self {
        Overlay {
            x: 0.0,
            y: 0.0,
            w: 0.42,
            h: 0.3,
        }
    }
}

impl Overlay {
    /// `None` for anything unusable, so a corrupt payload keeps the built-in corner.
    pub fn sanitized(self) -> Option<Overlay> {
        let values = [self.x, self.y, self.w, self.h];
        if values.iter().any(|value| !value.is_finite()) {
            return None;
        }
        Some(Overlay {
            x: self.x.clamp(0.0, 1.0),
            y: self.y.clamp(0.0, 1.0),
            w: self.w.clamp(0.05, 1.0),
            h: self.h.clamp(0.05, 1.0),
        })
    }
}

/// Flips applied to the captured webcam and to the counter layer, independently.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Mirror {
    pub source_h: bool,
    pub source_v: bool,
    pub overlay_h: bool,
    pub overlay_v: bool,
}

/// Untrusted shape from the UI: strings for every enum, values clamped (ADR-0006, ADR-0029).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DisplayConfigWire {
    pub theme: String,
    pub layout: String,
    pub overflow: String,
    pub mirror: Mirror,
    pub overlay: Option<Overlay>,
    pub elements: ElementsWire,
    /// Absent in older payloads: keeps the flat theme background the renderer always painted.
    pub background: Option<BackgroundWire>,
    /// Absent in older payloads too; see [`clamp_rotation`].
    pub rotation: Option<f32>,
}

impl Default for DisplayConfigWire {
    fn default() -> Self {
        DisplayConfigWire {
            theme: "classic".into(),
            layout: "fullscreen".into(),
            overflow: "compact".into(),
            mirror: Mirror::default(),
            overlay: None,
            elements: ElementsWire::default(),
            background: None,
            rotation: None,
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
            overflow: theme::parse_overflow(&wire.overflow),
            elements: Elements::resolve(&wire.elements),
            background: BackgroundStyle::resolve(wire.background.as_ref()),
            rotation: clamp_rotation(wire.rotation),
            mirror: wire.mirror,
            overlay: wire.overlay.and_then(Overlay::sanitized),
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

/// Largest card radius the UI offers, in pixels (ADR-0029).
const MAX_RADIUS: f32 = 40.0;

/// `#rgb` or `#rrggbb`, like the web; anything else keeps the preset colour.
fn parse_hex_color(value: &str) -> Option<(u8, u8, u8)> {
    let hex = value.strip_prefix('#')?;
    match hex.len() {
        3 => {
            let mut digits = hex.chars().map(|digit| digit.to_digit(16));
            let r = digits.next()?? as u8;
            let g = digits.next()?? as u8;
            let b = digits.next()?? as u8;
            Some((r * 17, g * 17, b * 17))
        }
        6 => {
            let value = u32::from_str_radix(hex, 16).ok()?;
            Some(((value >> 16) as u8, (value >> 8) as u8, value as u8))
        }
        _ => None,
    }
}

/// Percent 0..100 to the 0.0..1.0 multiplier; missing or corrupt values stay solid.
fn clamp_opacity(value: Option<f32>) -> f32 {
    match value {
        Some(value) if value.is_finite() => value.clamp(0.0, 100.0) / 100.0,
        _ => 1.0,
    }
}

/// `None` keeps the preset radius; corrupt values fall back to it too.
fn clamp_radius(value: Option<f32>) -> Option<f32> {
    value
        .filter(|radius| radius.is_finite())
        .map(|radius| radius.clamp(0.0, MAX_RADIUS))
}

/// Degrees `0..=360`; a missing or corrupt value keeps the counter upright.
fn clamp_rotation(value: Option<f32>) -> f32 {
    value
        .filter(|degrees| degrees.is_finite())
        .map(|degrees| degrees.clamp(0.0, 360.0))
        .unwrap_or(0.0)
}

/// The turn actually applied: `360` is a full turn, so it draws upright like `0`.
fn normalize_rotation(value: f32) -> f32 {
    if value.is_finite() {
        value.rem_euclid(360.0)
    } else {
        0.0
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        DisplayConfig {
            theme: "classic".into(),
            layout: Layout::Fullscreen,
            overflow: Overflow::Compact,
            elements: Elements::default(),
            background: BackgroundStyle::default(),
            rotation: 0.0,
            mirror: Mirror::default(),
            overlay: None,
        }
    }
}

/// One customisable counter line: font, size, colour and visibility (ADR-0029).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ElementStyle {
    /// `FontChoice::Theme` keeps the preset font.
    pub font: FontChoice,
    /// Multiplier applied to the preset size: 1.0 = the preset size.
    pub scale: f32,
    /// `None` keeps the preset colour, including the accent of a running cost.
    pub color: Option<(u8, u8, u8)>,
    /// A hidden line is not drawn and, in `fullscreen`, stops reserving its band.
    pub visible: bool,
}

impl Default for ElementStyle {
    fn default() -> Self {
        ElementStyle {
            font: FontChoice::Theme,
            scale: 1.0,
            color: None,
            visible: true,
        }
    }
}

impl ElementStyle {
    /// Untrusted payload values: missing pieces keep the preset, scales are clamped.
    fn resolve(wire: &ElementWire) -> ElementStyle {
        ElementStyle {
            font: wire
                .font
                .as_deref()
                .map(theme::parse_font_choice)
                .unwrap_or(FontChoice::Theme),
            scale: clamp_scale(wire.scale.unwrap_or(100.0) / 100.0),
            color: wire.color.as_deref().and_then(parse_hex_color),
            // Absent in older payloads: every line the preset draws stays visible.
            visible: wire.visible.unwrap_or(true),
        }
    }
}

/// The four lines the camera draws, each with its own style (ADR-0029).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Elements {
    /// `totalCost` above the amount in `fullscreen`; the widget hides it, like the web.
    pub label: ElementStyle,
    pub cost: ElementStyle,
    /// The elapsed time (or `starts in`) line.
    pub elapsed: ElementStyle,
    /// The cost-per-minute line under the time.
    pub cost_per_minute: ElementStyle,
}

impl Elements {
    fn resolve(wire: &ElementsWire) -> Elements {
        Elements {
            label: ElementStyle::resolve(&wire.label),
            cost: ElementStyle::resolve(&wire.cost),
            elapsed: ElementStyle::resolve(&wire.elapsed),
            cost_per_minute: ElementStyle::resolve(&wire.cost_per_minute),
        }
    }
}

/// Untrusted element style; every field left out keeps the preset.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ElementWire {
    pub font: Option<String>,
    /// Percent, 100 = the preset size.
    pub scale: Option<f32>,
    pub color: Option<String>,
    /// Absent means visible.
    pub visible: Option<bool>,
}

/// Untrusted per-element styles; a missing element keeps the preset.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ElementsWire {
    pub label: ElementWire,
    pub cost: ElementWire,
    /// The web customises the status line it shows; the camera draws no status line (the
    /// counter card has no room), so this is accepted and ignored on purpose.
    pub status: ElementWire,
    /// Older web builds sent `time`: the alias keeps those payloads landing here.
    #[serde(alias = "time")]
    pub elapsed: ElementWire,
    /// The web customises the person-time stats grid it shows; the camera draws no stats
    /// grid (the counter card has no room), so this is accepted and ignored on purpose.
    pub person_time: ElementWire,
    pub cost_per_minute: ElementWire,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackgroundMode {
    Solid,
    Gradient,
}

/// Linear gradient axis: left to right, or top to bottom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GradientDirection {
    Horizontal,
    Vertical,
}

/// How the counter card is painted (ADR-0029).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackgroundStyle {
    pub mode: BackgroundMode,
    /// Solid fill, or the gradient start; `None` keeps the theme background.
    pub color: Option<(u8, u8, u8)>,
    /// Gradient end; `None` keeps the theme's alternate background.
    pub color_alt: Option<(u8, u8, u8)>,
    pub direction: GradientDirection,
    /// 0.0 = fully transparent (only the text remains), 1.0 = solid.
    pub opacity: f32,
    /// `None` keeps the preset radius.
    pub radius: Option<f32>,
}

impl Default for BackgroundStyle {
    /// Today's look: a flat, opaque theme background with the preset radius.
    fn default() -> Self {
        BackgroundStyle {
            mode: BackgroundMode::Solid,
            color: None,
            color_alt: None,
            direction: GradientDirection::Vertical,
            opacity: 1.0,
            radius: None,
        }
    }
}

impl BackgroundStyle {
    /// `None` (no `background` in the payload) keeps the look the renderer always had.
    fn resolve(wire: Option<&BackgroundWire>) -> BackgroundStyle {
        match wire {
            None => BackgroundStyle::default(),
            Some(wire) => BackgroundStyle {
                // ADR-0029: the web has always painted a gradient, so an omitted `mode`
                // means `gradient`; only an explicit `solid` turns it off.
                mode: match wire.mode.as_deref() {
                    Some("solid") => BackgroundMode::Solid,
                    _ => BackgroundMode::Gradient,
                },
                color: wire.color.as_deref().and_then(parse_hex_color),
                color_alt: wire.color_alt.as_deref().and_then(parse_hex_color),
                direction: match wire.direction.as_deref() {
                    Some("horizontal") => GradientDirection::Horizontal,
                    _ => GradientDirection::Vertical,
                },
                opacity: clamp_opacity(wire.opacity),
                radius: clamp_radius(wire.radius),
            },
        }
    }

    fn color_or(&self, theme: &Theme) -> (u8, u8, u8) {
        self.color.unwrap_or(theme.background)
    }

    fn color_alt_or(&self, theme: &Theme) -> (u8, u8, u8) {
        self.color_alt.unwrap_or(theme.background_alt)
    }

    fn radius_or(&self, theme: &Theme) -> f32 {
        self.radius.unwrap_or(theme.radius)
    }

    fn alpha(&self) -> u8 {
        (self.opacity * 255.0).round().clamp(0.0, 255.0) as u8
    }
}

/// Untrusted card paint; a missing field keeps its default (ADR-0029).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BackgroundWire {
    pub mode: Option<String>,
    pub color: Option<String>,
    pub color_alt: Option<String>,
    pub direction: Option<String>,
    pub opacity: Option<f32>,
    pub radius: Option<f32>,
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
/// Letter spacing of the `labels.total_cost` line, in em (`letter-spacing: 0.1em` on the web).
const LABEL_TRACKING_EM: f32 = 0.1;
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
    let font = fonts.get(input.display.elements.cost.font.resolve(&theme));
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

/// Renders one frame. `background` is the captured webcam frame in overlay mode; `None`
/// leaves the black canvas of a counter-only frame visible behind the painted background.
///
/// The counter is painted on its own transparent layer and then composited, so the webcam
/// and the counter can be mirrored independently (ADR-0022) and the counter can turn without
/// dragging the webcam with it.
pub fn render(
    input: &CounterInput,
    fonts: &Fonts,
    now_ms: i64,
    background: Option<&Frame>,
) -> Frame {
    let theme = theme::theme(&input.display.theme, now_ms);
    let mirror = input.display.mirror;
    let (width, height) = input.size();
    let overlay_rect = input.display.overlay.map(|overlay| Rect {
        x: overlay.x,
        y: overlay.y,
        w: overlay.w,
        h: overlay.h,
    });
    let elements = input.display.elements;
    // The web hides `totalCost` inside the widget, a hidden line is not drawn at all, and an
    // empty label paints nothing: all three keep the layout the renderer always had. Only the
    // label band is reserved space above the stack, so only hiding the label releases it
    // (ADR-0029).
    let label_text = input.labels.total_cost.trim().to_uppercase();
    let label_scale = (input.display.layout == Layout::Fullscreen
        && elements.label.visible
        && !label_text.is_empty())
    .then_some(elements.label.scale);
    let layout = counter_layout_with_label(
        width as f32,
        height as f32,
        input.display.layout,
        overlay_rect,
        label_scale,
    );
    let mut pixmap = match background {
        Some(frame) => {
            let frame = frame
                .scaled_to(width, height)
                .flipped(mirror.source_h, mirror.source_v);
            pixmap_from_frame(&frame)
        }
        None => {
            // Nothing behind the counter: a black canvas, so a transparent or missing
            // background shows what the virtual camera actually has behind it (ADR-0029).
            let mut pixmap = Pixmap::new(width, height).expect("frame size");
            pixmap.fill(Color::BLACK);
            pixmap
        }
    };
    let background_style = input.display.background;
    let mut overlay = Pixmap::new(width, height).expect("frame size");
    // The background belongs to the counter layer, so mirroring moves it with its text
    // (ADR-0022); its alpha blends it into the webcam or the black canvas (ADR-0029).
    let (card, radius) = match layout.card {
        Some(card) => (card, background_style.radius_or(&theme)),
        // Fullscreen: the background covers the whole frame and has no card corners.
        None => (
            Rect {
                x: 0.0,
                y: 0.0,
                w: width as f32,
                h: height as f32,
            },
            0.0,
        ),
    };
    paint_background(&mut overlay, card, radius, &background_style, &theme);

    let values = counter_values(input, now_ms);
    // Same rule as `meetingSnapshot` in the web (`running`): the cost glows while counting.
    let running = input.end_at.is_none() && now_ms >= input.start_at;

    if let Some(band) = layout.label {
        let label_style = elements.label;
        let label_px = label_font_px(height as f32, label_style.scale);
        draw_centered_tracked(
            &mut overlay,
            &label_text,
            fonts.get(label_style.font.resolve(&theme)),
            label_px,
            element_color(label_style.color, theme.muted),
            band,
            label_px * LABEL_TRACKING_EM,
        );
    }

    let cost_style = elements.cost;
    if cost_style.visible {
        let cost_font_ref = fonts.get(cost_style.font.resolve(&theme));
        let cost_full = format_money(values.cost, &input.currency, &input.locale);
        let base_px = fit_font_px(
            &cost_full,
            cost_font_ref,
            layout.cost.w * COST_FILL_RATIO,
            layout.cost.h * COST_MAX_HEIGHT_RATIO,
        );
        let cost_px = (base_px * cost_style.scale).max(12.0);
        let cost = cost_text(input, fonts, now_ms, layout.cost.w, cost_px);
        let cost_value = if values.starts_in_ms > 0 {
            format_money(0.0, &input.currency, &input.locale)
        } else {
            cost.text
        };
        draw_centered(
            &mut overlay,
            &cost_value,
            cost_font_ref,
            cost_px * cost.scale,
            element_color(
                cost_style.color,
                if running { theme.accent } else { theme.text },
            ),
            layout.cost,
        );
    }

    let elapsed_style = elements.elapsed;
    if elapsed_style.visible {
        let time_px = (layout.time.h * 0.55).max(14.0) * elapsed_style.scale;
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
            &mut overlay,
            &time_value,
            fonts.get(elapsed_style.font.resolve(&theme)),
            time_px,
            element_color(elapsed_style.color, theme.muted),
            layout.time,
        );
    }

    let footer_style = elements.cost_per_minute;
    if footer_style.visible {
        let footer_px = (layout.footer.h * 0.5).clamp(12.0, 30.0) * footer_style.scale;
        let footer = format!(
            "{} {}",
            input.labels.cost_per_minute,
            format_money(values.cost_per_minute, &input.currency, &input.locale)
        );
        draw_centered(
            &mut overlay,
            &footer,
            fonts.get(footer_style.font.resolve(&theme)),
            footer_px,
            element_color(footer_style.color, theme.muted),
            layout.footer,
        );
    }

    draw_decoration(&mut overlay, &theme, fonts, layout.content);

    if mirror.overlay_h || mirror.overlay_v {
        flip_pixmap(&mut overlay, mirror.overlay_h, mirror.overlay_v);
    }
    // The counter turns as a single layer, card and background included, and never the
    // webcam: the layer goes through a scratch pixmap before it is composited.
    let rotation = normalize_rotation(input.display.rotation);
    let layer = if rotation == 0.0 {
        overlay
    } else {
        let (mut centre_x, mut centre_y) = match layout.card {
            Some(card) => (card.x + card.w / 2.0, card.y + card.h / 2.0),
            None => (width as f32 / 2.0, height as f32 / 2.0),
        };
        // Mirroring already moved the card, so the turn follows its new centre.
        if mirror.overlay_h {
            centre_x = width as f32 - centre_x;
        }
        if mirror.overlay_v {
            centre_y = height as f32 - centre_y;
        }
        rotate_layer(&overlay, rotation, centre_x, centre_y)
    };
    pixmap.draw_pixmap(
        0,
        0,
        layer.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );

    frame_from_pixmap(pixmap)
}

/// Turns the counter layer `degrees` clockwise around `(centre_x, centre_y)` into a scratch
/// pixmap: tiny-skia resamples card, background and glyphs in one pass, so the text turns as a
/// picture instead of glyph by glyph. `degrees` is never `0` here (that keeps today's pixels).
fn rotate_layer(layer: &Pixmap, degrees: f32, centre_x: f32, centre_y: f32) -> Pixmap {
    let mut rotated = Pixmap::new(layer.width(), layer.height()).expect("frame size");
    rotated.draw_pixmap(
        0,
        0,
        layer.as_ref(),
        &PixmapPaint {
            // Bilinear keeps the card edges and the glyphs from breaking up off 90°.
            quality: FilterQuality::Bilinear,
            ..PixmapPaint::default()
        },
        Transform::from_rotate_at(degrees, centre_x, centre_y),
        None,
    );
    rotated
}

/// Mirrors a layer in place, horizontally and/or vertically.
fn flip_pixmap(pixmap: &mut Pixmap, horizontal: bool, vertical: bool) {
    if !horizontal && !vertical {
        return;
    }
    let width = pixmap.width() as usize;
    let height = pixmap.height() as usize;
    let stride = width * 4;
    let data = pixmap.data_mut();
    let mut flipped = vec![0u8; data.len()];
    for y in 0..height {
        let source_y = if vertical { height - 1 - y } else { y };
        let source_row = &data[source_y * stride..source_y * stride + stride];
        let target_row = &mut flipped[y * stride..y * stride + stride];
        if horizontal {
            for x in 0..width {
                let from = (width - 1 - x) * 4;
                target_row[x * 4..x * 4 + 4].copy_from_slice(&source_row[from..from + 4]);
            }
        } else {
            target_row.copy_from_slice(source_row);
        }
    }
    data.copy_from_slice(&flipped);
}

fn color(rgb: (u8, u8, u8)) -> Color {
    Color::from_rgba8(rgb.0, rgb.1, rgb.2, 255)
}

/// A per-element colour override, or the preset rule when the UI sends `null`.
fn element_color(override_color: Option<(u8, u8, u8)>, preset: (u8, u8, u8)) -> Color {
    color(override_color.unwrap_or(preset))
}

/// Paints the card with the requested background (ADR-0029). The alpha travels in the paint, so
/// tiny-skia blends it into the webcam (or the black canvas) exactly like the glyphs: the text
/// stays fully opaque and only the panel fades.
fn paint_background(
    pixmap: &mut Pixmap,
    card: Rect,
    radius: f32,
    style: &BackgroundStyle,
    theme: &Theme,
) {
    let Some(rect) = tiny_skia::Rect::from_xywh(card.x, card.y, card.w, card.h) else {
        return;
    };
    let Some(path) = rounded_rect(rect, radius) else {
        return;
    };
    // Fully transparent: nothing to paint, so whatever is behind stays untouched.
    let alpha = style.alpha();
    if alpha == 0 {
        return;
    }
    let mut paint = Paint::default();
    match style.mode {
        BackgroundMode::Solid => {
            paint.set_color(with_alpha(style.color_or(theme), alpha));
        }
        BackgroundMode::Gradient => {
            let (start, end) = match style.direction {
                GradientDirection::Horizontal => (
                    Point::from_xy(rect.x(), rect.y()),
                    Point::from_xy(rect.x() + rect.width(), rect.y()),
                ),
                GradientDirection::Vertical => (
                    Point::from_xy(rect.x(), rect.y()),
                    Point::from_xy(rect.x(), rect.y() + rect.height()),
                ),
            };
            let Some(shader) = LinearGradient::new(
                start,
                end,
                vec![
                    GradientStop::new(0.0, with_alpha(style.color_or(theme), alpha)),
                    GradientStop::new(1.0, with_alpha(style.color_alt_or(theme), alpha)),
                ],
                SpreadMode::Pad,
                Transform::identity(),
            ) else {
                return;
            };
            paint.shader = shader;
        }
    }
    pixmap.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

fn with_alpha(rgb: (u8, u8, u8), alpha: u8) -> Color {
    Color::from_rgba8(rgb.0, rgb.1, rgb.2, alpha)
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
        tracking: 0.0,
    };
    draw_text(
        pixmap,
        &supported,
        &style,
        area.x + area.w - width,
        area.y + px,
    );
}

/// Font, size, colour, tracking and the box glyphs are clipped to (never paint over the webcam).
struct TextStyle<'a> {
    font: &'a ab_glyph::FontVec,
    px: f32,
    color: Color,
    clip: Rect,
    /// Extra advance after every glyph, in pixels (the label is tracked like the web).
    tracking: f32,
}

fn draw_centered(
    pixmap: &mut Pixmap,
    text: &str,
    font: &ab_glyph::FontVec,
    px: f32,
    color: Color,
    rect: Rect,
) {
    draw_centered_tracked(pixmap, text, font, px, color, rect, 0.0);
}

fn draw_centered_tracked(
    pixmap: &mut Pixmap,
    text: &str,
    font: &ab_glyph::FontVec,
    px: f32,
    color: Color,
    rect: Rect,
    tracking: f32,
) {
    let width = measure_tracked(text, font, px, tracking);
    let scaled = font.as_scaled(PxScale::from(px));
    let x = rect.x + (rect.w - width) / 2.0;
    let baseline = rect.y + rect.h / 2.0 + (scaled.ascent() + scaled.descent()) / 2.0;
    let style = TextStyle {
        font,
        px,
        color,
        clip: rect,
        tracking,
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
        pen_x += scaled.h_advance(id) + style.tracking;
        previous = Some(id);
    }
}

fn blend(pixmap: &mut Pixmap, x: i32, y: i32, color: Color, coverage: f32) {
    let (width, height) = (pixmap.width() as i32, pixmap.height() as i32);
    if x < 0 || y < 0 || x >= width || y >= height {
        return;
    }
    let coverage = coverage.clamp(0.0, 1.0);
    if coverage <= 0.0 {
        return;
    }
    let index = ((y as u32 * pixmap.width() + x as u32) * 4) as usize;
    let source = [color.red(), color.green(), color.blue()];
    let data = pixmap.data_mut();
    // tiny-skia stores premultiplied pixels: use its source-over maths so glyphs blend into
    // the painted card (which may be translucent) exactly like the card blends into the frame.
    let destination_alpha = data[index + 3] as f32 / 255.0;
    let alpha = coverage + destination_alpha * (1.0 - coverage);
    for (channel, source_value) in source.iter().enumerate() {
        let destination = data[index + channel] as f32 / 255.0;
        let out = source_value * coverage + destination * (1.0 - coverage);
        data[index + channel] = (out * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    data[index + 3] = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
}

/// Same as [`measure`], plus the tracking the label adds after every glyph but the last.
fn measure_tracked(text: &str, font: &ab_glyph::FontVec, px: f32, tracking: f32) -> f32 {
    let gaps = text.chars().count().saturating_sub(1) as f32;
    measure(text, font, px) + tracking * gaps
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

    /// Top and bottom edge of the painted pixels inside `rect`.
    fn ink_height(frame: &Frame, rect: Rect, background: (u8, u8, u8)) -> u32 {
        let mut min = u32::MAX;
        let mut max = 0;
        for y in rect.y as u32..(rect.y + rect.h) as u32 {
            for x in rect.x as u32..(rect.x + rect.w) as u32 {
                if frame.pixel(x, y) != background {
                    min = min.min(y);
                    max = max.max(y);
                }
            }
        }
        max.saturating_sub(min)
    }

    /// Whether any pixel inside `rect` has exactly `expected`.
    fn has_colour(frame: &Frame, rect: Rect, expected: (u8, u8, u8)) -> bool {
        for y in rect.y as u32..(rect.y + rect.h) as u32 {
            for x in rect.x as u32..(rect.x + rect.w) as u32 {
                if frame.pixel(x, y) == expected {
                    return true;
                }
            }
        }
        false
    }

    /// How many pixels of the whole frame have exactly `expected`.
    fn colour_count(frame: &Frame, expected: (u8, u8, u8)) -> usize {
        let mut count = 0;
        for y in 0..frame.height() {
            for x in 0..frame.width() {
                if frame.pixel(x, y) == expected {
                    count += 1;
                }
            }
        }
        count
    }

    /// Mean position of the pixels painted exactly with `expected`.
    fn colour_centre(frame: &Frame, expected: (u8, u8, u8)) -> Option<(f32, f32)> {
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut count = 0.0;
        for y in 0..frame.height() {
            for x in 0..frame.width() {
                if frame.pixel(x, y) == expected {
                    sum_x += x as f32;
                    sum_y += y as f32;
                    count += 1.0;
                }
            }
        }
        (count > 0.0).then(|| (sum_x / count, sum_y / count))
    }

    /// How many pixels inside `rect` differ between two renders.
    fn differing_pixels(left: &Frame, right: &Frame, rect: Rect) -> usize {
        let mut count = 0;
        for y in rect.y as u32..(rect.y + rect.h) as u32 {
            for x in rect.x as u32..(rect.x + rect.w) as u32 {
                if left.pixel(x, y) != right.pixel(x, y) {
                    count += 1;
                }
            }
        }
        count
    }

    /// How many pixels outside `rect` differ between two renders.
    fn differing_outside(left: &Frame, right: &Frame, rect: Rect) -> usize {
        let mut count = 0;
        for y in 0..left.height() {
            for x in 0..left.width() {
                if !rect.contains(x as f32, y as f32) && left.pixel(x, y) != right.pixel(x, y) {
                    count += 1;
                }
            }
        }
        count
    }

    /// Top row of the painted pixels inside `rect`.
    fn ink_top(frame: &Frame, rect: Rect, background: (u8, u8, u8)) -> Option<u32> {
        let mut min = u32::MAX;
        let mut max = 0;
        for y in rect.y as u32..(rect.y + rect.h) as u32 {
            for x in rect.x as u32..(rect.x + rect.w) as u32 {
                if frame.pixel(x, y) != background {
                    min = min.min(y);
                    max = max.max(y);
                }
            }
        }
        (max > min).then_some(min)
    }

    /// A hidden line must lose its own ink and change nothing outside its box.
    fn assert_only_this_line_disappeared(
        shown: &Frame,
        hidden: &Frame,
        rect: Rect,
        background: (u8, u8, u8),
    ) {
        assert!(
            ink_inside(shown, rect, background) > 0,
            "the line is painted"
        );
        assert_eq!(ink_inside(hidden, rect, background), 0, "no ink left");
        assert!(
            differing_pixels(shown, hidden, rect) > 0,
            "the pixels changed"
        );
        assert_eq!(
            differing_outside(shown, hidden, rect),
            0,
            "nothing else changes"
        );
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

    /// Horizontal span of the pixels painted with the theme background (the widget card).
    fn card_span(frame: &Frame) -> (u32, u32) {
        let mut min = u32::MAX;
        let mut max = 0;
        for y in 0..frame.height() {
            for x in 0..frame.width() {
                if frame.pixel(x, y) == (0x0f, 0x11, 0x15) {
                    min = min.min(x);
                    max = max.max(x);
                }
            }
        }
        (min, max)
    }

    /// Horizontal span of every pixel that is not the webcam colour: the whole counter layer,
    /// turn included, whether or not the resampling kept the card's exact background colour.
    fn layer_span(frame: &Frame, webcam: (u8, u8, u8)) -> (u32, u32) {
        let mut min = u32::MAX;
        let mut max = 0;
        for y in 0..frame.height() {
            for x in 0..frame.width() {
                if frame.pixel(x, y) != webcam {
                    min = min.min(x);
                    max = max.max(x);
                }
            }
        }
        (min, max)
    }

    #[test]
    fn mirroring_the_source_flips_the_webcam_but_not_the_card() {
        let mut input = input();
        input.display.layout = Layout::Widget;
        input.display.mirror.source_h = true;
        let webcam = (0x22, 0x44, 0x66);
        let mut background = Frame::filled(FRAME_WIDTH, FRAME_HEIGHT, webcam);
        background.set_pixel(0, 0, (255, 0, 0));

        let frame = render(&input, &fonts(), 1_060_000, Some(&background));
        assert_eq!(
            frame.pixel(FRAME_WIDTH - 1, 0),
            (255, 0, 0),
            "the marker moved to the other edge"
        );
        assert_eq!(frame.pixel(0, 0), webcam);
        let (min, _) = card_span(&frame);
        assert!(
            min > FRAME_WIDTH / 2,
            "the card stays bottom-right: starts at {min}"
        );
    }

    #[test]
    fn mirroring_the_overlay_flips_the_card_but_not_the_webcam() {
        let mut input = input();
        input.display.layout = Layout::Widget;
        let webcam = (0x22, 0x44, 0x66);
        let mut background = Frame::filled(FRAME_WIDTH, FRAME_HEIGHT, webcam);
        background.set_pixel(0, 0, (255, 0, 0));

        let normal = render(&input, &fonts(), 1_060_000, Some(&background));
        let (_, normal_max) = card_span(&normal);
        assert!(normal_max > FRAME_WIDTH / 2, "card on the right by default");

        input.display.mirror.overlay_h = true;
        let mirrored = render(&input, &fonts(), 1_060_000, Some(&background));
        let (mirrored_min, mirrored_max) = card_span(&mirrored);
        assert!(
            mirrored_max < FRAME_WIDTH / 2,
            "the card moved to the left: ends at {mirrored_max}"
        );
        assert!(mirrored_min < FRAME_WIDTH / 2);
        assert_eq!(
            mirrored.pixel(0, 0),
            (255, 0, 0),
            "the webcam keeps its orientation"
        );
        assert_eq!(
            mirrored.pixel(FRAME_WIDTH - 1, FRAME_HEIGHT - 1),
            webcam,
            "the mirrored card no longer covers the bottom-right corner"
        );
    }

    /// ADR-0022 flips the counter layer, so the turn has to follow the card to where the flip
    /// left it. Turning around the unmirrored centre would fling it out of the frame.
    #[test]
    fn a_mirrored_card_turns_around_its_mirrored_centre() {
        let fonts = fonts();
        let mut input = input();
        input.display.layout = Layout::Widget;
        input.display.mirror.overlay_h = true;
        let webcam = (0x22, 0x44, 0x66);
        let background = Frame::filled(FRAME_WIDTH, FRAME_HEIGHT, webcam);

        let upright = render(&input, &fonts, 1_060_000, Some(&background));
        let (left, right) = layer_span(&upright, webcam);
        assert!(
            right < FRAME_WIDTH / 2,
            "the flip parks the card on the left: ends at {right}"
        );
        let centre = (left + right) as f32 / 2.0;

        input.display.rotation = 90.0;
        let turned = render(&input, &fonts, 1_060_000, Some(&background));
        let (turned_left, turned_right) = layer_span(&turned, webcam);
        assert!(
            turned_right > turned_left,
            "the card is still drawn somewhere after the turn"
        );
        let turned_centre = (turned_left + turned_right) as f32 / 2.0;
        assert!(
            (turned_centre - centre).abs() <= 2.0,
            "the turn keeps the card around its mirrored centre: {turned_centre} vs {centre}"
        );
        assert!(
            turned_centre < FRAME_WIDTH as f32 / 2.0,
            "the turned card stays on the left: {turned_centre}"
        );
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
            r##"{"display":{"theme":"nope","layout":"widget","overflow":"shrink","elements":{"cost":{"font":"mono","scale":900,"color":"#FF0000"},"elapsed":{"scale":10,"color":"not-a-colour"}},"background":{"mode":"solid","color":"#fff","opacity":900,"radius":-5}}}"##,
        )
        .expect("display parses");
        assert_eq!(
            input.display.theme, "nope",
            "unknown themes fall back when rendering"
        );
        assert_eq!(input.display.layout, Layout::Widget);
        assert_eq!(input.display.overflow, Overflow::Shrink);
        assert_eq!(
            input.display.elements.cost.font,
            FontChoice::Font(theme::FontId::Mono)
        );
        assert_eq!(input.display.elements.cost.scale, 2.5, "clamped to 250%");
        assert_eq!(input.display.elements.cost.color, Some((255, 0, 0)));
        assert_eq!(input.display.elements.elapsed.scale, 0.5, "clamped to 50%");
        assert_eq!(
            input.display.elements.elapsed.color, None,
            "an invalid colour keeps the preset"
        );
        assert_eq!(
            input.display.elements.label,
            ElementStyle::default(),
            "elements left out keep the preset"
        );
        assert_eq!(input.display.background.mode, BackgroundMode::Solid);
        assert_eq!(
            input.display.background.color,
            Some((255, 255, 255)),
            "#fff expands to #ffffff"
        );
        assert_eq!(input.display.background.opacity, 1.0, "clamped to 100%");
        assert_eq!(input.display.background.radius, Some(0.0), "clamped to 0");
        assert_eq!(input.display.overlay, None, "no overlay by default");
    }

    #[test]
    fn an_omitted_background_mode_means_gradient_with_the_theme_colours() {
        let input: CounterInput =
            serde_json::from_str(r#"{"display":{"background":{"radius":0}}}"#)
                .expect("display parses");
        let background = input.display.background;
        assert_eq!(
            background.mode,
            BackgroundMode::Gradient,
            "ADR-0029 default"
        );
        assert_eq!(background.direction, GradientDirection::Vertical);
        assert_eq!(background.opacity, 1.0);
        assert_eq!(background.radius, Some(0.0));
        assert_eq!((background.color, background.color_alt), (None, None));

        let frame = render(&input, &fonts(), 1_060_000, None);
        let top = frame.pixel(5, 2);
        let bottom = frame.pixel(5, FRAME_HEIGHT - 2);
        assert_ne!(top, bottom, "the preset painted a gradient");
        assert!(
            top.0 <= 0x10 && top.1 <= 0x12 && top.2 <= 0x16,
            "the gradient starts at the theme background: {top:?}"
        );
        assert!(
            bottom.0 >= 0x17 && bottom.1 >= 0x1a && bottom.2 >= 0x21,
            "the gradient ends at the theme backgroundAlt: {bottom:?}"
        );
    }

    #[test]
    fn a_custom_overlay_rect_places_the_card() {
        let mut input = input();
        input.display.layout = Layout::Widget;
        input.display.overlay = Some(Overlay {
            x: 0.05,
            y: 0.1,
            w: 0.4,
            h: 0.3,
        });
        let webcam = (0x22, 0x44, 0x66);
        let background = Frame::filled(FRAME_WIDTH, FRAME_HEIGHT, webcam);
        let frame = render(&input, &fonts(), 1_060_000, Some(&background));
        // Inside the card, between its top edge (72) and the padded content (100).
        assert_eq!(frame.pixel(320, 80), (0x0f, 0x11, 0x15));
        assert_eq!(
            frame.pixel(FRAME_WIDTH - 2, FRAME_HEIGHT - 2),
            webcam,
            "the old bottom-right corner is webcam again"
        );
    }

    #[test]
    fn a_corrupt_overlay_falls_back_to_the_corner() {
        let broken = Overlay {
            x: f32::NAN,
            y: 0.1,
            w: 0.4,
            h: 0.3,
        };
        assert!(broken.sanitized().is_none());
        let ok = Overlay {
            x: 0.5,
            y: 0.5,
            w: 0.01,
            h: 2.0,
        }
        .sanitized()
        .expect("finite values are usable");
        assert_eq!((ok.w, ok.h), (0.05, 1.0), "clamped to a usable card");
    }

    #[test]
    fn parses_the_overlay_rect_from_the_ui() {
        let input: CounterInput = serde_json::from_str(
            r#"{"display":{"layout":"widget","overlay":{"x":0.1,"y":0.2,"w":0.3,"h":0.4}}}"#,
        )
        .expect("overlay parses");
        let overlay = input.display.overlay.expect("overlay");
        assert_eq!(
            (overlay.x, overlay.y, overlay.w, overlay.h),
            (0.1, 0.2, 0.3, 0.4)
        );
    }

    #[test]
    fn a_payload_without_elements_or_background_keeps_the_preset_look() {
        // Older web builds send neither `elements` nor `background`: the renderer must keep
        // the look it had before ADR-0029 (preset fonts, colours, sizes and a flat panel).
        let input: CounterInput = serde_json::from_str(
            r#"{"startAt":1000000,"ratePerMs":0.01,"endAt":1060000,"display":{"theme":"classic"}}"#,
        )
        .expect("payload parses");
        assert_eq!(input.display.elements, Elements::default());
        assert_eq!(input.display.background, BackgroundStyle::default());
        assert_eq!(
            input.display.background.mode,
            BackgroundMode::Solid,
            "no gradient without a background object"
        );

        let fonts = fonts();
        let frame = render(&input, &fonts, 1_100_000, None);
        let background = (0x0f, 0x11, 0x15);
        assert_eq!(frame.pixel(0, 0), background, "flat theme background");
        assert_eq!(frame.pixel(FRAME_WIDTH - 1, FRAME_HEIGHT - 1), background);
        let layout = counter_layout(FRAME_WIDTH as f32, FRAME_HEIGHT as f32, Layout::Fullscreen);
        assert!(ink_inside(&frame, layout.cost, background) > 0);
        assert!(ink_inside(&frame, layout.time, background) > 0);
        assert!(ink_inside(&frame, layout.footer, background) > 0);

        // The widget card is flat as well: same colour at its top and at its bottom.
        let mut widget = input;
        widget.display.layout = Layout::Widget;
        let frame = render(&widget, &fonts, 1_100_000, None);
        let layout = counter_layout(FRAME_WIDTH as f32, FRAME_HEIGHT as f32, Layout::Widget);
        let card = layout.card.expect("card");
        let centre = (card.x + card.w / 2.0) as u32;
        assert_eq!(frame.pixel(centre, card.y as u32 + 2), background);
        assert_eq!(
            frame.pixel(centre, (card.y + card.h - 2.0) as u32),
            background,
            "no gradient: the bottom matches the top"
        );
    }

    #[test]
    fn a_transparent_background_leaves_the_webcam_and_still_draws_the_text() {
        let mut input = input();
        input.display.layout = Layout::Widget;
        input.display.background = BackgroundStyle {
            mode: BackgroundMode::Solid,
            color: Some((255, 255, 255)),
            opacity: 0.0,
            ..BackgroundStyle::default()
        };
        let webcam = (0x22, 0x44, 0x66);
        let frame = render(
            &input,
            &fonts(),
            1_060_000,
            Some(&Frame::filled(FRAME_WIDTH, FRAME_HEIGHT, webcam)),
        );
        let layout = counter_layout(FRAME_WIDTH as f32, FRAME_HEIGHT as f32, Layout::Widget);
        let card = layout.card.expect("card");
        assert_eq!(
            frame.pixel((card.x + card.w / 2.0) as u32, card.y as u32 + 2),
            webcam,
            "the card paint is gone, so the webcam shows through"
        );
        assert!(
            ink_inside(&frame, layout.cost, webcam) > 0,
            "the text is still drawn over the webcam"
        );

        // Fullscreen has no webcam: the black canvas shows through instead of the panel.
        let mut fullscreen = input;
        fullscreen.display.layout = Layout::Fullscreen;
        let frame = render(&fullscreen, &fonts(), 1_060_000, None);
        assert_eq!(frame.pixel(0, 0), (0, 0, 0), "black canvas");
        let layout = counter_layout(FRAME_WIDTH as f32, FRAME_HEIGHT as f32, Layout::Fullscreen);
        assert!(ink_inside(&frame, layout.cost, (0, 0, 0)) > 0);
    }

    #[test]
    fn a_half_transparent_background_blends_with_what_is_behind() {
        let mut input = input();
        input.display.background = BackgroundStyle {
            mode: BackgroundMode::Solid,
            color: Some((255, 255, 255)),
            opacity: 0.5,
            ..BackgroundStyle::default()
        };
        let behind = (0x00, 0x20, 0x40);
        let frame = render(
            &input,
            &fonts(),
            1_060_000,
            Some(&Frame::filled(FRAME_WIDTH, FRAME_HEIGHT, behind)),
        );
        let blended = frame.pixel(0, 0);
        // alpha = round(0.5 * 255) = 128: out = behind * (1 - a) + white * a.
        assert!(
            (i32::from(blended.0) - 128).abs() <= 2
                && (i32::from(blended.1) - 144).abs() <= 2
                && (i32::from(blended.2) - 160).abs() <= 2,
            "half way between {behind:?} and white: {blended:?}"
        );
    }

    #[test]
    fn a_gradient_background_runs_along_its_direction() {
        let mut input = input();
        input.display.background = BackgroundStyle {
            mode: BackgroundMode::Gradient,
            color: Some((0, 0, 0)),
            color_alt: Some((255, 255, 255)),
            direction: GradientDirection::Vertical,
            opacity: 1.0,
            radius: None,
        };
        let frame = render(&input, &fonts(), 1_060_000, None);
        let top = frame.pixel(5, 2);
        let bottom = frame.pixel(5, FRAME_HEIGHT - 2);
        assert!(top.0 < bottom.0, "top to bottom: {top:?} -> {bottom:?}");
        assert_eq!(
            frame.pixel(2, 5),
            frame.pixel(FRAME_WIDTH - 2, 5),
            "a vertical gradient does not change left to right"
        );

        input.display.background.direction = GradientDirection::Horizontal;
        let frame = render(&input, &fonts(), 1_060_000, None);
        let left = frame.pixel(2, 5);
        let right = frame.pixel(FRAME_WIDTH - 2, 5);
        assert!(left.0 < right.0, "left to right: {left:?} -> {right:?}");
        assert_eq!(
            frame.pixel(5, 2),
            frame.pixel(5, FRAME_HEIGHT - 2),
            "a horizontal gradient does not change top to bottom"
        );
    }

    #[test]
    fn the_background_radius_controls_the_card_corners() {
        let mut input = input();
        input.display.layout = Layout::Widget;
        input.display.background = BackgroundStyle {
            mode: BackgroundMode::Solid,
            color: Some((255, 255, 255)),
            opacity: 1.0,
            radius: Some(0.0),
            ..BackgroundStyle::default()
        };
        let webcam = (0x22, 0x44, 0x66);
        let webcam_frame = Frame::filled(FRAME_WIDTH, FRAME_HEIGHT, webcam);
        let square = render(&input, &fonts(), 1_060_000, Some(&webcam_frame));
        let layout = counter_layout(FRAME_WIDTH as f32, FRAME_HEIGHT as f32, Layout::Widget);
        let card = layout.card.expect("card");
        let (corner_x, corner_y) = (card.x as u32, card.y as u32);
        assert_eq!(
            square.pixel(corner_x, corner_y),
            (255, 255, 255),
            "radius 0 gives a square corner"
        );
        let painted = colour_count(&square, (255, 255, 255));

        input.display.background.radius = Some(40.0);
        let rounded = render(&input, &fonts(), 1_060_000, Some(&webcam_frame));
        assert_eq!(
            rounded.pixel(corner_x, corner_y),
            webcam,
            "radius 40 clips the corner"
        );
        assert!(
            colour_count(&rounded, (255, 255, 255)) < painted,
            "rounded corners paint less of the card"
        );
        assert_ne!(
            rounded.pixel(corner_x + 40, corner_y + 1),
            webcam,
            "the card starts further in at the top edge"
        );
    }

    #[test]
    fn per_element_colours_override_the_preset_and_null_keeps_it() {
        let layout = counter_layout(FRAME_WIDTH as f32, FRAME_HEIGHT as f32, Layout::Fullscreen);
        let classic = theme::theme("classic", 1_060_000);

        let mut overridden = input();
        overridden.display.elements.cost.color = Some((255, 0, 0));
        overridden.display.elements.elapsed.color = Some((0, 255, 0));
        overridden.display.elements.cost_per_minute.color = Some((0, 0, 255));
        let frame = render(&overridden, &fonts(), 1_060_000, None);
        assert!(
            has_colour(&frame, layout.cost, (255, 0, 0)),
            "cost override"
        );
        assert!(
            has_colour(&frame, layout.time, (0, 255, 0)),
            "elapsed override"
        );
        assert!(
            has_colour(&frame, layout.footer, (0, 0, 255)),
            "cost-per-minute override"
        );
        assert!(!has_colour(&frame, layout.time, classic.muted));

        // `null` keeps the preset: the accent while running, the muted colour for the rest.
        let frame = render(&input(), &fonts(), 1_060_000, None);
        assert!(
            has_colour(&frame, layout.cost, classic.accent),
            "a running cost keeps the accent ({:?})",
            classic.accent
        );
        assert!(has_colour(&frame, layout.time, classic.muted));
        assert!(has_colour(&frame, layout.footer, classic.muted));

        let mut stopped = input();
        stopped.end_at = Some(1_030_000);
        let frame = render(&stopped, &fonts(), 1_060_000, None);
        assert!(
            has_colour(&frame, layout.cost, classic.text),
            "a stopped meeting draws the cost in the text colour"
        );
    }

    #[test]
    fn per_element_fonts_fall_back_to_the_preset_or_use_the_explicit_font() {
        let input: CounterInput = serde_json::from_str(
            r#"{"display":{"elements":{"cost":{"font":"mono"},"elapsed":{"font":"theme"},"label":{"font":"nope"}}}}"#,
        )
        .expect("display parses");
        assert_eq!(
            input.display.elements.cost.font,
            FontChoice::Font(theme::FontId::Mono)
        );
        assert_eq!(input.display.elements.elapsed.font, FontChoice::Theme);
        assert_eq!(
            input.display.elements.label.font,
            FontChoice::Theme,
            "unknown font ids keep the preset"
        );
        assert_eq!(
            input.display.elements.cost_per_minute,
            ElementStyle::default()
        );

        // The explicit face really reaches the drawing.
        let layout = counter_layout(FRAME_WIDTH as f32, FRAME_HEIGHT as f32, Layout::Fullscreen);
        let mut preset = input;
        preset.display.elements.cost.font = FontChoice::Theme;
        let preset_frame = render(&preset, &fonts(), 1_060_000, None);
        preset.display.elements.cost.font = FontChoice::Font(theme::FontId::Mono);
        let mono_frame = render(&preset, &fonts(), 1_060_000, None);
        assert!(
            differing_pixels(&preset_frame, &mono_frame, layout.cost) > 0,
            "the mono face changes the glyphs"
        );
    }

    #[test]
    fn an_element_scale_changes_its_size() {
        let layout = counter_layout(FRAME_WIDTH as f32, FRAME_HEIGHT as f32, Layout::Fullscreen);
        let background = (0x0f, 0x11, 0x15);
        let mut input = input();
        let preset = render(&input, &fonts(), 1_060_000, None);
        let preset_height = ink_height(&preset, layout.time, background);
        let preset_width = ink_bounds(&preset, layout.time, background)
            .map(|(min, max)| max - min)
            .unwrap_or(0);

        input.display.elements.elapsed.scale = 2.0;
        let doubled = render(&input, &fonts(), 1_060_000, None);
        let doubled_height = ink_height(&doubled, layout.time, background);
        let doubled_width = ink_bounds(&doubled, layout.time, background)
            .map(|(min, max)| max - min)
            .unwrap_or(0);
        assert!(
            doubled_height > preset_height,
            "{doubled_height} px tall at 200% vs {preset_height} px at 100%"
        );
        assert!(
            doubled_width > preset_width,
            "{doubled_width} px wide at 200% vs {preset_width} px at 100%"
        );
    }

    #[test]
    fn elapsed_styles_the_time_line() {
        let input: CounterInput = serde_json::from_str(
            r##"{"startAt":1000000,"ratePerMs":0.01,"display":{"elements":{"elapsed":{"font":"mono","scale":200,"color":"#00FF00"}}}}"##,
        )
        .expect("payload parses");
        assert_eq!(
            input.display.elements.elapsed.font,
            FontChoice::Font(theme::FontId::Mono)
        );
        assert_eq!(input.display.elements.elapsed.scale, 2.0);
        assert_eq!(input.display.elements.elapsed.color, Some((0, 255, 0)));

        let layout = counter_layout(FRAME_WIDTH as f32, FRAME_HEIGHT as f32, Layout::Fullscreen);
        let frame = render(&input, &fonts(), 1_060_000, None);
        assert!(
            has_colour(&frame, layout.time, (0, 255, 0)),
            "`elapsed` paints the elapsed-time line"
        );
        assert!(
            !has_colour(&frame, layout.footer, (0, 255, 0)),
            "`elapsed` does not leak into the cost-per-minute footer"
        );
    }

    #[test]
    fn an_old_payload_styles_the_time_line_through_the_time_alias() {
        let input: CounterInput = serde_json::from_str(
            r##"{"display":{"elements":{"time":{"font":"mono","scale":200,"color":"#FF0000"}}}}"##,
        )
        .expect("older payloads still parse");
        assert_eq!(
            input.display.elements.elapsed.font,
            FontChoice::Font(theme::FontId::Mono)
        );
        assert_eq!(input.display.elements.elapsed.scale, 2.0);
        assert_eq!(input.display.elements.elapsed.color, Some((255, 0, 0)));

        let layout = counter_layout(FRAME_WIDTH as f32, FRAME_HEIGHT as f32, Layout::Fullscreen);
        let frame = render(&input, &fonts(), 1_060_000, None);
        assert!(
            has_colour(&frame, layout.time, (255, 0, 0)),
            "`time` still lands on the elapsed-time line"
        );
    }

    #[test]
    fn cost_per_minute_styles_the_footer() {
        let layout = counter_layout(FRAME_WIDTH as f32, FRAME_HEIGHT as f32, Layout::Fullscreen);
        let background = (0x0f, 0x11, 0x15);
        let preset = render(&input(), &fonts(), 1_060_000, None);
        let preset_height = ink_height(&preset, layout.footer, background);

        let styled: CounterInput = serde_json::from_str(
            r##"{"startAt":1000000,"ratePerMs":0.01,"display":{"elements":{"costPerMinute":{"font":"mono","scale":200,"color":"#0000FF"}}}}"##,
        )
        .expect("payload parses");
        assert_eq!(
            styled.display.elements.cost_per_minute.font,
            FontChoice::Font(theme::FontId::Mono)
        );
        assert_eq!(styled.display.elements.cost_per_minute.scale, 2.0);
        assert_eq!(
            styled.display.elements.cost_per_minute.color,
            Some((0, 0, 255))
        );

        let frame = render(&styled, &fonts(), 1_060_000, None);
        assert!(
            has_colour(&frame, layout.footer, (0, 0, 255)),
            "`costPerMinute` paints the footer"
        );
        assert!(
            !has_colour(&frame, layout.time, (0, 0, 255)),
            "`costPerMinute` does not leak into the elapsed line"
        );
        assert!(
            ink_height(&frame, layout.footer, background) > preset_height,
            "200 % grows the footer glyphs"
        );
    }

    #[test]
    fn status_and_person_time_styles_are_accepted_but_never_drawn() {
        // The web customises a status line and the person-time stats grid; the camera draws
        // neither (no room in the counter card), so a loud style on them changes no pixel.
        let ignored: CounterInput = serde_json::from_str(
            r##"{"startAt":1000000,"ratePerMs":0.01,"display":{"elements":{"status":{"font":"mono","scale":250,"color":"#FF0000","visible":false},"personTime":{"font":"mono","scale":250,"color":"#00FF00","visible":false}}}}"##,
        )
        .expect("the camera accepts the styles the web sends");

        let fonts = fonts();
        let frame = render(&ignored, &fonts, 1_060_000, None);
        let preset = render(&input(), &fonts, 1_060_000, None);
        assert_eq!(
            frame, preset,
            "the camera draws neither a status line nor the person-time stats grid"
        );
    }

    /// The layout `render` builds for a fullscreen frame with the label at `scale`.
    fn fullscreen_layout(scale: f32) -> crate::camera::layout::CounterLayout {
        counter_layout_with_label(
            FRAME_WIDTH as f32,
            FRAME_HEIGHT as f32,
            Layout::Fullscreen,
            None,
            Some(scale),
        )
    }

    #[test]
    fn fullscreen_paints_the_label_above_the_cost() {
        let fonts = fonts();
        let background = (0x0f, 0x11, 0x15);
        let layout = fullscreen_layout(1.0);
        let band = layout.label.expect("fullscreen reserves a label band");

        let frame = render(&input(), &fonts, 1_060_000, None);
        let preset_ink = ink_inside(&frame, band, background);
        assert!(preset_ink > 0, "the label is painted in its band");
        assert_eq!(
            ink_outside(&frame, layout.content, background),
            0,
            "the label neither overlaps nor overflows the content"
        );
        let preset_height = ink_height(&frame, band, background);
        assert!(
            preset_height as f32 > band.h * 0.4,
            "{preset_height} px of ink in a {} px band: about 4 % of the frame height",
            band.h
        );
        assert!(
            ink_inside(&frame, layout.cost, background) > 0,
            "the cost is still drawn below the label"
        );

        // A style override reaches the glyphs: 200 % paints a bigger label.
        let mut scaled_input = input();
        scaled_input.display.elements.label.scale = 2.0;
        let scaled_layout = fullscreen_layout(2.0);
        let scaled_band = scaled_layout.label.expect("band");
        let scaled = render(&scaled_input, &fonts, 1_060_000, None);
        let scaled_ink = ink_inside(&scaled, scaled_band, background);
        assert!(
            scaled_band.h > band.h && scaled_ink > preset_ink,
            "200 % paints {scaled_ink} px vs {preset_ink} px at 100 %"
        );
        assert!(
            ink_height(&scaled, scaled_band, background) > preset_height,
            "the label glyphs grow with the scale"
        );
        assert!(
            ink_inside(&scaled, scaled_layout.cost, background) > 0,
            "the cost keeps its (smaller) box and its fit/overflow rules"
        );
    }

    #[test]
    fn the_widget_never_draws_the_label() {
        let fonts = fonts();
        let mut widget = input();
        widget.display.layout = Layout::Widget;
        let webcam = Frame::filled(FRAME_WIDTH, FRAME_HEIGHT, (0x22, 0x44, 0x66));
        let without = render(&widget, &fonts, 1_060_000, Some(&webcam));

        // Same frame with a label the web would hide in the widget: nothing may change.
        let mut labelled = widget.clone();
        labelled.labels.total_cost = "Total cost".into();
        labelled.display.elements.label.scale = 2.5;
        labelled.display.elements.label.color = Some((255, 0, 0));
        labelled.display.elements.label.font = FontChoice::Font(theme::FontId::Mono);
        let with = render(&labelled, &fonts, 1_060_000, Some(&webcam));

        assert_eq!(without, with, "widget frames stay pixel-identical");
        let whole_frame = Rect {
            x: 0.0,
            y: 0.0,
            w: FRAME_WIDTH as f32,
            h: FRAME_HEIGHT as f32,
        };
        assert!(
            !has_colour(&with, whole_frame, (255, 0, 0)),
            "the label colour never reaches a widget frame"
        );
    }

    #[test]
    fn an_empty_label_draws_nothing_and_keeps_todays_layout() {
        let fonts = fonts();
        let background = (0x0f, 0x11, 0x15);
        let mut input = input();
        input.labels.total_cost = String::new();
        let blank = render(&input, &fonts, 1_060_000, None);

        // Today's geometry: the cost, the time and the status keep their boxes.
        let today = counter_layout(FRAME_WIDTH as f32, FRAME_HEIGHT as f32, Layout::Fullscreen);
        assert!(ink_inside(&blank, today.cost, background) > 0);
        assert!(ink_inside(&blank, today.time, background) > 0);
        assert!(ink_inside(&blank, today.footer, background) > 0);

        // A style that would be loud still paints nothing when the text is empty.
        let mut styled = input.clone();
        styled.display.elements.label.scale = 2.5;
        styled.display.elements.label.color = Some((255, 0, 0));
        let styled_frame = render(&styled, &fonts, 1_060_000, None);
        assert_eq!(blank, styled_frame, "no band, no glyphs");

        // And the text really is what moves the cost down.
        let mut labelled = input;
        labelled.labels.total_cost = "Total cost".into();
        let labelled_frame = render(&labelled, &fonts, 1_060_000, None);
        assert_ne!(blank, labelled_frame, "the label changes the geometry");
    }

    #[test]
    fn the_label_is_uppercased_like_the_web() {
        let fonts = fonts();
        let mut input = input();
        input.labels.total_cost = "Total cost".into();
        let mixed_case = render(&input, &fonts, 1_060_000, None);
        input.labels.total_cost = "TOTAL COST".into();
        let uppercase = render(&input, &fonts, 1_060_000, None);
        assert_eq!(mixed_case, uppercase, "`text-transform: uppercase`");
    }

    #[test]
    fn the_label_keeps_the_muted_preset_when_its_colour_is_null() {
        let fonts = fonts();
        let band = fullscreen_layout(1.0).label.expect("band");
        let classic = theme::theme("classic", 1_060_000);

        let preset = render(&input(), &fonts, 1_060_000, None);
        assert!(
            has_colour(&preset, band, classic.muted),
            "color: null keeps the muted preset ({:?})",
            classic.muted
        );

        let mut overridden = input();
        overridden.display.elements.label.color = Some((255, 0, 0));
        let frame = render(&overridden, &fonts, 1_060_000, None);
        assert!(
            has_colour(&frame, band, (255, 0, 0)),
            "an explicit colour replaces the preset"
        );
        assert!(
            colour_count(&frame, classic.muted) < colour_count(&preset, classic.muted),
            "the label no longer paints muted pixels"
        );
    }

    #[test]
    fn a_payload_without_visibility_or_rotation_keeps_every_line_visible_and_upright() {
        // The web sends both fields now; older payloads omit them and must render as before.
        let payload: CounterInput = serde_json::from_str(
            r#"{"startAt":1000000,"ratePerMs":0.01,"display":{"theme":"classic"}}"#,
        )
        .expect("payload parses");
        assert_eq!(payload.display.rotation, 0.0, "no rotation by default");
        assert_eq!(payload.display.elements, Elements::default());
        for style in [
            payload.display.elements.label,
            payload.display.elements.cost,
            payload.display.elements.elapsed,
            payload.display.elements.cost_per_minute,
        ] {
            assert!(style.visible, "an absent `visible` means visible");
        }

        let fonts = fonts();
        assert_eq!(
            render(&payload, &fonts, 1_060_000, None),
            render(&input(), &fonts, 1_060_000, None),
            "the defaults render exactly like today"
        );
    }

    #[test]
    fn parses_per_element_visibility_and_the_rotation() {
        let input: CounterInput = serde_json::from_str(
            r#"{"display":{"rotation":90,"elements":{"cost":{"visible":false},"status":{"visible":false},"personTime":{"visible":false}}}}"#,
        )
        .expect("display parses");
        assert_eq!(input.display.rotation, 90.0);
        assert!(!input.display.elements.cost.visible);
        assert!(input.display.elements.label.visible);
        assert!(input.display.elements.elapsed.visible);
        assert!(input.display.elements.cost_per_minute.visible);

        let clamped: CounterInput =
            serde_json::from_str(r#"{"display":{"rotation":400}}"#).expect("display parses");
        assert_eq!(clamped.display.rotation, 360.0, "clamped to a full turn");
        let negative: CounterInput =
            serde_json::from_str(r#"{"display":{"rotation":-20}}"#).expect("display parses");
        assert_eq!(negative.display.rotation, 0.0, "clamped to upright");
    }

    #[test]
    fn hiding_the_label_in_fullscreen_gives_its_band_back_to_the_cost() {
        let fonts = fonts();
        let background = (0x0f, 0x11, 0x15);
        let classic = theme::theme("classic", 1_060_000);
        let labelled_layout = fullscreen_layout(1.0);
        let band = labelled_layout.label.expect("band");
        let today = counter_layout(FRAME_WIDTH as f32, FRAME_HEIGHT as f32, Layout::Fullscreen);

        let shown = render(&input(), &fonts, 1_060_000, None);
        assert!(
            ink_inside(&shown, band, background) > 0,
            "the label is painted"
        );
        let shown_top = ink_top(&shown, labelled_layout.cost, background).expect("cost ink");

        let mut hidden_input = input();
        hidden_input.display.elements.label.visible = false;
        let hidden = render(&hidden_input, &fonts, 1_060_000, None);

        assert_eq!(
            ink_inside(&hidden, band, background),
            0,
            "no label ink at all"
        );
        let hidden_top = ink_top(&hidden, today.cost, background).expect("cost ink");
        assert!(
            hidden_top < shown_top,
            "the cost grew into the band and moved up: {hidden_top} px vs {shown_top} px"
        );

        // Hidden is exactly "no label": the band is gone and the stack fills the content.
        let mut blank = input();
        blank.labels.total_cost = String::new();
        assert_eq!(hidden, render(&blank, &fonts, 1_060_000, None));

        // The lines below keep their old boxes and their ink.
        assert!(ink_inside(&hidden, today.time, background) > 0);
        assert!(ink_inside(&hidden, today.footer, background) > 0);
        assert!(has_colour(&hidden, today.cost, classic.accent));
    }

    #[test]
    fn hiding_the_time_line_leaves_the_other_lines_alone() {
        let fonts = fonts();
        let background = (0x0f, 0x11, 0x15);
        let layout = fullscreen_layout(1.0);
        let shown = render(&input(), &fonts, 1_060_000, None);

        let mut hidden_input = input();
        hidden_input.display.elements.elapsed.visible = false;
        let hidden = render(&hidden_input, &fonts, 1_060_000, None);

        assert_only_this_line_disappeared(&shown, &hidden, layout.time, background);
        assert!(
            ink_inside(&hidden, layout.cost, background) > 0,
            "the cost stays where it was"
        );
        assert!(
            ink_inside(&hidden, layout.footer, background) > 0,
            "the footer stays where it was"
        );
    }

    #[test]
    fn hiding_the_footer_leaves_the_other_lines_alone() {
        let fonts = fonts();
        let background = (0x0f, 0x11, 0x15);
        let layout = fullscreen_layout(1.0);
        let shown = render(&input(), &fonts, 1_060_000, None);

        let mut hidden_input = input();
        hidden_input.display.elements.cost_per_minute.visible = false;
        let hidden = render(&hidden_input, &fonts, 1_060_000, None);

        assert_only_this_line_disappeared(&shown, &hidden, layout.footer, background);
        assert!(
            ink_inside(&hidden, layout.cost, background) > 0,
            "the cost stays where it was"
        );
        assert!(
            ink_inside(&hidden, layout.time, background) > 0,
            "the time stays where it was"
        );
    }

    #[test]
    fn a_hidden_cost_only_removes_the_cost_ink() {
        let fonts = fonts();
        let background = (0x0f, 0x11, 0x15);
        let layout = fullscreen_layout(1.0);
        let shown = render(&input(), &fonts, 1_060_000, None);

        let mut hidden_input = input();
        hidden_input.display.elements.cost.visible = false;
        let hidden = render(&hidden_input, &fonts, 1_060_000, None);

        assert_eq!(ink_inside(&hidden, layout.cost, background), 0);
        assert!(ink_inside(&hidden, layout.time, background) > 0);
        assert!(ink_inside(&hidden, layout.footer, background) > 0);
        assert_eq!(
            differing_outside(&shown, &hidden, layout.cost),
            0,
            "nothing else changes"
        );
    }

    #[test]
    fn a_full_turn_renders_exactly_like_no_turn() {
        let fonts = fonts();
        let upright = render(&input(), &fonts, 1_060_000, None);

        let mut zero = input();
        zero.display.rotation = 0.0;
        assert_eq!(
            render(&zero, &fonts, 1_060_000, None),
            upright,
            "0 is today"
        );

        let mut full = input();
        full.display.rotation = 360.0;
        assert_eq!(
            render(&full, &fonts, 1_060_000, None),
            upright,
            "a full turn is upright"
        );
    }

    #[test]
    fn a_quarter_turn_stands_the_widget_card_around_its_own_centre() {
        let fonts = fonts();
        let mut input = input();
        input.display.layout = Layout::Widget;
        let webcam = (0x22, 0x44, 0x66);
        let background = Frame::filled(FRAME_WIDTH, FRAME_HEIGHT, webcam);
        let card = counter_layout(FRAME_WIDTH as f32, FRAME_HEIGHT as f32, Layout::Widget)
            .card
            .expect("card");
        let centre = (
            (card.x + card.w / 2.0) as u32,
            (card.y + card.h / 2.0) as u32,
        );
        // Far from the centre sideways, so the turn moves it off this pixel; and a pixel
        // above the card that only the turned card may reach.
        let corner = ((card.x + card.w) as u32 - 20, (card.y + card.h) as u32 - 20);
        let above = (centre.0, centre.1 - 200);

        let upright = render(&input, &fonts, 1_060_000, Some(&background));
        assert_eq!(
            upright.pixel(corner.0, corner.1),
            (0x0f, 0x11, 0x15),
            "the card covers that pixel upright"
        );
        assert_eq!(upright.pixel(above.0, above.1), webcam);

        input.display.rotation = 90.0;
        let turned = render(&input, &fonts, 1_060_000, Some(&background));
        assert_eq!(
            turned.pixel(corner.0, corner.1),
            webcam,
            "the card left the old bottom-right corner"
        );
        assert_ne!(
            turned.pixel(above.0, above.1),
            webcam,
            "the card stands above its centre after a quarter turn"
        );

        // Only the counter turns: the webcam stays exactly as captured.
        let far_away = Rect {
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 200.0,
        };
        assert_eq!(
            ink_inside(&turned, far_away, webcam),
            0,
            "the webcam outside the card is untouched by the rotation"
        );

        // The turn is clockwise, like `rotate(90deg)` on the web: the cost, which sits above
        // the card centre, ends up to its right.
        let accent = theme::theme("classic", 1_060_000).accent;
        let upright_cost = colour_centre(&upright, accent).expect("the running cost");
        assert!(
            upright_cost.1 < centre.1 as f32,
            "the cost sits above the card centre, at {:?}",
            upright_cost
        );
        let turned_cost = colour_centre(&turned, accent).expect("the running cost");
        assert!(
            turned_cost.0 > centre.0 as f32,
            "a quarter turn moves the cost to the right: {:?}",
            turned_cost
        );
    }

    #[test]
    fn a_quarter_turn_turns_the_fullscreen_layer_around_the_frame_centre() {
        let mut input = input();
        input.display.rotation = 90.0;
        let turned = render(&input, &fonts(), 1_060_000, None);
        assert_eq!(
            turned.pixel(0, 0),
            (0, 0, 0),
            "the black canvas shows where the turned panel ends"
        );
        assert_ne!(
            turned.pixel(FRAME_WIDTH / 2, FRAME_HEIGHT / 2),
            (0, 0, 0),
            "the counter still covers the middle of the frame"
        );
    }
}
