//! Development helper for the DirectShow filter: publishes rendered counter frames to
//! the virtual-camera mapping without starting the app.
//!
//! ```text
//! cargo run --example produce_frames -- [seconds] [width] [height] [switch_w switch_h]
//! ```
//!
//! With `switch_w switch_h` it changes the output size halfway through without stopping,
//! which is how a resolution change behaves in the app (ADR-0019).
//!
//! It is Windows-only because the shared-memory sink is (ADR-0013).

#[cfg(windows)]
fn main() {
    use std::time::{Duration, Instant};

    use meetburner_lib::camera::engine::{now_ms, RenderLoop};
    use meetburner_lib::camera::fonts::Fonts;
    use meetburner_lib::camera::frame::{FRAME_HEIGHT, FRAME_WIDTH};
    use meetburner_lib::camera::renderer::{CounterInput, DisplayConfig, Labels, Mirror};
    use meetburner_lib::camera::sink::windows::WinSharedMemSink;
    use meetburner_lib::camera::theme::Layout;

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
    let switch_to: Option<(u32, u32)> = args
        .next()
        .and_then(|value| value.parse::<u32>().ok())
        .zip(args.next().and_then(|value| value.parse::<u32>().ok()));

    // `MEETBURNER_SOURCE=<id>` composites the counter over that camera (M7).
    let source = std::env::var("MEETBURNER_SOURCE").ok();
    let mut display = DisplayConfig::default();
    if source.is_some() {
        display.layout = Layout::Widget;
    }
    // `MEETBURNER_MIRROR=sourceH,overlayV` mirrors either layer (ADR-0022).
    if let Ok(value) = std::env::var("MEETBURNER_MIRROR") {
        let mut mirror = Mirror::default();
        for flag in value.split(',') {
            match flag.trim().to_ascii_lowercase().as_str() {
                "sourceh" => mirror.source_h = true,
                "sourcev" => mirror.source_v = true,
                "overlayh" => mirror.overlay_h = true,
                "overlayv" => mirror.overlay_v = true,
                _ => {}
            }
        }
        display.mirror = mirror;
    }

    let input = CounterInput {
        // A meeting that started 65 s ago, so the elapsed time and the cost are visible.
        start_at: now_ms() - 65_000,
        end_at: None,
        rate_per_ms: 0.01,
        currency: "EUR".into(),
        locale: "es".into(),
        labels: Labels::default(),
        display,
        width,
        height,
        source,
    };

    let fonts = Fonts::load_system().expect("a system font is available");
    let mut render_loop = RenderLoop::new(fonts, Box::new(WinSharedMemSink::new()));
    println!("publishing {width}x{height} frames for {seconds}s (Ctrl+C to stop)");
    for source in meetburner_lib::camera::list_sources() {
        println!("webcam source {} = {}", source.id, source.name);
    }

    let started = Instant::now();
    let deadline = started + Duration::from_secs(seconds);
    let half = started + Duration::from_secs(seconds / 2);
    let mut published = (width, height);
    while Instant::now() < deadline {
        if let Some((next_width, next_height)) = switch_to {
            if published != (next_width, next_height) && Instant::now() >= half {
                published = (next_width, next_height);
                println!("switching to {next_width}x{next_height} without stopping the mapping");
            }
        }
        let mut current = input.clone();
        current.width = published.0;
        current.height = published.1;
        if let Err(error) = render_loop.tick(&current, now_ms()) {
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
