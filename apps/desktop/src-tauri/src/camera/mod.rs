//! Camera pipeline: frame rendering, virtual-camera sinks and the render thread.
//! See ADR-0002, ADR-0013 (frame IPC) and ADR-0014 (renderer data).

pub mod driver;
pub mod engine;
pub mod fonts;
pub mod format;
pub mod frame;
pub mod ipc;
pub mod layout;
pub mod renderer;
pub mod sink;
pub mod source;
pub mod theme;
pub mod themes_data;

pub use driver::{DriverStatus, DRIVER_FILE};
pub use format::{format_duration, format_money, format_money_compact};
pub use frame::{normalize_size, Frame, FPS, FRAME_HEIGHT, FRAME_WIDTH, SUPPORTED_SIZES};
pub use layout::{counter_layout, fit_cost, CounterLayout, FitCost, Rect};
pub use renderer::{fit_font_px, render, CounterInput, CounterValues, DisplayConfig, Labels};
pub use source::{list_sources, CameraSource, FrameSource, WebcamSource};
pub use theme::{theme, FontChoice, FontId, Layout, Overflow, Theme};
