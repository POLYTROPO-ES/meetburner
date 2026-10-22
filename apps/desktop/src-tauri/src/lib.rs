// Camera pipeline (renderer, source, VirtualCameraSink) lands here in M4–M8; see docs/plan/roadmap.md.

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running MeetBurner");
}
