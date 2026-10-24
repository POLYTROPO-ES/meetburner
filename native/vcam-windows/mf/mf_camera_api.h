#pragma once
/*
 * C exports of MeetBurnerMfCamera.dll. The Rust app resolves them with
 * LoadLibraryW + GetProcAddress (camera/mf.rs, ADR-0031), so this DLL can be built and
 * shipped on every Windows version: on Windows 10 the exports simply report "not
 * supported" and MFCreateVirtualCamera is never resolved.
 *
 * Keep MB_MF_CLSID, MB_MF_FRIENDLY_NAME and the export names in sync with
 * apps/desktop/src-tauri/src/camera/mf.rs.
 */
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#ifdef __cplusplus
extern "C" {
#endif

/* CLSID of the in-proc IMFMediaSource (dll.cpp, camera/mf.rs). */
#define MB_MF_CLSID L"{9D6A2B41-3C57-4A0E-8F2D-7B1E5C3A9F64}"

/* Distinct from the DirectShow filter ("MeetBurner Camera"), so meeting apps never list
 * two identical entries (ADR-0031). */
#define MB_MF_FRIENDLY_NAME L"MeetBurner Camera (Windows 11)"

/* 1 when the OS is Windows 11 (build 22000+) and MFCreateVirtualCamera can be loaded. */
int __stdcall MB_MfSupported(void);

/* Registers the COM server under HKCU, creates the per-user virtual camera and starts it.
 * `friendlyName` may be NULL to use MB_MF_FRIENDLY_NAME. Returns an HRESULT. */
HRESULT __stdcall MB_MfRegister(LPCWSTR friendlyName);

/* Removes the virtual camera from the system and deletes the per-user COM registration.
 * Returns an HRESULT. */
HRESULT __stdcall MB_MfUnregister(void);

#ifdef __cplusplus
}
#endif
