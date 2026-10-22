//! Development helper for the DirectShow filter: publishes rendered counter frames to
//! the virtual-camera mapping without starting the app.
//!
//! ```text
//! cargo run --example produce_frames -- [seconds] [width] [height]
//! ```
//!
//! It is Windows-only because the shared-memory sink is (ADR-0013).

#[cfg(windows)]
fn main() {
    use std::time::{Duration, Instant};

    use meetburner_lib::camera::engine::{now_ms, RenderLoop};
    use meetburner_lib::camera::fonts::Fonts;
    use meetburner_lib::camera::frame::{FRAME_HEIGHT, FRAME_WIDTH};
    use meetburner_lib::camera::renderer::{CounterInput, DisplayConfig, Labels};
    use meetburner_lib::camera::sink::windows::WinSharedMemSink;

    let mut args = std::env::args().skip(1);
    let seconds: u64 = args
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(30);
    let width: u32 = args
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(FRAME_WIDTH);
    let height: u32 = args
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(FRAME_HEIGHT);

    let input = CounterInput {
        // A meeting that started 65 s ago, so the elapsed time and the cost are visible.
        start_at: now_ms() - 65_000,
        end_at: None,
        rate_per_ms: 0.01,
        currency: "EUR".into(),
        locale: "es".into(),
        labels: Labels::default(),
        display: DisplayConfig::default(),
        width,
        height,
    };

    let fonts = Fonts::load_system().expect("a system font is available");
    let mut render_loop = RenderLoop::new(fonts, Box::new(WinSharedMemSink::new()));
    println!("publishing {width}x{height} frames for {seconds}s (Ctrl+C to stop)");

    let deadline = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < deadline {
        if let Err(error) = render_loop.tick(&input, now_ms()) {
            eprintln!("frame dropped: {error}");
        }
        std::thread::sleep(Duration::from_millis(33));
    }
    render_loop.stop();
    println!("stopped");
}

#[cfg(not(windows))]
fn main() {
    eprintln!("produce_frames is Windows-only (the virtual camera sink is).");
    std::process::exit(1);
}
