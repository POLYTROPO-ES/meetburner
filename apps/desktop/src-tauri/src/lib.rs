//! MeetBurner desktop shell. The camera pipeline lives in `camera` (ADR-0002).

pub mod camera;

use std::sync::Mutex;

use tauri::State;

use crate::camera::engine::CameraEngine;
use crate::camera::renderer::CounterInput;

/// One camera at a time: `camera_start` replaces the running engine.
#[derive(Default)]
struct CameraState(Mutex<Option<CameraEngine>>);

#[tauri::command]
fn camera_start(config: CounterInput, state: State<'_, CameraState>) -> Result<(), String> {
    let mut engine = state
        .0
        .lock()
        .map_err(|_| "camera state is poisoned".to_string())?;
    if let Some(mut running) = engine.take() {
        running.stop();
    }
    *engine = Some(CameraEngine::start(config).map_err(|error| error.to_string())?);
    Ok(())
}

#[tauri::command]
fn camera_update(config: CounterInput, state: State<'_, CameraState>) -> Result<(), String> {
    let engine = state
        .0
        .lock()
        .map_err(|_| "camera state is poisoned".to_string())?;
    let running = engine.as_ref().ok_or("the camera is not running")?;
    running.update(config);
    Ok(())
}

#[tauri::command]
fn camera_stop(state: State<'_, CameraState>) -> Result<(), String> {
    let mut engine = state
        .0
        .lock()
        .map_err(|_| "camera state is poisoned".to_string())?;
    if let Some(mut running) = engine.take() {
        running.stop();
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(CameraState::default())
        .invoke_handler(tauri::generate_handler![
            camera_start,
            camera_update,
            camera_stop
        ])
        .run(tauri::generate_context!())
        .expect("error while running MeetBurner");
}
