//! Layout math for the counter frame: fullscreen or a widget card over a webcam (ADR-0006).

use crate::camera::theme::{Layout, Overflow};

/// Frame margin, in pixels, when the counter fills the whole frame.
pub const FULLSCREEN_INSET: f32 = 64.0;
/// Distance from the widget card to the bottom-right corner of the frame.
pub const WIDGET_MARGIN: f32 = 32.0;
/// Widget card size as a fraction of the frame.
pub const WIDGET_WIDTH_RATIO: f32 = 0.42;
pub const WIDGET_HEIGHT_RATIO: f32 = 0.30;
/// Inner padding of the widget card.
pub const WIDGET_PADDING: f32 = 28.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn inset(&self, by: f32) -> Rect {
        Rect {
            x: self.x + by,
            y: self.y + by,
            w: (self.w - 2.0 * by).max(0.0),
            h: (self.h - 2.0 * by).max(0.0),
        }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// Where the counter goes inside the frame and how its text is stacked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CounterLayout {
    /// `None` for `fullscreen`; the widget card rectangle otherwise.
    pub card: Option<Rect>,
    /// Area available for the counter content (card minus padding, or the fullscreen inset).
    pub content: Rect,
    pub cost: Rect,
    pub time: Rect,
    pub footer: Rect,
}

pub fn counter_layout(width: f32, height: f32, layout: Layout) -> CounterLayout {
    counter_layout_in(width, height, layout, None)
}

/// Same as [`counter_layout`], but the widget card can be placed anywhere in the frame
/// (`overlay`, as fractions of the frame). `None` keeps the built-in bottom-right corner.
pub fn counter_layout_in(
    width: f32,
    height: f32,
    layout: Layout,
    overlay: Option<Rect>,
) -> CounterLayout {
    let (card, content) = match layout {
        Layout::Fullscreen => (
            None,
            Rect {
                x: 0.0,
                y: 0.0,
                w: width,
                h: height,
            }
            .inset(FULLSCREEN_INSET),
        ),
        Layout::Widget => {
            let card = match overlay {
                Some(custom) => custom_snapped(custom, width, height),
                None => {
                    let w = (width * WIDGET_WIDTH_RATIO).round();
                    let h = (height * WIDGET_HEIGHT_RATIO).round();
                    Rect {
                        x: (width - w - WIDGET_MARGIN).round(),
                        y: (height - h - WIDGET_MARGIN).round(),
                        w,
                        h,
                    }
                }
            };
            // A card dragged small still needs room for the numbers.
            let padding = WIDGET_PADDING.min(card.w * 0.2).min(card.h * 0.2);
            (Some(card), card.inset(padding))
        }
    };
    let cost_h = content.h * 0.62;
    let time_h = content.h * 0.22;
    CounterLayout {
        card,
        content,
        cost: Rect {
            x: content.x,
            y: content.y,
            w: content.w,
            h: cost_h,
        },
        time: Rect {
            x: content.x,
            y: content.y + cost_h,
            w: content.w,
            h: time_h,
        },
        footer: Rect {
            x: content.x,
            y: content.y + cost_h + time_h,
            w: content.w,
            h: content.h - cost_h - time_h,
        },
    }
}

/// Clamps a fractional card to the frame and snaps it to whole pixels, so the card edges
/// stay crisp over the webcam (ADR-0006).
fn custom_snapped(custom: Rect, width: f32, height: f32) -> Rect {
    let w = (custom.w * width).round().clamp(1.0, width.max(1.0));
    let h = (custom.h * height).round().clamp(1.0, height.max(1.0));
    Rect {
        x: (custom.x * width).round().clamp(0.0, (width - w).max(0.0)),
        y: (custom.y * height)
            .round()
            .clamp(0.0, (height - h).max(0.0)),
        w,
        h,
    }
}

/// Result of fitting the cost text: which text to draw and the extra scale factor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FitCost {
    pub compact: bool,
    pub scale: f32,
}

/// Mirror of `fitCost` in `packages/core/src/themes.ts`.
pub fn fit_cost(available: f32, full: f32, compact: f32, mode: Overflow) -> FitCost {
    let shrink = |width: f32| {
        if width <= available {
            1.0
        } else {
            ((available / width) * 100.0).floor() / 100.0
        }
    };
    if full <= available || available <= 0.0 {
        return FitCost {
            compact: false,
            scale: 1.0,
        };
    }
    match mode {
        Overflow::Compact => FitCost {
            compact: true,
            scale: shrink(compact),
        },
        Overflow::Shrink => FitCost {
            compact: false,
            scale: shrink(full),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fullscreen_uses_the_whole_frame_minus_the_inset() {
        let layout = counter_layout(1280.0, 720.0, Layout::Fullscreen);
        assert_eq!(layout.card, None);
        assert_eq!(
            layout.content,
            Rect {
                x: FULLSCREEN_INSET,
                y: FULLSCREEN_INSET,
                w: 1280.0 - 2.0 * FULLSCREEN_INSET,
                h: 720.0 - 2.0 * FULLSCREEN_INSET,
            }
        );
    }

    #[test]
    fn widget_sits_in_the_bottom_right_corner() {
        let layout = counter_layout(1280.0, 720.0, Layout::Widget);
        let card = layout.card.expect("widget layout has a card");
        assert_eq!(card.w, 538.0, "42% of 1280, snapped to whole pixels");
        assert_eq!(card.h, 216.0);
        assert_eq!(card.x, 1280.0 - card.w - WIDGET_MARGIN);
        assert_eq!(card.y, 720.0 - card.h - WIDGET_MARGIN);
        assert_eq!(
            layout.content,
            card.inset(WIDGET_PADDING),
            "the content is the card minus its padding"
        );
    }

    #[test]
    fn a_custom_rectangle_places_the_widget_card_anywhere() {
        let overlay = Rect {
            x: 0.1,
            y: 0.2,
            w: 0.5,
            h: 0.25,
        };
        let layout = counter_layout_in(1280.0, 720.0, Layout::Widget, Some(overlay));
        let card = layout.card.expect("card");
        assert_eq!(card.x, 128.0);
        assert_eq!(card.y, 144.0);
        assert_eq!(card.w, 640.0);
        assert_eq!(card.h, 180.0);
        assert!(card.contains(layout.content.x + 1.0, layout.content.y + 1.0));
    }

    #[test]
    fn a_card_dragged_past_the_edge_is_kept_inside() {
        let overlay = Rect {
            x: 0.9,
            y: 0.9,
            w: 0.5,
            h: 0.5,
        };
        let layout = counter_layout_in(1280.0, 720.0, Layout::Widget, Some(overlay));
        let card = layout.card.expect("card");
        assert!(card.x + card.w <= 1280.0 + 0.01);
        assert!(card.y + card.h <= 720.0 + 0.01);
        assert!(layout.content.w > 0.0 && layout.content.h > 0.0);
    }

    #[test]
    fn fullscreen_ignores_a_custom_rectangle() {
        let overlay = Rect {
            x: 0.5,
            y: 0.5,
            w: 0.2,
            h: 0.2,
        };
        let layout = counter_layout_in(1280.0, 720.0, Layout::Fullscreen, Some(overlay));
        assert_eq!(layout.card, None);
    }

    #[test]
    fn text_boxes_stack_inside_the_content_area() {
        for layout_kind in [Layout::Fullscreen, Layout::Widget] {
            let layout = counter_layout(1280.0, 720.0, layout_kind);
            let content = layout.content;
            assert!(layout.cost.y >= content.y);
            assert!(
                layout.cost.h > layout.time.h,
                "the cost is the biggest value"
            );
            assert_eq!(layout.time.y, layout.cost.y + layout.cost.h);
            assert_eq!(layout.footer.y, layout.time.y + layout.time.h);
            assert!(layout.footer.y + layout.footer.h <= content.y + content.h + 0.01);
            for box_ in [layout.cost, layout.time, layout.footer] {
                assert!(box_.x >= content.x && box_.x + box_.w <= content.x + content.w + 0.01);
            }
        }
    }

    #[test]
    fn fit_cost_matches_the_web_rules() {
        let fits = fit_cost(100.0, 80.0, 60.0, Overflow::Compact);
        assert_eq!(
            fits,
            FitCost {
                compact: false,
                scale: 1.0
            }
        );

        let compact = fit_cost(100.0, 200.0, 120.0, Overflow::Compact);
        assert_eq!(
            compact,
            FitCost {
                compact: true,
                scale: 0.83
            }
        );

        let shrink = fit_cost(100.0, 200.0, 120.0, Overflow::Shrink);
        assert_eq!(
            shrink,
            FitCost {
                compact: false,
                scale: 0.5
            }
        );

        let compact_fits = fit_cost(100.0, 200.0, 90.0, Overflow::Compact);
        assert_eq!(
            compact_fits,
            FitCost {
                compact: true,
                scale: 1.0
            }
        );
    }

    #[test]
    fn rect_helpers_work_in_pixels() {
        let rect = Rect {
            x: 10.0,
            y: 20.0,
            w: 100.0,
            h: 50.0,
        };
        assert!(rect.contains(10.0, 20.0));
        assert!(rect.contains(109.0, 69.0));
        assert!(!rect.contains(110.0, 20.0));
        assert_eq!(
            rect.inset(5.0),
            Rect {
                x: 15.0,
                y: 25.0,
                w: 90.0,
                h: 40.0
            }
        );
    }
}
