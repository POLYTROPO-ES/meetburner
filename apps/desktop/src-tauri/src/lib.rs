//! MeetBurner desktop shell. The camera pipeline lives in `camera` (ADR-0002).

pub mod camera;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::{Manager, State};

use crate::camera::driver::{self, DriverStatus};
use crate::camera::engine::CameraEngine;
use crate::camera::renderer::CounterInput;

/// One camera at a time: `camera_start` replaces the running engine.
#[derive(Default)]
struct CameraState(Mutex<Option<CameraEngine>>);

fn driver_candidates(app: &tauri::AppHandle) -> Vec<PathBuf> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf));
    let resource_dir = app.path().resource_dir().ok();
    driver::candidates(resource_dir.as_deref(), exe_dir.as_deref())
}

fn driver_status(app: &tauri::AppHandle) -> DriverStatus {
    let found = driver::find(&driver_candidates(app));
    let registered = driver::registered_path();
    DriverStatus {
        registered: registered.is_some(),
        available: found.is_some(),
        path: registered.or_else(|| found.map(|path| path.display().to_string())),
    }
}

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

/// Whether "MeetBurner Camera" is registered for the current Windows account.
#[tauri::command]
fn camera_driver_status(app: tauri::AppHandle) -> Result<DriverStatus, String> {
    Ok(driver_status(&app))
}

/// Registers the driver bundled with the app; no admin prompt, no scripts.
#[tauri::command]
fn camera_register_driver(app: tauri::AppHandle) -> Result<DriverStatus, String> {
    let dll = driver::find(&driver_candidates(&app)).ok_or_else(|| {
        format!(
            "{} was not found next to the app; rebuild the desktop bundle",
            driver::DRIVER_FILE
        )
    })?;
    driver::register(&dll)?;
    Ok(driver_status(&app))
}

/// Removes the driver registration (the "clean" button).
#[tauri::command]
fn camera_unregister_driver(app: tauri::AppHandle) -> Result<DriverStatus, String> {
    driver::remove_registration()?;
    Ok(driver_status(&app))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(CameraState::default())
        .invoke_handler(tauri::generate_handler![
            camera_start,
            camera_update,
            camera_stop,
            camera_driver_status,
            camera_register_driver,
            camera_unregister_driver
        ])
        .run(tauri::generate_context!())
        .expect("error while running MeetBurner");
}
