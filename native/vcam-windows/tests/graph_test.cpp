// Diagnostic tool: builds a minimal graph (MeetBurner Camera -> Null Renderer) and
// prints the HRESULT of each step. Usage: graph_test.exe [width height]
#include <windows.h>
#include <dshow.h>

#include <cstdio>

namespace {

// Keep in sync with dshow/vcam.cpp and dshow/dll.cpp.
const CLSID kCameraClsid = {0xDBD3A890,
                            0x1830,
                            0x4222,
                            {0xAF, 0xEE, 0xB1, 0xBA, 0x2C, 0x2D, 0xAB, 0x49}};
const CLSID kNullRendererClsid = {0xC1F400A4,
                                  0x3F08,
                                  0x11D3,
                                  {0x9F, 0x0B, 0x00, 0x60, 0x08, 0x03, 0x9E, 0x37}};

IPin* FindPin(IBaseFilter* filter, PIN_DIRECTION direction) {
    IEnumPins* pins = nullptr;
    if (FAILED(filter->EnumPins(&pins))) return nullptr;
    IPin* pin = nullptr;
    while (pins->Next(1, &pin, nullptr) == S_OK) {
        PIN_DIRECTION found = PINDIR_INPUT;
        pin->QueryDirection(&found);
        if (found == direction) {
            pins->Release();
            return pin;
        }
        pin->Release();
        pin = nullptr;
    }
    pins->Release();
    return nullptr;
}

IPin* PinById(IBaseFilter* filter, const wchar_t* id) {
    IPin* pin = nullptr;
    if (FAILED(filter->FindPin(id, &pin))) return nullptr;
    return pin;
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    const int width = argc > 1 ? _wtoi(argv[1]) : 1280;
    const int height = argc > 2 ? _wtoi(argv[2]) : 720;
    CoInitializeEx(nullptr, COINIT_MULTITHREADED);

    IGraphBuilder* graph = nullptr;
    HRESULT hr = CoCreateInstance(CLSID_FilterGraph, nullptr, CLSCTX_INPROC_SERVER,
                                  IID_PPV_ARGS(&graph));
    printf("CoCreateInstance(FilterGraph): 0x%08lX\n", hr);
    if (FAILED(hr)) return 1;

    IBaseFilter* camera = nullptr;
    hr = CoCreateInstance(kCameraClsid, nullptr, CLSCTX_INPROC_SERVER,
                          IID_PPV_ARGS(&camera));
    printf("CoCreateInstance(MeetBurnerCamera): 0x%08lX\n", hr);

    // Also fetch it through the device enumerator, like a meeting app does.
    {
        ICreateDevEnum* enumerator = nullptr;
        CoCreateInstance(CLSID_SystemDeviceEnum, nullptr, CLSCTX_INPROC_SERVER,
                         IID_PPV_ARGS(&enumerator));
        IEnumMoniker* monikers = nullptr;
        if (enumerator &&
            enumerator->CreateClassEnumerator(CLSID_VideoInputDeviceCategory, &monikers, 0) == S_OK) {
            IMoniker* moniker = nullptr;
            IBaseFilter* from_moniker = nullptr;
            while (monikers->Next(1, &moniker, nullptr) == S_OK) {
                IPropertyBag* bag = nullptr;
                moniker->BindToStorage(nullptr, nullptr, IID_PPV_ARGS(&bag));
                VARIANT name;
                VariantInit(&name);
                bag->Read(L"FriendlyName", &name, nullptr);
                const bool ours = name.bstrVal && wcscmp(name.bstrVal, L"MeetBurner Camera") == 0;
                VariantClear(&name);
                bag->Release();
                if (ours) {
                    hr = moniker->BindToObject(nullptr, nullptr, IID_PPV_ARGS(&from_moniker));
                    printf("BindToObject(moniker): 0x%08lX\n", hr);
                    moniker->Release();
                    break;
                }
                moniker->Release();
                moniker = nullptr;
            }
            if (monikers) {
                monikers->Release();
                monikers = nullptr;
            }
            if (from_moniker) from_moniker->Release();
        }
        if (enumerator) enumerator->Release();
    }

    if (FAILED(hr) || !camera) {
        printf("cannot create the camera filter\n");
        return 1;
    }

    hr = graph->AddFilter(camera, L"MeetBurner Camera");
    printf("AddFilter(camera): 0x%08lX\n", hr);

    IBaseFilter* renderer = nullptr;
    hr = CoCreateInstance(kNullRendererClsid, nullptr, CLSCTX_INPROC_SERVER,
                          IID_PPV_ARGS(&renderer));
    printf("CoCreateInstance(NullRenderer): 0x%08lX\n", hr);
    if (SUCCEEDED(hr)) {
        hr = graph->AddFilter(renderer, L"Null Renderer");
        printf("AddFilter(renderer): 0x%08lX\n", hr);
    }

    IPin* out = PinById(camera, L"Output");
    printf("FindPin(Output): %p\n", static_cast<void*>(out));
    IPin* in = renderer ? FindPin(renderer, PINDIR_INPUT) : nullptr;
    printf("find renderer input: %p\n", static_cast<void*>(in));

    if (out && in) {
        AM_MEDIA_TYPE* type = nullptr;
        IAMStreamConfig* config = nullptr;
        if (SUCCEEDED(out->QueryInterface(IID_PPV_ARGS(&config)))) {
            hr = config->GetFormat(&type);
            printf("GetFormat: 0x%08lX\n", hr);
            if (SUCCEEDED(hr) && type) {
                if (type->pbFormat) CoTaskMemFree(type->pbFormat);
                CoTaskMemFree(type);
            }
            config->Release();
        }

        hr = graph->ConnectDirect(out, in, nullptr);
        printf("ConnectDirect: 0x%08lX\n", hr);

        if (SUCCEEDED(hr)) {
            IMediaControl* control = nullptr;
            graph->QueryInterface(IID_PPV_ARGS(&control));
            hr = control->Run();
            printf("Run: 0x%08lX\n", hr);
            Sleep(500);
            control->Stop();
            control->Release();
        }
    }

    if (in) in->Release();
    if (out) out->Release();
    if (renderer) renderer->Release();
    camera->Release();
    graph->Release();
    CoUninitialize();
    return 0;
}
