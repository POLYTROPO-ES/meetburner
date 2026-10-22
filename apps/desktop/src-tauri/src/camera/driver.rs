//! Registers the DirectShow driver from the app, so the user never runs scripts
//! or `regsvr32` (ADR-0017). Registration is per-user (HKCU) and needs no UAC.

use std::path::{Path, PathBuf};

pub const DRIVER_FILE: &str = "MeetBurnerCamera.dll";
pub const CLSID_TEXT: &str = "{DBD3A890-1830-4222-AFEE-B1BA2C2DAB49}";
/// `CLSID_VideoInputDeviceCategory`, the list meeting apps enumerate.
pub const CATEGORY_TEXT: &str = "{860BB310-5D01-11D0-BD3B-00A0C911CE86}";

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverStatus {
    pub registered: bool,
    pub available: bool,
    pub path: Option<String>,
}

/// Where the driver can live, in priority order. `resource_dir` and `exe_dir` come from
/// Tauri; the development tree is the last resort for `tauri dev` and `cargo run`.
pub fn candidates(resource_dir: Option<&Path>, exe_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(explicit) = std::env::var_os("MEETBURNER_VCAM_DLL") {
        candidates.push(PathBuf::from(explicit));
    }
    if let Some(dir) = exe_dir {
        candidates.push(dir.join(DRIVER_FILE));
        candidates.push(dir.join("resources").join(DRIVER_FILE));
    }
    if let Some(dir) = resource_dir {
        candidates.push(dir.join(DRIVER_FILE));
        candidates.push(dir.join("resources").join(DRIVER_FILE));
    }
    candidates.push(dev_driver_path());
    candidates
}

/// Built by `cmake --build native/vcam-windows/build --config Release`.
pub fn dev_driver_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../native/vcam-windows/build/Release")
        .join(DRIVER_FILE)
}

pub fn find(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|path| path.is_file()).cloned()
}

/// Path recorded in `InprocServer32`, when the driver is registered.
pub fn registered_path() -> Option<String> {
    #[cfg(windows)]
    {
        imp::registered_path()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// Calls `DllRegisterServer` in the given DLL (per-user keys, no elevation).
pub fn register(dll: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        imp::call_export(dll, "DllRegisterServer")
    }
    #[cfg(not(windows))]
    {
        let _ = dll;
        Err("the DirectShow driver is Windows-only".to_string())
    }
}

/// Removes every registry key the driver owns. Works even if the DLL is gone.
pub fn remove_registration() -> Result<(), String> {
    #[cfg(windows)]
    {
        imp::remove_keys()
    }
    #[cfg(not(windows))]
    {
        Ok(())
    }
}

#[cfg(windows)]
mod imp {
    use std::ffi::{c_void, CString};
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows_sys::Win32::Foundation::{
        FreeLibrary, GetLastError, ERROR_FILE_NOT_FOUND, ERROR_SUCCESS,
    };
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows_sys::Win32::System::Registry::{
        RegDeleteTreeW, RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ,
    };

    use super::{CATEGORY_TEXT, CLSID_TEXT};

    fn wide(value: &str) -> Vec<u16> {
        std::ffi::OsStr::new(value)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    pub fn call_export(dll: &Path, symbol: &str) -> Result<(), String> {
        let path = wide(&dll.display().to_string());
        let module = unsafe { LoadLibraryW(path.as_ptr()) };
        if module.is_null() {
            return Err(format!(
                "cannot load {} (Win32 error {})",
                dll.display(),
                unsafe { GetLastError() }
            ));
        }
        let outcome = (|| {
            let name = CString::new(symbol).expect("static export name");
            let Some(entry) = (unsafe { GetProcAddress(module, name.as_ptr() as *const u8) })
            else {
                return Err(format!("{symbol} is missing from {}", dll.display()));
            };
            let hr = unsafe { entry() } as i32;
            if hr < 0 {
                Err(format!("{symbol} failed (0x{:08X})", hr as u32))
            } else {
                Ok(())
            }
        })();
        unsafe { FreeLibrary(module) };
        outcome
    }

    pub fn registered_path() -> Option<String> {
        let subkey = wide(&format!(
            "Software\\Classes\\CLSID\\{CLSID_TEXT}\\InprocServer32"
        ));
        let mut buffer = [0u16; 1024];
        let mut size = (buffer.len() * std::mem::size_of::<u16>()) as u32;
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                subkey.as_ptr(),
                std::ptr::null(),
                RRF_RT_REG_SZ,
                std::ptr::null_mut(),
                buffer.as_mut_ptr() as *mut c_void,
                &mut size,
            )
        };
        if status != ERROR_SUCCESS {
            return None;
        }
        let length = (size as usize / 2).saturating_sub(1);
        Some(String::from_utf16_lossy(&buffer[..length]))
    }

    pub fn remove_keys() -> Result<(), String> {
        let keys = [
            format!("Software\\Classes\\CLSID\\{CLSID_TEXT}"),
            format!("Software\\Classes\\CLSID\\{CATEGORY_TEXT}\\Instance\\{CLSID_TEXT}"),
        ];
        for key in keys {
            let subkey = wide(&key);
            let status = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, subkey.as_ptr()) };
            if status != ERROR_SUCCESS && status != ERROR_FILE_NOT_FOUND {
                return Err(format!("cannot remove {key} (Win32 error {status})"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn candidates_cover_the_bundle_the_resources_folder_and_the_dev_tree() {
        let bundled = Path::new("C:\\Program Files\\MeetBurner");
        let exe = Path::new("C:\\Apps");
        let candidates = candidates(Some(bundled), Some(exe));
        assert!(candidates.contains(&exe.join(DRIVER_FILE)));
        assert!(candidates.contains(&exe.join("resources").join(DRIVER_FILE)));
        assert!(candidates.contains(&bundled.join(DRIVER_FILE)));
        assert!(candidates.contains(&bundled.join("resources").join(DRIVER_FILE)));
        assert!(candidates.contains(&dev_driver_path()));
    }

    #[test]
    fn finds_the_first_existing_driver() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("meetburner-driver-{unique}"));
        fs::create_dir_all(&dir).expect("temp dir");
        let dll = dir.join(DRIVER_FILE);
        fs::write(&dll, b"stub").expect("stub dll");

        let missing = dir.join("missing").join(DRIVER_FILE);
        assert_eq!(find(&[missing, dll.clone()]), Some(dll.clone()));

        fs::remove_file(&dll).expect("remove stub");
        fs::remove_dir_all(&dir).expect("remove temp dir");
        assert_eq!(find(&[dir.join(DRIVER_FILE)]), None);
    }

    #[cfg(windows)]
    #[test]
    fn registers_and_cleans_up_when_the_dev_driver_exists() {
        // Never disturb an installation the user already made.
        if registered_path().is_some() {
            return;
        }
        let Some(dll) = find(&[dev_driver_path()]) else {
            return; // CI does not build the native project
        };
        register(&dll).expect("register the driver");
        let recorded = registered_path().expect("the registration is visible");
        assert!(
            recorded.to_lowercase().contains("meetburnercamera.dll"),
            "unexpected InprocServer32: {recorded}"
        );
        remove_registration().expect("clean the registration");
        assert!(registered_path().is_none());
    }
}
