//! Camera pipeline: frame rendering, virtual-camera sinks and the render thread.
//! See ADR-0002, ADR-0013 (frame IPC) and ADR-0014 (renderer data).

pub mod driver;
pub mod engine;
pub mod fonts;
pub mod format;
pub mod frame;
pub mod ipc;
pub mod layout;
pub mod mf;
pub mod renderer;
pub mod sink;
pub mod source;
pub mod theme;
pub mod themes_data;

pub use driver::{DriverStatus, DRIVER_FILE};
pub use format::{format_duration, format_money, format_money_compact};
pub use frame::{normalize_size, Frame, FPS, FRAME_HEIGHT, FRAME_WIDTH, SUPPORTED_SIZES};
pub use layout::{
    counter_layout, counter_layout_in, counter_layout_with_label, fit_cost, label_font_px,
    CounterLayout, FitCost, Rect, LABEL_HEIGHT_RATIO, LABEL_LINE_HEIGHT,
};
pub use renderer::{
    fit_font_px, render, render_animated, BackgroundMode, BackgroundStyle, CounterInput,
    CounterValues, DisplayConfig, ElementStyle, Elements, FlapState, GradientDirection, Labels,
    Mirror, FLIP_MS,
};
pub use source::{list_sources, CameraSource, FrameSource, WebcamSource};
pub use theme::{theme, FontChoice, FontId, Layout, Overflow, PlateStyle, Theme};
