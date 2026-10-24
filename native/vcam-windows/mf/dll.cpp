// MeetBurner Camera (Windows 11): an in-proc IMFMediaSource that publishes the frames the
// Rust app writes into shared memory (ADR-0013), plus the COM server, the per-user COM
// registration and the MB_Mf* exports the app calls. See ADR-0031.
//
// Design notes:
//  * The media source is "pull" mode: every RequestSample reads the latest shared-memory
//    frame, copies it into a top-down RGB32 sample and queues MEMediaSample. This is the
//    shape the frame server expects from a custom media source and needs no worker thread.
//  * MF_MT_DEFAULT_STRIDE is positive (5120), which tells Media Foundation the payload is
//    top-down — the producer's own order — so no row flip is needed or wanted. A flip
//    would render the picture upside down in every consumer (ADR-0031 amendment).
//  * MFCreateVirtualCamera is resolved at run time: the DLL builds and loads on Windows 10,
//    where the function is absent, and MB_MfSupported reports 0 there.
#include "mf_camera_api.h"
#include "mfvirtualcamera_api.h"

#include <mferror.h>
#include <mfapi.h>
#include <mfidl.h>
#include <mfobjects.h>
// ks.h declares IKsControl (and its IID) only once IUnknown is known, so it comes after
// the MF headers and an explicit unknwn.h.
#include <unknwn.h>
#include <ks.h>
#include <winternl.h>

#include <cstring>
#include <mutex>
#include <new>
#include <string>
#include <vector>

#include "../common/meetburner_camera.h"
#include "frames.h"

namespace {

// ks.h only declares IKsControl when `_UNKNOWN_H_` is defined, which no current C++ SDK
// header does. The frame server requires the interface, so it is declared here with the
// documented IID (KSDDKAPI): all three methods answer "no such control".
MIDL_INTERFACE("28F54685-06FD-11D2-B27A-00A0C9223196")
IMeetBurnerKsControl : public IUnknown {
public:
    virtual HRESULT STDMETHODCALLTYPE KsProperty(PKSPROPERTY property, ULONG property_length,
                                                 LPVOID property_data, ULONG data_length,
                                                 ULONG* bytes_returned) = 0;
    virtual HRESULT STDMETHODCALLTYPE KsMethod(PKSMETHOD method, ULONG method_length,
                                               LPVOID method_data, ULONG data_length,
                                               ULONG* bytes_returned) = 0;
    virtual HRESULT STDMETHODCALLTYPE KsEvent(PKSEVENT event, ULONG event_length,
                                              LPVOID event_data, ULONG data_length,
                                              ULONG* bytes_returned) = 0;
};

const CLSID CLSID_MeetBurnerMfCamera = {
    0x9D6A2B41, 0x3C57, 0x4A0E, {0x8F, 0x2D, 0x7B, 0x1E, 0x5C, 0x3A, 0x9F, 0x64}};

// PINNAME_VIDEO_CAPTURE from uuids.h. Defined here so the DLL does not link strmiids.
const GUID kPinNameVideoCapture = {
    0xFB6C4281, 0x0353, 0x11D1, {0x90, 0x5F, 0x00, 0x00, 0xC0, 0xCC, 0x16, 0xBA}};

const wchar_t* const kMarkerValue = L"MeetBurnerMfRegistered";
const wchar_t* const kNameValue = L"MeetBurnerMfFriendlyName";
const wchar_t* const kThreadingValue = L"ThreadingModel";

constexpr UINT32 kDefaultWidth = 1280;
constexpr UINT32 kDefaultHeight = 720;
constexpr UINT32 kDefaultFps = 30;
// A consumer asking for anything bigger is clamped: the producer never publishes more.
constexpr UINT32 kMaxWidth = 3840;
constexpr UINT32 kMaxHeight = 2160;
constexpr LONGLONG kFrameDuration = 333333; // 30 fps in 100 ns units
// A frame older than this means the app stopped publishing (ADR-0019).
constexpr uint64_t kStaleMs = 2000;

HMODULE g_module = nullptr;
LONG g_objects = 0;

template <typename T>
class ComPtr {
public:
    ComPtr() = default;
    ~ComPtr() { reset(); }
    ComPtr(const ComPtr&) = delete;
    ComPtr& operator=(const ComPtr&) = delete;

    T* get() const { return ptr_; }
    T** put() {
        reset();
        return &ptr_;
    }
    T* operator->() const { return ptr_; }
    explicit operator bool() const { return ptr_ != nullptr; }
    void reset() {
        if (ptr_) {
            ptr_->Release();
            ptr_ = nullptr;
        }
    }
    // Takes ownership of `ptr` (no AddRef).
    void reset(T* ptr) {
        reset();
        ptr_ = ptr;
    }
    T* detach() {
        T* ptr = ptr_;
        ptr_ = nullptr;
        return ptr;
    }

private:
    T* ptr_ = nullptr;
};

bool CreateKey(const std::wstring& path, HKEY* key) {
    return RegCreateKeyExW(HKEY_CURRENT_USER, path.c_str(), 0, nullptr, 0, KEY_WRITE, nullptr,
                           key, nullptr) == ERROR_SUCCESS;
}

bool SetString(HKEY key, const wchar_t* name, const wchar_t* value) {
    const DWORD bytes = static_cast<DWORD>((wcslen(value) + 1) * sizeof(wchar_t));
    return RegSetValueExW(key, name, 0, REG_SZ, reinterpret_cast<const BYTE*>(value), bytes) ==
           ERROR_SUCCESS;
}

bool SetDword(HKEY key, const wchar_t* name, DWORD value) {
    return RegSetValueExW(key, name, 0, REG_DWORD, reinterpret_cast<const BYTE*>(&value),
                          sizeof(value)) == ERROR_SUCCESS;
}

std::wstring ClsidKey() { return std::wstring(L"Software\\Classes\\CLSID\\") + MB_MF_CLSID; }

LONG WindowsBuildNumber() {
    HMODULE ntdll = GetModuleHandleW(L"ntdll.dll");
    if (!ntdll) return 0;
    using RtlGetVersionFn = LONG(WINAPI*)(PRTL_OSVERSIONINFOW);
    auto fn = reinterpret_cast<RtlGetVersionFn>(GetProcAddress(ntdll, "RtlGetVersion"));
    if (!fn) return 0;
    RTL_OSVERSIONINFOW info = {};
    info.dwOSVersionInfoSize = sizeof(info);
    if (fn(&info) != 0) return 0;
    return static_cast<LONG>(info.dwBuildNumber);
}

// The documented location is mfsensorgroup.dll; some builds also expose mfvirtualcamera.dll.
// Try both and keep the module that actually exports the function loaded for the process.
MFCreateVirtualCameraFn CreateVirtualCamera() {
    static MFCreateVirtualCameraFn entry = [] {
        const wchar_t* const modules[] = {L"mfvirtualcamera.dll", L"mfsensorgroup.dll"};
        for (const wchar_t* name : modules) {
            HMODULE module = LoadLibraryW(name);
            if (!module) continue;
            auto fn = reinterpret_cast<MFCreateVirtualCameraFn>(
                GetProcAddress(module, "MFCreateVirtualCamera"));
            if (fn) return fn;
            FreeLibrary(module);
        }
        return static_cast<MFCreateVirtualCameraFn>(nullptr);
    }();
    return entry;
}

HRESULT RegisterComServer(const wchar_t* friendly_name) {
    wchar_t module_path[MAX_PATH] = {};
    if (!GetModuleFileNameW(g_module, module_path, MAX_PATH)) {
        return HRESULT_FROM_WIN32(GetLastError());
    }
    const std::wstring clsid = ClsidKey();
    HKEY key = nullptr;
    if (!CreateKey(clsid, &key)) return HRESULT_FROM_WIN32(GetLastError());
    SetString(key, nullptr, friendly_name);
    RegCloseKey(key);

    if (!CreateKey(clsid + L"\\InprocServer32", &key)) return HRESULT_FROM_WIN32(GetLastError());
    SetString(key, nullptr, module_path);
    SetString(key, kThreadingValue, L"Both");
    RegCloseKey(key);
    return S_OK;
}

void UnregisterComServer() {
    RegDeleteTreeW(HKEY_CURRENT_USER, ClsidKey().c_str());
}

// Records that the virtual camera was created, so the app can report "registered"
// (camera/mf.rs). Removed together with the CLSID tree by MB_MfUnregister.
void WriteMarker(const wchar_t* friendly_name) {
    HKEY key = nullptr;
    if (!CreateKey(ClsidKey(), &key)) return;
    SetDword(key, kMarkerValue, 1);
    SetString(key, kNameValue, friendly_name);
    RegCloseKey(key);
}

// Friendly name recorded by the last MB_MfRegister; MFCreateVirtualCamera keys the camera
// off it, so unregister has to reuse exactly the same one.
std::wstring RegisteredName() {
    wchar_t buffer[256] = {};
    DWORD size = sizeof(buffer);
    const std::wstring key = ClsidKey();
    const LSTATUS status = RegGetValueW(HKEY_CURRENT_USER, key.c_str(), kNameValue, RRF_RT_REG_SZ,
                                        nullptr, buffer, &size);
    return status == ERROR_SUCCESS ? std::wstring(buffer) : std::wstring(MB_MF_FRIENDLY_NAME);
}

uint64_t UnixMsNow() {
    FILETIME file_time;
    GetSystemTimeAsFileTime(&file_time);
    ULARGE_INTEGER ticks;
    ticks.LowPart = file_time.dwLowDateTime;
    ticks.HighPart = file_time.dwHighDateTime;
    return (ticks.QuadPart - 116444736000000000ULL) / 10000ULL; // 1601 -> 1970, 100 ns -> ms
}

bool IsStale(const meetburner_frame_header& header) {
    return header.state != MB_CAMERA_STATE_LIVE || header.timestamp_ms == 0 ||
           UnixMsNow() > header.timestamp_ms + kStaleMs;
}

HRESULT MakeStreamAttributes(IMFAttributes* attributes, UINT32 stream_id) {
    if (!attributes) return E_POINTER;
    HRESULT hr = attributes->SetGUID(MF_DEVICESTREAM_STREAM_CATEGORY, kPinNameVideoCapture);
    if (FAILED(hr)) return hr;
    hr = attributes->SetUINT32(MF_DEVICESTREAM_STREAM_ID, stream_id);
    if (FAILED(hr)) return hr;
    // Two cameras (DirectShow and MF) may serve the same meeting app: let the frame server
    // share the stream instead of taking exclusive control.
    hr = attributes->SetUINT32(MF_DEVICESTREAM_FRAMESERVER_SHARED, 1);
    if (FAILED(hr)) return hr;
    return attributes->SetUINT32(MF_DEVICESTREAM_ATTRIBUTE_FRAMESOURCE_TYPES,
                                 MFFrameSourceTypes_Color);
}

class MediaSource;

// One video stream. RequestSample is the whole delivery loop: the frame server asks, the
// stream answers with the newest frame (or the "not running" placeholder).
class MediaStream final : public IMFMediaStream2 {
public:
    MediaStream(MediaSource* source, DWORD stream_id);

    // IUnknown
    STDMETHODIMP QueryInterface(REFIID riid, void** ppv) override;
    STDMETHODIMP_(ULONG) AddRef() override;
    STDMETHODIMP_(ULONG) Release() override;

    // IMFMediaEventGenerator
    STDMETHODIMP GetEvent(DWORD flags, IMFMediaEvent** event) override;
    STDMETHODIMP BeginGetEvent(IMFAsyncCallback* callback, IUnknown* state) override;
    STDMETHODIMP EndGetEvent(IMFAsyncResult* result, IMFMediaEvent** event) override;
    STDMETHODIMP QueueEvent(MediaEventType type, REFGUID extended_type, HRESULT status,
                            const PROPVARIANT* value) override;

    // IMFMediaStream
    STDMETHODIMP GetMediaSource(IMFMediaSource** media_source) override;
    STDMETHODIMP GetStreamDescriptor(IMFStreamDescriptor** descriptor) override;
    STDMETHODIMP RequestSample(IUnknown* token) override;

    // IMFMediaStream2
    STDMETHODIMP SetStreamState(MF_STREAM_STATE state) override;
    STDMETHODIMP GetStreamState(MF_STREAM_STATE* state) override;

    HRESULT Initialize();
    HRESULT Start(IMFMediaType* media_type);
    HRESULT Stop(bool send_event);
    void Shutdown();
    // Called by the source when it goes away: the stream must not keep a dangling pointer.
    void DetachSource();
    IMFAttributes* attributes() const { return attributes_; }

private:
    ~MediaStream();

    HRESULT ProduceSample(IUnknown* token);
    HRESULT EnsureReader();
    void EnsurePlaceholder();
    HRESULT CheckShutdown() const { return shutdown_ ? MF_E_SHUTDOWN : S_OK; }

    std::mutex lock_;
    LONG ref_ = 1;
    MediaSource* source_ = nullptr;
    DWORD stream_id_ = 0;
    IMFMediaEventQueue* events_ = nullptr;
    IMFStreamDescriptor* descriptor_ = nullptr;
    IMFAttributes* attributes_ = nullptr;
    IMFMediaType* media_type_ = nullptr;
    MF_STREAM_STATE state_ = MF_STREAM_STATE_STOPPED;
    bool shutdown_ = false;

    UINT32 width_ = kDefaultWidth;
    UINT32 height_ = kDefaultHeight;
    std::vector<uint8_t> scratch_;      // newest frame, top-down
    std::vector<uint8_t> placeholder_;  // "not running", top-down
    mb_reader reader_ = {};
    bool reader_open_ = false;
    uint64_t next_open_ms_ = 0;

    // Sample times follow the frame header timestamp, anchored on QPC (MF requires QPC).
    uint64_t anchor_ts_ms_ = 0;
    LONGLONG anchor_qpc_ = 0;
    LONGLONG last_sample_time_ = 0;
};

class MediaSource final : public IMFMediaSourceEx, public IMFGetService, public IMeetBurnerKsControl {
public:
    MediaSource();

    // IUnknown
    STDMETHODIMP QueryInterface(REFIID riid, void** ppv) override;
    STDMETHODIMP_(ULONG) AddRef() override;
    STDMETHODIMP_(ULONG) Release() override;

    // IMFMediaEventGenerator
    STDMETHODIMP GetEvent(DWORD flags, IMFMediaEvent** event) override;
    STDMETHODIMP BeginGetEvent(IMFAsyncCallback* callback, IUnknown* state) override;
    STDMETHODIMP EndGetEvent(IMFAsyncResult* result, IMFMediaEvent** event) override;
    STDMETHODIMP QueueEvent(MediaEventType type, REFGUID extended_type, HRESULT status,
                            const PROPVARIANT* value) override;

    // IMFMediaSource
    STDMETHODIMP CreatePresentationDescriptor(IMFPresentationDescriptor** descriptor) override;
    STDMETHODIMP GetCharacteristics(DWORD* characteristics) override;
    STDMETHODIMP Start(IMFPresentationDescriptor* descriptor, const GUID* time_format,
                       const PROPVARIANT* start_position) override;
    STDMETHODIMP Stop() override;
    STDMETHODIMP Pause() override;
    STDMETHODIMP Shutdown() override;

    // IMFMediaSourceEx
    STDMETHODIMP GetSourceAttributes(IMFAttributes** attributes) override;
    STDMETHODIMP GetStreamAttributes(DWORD stream_id, IMFAttributes** attributes) override;
    STDMETHODIMP SetD3DManager(IUnknown* manager) override;

    // IMFGetService
    STDMETHODIMP GetService(REFGUID service, REFIID riid, LPVOID* object) override;

    // IKsControl: no camera controls, which is a valid answer for the frame server.
    STDMETHODIMP KsProperty(PKSPROPERTY property, ULONG property_length, LPVOID property_data,
                            ULONG data_length, ULONG* bytes_returned) override;
    STDMETHODIMP KsMethod(PKSMETHOD method, ULONG method_length, LPVOID method_data,
                          ULONG data_length, ULONG* bytes_returned) override;
    STDMETHODIMP KsEvent(PKSEVENT event, ULONG event_length, LPVOID event_data, ULONG data_length,
                         ULONG* bytes_returned) override;

    HRESULT Initialize();

private:
    ~MediaSource();
    HRESULT CheckShutdown() const { return shutdown_ ? MF_E_SHUTDOWN : S_OK; }

    std::mutex lock_;
    LONG ref_ = 1;
    IMFMediaEventQueue* events_ = nullptr;
    IMFAttributes* attributes_ = nullptr;
    IMFPresentationDescriptor* presentation_ = nullptr;
    MediaStream* stream_ = nullptr;
    bool started_ = false;
    bool announced_ = false;
    bool shutdown_ = false;
};

// ---------------------------------------------------------------- MediaStream

MediaStream::MediaStream(MediaSource* source, DWORD stream_id)
    : source_(source), stream_id_(stream_id) {
    InterlockedIncrement(&g_objects);
    // No AddRef: the source owns the stream and clears this pointer before it dies.
    scratch_.resize(MB_CAMERA_CAPACITY_PIXELS * 4);
}

MediaStream::~MediaStream() {
    if (reader_open_) mb_reader_close(&reader_);
    if (media_type_) media_type_->Release();
    if (attributes_) attributes_->Release();
    if (descriptor_) descriptor_->Release();
    if (events_) {
        events_->Shutdown();
        events_->Release();
    }
    InterlockedDecrement(&g_objects);
}

STDMETHODIMP MediaStream::QueryInterface(REFIID riid, void** ppv) {
    if (!ppv) return E_POINTER;
    *ppv = nullptr;
    if (IsEqualIID(riid, IID_IUnknown) || IsEqualIID(riid, IID_IMFMediaEventGenerator) ||
        IsEqualIID(riid, IID_IMFMediaStream) || IsEqualIID(riid, IID_IMFMediaStream2)) {
        *ppv = static_cast<IMFMediaStream2*>(this);
    } else {
        return E_NOINTERFACE;
    }
    AddRef();
    return S_OK;
}

STDMETHODIMP_(ULONG) MediaStream::AddRef() {
    return static_cast<ULONG>(InterlockedIncrement(&ref_));
}

STDMETHODIMP_(ULONG) MediaStream::Release() {
    const LONG now = InterlockedDecrement(&ref_);
    if (now == 0) delete this;
    return static_cast<ULONG>(now);
}

STDMETHODIMP MediaStream::GetEvent(DWORD flags, IMFMediaEvent** event) {
    // GetEvent may block; never hold the lock across it.
    ComPtr<IMFMediaEventQueue> queue;
    {
        std::lock_guard<std::mutex> guard(lock_);
        const HRESULT hr = CheckShutdown();
        if (FAILED(hr)) return hr;
        queue.reset(events_);
        if (!queue) return E_UNEXPECTED;
        queue->AddRef();
    }
    return queue->GetEvent(flags, event);
}

STDMETHODIMP MediaStream::BeginGetEvent(IMFAsyncCallback* callback, IUnknown* state) {
    std::lock_guard<std::mutex> guard(lock_);
    const HRESULT hr = CheckShutdown();
    if (FAILED(hr)) return hr;
    return events_->BeginGetEvent(callback, state);
}

STDMETHODIMP MediaStream::EndGetEvent(IMFAsyncResult* result, IMFMediaEvent** event) {
    std::lock_guard<std::mutex> guard(lock_);
    const HRESULT hr = CheckShutdown();
    if (FAILED(hr)) return hr;
    return events_->EndGetEvent(result, event);
}

STDMETHODIMP MediaStream::QueueEvent(MediaEventType type, REFGUID extended_type, HRESULT status,
                                     const PROPVARIANT* value) {
    std::lock_guard<std::mutex> guard(lock_);
    const HRESULT hr = CheckShutdown();
    if (FAILED(hr)) return hr;
    return events_->QueueEventParamVar(type, extended_type, status, value);
}

STDMETHODIMP MediaStream::GetMediaSource(IMFMediaSource** media_source) {
    if (!media_source) return E_POINTER;
    *media_source = nullptr;
    std::lock_guard<std::mutex> guard(lock_);
    const HRESULT hr = CheckShutdown();
    if (FAILED(hr)) return hr;
    if (!source_) return E_UNEXPECTED;
    source_->AddRef();
    *media_source = source_;
    return S_OK;
}

STDMETHODIMP MediaStream::GetStreamDescriptor(IMFStreamDescriptor** descriptor) {
    if (!descriptor) return E_POINTER;
    *descriptor = nullptr;
    std::lock_guard<std::mutex> guard(lock_);
    const HRESULT hr = CheckShutdown();
    if (FAILED(hr)) return hr;
    descriptor_->AddRef();
    *descriptor = descriptor_;
    return S_OK;
}

STDMETHODIMP MediaStream::RequestSample(IUnknown* token) {
    std::lock_guard<std::mutex> guard(lock_);
    const HRESULT hr = CheckShutdown();
    if (FAILED(hr)) return hr;
    if (state_ != MF_STREAM_STATE_RUNNING) return MF_E_INVALIDREQUEST;
    return ProduceSample(token);
}

STDMETHODIMP MediaStream::SetStreamState(MF_STREAM_STATE state) {
    std::lock_guard<std::mutex> guard(lock_);
    const HRESULT hr = CheckShutdown();
    if (FAILED(hr)) return hr;
    if (state_ == state) return S_OK;
    switch (state) {
    case MF_STREAM_STATE_PAUSED:
        if (state_ != MF_STREAM_STATE_RUNNING) return MF_E_INVALID_STATE_TRANSITION;
        break;
    case MF_STREAM_STATE_RUNNING:
    case MF_STREAM_STATE_STOPPED:
        break;
    default:
        return MF_E_INVALID_STATE_TRANSITION;
    }
    state_ = state;
    return S_OK;
}

STDMETHODIMP MediaStream::GetStreamState(MF_STREAM_STATE* state) {
    if (!state) return E_POINTER;
    std::lock_guard<std::mutex> guard(lock_);
    const HRESULT hr = CheckShutdown();
    if (FAILED(hr)) return hr;
    *state = state_;
    return S_OK;
}

HRESULT MediaStream::Initialize() {
    ComPtr<IMFMediaType> type;
    HRESULT hr = MFCreateMediaType(type.put());
    if (FAILED(hr)) return hr;
    hr = type->SetGUID(MF_MT_MAJOR_TYPE, MFMediaType_Video);
    if (FAILED(hr)) return hr;
    hr = type->SetGUID(MF_MT_SUBTYPE, MFVideoFormat_RGB32);
    if (FAILED(hr)) return hr;
    hr = type->SetUINT32(MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive);
    if (FAILED(hr)) return hr;
    hr = type->SetUINT32(MF_MT_ALL_SAMPLES_INDEPENDENT, TRUE);
    if (FAILED(hr)) return hr;
    hr = type->SetUINT32(MF_MT_FIXED_SIZE_SAMPLES, TRUE);
    if (FAILED(hr)) return hr;
    hr = MFSetAttributeSize(type.get(), MF_MT_FRAME_SIZE, kDefaultWidth, kDefaultHeight);
    if (FAILED(hr)) return hr;
    hr = MFSetAttributeRatio(type.get(), MF_MT_FRAME_RATE, kDefaultFps, 1);
    if (FAILED(hr)) return hr;
    hr = MFSetAttributeRatio(type.get(), MF_MT_PIXEL_ASPECT_RATIO, 1, 1);
    if (FAILED(hr)) return hr;
    // Positive stride: top-down rows, the order the producer writes. The sample is filled
    // with FitBgra, which keeps that order (ADR-0031 amendment); DirectShow is the camera
    // that needs bottom-up rows, not this one.
    hr = type->SetUINT32(MF_MT_DEFAULT_STRIDE, kDefaultWidth * 4);
    if (FAILED(hr)) return hr;
    hr = type->SetUINT32(MF_MT_SAMPLE_SIZE, kDefaultWidth * kDefaultHeight * 4);
    if (FAILED(hr)) return hr;
    hr = type->SetUINT32(MF_MT_AVG_BITRATE, kDefaultWidth * kDefaultHeight * 4 * 8 * kDefaultFps);
    if (FAILED(hr)) return hr;

    IMFMediaType* types[1] = {type.get()};
    hr = MFCreateStreamDescriptor(stream_id_, 1, types, &descriptor_);
    if (FAILED(hr)) return hr;
    hr = MakeStreamAttributes(descriptor_, stream_id_);
    if (FAILED(hr)) return hr;

    ComPtr<IMFMediaTypeHandler> handler;
    hr = descriptor_->GetMediaTypeHandler(handler.put());
    if (FAILED(hr)) return hr;
    hr = handler->SetCurrentMediaType(type.get());
    if (FAILED(hr)) return hr;

    hr = MFCreateAttributes(&attributes_, 4);
    if (FAILED(hr)) return hr;
    hr = MakeStreamAttributes(attributes_, stream_id_);
    if (FAILED(hr)) return hr;

    hr = MFCreateEventQueue(&events_);
    if (FAILED(hr)) return hr;

    media_type_ = type.detach();
    return S_OK;
}

HRESULT MediaStream::Start(IMFMediaType* media_type) {
    std::lock_guard<std::mutex> guard(lock_);
    const HRESULT check = CheckShutdown();
    if (FAILED(check)) return check;
    if (media_type) {
        UINT32 width = kDefaultWidth;
        UINT32 height = kDefaultHeight;
        MFGetAttributeSize(media_type, MF_MT_FRAME_SIZE, &width, &height);
        // Refuse rather than fill a buffer smaller than the negotiated media type.
        if (width == 0 || height == 0 || width > kMaxWidth || height > kMaxHeight) {
            return MF_E_INVALIDMEDIATYPE;
        }
        width_ = width;
        height_ = height;
        media_type->AddRef();
        if (media_type_) media_type_->Release();
        media_type_ = media_type;
    }
    anchor_ts_ms_ = 0;
    last_sample_time_ = 0;
    EnsurePlaceholder();
    state_ = MF_STREAM_STATE_RUNNING;
    // MEStreamStarted belongs to the stream queue; MENewStream / MESourceStarted are queued
    // by the source (frame-server custom media source contract).
    return events_->QueueEventParamVar(MEStreamStarted, GUID_NULL, S_OK, nullptr);
}

HRESULT MediaStream::Stop(bool send_event) {
    std::lock_guard<std::mutex> guard(lock_);
    const HRESULT check = CheckShutdown();
    if (FAILED(check)) return check;
    state_ = MF_STREAM_STATE_STOPPED;
    anchor_ts_ms_ = 0;
    if (!send_event) return S_OK;
    return events_->QueueEventParamVar(MEStreamStopped, GUID_NULL, S_OK, nullptr);
}

void MediaStream::Shutdown() {
    std::lock_guard<std::mutex> guard(lock_);
    if (shutdown_) return;
    shutdown_ = true;
    if (events_) {
        events_->Shutdown();
        events_->Release();
        events_ = nullptr;
    }
    if (reader_open_) {
        mb_reader_close(&reader_);
        reader_open_ = false;
    }
    if (media_type_) {
        media_type_->Release();
        media_type_ = nullptr;
    }
}

void MediaStream::DetachSource() {
    std::lock_guard<std::mutex> guard(lock_);
    source_ = nullptr;
}

HRESULT MediaStream::EnsureReader() {
    if (reader_open_) return S_OK;
    const uint64_t now = GetTickCount64();
    if (now < next_open_ms_) return S_FALSE; // retry at most once a second
    next_open_ms_ = now + 1000;
    reader_open_ = mb_reader_open(&reader_, MB_CAMERA_MAPPING_NAME, MB_CAMERA_EVENT_NAME);
    return reader_open_ ? S_OK : S_FALSE;
}

void MediaStream::EnsurePlaceholder() {
    const size_t bytes = static_cast<size_t>(width_) * height_ * 4;
    if (placeholder_.size() != bytes) placeholder_ = RenderNotRunning(width_, height_);
}

HRESULT MediaStream::ProduceSample(IUnknown* token) {
    const uint8_t* source = placeholder_.data();
    UINT32 source_width = width_;
    UINT32 source_height = height_;
    bool live = false;
    meetburner_frame_header header = {};

    EnsureReader();
    if (reader_open_ &&
        mb_read_frame(&reader_, scratch_.data(), scratch_.size(), &header) && !IsStale(header)) {
        source = scratch_.data();
        source_width = header.width;
        source_height = header.height;
        live = true;
    }

    const size_t bytes = static_cast<size_t>(width_) * height_ * 4;
    ComPtr<IMFMediaBuffer> buffer;
    HRESULT hr = MFCreateMemoryBuffer(static_cast<DWORD>(bytes), buffer.put());
    if (FAILED(hr)) return hr;
    BYTE* destination = nullptr;
    DWORD maximum = 0;
    hr = buffer->Lock(&destination, &maximum, nullptr);
    if (FAILED(hr)) return hr;
    if (maximum < bytes || !destination) {
        buffer->Unlock();
        return E_OUTOFMEMORY;
    }
    FitBgra(source, source_width, source_height, destination, width_ * 4, width_, height_);
    buffer->Unlock();
    hr = buffer->SetCurrentLength(static_cast<DWORD>(bytes));
    if (FAILED(hr)) return hr;

    ComPtr<IMFSample> sample;
    hr = MFCreateSample(sample.put());
    if (FAILED(hr)) return hr;
    hr = sample->AddBuffer(buffer.get());
    if (FAILED(hr)) return hr;

    // Sample times must be QPC based; the frame header gives the cadence.
    const LONGLONG now = MFGetSystemTime();
    LONGLONG sample_time;
    if (live) {
        if (anchor_ts_ms_ == 0) {
            anchor_ts_ms_ = header.timestamp_ms;
            anchor_qpc_ = now;
        }
        sample_time =
            anchor_qpc_ + static_cast<LONGLONG>(header.timestamp_ms - anchor_ts_ms_) * 10000;
    } else {
        anchor_ts_ms_ = 0;
        sample_time = last_sample_time_ > 0 ? last_sample_time_ + kFrameDuration : now;
    }
    if (sample_time <= last_sample_time_) sample_time = last_sample_time_ + 1;
    last_sample_time_ = sample_time;
    sample->SetSampleTime(sample_time);
    sample->SetSampleDuration(kFrameDuration);
    if (token) sample->SetUnknown(MFSampleExtension_Token, token);
    return events_->QueueEventParamUnk(MEMediaSample, GUID_NULL, S_OK, sample.get());
}

// ---------------------------------------------------------------- MediaSource

MediaSource::MediaSource() {
    InterlockedIncrement(&g_objects);
}

MediaSource::~MediaSource() {
    if (stream_) {
        stream_->DetachSource();
        stream_->Shutdown();
        stream_->Release();
        stream_ = nullptr;
    }
    if (presentation_) presentation_->Release();
    if (attributes_) attributes_->Release();
    if (events_) {
        events_->Shutdown();
        events_->Release();
    }
    InterlockedDecrement(&g_objects);
}

STDMETHODIMP MediaSource::QueryInterface(REFIID riid, void** ppv) {
    if (!ppv) return E_POINTER;
    *ppv = nullptr;
    if (IsEqualIID(riid, IID_IUnknown) || IsEqualIID(riid, IID_IMFMediaEventGenerator) ||
        IsEqualIID(riid, IID_IMFMediaSource)) {
        *ppv = static_cast<IMFMediaSource*>(this);
    } else if (IsEqualIID(riid, IID_IMFMediaSourceEx)) {
        *ppv = static_cast<IMFMediaSourceEx*>(this);
    } else if (IsEqualIID(riid, IID_IMFGetService)) {
        *ppv = static_cast<IMFGetService*>(this);
    } else if (IsEqualIID(riid, __uuidof(IMeetBurnerKsControl))) {
        *ppv = static_cast<IMeetBurnerKsControl*>(this);
    } else {
        return E_NOINTERFACE;
    }
    AddRef();
    return S_OK;
}

STDMETHODIMP_(ULONG) MediaSource::AddRef() {
    return static_cast<ULONG>(InterlockedIncrement(&ref_));
}

STDMETHODIMP_(ULONG) MediaSource::Release() {
    const LONG now = InterlockedDecrement(&ref_);
    if (now == 0) delete this;
    return static_cast<ULONG>(now);
}

STDMETHODIMP MediaSource::GetEvent(DWORD flags, IMFMediaEvent** event) {
    ComPtr<IMFMediaEventQueue> queue;
    {
        std::lock_guard<std::mutex> guard(lock_);
        const HRESULT hr = CheckShutdown();
        if (FAILED(hr)) return hr;
        queue.reset(events_);
        if (!queue) return E_UNEXPECTED;
        queue->AddRef();
    }
    return queue->GetEvent(flags, event);
}

STDMETHODIMP MediaSource::BeginGetEvent(IMFAsyncCallback* callback, IUnknown* state) {
    std::lock_guard<std::mutex> guard(lock_);
    const HRESULT hr = CheckShutdown();
    if (FAILED(hr)) return hr;
    return events_->BeginGetEvent(callback, state);
}

STDMETHODIMP MediaSource::EndGetEvent(IMFAsyncResult* result, IMFMediaEvent** event) {
    std::lock_guard<std::mutex> guard(lock_);
    const HRESULT hr = CheckShutdown();
    if (FAILED(hr)) return hr;
    return events_->EndGetEvent(result, event);
}

STDMETHODIMP MediaSource::QueueEvent(MediaEventType type, REFGUID extended_type, HRESULT status,
                                     const PROPVARIANT* value) {
    std::lock_guard<std::mutex> guard(lock_);
    const HRESULT hr = CheckShutdown();
    if (FAILED(hr)) return hr;
    return events_->QueueEventParamVar(type, extended_type, status, value);
}

STDMETHODIMP MediaSource::CreatePresentationDescriptor(IMFPresentationDescriptor** descriptor) {
    if (!descriptor) return E_POINTER;
    *descriptor = nullptr;
    std::lock_guard<std::mutex> guard(lock_);
    const HRESULT hr = CheckShutdown();
    if (FAILED(hr)) return hr;
    return presentation_->Clone(descriptor);
}

STDMETHODIMP MediaSource::GetCharacteristics(DWORD* characteristics) {
    if (!characteristics) return E_POINTER;
    *characteristics = 0;
    std::lock_guard<std::mutex> guard(lock_);
    const HRESULT hr = CheckShutdown();
    if (FAILED(hr)) return hr;
    *characteristics = MFMEDIASOURCE_IS_LIVE;
    return S_OK;
}

STDMETHODIMP MediaSource::Start(IMFPresentationDescriptor* descriptor, const GUID* time_format,
                                const PROPVARIANT* start_position) {
    if (!descriptor || !start_position) return E_INVALIDARG;
    if (time_format && !IsEqualGUID(*time_format, GUID_NULL)) return MF_E_UNSUPPORTED_TIME_FORMAT;

    std::lock_guard<std::mutex> guard(lock_);
    HRESULT hr = CheckShutdown();
    if (FAILED(hr)) return hr;
    if (started_) return MF_E_INVALID_STATE_TRANSITION;

    DWORD count = 0;
    hr = descriptor->GetStreamDescriptorCount(&count);
    if (FAILED(hr)) return hr;
    if (count != 1) return E_INVALIDARG;
    BOOL selected = FALSE;
    ComPtr<IMFStreamDescriptor> stream_descriptor;
    hr = descriptor->GetStreamDescriptorByIndex(0, &selected, stream_descriptor.put());
    if (FAILED(hr)) return hr;
    if (!selected) return E_INVALIDARG;

    ComPtr<IMFMediaType> media_type;
    ComPtr<IMFMediaTypeHandler> handler;
    hr = stream_descriptor->GetMediaTypeHandler(handler.put());
    if (SUCCEEDED(hr)) hr = handler->GetCurrentMediaType(media_type.put());
    if (FAILED(hr)) return hr;

    hr = presentation_->SelectStream(0);
    if (FAILED(hr)) return hr;
    // MENewStream the first time, MEUpdatedStream when the stream is selected again.
    hr = events_->QueueEventParamUnk(announced_ ? MEUpdatedStream : MENewStream, GUID_NULL, S_OK,
                                     stream_);
    if (FAILED(hr)) return hr;
    announced_ = true;

    hr = stream_->Start(media_type.get());
    if (FAILED(hr)) return hr;

    PROPVARIANT started;
    PropVariantInit(&started);
    started.vt = VT_I8;
    started.hVal.QuadPart = MFGetSystemTime();
    hr = events_->QueueEventParamVar(MESourceStarted, GUID_NULL, S_OK, &started);
    PropVariantClear(&started);
    if (FAILED(hr)) return hr;
    started_ = true;
    return S_OK;
}

STDMETHODIMP MediaSource::Stop() {
    std::lock_guard<std::mutex> guard(lock_);
    HRESULT hr = CheckShutdown();
    if (FAILED(hr)) return hr;
    if (!started_) return MF_E_INVALID_STATE_TRANSITION;
    started_ = false;
    hr = stream_->Stop(true);
    if (FAILED(hr)) return hr;
    presentation_->DeselectStream(0);
    return events_->QueueEventParamVar(MESourceStopped, GUID_NULL, S_OK, nullptr);
}

STDMETHODIMP MediaSource::Pause() {
    // The frame server does not pause live camera sources (documented contract).
    return MF_E_INVALID_STATE_TRANSITION;
}

STDMETHODIMP MediaSource::Shutdown() {
    MediaStream* stream = nullptr;
    {
        std::lock_guard<std::mutex> guard(lock_);
        if (shutdown_) return S_OK;
        shutdown_ = true;
        started_ = false;
        if (events_) {
            events_->Shutdown();
        }
        if (presentation_) {
            presentation_->Release();
            presentation_ = nullptr;
        }
        if (attributes_) {
            attributes_->Release();
            attributes_ = nullptr;
        }
        stream = stream_;
        stream_ = nullptr;
    }
    // Outside the source lock: the stream takes its own lock in Shutdown/DetachSource.
    if (stream) {
        stream->DetachSource();
        stream->Shutdown();
        stream->Release();
    }
    return S_OK;
}

STDMETHODIMP MediaSource::GetSourceAttributes(IMFAttributes** attributes) {
    if (!attributes) return E_POINTER;
    *attributes = nullptr;
    std::lock_guard<std::mutex> guard(lock_);
    const HRESULT hr = CheckShutdown();
    if (FAILED(hr)) return hr;
    // Frame-server custom media sources must return the source's own attribute store, not a
    // copy: the pipeline writes into it.
    attributes_->AddRef();
    *attributes = attributes_;
    return S_OK;
}

STDMETHODIMP MediaSource::GetStreamAttributes(DWORD stream_id, IMFAttributes** attributes) {
    if (!attributes) return E_POINTER;
    *attributes = nullptr;
    std::lock_guard<std::mutex> guard(lock_);
    const HRESULT hr = CheckShutdown();
    if (FAILED(hr)) return hr;
    if (stream_id != 0 || !stream_) return MF_E_INVALIDSTREAMNUMBER;
    IMFAttributes* store = stream_->attributes();
    if (!store) return E_UNEXPECTED;
    store->AddRef();
    *attributes = store;
    return S_OK;
}

STDMETHODIMP MediaSource::SetD3DManager(IUnknown*) {
    // CPU memory buffers only; the frame server may ignore this.
    return E_NOTIMPL;
}

STDMETHODIMP MediaSource::GetService(REFGUID, REFIID, LPVOID* object) {
    if (!object) return E_POINTER;
    *object = nullptr;
    return MF_E_UNSUPPORTED_SERVICE;
}

STDMETHODIMP MediaSource::KsProperty(PKSPROPERTY, ULONG, LPVOID, ULONG, ULONG* bytes_returned) {
    if (bytes_returned) *bytes_returned = 0;
    return HRESULT_FROM_WIN32(ERROR_SET_NOT_FOUND);
}

STDMETHODIMP MediaSource::KsMethod(PKSMETHOD, ULONG, LPVOID, ULONG, ULONG* bytes_returned) {
    if (bytes_returned) *bytes_returned = 0;
    return HRESULT_FROM_WIN32(ERROR_SET_NOT_FOUND);
}

STDMETHODIMP MediaSource::KsEvent(PKSEVENT, ULONG, LPVOID, ULONG, ULONG* bytes_returned) {
    if (bytes_returned) *bytes_returned = 0;
    return HRESULT_FROM_WIN32(ERROR_SET_NOT_FOUND);
}

HRESULT MediaSource::Initialize() {
    HRESULT hr = MFCreateAttributes(&attributes_, 4);
    if (FAILED(hr)) return hr;
    hr = MFCreateEventQueue(&events_);
    if (FAILED(hr)) return hr;

    stream_ = new (std::nothrow) MediaStream(this, 0);
    if (!stream_) return E_OUTOFMEMORY;
    hr = stream_->Initialize();
    if (FAILED(hr)) return hr;

    ComPtr<IMFStreamDescriptor> stream_descriptor;
    hr = stream_->GetStreamDescriptor(stream_descriptor.put());
    if (FAILED(hr)) return hr;
    hr = MFCreatePresentationDescriptor(1, stream_descriptor.put(), &presentation_);
    if (FAILED(hr)) return hr;
    return presentation_->SelectStream(0);
}

// ---------------------------------------------------------------- COM server

class ClassFactory final : public IClassFactory {
public:
    ClassFactory() : ref_(1) {}

    STDMETHODIMP QueryInterface(REFIID riid, void** ppv) override {
        if (!ppv) return E_POINTER;
        if (IsEqualIID(riid, IID_IUnknown) || IsEqualIID(riid, IID_IClassFactory)) {
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
        auto* source = new (std::nothrow) MediaSource();
        if (!source) return E_OUTOFMEMORY;
        const HRESULT hr = source->Initialize();
        if (FAILED(hr)) {
            source->Release();
            return hr;
        }
        const HRESULT result = source->QueryInterface(riid, ppv);
        source->Release();
        return result;
    }
    STDMETHODIMP LockServer(BOOL) override { return S_OK; }

private:
    ~ClassFactory() = default;
    LONG ref_;
};

// MFCreateVirtualCamera needs Media Foundation; the app process may not have started it.
class MfSession {
public:
    MfSession() : hr_(MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET)) {}
    ~MfSession() {
        if (SUCCEEDED(hr_)) MFShutdown();
    }
    HRESULT hr() const { return hr_; }

private:
    HRESULT hr_;
};

} // namespace

// ---------------------------------------------------------------- exports

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
    if (!IsEqualCLSID(clsid, CLSID_MeetBurnerMfCamera)) return CLASS_E_CLASSNOTAVAILABLE;
    auto* factory = new (std::nothrow) ClassFactory();
    if (!factory) return E_OUTOFMEMORY;
    const HRESULT hr = factory->QueryInterface(riid, ppv);
    factory->Release();
    return hr;
}

extern "C" HRESULT __stdcall DllCanUnloadNow() {
    return g_objects == 0 ? S_OK : S_FALSE;
}

extern "C" HRESULT __stdcall DllRegisterServer() {
    return RegisterComServer(MB_MF_FRIENDLY_NAME);
}

extern "C" HRESULT __stdcall DllUnregisterServer() {
    UnregisterComServer();
    return S_OK;
}

extern "C" int __stdcall MB_MfSupported(void) {
    if (WindowsBuildNumber() < 22000) return 0;
    return CreateVirtualCamera() ? 1 : 0;
}

extern "C" HRESULT __stdcall MB_MfRegister(LPCWSTR friendlyName) {
    if (!MB_MfSupported()) return HRESULT_FROM_WIN32(ERROR_NOT_SUPPORTED);
    const wchar_t* name = (friendlyName && *friendlyName) ? friendlyName : MB_MF_FRIENDLY_NAME;

    MfSession session;
    if (FAILED(session.hr())) return session.hr();
    HRESULT hr = RegisterComServer(name);
    if (FAILED(hr)) return hr;

    MFCreateVirtualCameraFn create = CreateVirtualCamera();
    if (!create) return HRESULT_FROM_WIN32(ERROR_PROC_NOT_FOUND);
    IMFVirtualCamera* camera = nullptr;
    // System lifetime - per-user access: the camera survives restarts and belongs to the
    // Windows account that registered it (ADR-0031).
    hr = create(MFVirtualCameraType_SoftwareCameraSource, MFVirtualCameraLifetime_System,
                MFVirtualCameraAccess_CurrentUser, name, MB_MF_CLSID, nullptr, 0, &camera);
    if (FAILED(hr)) return hr;
    hr = camera->Start(nullptr);
    if (SUCCEEDED(hr)) WriteMarker(name);
    camera->Release();
    return hr;
}

extern "C" HRESULT __stdcall MB_MfUnregister(void) {
    if (!MB_MfSupported()) return HRESULT_FROM_WIN32(ERROR_NOT_SUPPORTED);

    MfSession session;
    if (FAILED(session.hr())) return session.hr();
    HRESULT hr = S_OK;
    MFCreateVirtualCameraFn create = CreateVirtualCamera();
    if (create) {
        const std::wstring name = RegisteredName();
        IMFVirtualCamera* camera = nullptr;
        hr = create(MFVirtualCameraType_SoftwareCameraSource, MFVirtualCameraLifetime_System,
                    MFVirtualCameraAccess_CurrentUser, name.c_str(), MB_MF_CLSID, nullptr, 0,
                    &camera);
        if (SUCCEEDED(hr)) {
            (void)camera->Stop();
            hr = camera->Remove();
            camera->Release();
        }
    } else {
        hr = HRESULT_FROM_WIN32(ERROR_PROC_NOT_FOUND);
    }
    UnregisterComServer();
    return hr;
}
