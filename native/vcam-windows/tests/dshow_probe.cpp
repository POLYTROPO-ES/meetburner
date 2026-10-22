// Diagnostic tool: prints what a DirectShow consumer sees for "MeetBurner Camera".
// Usage: dshow_probe.exe
#include <windows.h>
#include <dshow.h>
#include <uuids.h>

#include <cstdio>
#include <string>

namespace {

const GUID kPinCategoryCapture = {0xfb6c4281,
                                  0x0353,
                                  0x11d1,
                                  {0x90, 0x5f, 0x00, 0x00, 0xc0, 0xcc, 0x16, 0xba}};

void PrintGuid(const char* label, const GUID& guid) {
    wchar_t buffer[64] = {};
    StringFromGUID2(guid, buffer, 64);
    printf("%s %ls\n", label, buffer);
}

} // namespace

int wmain() {
    CoInitializeEx(nullptr, COINIT_MULTITHREADED);

    ICreateDevEnum* device_enumerator = nullptr;
    HRESULT hr = CoCreateInstance(CLSID_SystemDeviceEnum, nullptr, CLSCTX_INPROC_SERVER,
                                  IID_PPV_ARGS(&device_enumerator));
    if (FAILED(hr)) {
        printf("CoCreateInstance(CLSID_SystemDeviceEnum) failed: 0x%08lX\n", hr);
        return 1;
    }

    IEnumMoniker* monikers = nullptr;
    hr = device_enumerator->CreateClassEnumerator(CLSID_VideoInputDeviceCategory, &monikers, 0);
    if (hr != S_OK) {
        printf("no video input devices (0x%08lX)\n", hr);
        return 1;
    }

    IBaseFilter* filter = nullptr;
    IMoniker* moniker = nullptr;
    while (monikers->Next(1, &moniker, nullptr) == S_OK) {
        IPropertyBag* bag = nullptr;
        moniker->BindToStorage(nullptr, nullptr, IID_PPV_ARGS(&bag));
        VARIANT value;
        VariantInit(&value);
        bag->Read(L"FriendlyName", &value, nullptr);
        const std::wstring name = value.bstrVal ? value.bstrVal : L"";
        VariantClear(&value);
        bag->Release();
        if (name == L"MeetBurner Camera") {
            hr = moniker->BindToObject(nullptr, nullptr, IID_PPV_ARGS(&filter));
            printf("found device, BindToObject: 0x%08lX\n", hr);
            moniker->Release();
            break;
        }
        moniker->Release();
        moniker = nullptr;
    }
    monikers->Release();
    device_enumerator->Release();

    if (!filter) {
        printf("MeetBurner Camera is not registered\n");
        return 1;
    }

    IEnumPins* pins = nullptr;
    hr = filter->EnumPins(&pins);
    printf("EnumPins: 0x%08lX\n", hr);
    if (FAILED(hr)) return 1;

    IPin* pin = nullptr;
    ULONG fetched = 0;
    while (pins->Next(1, &pin, &fetched) == S_OK) {
        PIN_INFO info = {};
        pin->QueryPinInfo(&info);
        printf("pin '%ls' dir=%d\n", info.achName, info.dir);
        if (info.pFilter) info.pFilter->Release();

        IKsPropertySet* properties = nullptr;
        hr = pin->QueryInterface(IID_PPV_ARGS(&properties));
        printf("  IKsPropertySet: 0x%08lX\n", hr);
        if (SUCCEEDED(hr)) {
            GUID category = {};
            DWORD returned = 0;
            const HRESULT get = properties->Get(AMPROPSETID_Pin, AMPROPERTY_PIN_CATEGORY, nullptr,
                                                0, &category, sizeof(category), &returned);
            printf("  Get(PIN_CATEGORY): 0x%08lX returned=%lu\n", get, returned);
            if (get == S_OK) {
                PrintGuid("  category:", category);
                printf("  is capture: %d\n", IsEqualGUID(category, kPinCategoryCapture));
            }
            DWORD supported = 0;
            hr = properties->QuerySupported(AMPROPSETID_Pin, AMPROPERTY_PIN_CATEGORY, &supported);
            printf("  QuerySupported: 0x%08lX support=%lu\n", hr, supported);
            properties->Release();
        }

        IEnumMediaTypes* types = nullptr;
        hr = pin->EnumMediaTypes(&types);
        if (SUCCEEDED(hr)) {
            AM_MEDIA_TYPE* type = nullptr;
            ULONG type_count = 0;
            while (types->Next(1, &type, &type_count) == S_OK && type_count > 0) {
                if (type->pbFormat && type->cbFormat >= sizeof(VIDEOINFOHEADER)) {
                    const auto* vih = reinterpret_cast<VIDEOINFOHEADER*>(type->pbFormat);
                    printf("  media type: %ldx%ld %u bpp\n", vih->bmiHeader.biWidth,
                           vih->bmiHeader.biHeight, vih->bmiHeader.biBitCount);
                } else {
                    printf("  media type: cbFormat=%lu\n", type->cbFormat);
                }
                if (type->pbFormat) CoTaskMemFree(type->pbFormat);
                CoTaskMemFree(type);
                type = nullptr;
            }
            types->Release();
        } else {
            printf("  EnumMediaTypes: 0x%08lX\n", hr);
        }
        pin->Release();
    }
    pins->Release();
    filter->Release();
    CoUninitialize();
    return 0;
}
