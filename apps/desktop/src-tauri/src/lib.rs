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

/// Longest side of a setup preview frame: sharp enough for the stage, small over IPC.
const PREVIEW_LONGEST: u32 = 480;

/// Newest webcam frame for the setup preview (ADR-0035): `[width u32 LE][height u32 LE]` then
/// RGBA, empty while there is none. The webview never opens the webcam itself: Windows and most
/// Linux drivers allow one consumer, and the camera pipeline already is it.
#[tauri::command]
fn camera_preview_frame(state: State<'_, CameraState>) -> Result<tauri::ipc::Response, String> {
    let engine = state
        .0
        .lock()
        .map_err(|_| "camera state is poisoned".to_string())?;
    let frame = match engine.as_ref() {
        Some(running) => running.preview(PREVIEW_LONGEST)?,
        None => None,
    };
    Ok(tauri::ipc::Response::new(
        frame
            .map(|frame| frame.to_rgba_payload())
            .unwrap_or_default(),
    ))
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

/// `MeetBurnerMfCamera.dll` next to the exe, in `resources/` or in the dev CMake build.
fn mf_dll(app: &tauri::AppHandle) -> Option<PathBuf> {
    let exe = exe_dir().ok()?;
    let resource_dir = app.path().resource_dir().ok();
    camera::mf::find(&camera::mf::candidates(resource_dir.as_deref(), Some(&exe)))
}

/// Whether the Windows 11 Media Foundation camera can be used and is registered (ADR-0031).
/// Always `supported: false` on Windows 10 and off Windows; the DirectShow camera still works.
#[tauri::command]
fn camera_mf_status(app: tauri::AppHandle) -> camera::mf::MfStatus {
    camera::mf::status(mf_dll(&app).as_deref())
}

/// Registers "MeetBurner Camera (Windows 11)"; per user, no UAC (ADR-0031).
#[tauri::command]
fn camera_mf_register(app: tauri::AppHandle) -> Result<camera::mf::MfStatus, String> {
    camera::mf::register(mf_dll(&app).as_deref())
}

/// Removes the MF camera and its per-user COM registration.
#[tauri::command]
fn camera_mf_unregister(app: tauri::AppHandle) -> Result<camera::mf::MfStatus, String> {
    camera::mf::unregister(mf_dll(&app).as_deref())
}

/// Command-line mode, decided before any window exists (ADR-0039).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// Normal start: open the app window.
    Run,
    /// Uninstall helper: remove the per-user camera registrations and exit.
    UnregisterCameras,
}

/// `--unregister-cameras` anywhere on the command line selects the uninstall mode; every other
/// argument is ignored (ADR-0039).
pub fn parse_command<I, S>(args: I) -> Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    if args
        .into_iter()
        .any(|argument| argument.as_ref() == "--unregister-cameras")
    {
        Command::UnregisterCameras
    } else {
        Command::Run
    }
}

/// What the uninstall mode does with the Windows 11 MF camera (ADR-0039).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MfUnregister {
    /// The camera is not registered: leave the registry alone.
    Nothing,
    /// The camera is registered: run `MB_MfUnregister` against this DLL.
    Remove(PathBuf),
}

/// Decides the MF step without touching the registry, so the "nothing registered" path is
/// testable off Windows (ADR-0008, ADR-0039).
pub fn mf_unregister_step(registered: bool, dll: Option<&Path>) -> Result<MfUnregister, String> {
    if !registered {
        return Ok(MfUnregister::Nothing);
    }
    match dll {
        Some(dll) => Ok(MfUnregister::Remove(dll.to_path_buf())),
        None => Err(format!(
            "{} was not found next to the app",
            camera::mf::DLL_FILE
        )),
    }
}

/// Removes both per-user camera registrations (ADR-0039): the DirectShow driver first, always
/// (a no-op off Windows and when nothing is registered), then the Windows 11 MF camera when it
/// is registered. The installers call it as `meetburner.exe --unregister-cameras`.
pub fn unregister_cameras() -> Result<(), String> {
    driver::remove_registration()?;
    // No AppHandle in command-line mode: the DLL is looked up next to the exe.
    let exe = exe_dir()?;
    let dll = camera::mf::find(&camera::mf::candidates(None, Some(&exe)));
    let registered = camera::mf::status(dll.as_deref()).registered;
    if let MfUnregister::Remove(dll) = mf_unregister_step(registered, dll.as_deref())? {
        camera::mf::unregister(Some(&dll))?;
    }
    Ok(())
}

/// Exit code for `--unregister-cameras`. The release binary uses the Windows subsystem, so the
/// installer can only read the exit code; the reason goes to stderr for the NSIS log (ADR-0039).
pub fn unregister_cameras_cli() -> i32 {
    match unregister_cameras() {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("MeetBurner: cannot unregister the cameras: {error}");
            1
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(CameraState::default())
        .invoke_handler(tauri::generate_handler![
            camera_start,
            camera_update,
            camera_stop,
            camera_preview_frame,
            camera_driver_status,
            camera_driver_ensure,
            camera_register_driver,
            camera_unregister_driver,
            camera_mf_status,
            camera_mf_register,
            camera_mf_unregister,
            camera_list_sources,
            camera_v4l2_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running MeetBurner");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_arguments_run_the_app() {
        assert_eq!(parse_command(Vec::<String>::new()), Command::Run);
        assert_eq!(parse_command(["meetburner.exe"]), Command::Run);
        assert_eq!(
            parse_command(["meetburner.exe", "--some-other-flag"]),
            Command::Run
        );
    }

    #[test]
    fn the_flag_alone_means_unregister() {
        assert_eq!(
            parse_command(["meetburner.exe", "--unregister-cameras"]),
            Command::UnregisterCameras
        );
    }

    #[test]
    fn the_flag_among_other_arguments_means_unregister() {
        assert_eq!(
            parse_command(["--first", "--unregister-cameras", "--last"]),
            Command::UnregisterCameras
        );
    }

    #[test]
    fn an_argument_that_only_looks_like_the_flag_is_ignored() {
        assert_eq!(
            parse_command(["meetburner.exe", "--unregister-cameras=1"]),
            Command::Run
        );
        assert_eq!(
            parse_command(["meetburner.exe", "x--unregister-cameras"]),
            Command::Run
        );
    }

    #[test]
    fn nothing_registered_leaves_the_registry_alone() {
        assert_eq!(
            mf_unregister_step(false, None).expect("nothing to do"),
            MfUnregister::Nothing
        );
        let dll = Path::new("C:\\Apps").join(camera::mf::DLL_FILE);
        assert_eq!(
            mf_unregister_step(false, Some(&dll)).expect("nothing to do"),
            MfUnregister::Nothing,
            "an unregistered camera must not be touched, even when the DLL exists"
        );
    }

    #[test]
    fn a_registered_camera_removes_the_bundled_dll() {
        let dll = Path::new("C:\\Apps").join(camera::mf::DLL_FILE);
        assert_eq!(
            mf_unregister_step(true, Some(&dll)).expect("remove"),
            MfUnregister::Remove(dll.clone())
        );
    }

    #[test]
    fn a_registered_camera_without_its_dll_names_the_missing_file() {
        let error = mf_unregister_step(true, None).expect_err("no DLL to unregister with");
        assert_eq!(
            error,
            format!("{} was not found next to the app", camera::mf::DLL_FILE)
        );
    }
}
