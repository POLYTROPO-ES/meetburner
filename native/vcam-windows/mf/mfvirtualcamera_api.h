#pragma once
/*
 * Minimal mirror of the Windows 11 SDK header <mfvirtualcamera.h> (Windows SDK
 * 10.0.22000 and later). The DLL resolves MFCreateVirtualCamera at run time (ADR-0031),
 * so declaring the few pieces it uses here keeps the build working with older SDKs and
 * keeps the DLL loadable on Windows 10, where the function does not exist.
 *
 * The vtable order of IMFVirtualCamera must match the SDK: only Start, Stop and Remove are
 * called, but the slots before them have to line up.
 */
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <rpc.h>
#include <rpcndr.h>
#include <mfobjects.h>

enum MFVirtualCameraType {
    MFVirtualCameraType_SoftwareCameraSource = 0,
};

enum MFVirtualCameraLifetime {
    MFVirtualCameraLifetime_Session = 0,
    MFVirtualCameraLifetime_System = 1,
};

enum MFVirtualCameraAccess {
    MFVirtualCameraAccess_CurrentUser = 0,
    MFVirtualCameraAccess_AllUsers = 1,
};

MIDL_INTERFACE("1C08A864-EF6C-4C75-AF59-5F2D68DA9563")
IMFVirtualCamera : public IMFAttributes {
public:
    virtual HRESULT STDMETHODCALLTYPE AddDeviceSourceInfo(LPCWSTR device_source_info) = 0;
    virtual HRESULT STDMETHODCALLTYPE AddProperty(const void* key, UINT32 type, const BYTE* data,
                                                  UINT32 size) = 0;
    virtual HRESULT STDMETHODCALLTYPE AddRegistryEntry(LPCWSTR entry_name, LPCWSTR subkey_path,
                                                       UINT32 reg_type, const BYTE* data,
                                                       UINT32 size) = 0;
    virtual HRESULT STDMETHODCALLTYPE Start(IMFAsyncCallback* callback) = 0;
    virtual HRESULT STDMETHODCALLTYPE Stop(void) = 0;
    virtual HRESULT STDMETHODCALLTYPE Remove(void) = 0;
};

using MFCreateVirtualCameraFn = HRESULT(WINAPI*)(MFVirtualCameraType type,
                                                 MFVirtualCameraLifetime lifetime,
                                                 MFVirtualCameraAccess access,
                                                 LPCWSTR friendly_name, LPCWSTR source_id,
                                                 const GUID* categories, ULONG category_count,
                                                 IMFVirtualCamera** camera);
