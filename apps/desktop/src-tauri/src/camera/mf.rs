//! Media Foundation virtual camera (Windows 11, build 22000+): the DLL that sits next to
//! the DirectShow filter and the per-user registration commands (ADR-0031).
//!
//! The frames are not produced here: both cameras read the same shared memory that
//! `camera::sink::windows::WinSharedMemSink` writes (ADR-0013), so either camera can be
//! active at the same time and the producer never changes.
//!
//! `MeetBurnerMfCamera.dll` resolves `MFCreateVirtualCamera` at run time and reports
//! `MB_MfSupported() == 0` on Windows 10, where the API does not exist. That keeps the app
//! (and the DLL) usable on Windows 10; the UI can say "requires Windows 11".

use std::path::{Path, PathBuf};

/// Must match `MF_DLL_FILE`/the target name in `native/vcam-windows/CMakeLists.txt`.
pub const DLL_FILE: &str = "MeetBurnerMfCamera.dll";
/// Distinct from the DirectShow filter's "MeetBurner Camera" (ADR-0031).
pub const FRIENDLY_NAME: &str = "MeetBurner Camera (Windows 11)";
/// Must match `MB_MF_CLSID` in `native/vcam-windows/mf/mf_camera_api.h`.
pub const CLSID_TEXT: &str = "{9D6A2B41-3C57-4A0E-8F2D-7B1E5C3A9F64}";
/// `MFCreateVirtualCamera` is available from Windows 11 21H2 (build 22000).
pub const MIN_BUILD: u32 = 22000;
/// Written by `MB_MfRegister` after `IMFVirtualCamera::Start` succeeds; removed by
/// `MB_MfUnregister` together with the CLSID key.
pub const MARKER_VALUE: &str = "MeetBurnerMfRegistered";
/// Friendly name recorded at registration, so the status shows what Windows actually has.
pub const NAME_VALUE: &str = "MeetBurnerMfFriendlyName";

/// Payload of `camera_mf_status`. `supported` is false on Windows 10 and when the DLL is
/// missing; the UI uses it to explain that the MF camera needs Windows 11.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MfStatus {
    pub supported: bool,
    pub registered: bool,
    pub name: String,
}

impl Default for MfStatus {
    fn default() -> Self {
        MfStatus {
            supported: false,
            registered: false,
            name: FRIENDLY_NAME.to_string(),
        }
    }
}

/// Pure support gate: Windows 11 21H2 and later.
pub fn is_supported_build(build: u32) -> bool {
    build >= MIN_BUILD
}

/// Where the DLL can live, in the same priority order as the DirectShow driver.
pub fn candidates(resource_dir: Option<&Path>, exe_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(explicit) = std::env::var_os("MEETBURNER_MF_DLL") {
        candidates.push(PathBuf::from(explicit));
    }
    if let Some(dir) = exe_dir {
        candidates.push(dir.join(DLL_FILE));
        candidates.push(dir.join("resources").join(DLL_FILE));
    }
    if let Some(dir) = resource_dir {
        candidates.push(dir.join(DLL_FILE));
        candidates.push(dir.join("resources").join(DLL_FILE));
    }
    candidates.push(dev_dll_path());
    candidates
}

/// Built by `cmake --build native/vcam-windows/build --config Release`.
pub fn dev_dll_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../native/vcam-windows/build/Release")
        .join(DLL_FILE)
}

pub fn find(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|path| path.is_file()).cloned()
}

/// Current state for the setup page. Read-only: registering stays an explicit user action
/// (ADR-0017).
pub fn status(dll: Option<&Path>) -> MfStatus {
    MfStatus {
        supported: dll.is_some() && current_build().is_some_and(is_supported_build),
        registered: registered_marker(),
        name: registered_name().unwrap_or_else(|| FRIENDLY_NAME.to_string()),
    }
}

/// Registers the COM server and the per-user virtual camera. The DLL does the work; this
/// only resolves the exports (ADR-0031).
pub fn register(dll: Option<&Path>) -> Result<MfStatus, String> {
    #[cfg(not(windows))]
    {
        let _ = dll;
        return Err(WINDOWS_ONLY.to_string());
    }
    #[cfg(windows)]
    {
        let dll = dll.ok_or_else(missing_dll)?;
        imp::call_register(dll, FRIENDLY_NAME)?;
        Ok(status(Some(dll)))
    }
}

/// Removes the virtual camera and its per-user COM registration.
pub fn unregister(dll: Option<&Path>) -> Result<MfStatus, String> {
    #[cfg(not(windows))]
    {
        let _ = dll;
        return Err(WINDOWS_ONLY.to_string());
    }
    #[cfg(windows)]
    {
        let dll = dll.ok_or_else(missing_dll)?;
        imp::call_unregister(dll)?;
        Ok(status(Some(dll)))
    }
}

#[cfg(not(windows))]
const WINDOWS_ONLY: &str = "the Media Foundation camera is Windows-only";

#[cfg(windows)]
fn missing_dll() -> String {
    format!("{DLL_FILE} was not found next to the app; rebuild the desktop bundle")
}

/// Windows build number, or `None` off Windows or when `RtlGetVersion` cannot answer.
pub fn current_build() -> Option<u32> {
    #[cfg(windows)]
    {
        imp::current_build()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

fn registered_marker() -> bool {
    #[cfg(windows)]
    {
        imp::read_dword(MARKER_VALUE).is_some()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn registered_name() -> Option<String> {
    #[cfg(windows)]
    {
        imp::read_string(NAME_VALUE)
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// Human wording for the HRESULTs callers care about; everything else keeps its code.
#[cfg_attr(not(windows), allow(dead_code))]
fn describe_hr(what: &str, hr: i32) -> String {
    match hr as u32 {
        0x8007_0032 => {
            "the Media Foundation virtual camera requires Windows 11 (build 22000 or later)"
                .to_string()
        }
        0x8007_007F => "MFCreateVirtualCamera is not available on this Windows build".to_string(),
        0x8007_0005 => {
            "Windows denied access to the camera; check Settings > Privacy > Camera".to_string()
        }
        _ => format!("{what} failed (0x{:08X})", hr as u32),
    }
}

#[cfg(windows)]
mod imp {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows_sys::Win32::Foundation::{FreeLibrary, GetLastError, HMODULE};
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows_sys::Win32::System::Registry::{
        RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RRF_RT_REG_SZ,
    };

    use super::{describe_hr, CLSID_TEXT};

    type SupportedFn = unsafe extern "system" fn() -> i32;
    type RegisterFn = unsafe extern "system" fn(*const u16) -> i32;
    type UnregisterFn = unsafe extern "system" fn() -> i32;

    /// The three exports of `MeetBurnerMfCamera.dll`, resolved once per call.
    pub struct MfApi {
        module: HMODULE,
        supported: SupportedFn,
        register: RegisterFn,
        unregister: UnregisterFn,
    }

    impl MfApi {
        pub fn load(path: &Path) -> Result<MfApi, String> {
            let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            let module = unsafe { LoadLibraryW(wide.as_ptr()) };
            if module.is_null() {
                return Err(format!(
                    "cannot load {} (Win32 error {})",
                    path.display(),
                    unsafe { GetLastError() }
                ));
            }
            let resolved = (|| {
                Ok(MfApi {
                    module,
                    supported: resolve(module, path, b"MB_MfSupported\0")?,
                    register: resolve(module, path, b"MB_MfRegister\0")?,
                    unregister: resolve(module, path, b"MB_MfUnregister\0")?,
                })
            })();
            if resolved.is_err() {
                unsafe { FreeLibrary(module) };
            }
            resolved
        }

        pub fn supported(&self) -> bool {
            unsafe { (self.supported)() != 0 }
        }
    }

    impl Drop for MfApi {
        fn drop(&mut self) {
            unsafe { FreeLibrary(self.module) };
        }
    }

    fn resolve<T: Copy>(module: HMODULE, path: &Path, name: &'static [u8]) -> Result<T, String> {
        let symbol = unsafe { GetProcAddress(module, name.as_ptr()) }.ok_or_else(|| {
            let text = std::str::from_utf8(&name[..name.len() - 1]).unwrap_or("export");
            format!("{text} is missing from {}", path.display())
        })?;
        Ok(unsafe { std::mem::transmute_copy(&symbol) })
    }

    #[repr(C)]
    struct OsVersionInfoW {
        size: u32,
        major: u32,
        minor: u32,
        build: u32,
        platform_id: u32,
        csd_version: [u16; 128],
    }

    type RtlGetVersionFn = unsafe extern "system" fn(*mut OsVersionInfoW) -> i32;

    pub fn current_build() -> Option<u32> {
        // RtlGetVersion is the reliable build number: GetVersionExW lies unless the exe is
        // manifested for the current OS, and this DLL is loaded into third-party processes.
        let ntdll: Vec<u16> = OsStr::new("ntdll.dll")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let module = unsafe { LoadLibraryW(ntdll.as_ptr()) };
        if module.is_null() {
            return None;
        }
        let symbol = unsafe { GetProcAddress(module, c"RtlGetVersion".as_ptr() as *const u8) };
        let build = symbol.and_then(|symbol| {
            let rtl: RtlGetVersionFn = unsafe { std::mem::transmute_copy(&symbol) };
            let mut info: OsVersionInfoW = unsafe { std::mem::zeroed() };
            info.size = std::mem::size_of::<OsVersionInfoW>() as u32;
            let status = unsafe { rtl(&mut info) };
            (status >= 0).then_some(info.build)
        });
        unsafe { FreeLibrary(module) };
        build
    }

    pub fn call_register(path: &Path, name: &str) -> Result<(), String> {
        let api = MfApi::load(path)?;
        if !api.supported() {
            return Err(describe_hr("MB_MfRegister", 0x8007_0032u32 as i32));
        }
        let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        hr("MB_MfRegister", unsafe { (api.register)(wide.as_ptr()) })
    }

    pub fn call_unregister(path: &Path) -> Result<(), String> {
        let api = MfApi::load(path)?;
        if !api.supported() {
            return Err(describe_hr("MB_MfUnregister", 0x8007_0032u32 as i32));
        }
        hr("MB_MfUnregister", unsafe { (api.unregister)() })
    }

    fn hr(what: &str, value: i32) -> Result<(), String> {
        if value >= 0 {
            Ok(())
        } else {
            Err(describe_hr(what, value))
        }
    }

    fn clsid_key() -> Vec<u16> {
        OsStr::new(&format!("Software\\Classes\\CLSID\\{CLSID_TEXT}"))
            .encode_wide()
            .chain(Some(0))
            .collect()
    }

    pub fn read_dword(name: &str) -> Option<u32> {
        let key = clsid_key();
        let name: Vec<u16> = OsStr::new(name).encode_wide().chain(Some(0)).collect();
        let mut value: u32 = 0;
        let mut size = std::mem::size_of::<u32>() as u32;
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                &mut value as *mut u32 as *mut std::ffi::c_void,
                &mut size,
            )
        };
        (status == 0).then_some(value)
    }

    pub fn read_string(name: &str) -> Option<String> {
        let key = clsid_key();
        let name: Vec<u16> = OsStr::new(name).encode_wide().chain(Some(0)).collect();
        let mut buffer = [0u16; 512];
        let mut size = (buffer.len() * std::mem::size_of::<u16>()) as u32;
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_SZ,
                std::ptr::null_mut(),
                buffer.as_mut_ptr() as *mut std::ffi::c_void,
                &mut size,
            )
        };
        if status != 0 {
            return None;
        }
        let length = (size as usize / 2).saturating_sub(1);
        Some(String::from_utf16_lossy(&buffer[..length]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_support_gate_needs_windows_11() {
        assert!(!is_supported_build(0));
        assert!(!is_supported_build(19045), "Windows 10 22H2");
        assert!(!is_supported_build(MIN_BUILD - 1));
        assert!(is_supported_build(MIN_BUILD), "Windows 11 21H2");
        assert!(is_supported_build(22631), "Windows 11 23H2");
    }

    #[test]
    fn status_defaults_say_unsupported_and_keep_the_name() {
        let status = MfStatus::default();
        assert!(!status.supported);
        assert!(!status.registered);
        assert_eq!(status.name, FRIENDLY_NAME);
        assert_eq!(status.name, "MeetBurner Camera (Windows 11)");
    }

    #[test]
    fn status_without_the_dll_is_unsupported() {
        let status = status(None);
        assert!(!status.supported, "no DLL, nothing to register");
        assert!(!status.name.is_empty());
    }

    #[test]
    fn candidates_cover_the_bundle_the_resources_folder_and_the_dev_tree() {
        let bundled = Path::new("C:\\Program Files\\MeetBurner");
        let exe = Path::new("C:\\Apps");
        let candidates = candidates(Some(bundled), Some(exe));
        assert!(candidates.contains(&exe.join(DLL_FILE)));
        assert!(candidates.contains(&exe.join("resources").join(DLL_FILE)));
        assert!(candidates.contains(&bundled.join(DLL_FILE)));
        assert!(candidates.contains(&bundled.join("resources").join(DLL_FILE)));
        assert!(candidates.contains(&dev_dll_path()));
    }

    #[test]
    fn finds_the_first_existing_dll() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("meetburner-mf-{unique}"));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let dll = dir.join(DLL_FILE);
        std::fs::write(&dll, b"stub").expect("stub dll");

        let missing = dir.join("missing").join(DLL_FILE);
        assert_eq!(find(&[missing, dll.clone()]), Some(dll.clone()));

        std::fs::remove_file(&dll).expect("remove stub");
        std::fs::remove_dir_all(&dir).expect("remove temp dir");
        assert_eq!(find(&[dir.join(DLL_FILE)]), None);
    }

    #[test]
    fn windows_10_and_missing_exports_are_explained() {
        assert!(describe_hr("MB_MfRegister", 0x8007_0032u32 as i32).contains("Windows 11"));
        assert!(
            describe_hr("MB_MfUnregister", 0x8007_007Fu32 as i32).contains("MFCreateVirtualCamera")
        );
        assert!(describe_hr("MB_MfRegister", 0x8007_0005u32 as i32).contains("Privacy"));
        assert_eq!(
            describe_hr("MB_MfRegister", 0x8000_4005u32 as i32),
            "MB_MfRegister failed (0x80004005)"
        );
    }

    #[cfg(windows)]
    #[test]
    fn a_missing_dll_reports_a_load_error() {
        let missing = std::env::temp_dir()
            .join("meetburner-no-such-dll-2026")
            .join(DLL_FILE);
        let error = imp::MfApi::load(&missing).err().expect("cannot load");
        assert!(error.contains("cannot load"), "{error}");
        assert!(error.contains(DLL_FILE), "{error}");
    }

    #[cfg(windows)]
    #[test]
    fn a_dll_without_the_exports_names_the_missing_symbol() {
        let system_root = std::env::var_os("SystemRoot").expect("SystemRoot");
        let kernel32 = PathBuf::from(system_root)
            .join("System32")
            .join("kernel32.dll");
        let error = imp::MfApi::load(&kernel32).err().expect("missing exports");
        assert!(error.contains("MB_MfSupported"), "{error}");
        assert!(error.contains("kernel32.dll"), "{error}");
    }

    #[cfg(windows)]
    #[test]
    fn the_host_build_number_is_read() {
        // The host runs Windows 10 or later; RtlGetVersion must return something sane.
        let build = current_build().expect("build number");
        assert!(build >= 10240, "unexpected build {build}");
    }
}
