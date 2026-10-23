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

/// Folder that holds `MeetBurner.exe`; the embedded driver is extracted there (ADR-0026).
fn exe_dir() -> Result<PathBuf, String> {
    std::env::current_exe()
        .map_err(|error| format!("cannot locate MeetBurner.exe: {error}"))?
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "cannot locate the folder that holds MeetBurner.exe".to_string())
}

fn driver_candidates(app: &tauri::AppHandle, exe_dir: &Path) -> Vec<PathBuf> {
    let resource_dir = app.path().resource_dir().ok();
    driver::candidates(resource_dir.as_deref(), Some(exe_dir))
}

/// Status for the setup page. Read-only: the embedded driver is written on the first
/// registration, so `path` can name the file that will be extracted (ADR-0026).
fn driver_status(app: &tauri::AppHandle) -> Result<DriverStatus, String> {
    let exe = exe_dir()?;
    let found = driver::find(&driver_candidates(app, &exe));
    let registered = driver::registered_path();
    let embedded_target = || {
        driver::EMBEDDED_DRIVER.map(|_| {
            driver::extract_target(&exe, driver::EMBEDDED_DRIVER_VERSION)
                .display()
                .to_string()
        })
    };
    Ok(DriverStatus {
        registered: registered.is_some(),
        available: found.is_some() || driver::EMBEDDED_DRIVER.is_some(),
        path: registered
            .clone()
            .or_else(|| found.as_deref().map(|path| path.display().to_string()))
            .or_else(embedded_target),
        installed_version: driver::registered_version(),
        expected_version: driver::expected_version(found.as_deref()),
    })
}

/// Result of checking the registration on start: the status, plus whether it was refreshed.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DriverUpdate {
    status: DriverStatus,
    updated: bool,
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

/// Physical cameras the overlay can composite the counter over (M7).
#[tauri::command]
fn camera_list_sources() -> Vec<camera::CameraSource> {
    camera::list_sources()
}

/// Whether the Linux sink can publish: `available` is false without a v4l2loopback device
/// (and always false off Linux).
#[tauri::command]
fn camera_v4l2_status() -> camera::sink::V4l2Status {
    camera::sink::v4l2_status()
}

/// Whether "MeetBurner Camera" is registered for the current Windows account.
#[tauri::command]
fn camera_driver_status(app: tauri::AppHandle) -> Result<DriverStatus, String> {
    driver_status(&app)
}

/// Called on start: re-registers the bundled driver when the installed one is older
/// (ADR-0020). Never registers on its own: that stays an explicit user action (ADR-0017).
#[tauri::command]
fn camera_driver_ensure(app: tauri::AppHandle) -> Result<DriverUpdate, String> {
    let exe = exe_dir()?;
    let updated = driver::ensure_installed(&driver_candidates(&app, &exe), &exe)?;
    Ok(DriverUpdate {
        status: driver_status(&app)?,
        updated,
    })
}

/// Registers the driver bundled with the app; no admin prompt, no scripts.
#[tauri::command]
fn camera_register_driver(app: tauri::AppHandle) -> Result<DriverStatus, String> {
    let exe = exe_dir()?;
    let dll = driver::locate(&driver_candidates(&app, &exe), &exe)?.ok_or_else(|| {
        format!(
            "{} was not found next to the app; rebuild the desktop bundle",
            driver::DRIVER_FILE
        )
    })?;
    driver::register(&dll)?;
    driver_status(&app)
}

/// Removes the driver registration (the "clean" button).
#[tauri::command]
fn camera_unregister_driver(app: tauri::AppHandle) -> Result<DriverStatus, String> {
    driver::remove_registration()?;
    driver_status(&app)
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
            camera_driver_ensure,
            camera_register_driver,
            camera_unregister_driver,
            camera_list_sources,
            camera_v4l2_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running MeetBurner");
}
