// DLL entry points and per-user (HKCU) registration for the MeetBurner Camera filter.
// Per-user registration means no UAC; regsvr32 /u removes every key it created (ADR-0016).
#include "vcam.h"

#include <olectl.h>
#include <wchar.h>

#include <new>
#include <string>

namespace {

const wchar_t* const kClsidText = L"{DBD3A890-1830-4222-AFEE-B1BA2C2DAB49}";
// CLSID_VideoInputDeviceCategory: what meeting apps enumerate to list cameras.
const wchar_t* const kVideoInputCategory = L"{860BB310-5D01-11D0-BD3B-00A0C911CE86}";

HMODULE g_module = nullptr;

bool CreateKey(const std::wstring& path, HKEY* key) {
    return RegCreateKeyExW(HKEY_CURRENT_USER, path.c_str(), 0, nullptr, 0, KEY_WRITE, nullptr, key,
                           nullptr) == ERROR_SUCCESS;
}

bool SetString(HKEY key, const wchar_t* name, const wchar_t* value) {
    const DWORD bytes = static_cast<DWORD>((wcslen(value) + 1) * sizeof(wchar_t));
    return RegSetValueExW(key, name, 0, REG_SZ, reinterpret_cast<const BYTE*>(value), bytes) ==
           ERROR_SUCCESS;
}

HRESULT Win32Error() { return HRESULT_FROM_WIN32(GetLastError()); }

class ClassFactory final : public IClassFactory {
public:
    ClassFactory() : ref_(1) { object_created(); }

    STDMETHODIMP QueryInterface(REFIID riid, void** ppv) override {
        if (!ppv) return E_POINTER;
        if (riid == IID_IUnknown || riid == IID_IClassFactory) {
            *ppv = static_cast<IClassFactory*>(this);
            AddRef();
            return S_OK;
        }
        *ppv = nullptr;
        return E_NOINTERFACE;
    }
    STDMETHODIMP_(ULONG) AddRef() override { return static_cast<ULONG>(InterlockedIncrement(&ref_)); }
    STDMETHODIMP_(ULONG) Release() override {
        const LONG now = InterlockedDecrement(&ref_);
        if (now == 0) delete this;
        return static_cast<ULONG>(now);
    }

    STDMETHODIMP CreateInstance(IUnknown* outer, REFIID riid, void** ppv) override {
        if (!ppv) return E_POINTER;
        *ppv = nullptr;
        if (outer) return CLASS_E_NOAGGREGATION;
        auto* filter = new (std::nothrow) CameraFilter();
        if (!filter) return E_OUTOFMEMORY;
        const HRESULT hr = filter->QueryInterface(riid, ppv);
        filter->Release();
        return hr;
    }

    STDMETHODIMP LockServer(BOOL) override { return S_OK; }

private:
    ~ClassFactory() { object_destroyed(); }

    LONG ref_;
};

} // namespace

BOOL APIENTRY DllMain(HMODULE module, DWORD reason, LPVOID) {
    if (reason == DLL_PROCESS_ATTACH) {
        g_module = module;
        DisableThreadLibraryCalls(module);
    }
    return TRUE;
}

extern "C" HRESULT __stdcall DllGetClassObject(REFCLSID clsid, REFIID riid, void** ppv) {
    if (!ppv) return E_POINTER;
    *ppv = nullptr;
    if (!IsEqualGUID(clsid, CLSID_MeetBurnerCamera)) return CLASS_E_CLASSNOTAVAILABLE;
    auto* factory = new (std::nothrow) ClassFactory();
    if (!factory) return E_OUTOFMEMORY;
    const HRESULT hr = factory->QueryInterface(riid, ppv);
    factory->Release();
    return hr;
}

extern "C" HRESULT __stdcall DllCanUnloadNow() {
    return live_object_count() == 0 ? S_OK : S_FALSE;
}

extern "C" HRESULT __stdcall DllRegisterServer() {
    wchar_t module_path[MAX_PATH] = {};
    if (!GetModuleFileNameW(g_module, module_path, MAX_PATH)) return Win32Error();

    const std::wstring clsid = std::wstring(L"Software\\Classes\\CLSID\\") + kClsidText;
    HKEY key = nullptr;

    if (!CreateKey(clsid, &key)) return Win32Error();
    SetString(key, nullptr, kMeetBurnerCameraName);
    RegCloseKey(key);

    if (!CreateKey(clsid + L"\\InprocServer32", &key)) return Win32Error();
    SetString(key, nullptr, module_path);
    SetString(key, L"ThreadingModel", L"Both");
    RegCloseKey(key);

    if (!CreateKey(clsid + L"\\Instance", &key)) return Win32Error();
    SetString(key, nullptr, kMeetBurnerCameraName);
    SetString(key, L"CLSID", kClsidText);
    SetString(key, L"FriendlyName", kMeetBurnerCameraName);
    RegCloseKey(key);

    const std::wstring category =
        std::wstring(L"Software\\Classes\\CLSID\\") + kVideoInputCategory + L"\\Instance\\" +
        kClsidText;
    if (!CreateKey(category, &key)) return Win32Error();
    SetString(key, nullptr, kMeetBurnerCameraName);
    SetString(key, L"CLSID", kClsidText);
    SetString(key, L"FriendlyName", kMeetBurnerCameraName);
    RegCloseKey(key);

    return S_OK;
}

extern "C" HRESULT __stdcall DllUnregisterServer() {
    const std::wstring clsid = std::wstring(L"Software\\Classes\\CLSID\\") + kClsidText;
    const std::wstring category =
        std::wstring(L"Software\\Classes\\CLSID\\") + kVideoInputCategory + L"\\Instance\\" +
        kClsidText;
    RegDeleteTreeW(HKEY_CURRENT_USER, category.c_str());
    RegDeleteTreeW(HKEY_CURRENT_USER, clsid.c_str());
    return S_OK;
}
