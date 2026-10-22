// MeetBurner Camera: a push-mode DirectShow source filter. It is deliberately
// self-contained (no DirectShow base classes, which are not part of the SDK) and
// only depends on the frame contract in ../common/meetburner_camera.h.
#include "vcam.h"

#include <olectl.h>
#include <uuids.h>
#include <vfwmsgs.h>
#include <wchar.h>

#include <cstring>
#include <new>

#include "../common/meetburner_camera.h"
#include "frames.h"

const CLSID CLSID_MeetBurnerCamera = {0xDBD3A890,
                                      0x1830,
                                      0x4222,
                                      {0xAF, 0xEE, 0xB1, 0xBA, 0x2C, 0x2D, 0xAB, 0x49}};
const wchar_t* const kMeetBurnerCameraName = L"MeetBurner Camera";

extern "C" unsigned int __stdcall MeetBurnerCameraVersion() {
    return MB_CAMERA_DRIVER_VERSION;
}

namespace {

const wchar_t* const kPinId = L"Output";
const REFERENCE_TIME kFrameDuration = 333333; // 30 fps in 100 ns units
const UINT32 kDefaultWidth = 1280;
const UINT32 kDefaultHeight = 720;

LONG g_objects = 0;

} // namespace

AM_MEDIA_TYPE MakeVideoType(UINT32 width, UINT32 height) {
    AM_MEDIA_TYPE mt;
    ZeroMemory(&mt, sizeof(mt));
    mt.majortype = MEDIATYPE_Video;
    mt.subtype = MEDIASUBTYPE_RGB32;
    mt.bFixedSizeSamples = TRUE;
    mt.bTemporalCompression = FALSE;
    mt.lSampleSize = (ULONG)mb_payload_size(width, height);
    mt.formattype = FORMAT_VideoInfo;

    auto* vih = static_cast<VIDEOINFOHEADER*>(CoTaskMemAlloc(sizeof(VIDEOINFOHEADER)));
    if (!vih) return mt;
    ZeroMemory(vih, sizeof(*vih));
    vih->AvgTimePerFrame = kFrameDuration;
    vih->bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
    vih->bmiHeader.biWidth = static_cast<LONG>(width);
    // Bottom-up rows: Media Foundation (Chrome, Teams, Meet) ignores a negative height and
    // would show a top-down buffer upside down (ADR-0019).
    vih->bmiHeader.biHeight = static_cast<LONG>(height);
    vih->bmiHeader.biPlanes = 1;
    vih->bmiHeader.biBitCount = 32;
    vih->bmiHeader.biCompression = BI_RGB;
    vih->bmiHeader.biSizeImage = static_cast<DWORD>(mb_payload_size(width, height));
    vih->rcSource.right = vih->rcTarget.right = static_cast<LONG>(width);
    vih->rcSource.bottom = vih->rcTarget.bottom = static_cast<LONG>(height);
    mt.pbFormat = reinterpret_cast<BYTE*>(vih);
    mt.cbFormat = sizeof(VIDEOINFOHEADER);
    return mt;
}

void FreeMediaType(AM_MEDIA_TYPE& mt) {
    if (mt.pbFormat) CoTaskMemFree(mt.pbFormat);
    if (mt.pUnk) mt.pUnk->Release();
    ZeroMemory(&mt, sizeof(mt));
}

bool CopyMediaType(AM_MEDIA_TYPE& dst, const AM_MEDIA_TYPE& src) {
    dst = src;
    dst.pbFormat = nullptr;
    dst.cbFormat = 0;
    if (src.cbFormat) {
        dst.pbFormat = static_cast<BYTE*>(CoTaskMemAlloc(src.cbFormat));
        if (!dst.pbFormat) return false;
        memcpy(dst.pbFormat, src.pbFormat, src.cbFormat);
        dst.cbFormat = src.cbFormat;
    }
    if (src.pUnk) src.pUnk->AddRef();
    return true;
}

bool VideoFormat(const AM_MEDIA_TYPE* mt, UINT32* width, UINT32* height) {
    if (!mt || !mt->pbFormat) return false;
    if (!IsEqualGUID(mt->majortype, MEDIATYPE_Video)) return false;
    if (!IsEqualGUID(mt->subtype, MEDIASUBTYPE_RGB32)) return false;
    if (!IsEqualGUID(mt->formattype, FORMAT_VideoInfo)) return false;
    if (mt->cbFormat < sizeof(VIDEOINFOHEADER)) return false;
    const auto* vih = reinterpret_cast<const VIDEOINFOHEADER*>(mt->pbFormat);
    if (vih->bmiHeader.biBitCount != 32 || vih->bmiHeader.biCompression != BI_RGB) return false;
    const LONG w = vih->bmiHeader.biWidth;
    const LONG h = vih->bmiHeader.biHeight;
    if (w < 16 || h == 0) return false;
    if (width) *width = static_cast<UINT32>(w);
    if (height) *height = static_cast<UINT32>(h < 0 ? -h : h);
    return true;
}

// A frame older than this means the app stopped publishing, even if the mapping still holds
// its last pixels (a killed process keeps the section alive through the readers).
const uint64_t kStaleMs = 2000;

uint64_t UnixMsNow() {
    FILETIME file_time;
    GetSystemTimeAsFileTime(&file_time);
    ULARGE_INTEGER ticks;
    ticks.LowPart = file_time.dwLowDateTime;
    ticks.HighPart = file_time.dwHighDateTime;
    return (ticks.QuadPart - 116444736000000000ULL) / 10000ULL; // 1601 -> 1970, 100 ns -> ms
}

bool IsStale(const meetburner_frame_header& header) {
    return header.timestamp_ms == 0 || UnixMsNow() > header.timestamp_ms + kStaleMs;
}

// Size the producer is publishing right now, or the default before it starts.
void ProducerSize(UINT32* width, UINT32* height) {
    *width = kDefaultWidth;
    *height = kDefaultHeight;
    mb_reader reader;
    if (!mb_reader_open(&reader, MB_CAMERA_MAPPING_NAME, MB_CAMERA_EVENT_NAME)) return;
    meetburner_frame_header header = {};
    if (mb_peek_header(&reader, &header) && header.width > 0 && header.height > 0 &&
        header.width <= MB_CAMERA_MAX_WIDTH && header.height <= MB_CAMERA_MAX_HEIGHT) {
        *width = header.width;
        *height = header.height;
    }
    mb_reader_close(&reader);
}

// Copies a top-down BGRA frame into a bottom-up destination, preserving the aspect ratio.
// Row 0 of the destination is the bottom of the picture, so a producer size that differs
// from the negotiated media type still shows the counter upright inside the frame.
void BlitFit(const BYTE* src, UINT32 src_w, UINT32 src_h, BYTE* dst, UINT32 dst_w,
             UINT32 dst_h) {
    if (!src || !dst || src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0) return;
    const DWORD background = 0xFF0F1115u; // BGRA of RGB(0f, 11, 15)
    auto* pixels = reinterpret_cast<DWORD*>(dst);
    const size_t count = static_cast<size_t>(dst_w) * dst_h;
    for (size_t i = 0; i < count; ++i) pixels[i] = background;

    UINT32 out_w;
    UINT32 out_h;
    const UINT64 src_cross = static_cast<UINT64>(src_w) * dst_h;
    const UINT64 dst_cross = static_cast<UINT64>(dst_w) * src_h;
    if (src_cross >= dst_cross) {
        out_w = dst_w;
        out_h = static_cast<UINT32>(static_cast<UINT64>(dst_w) * src_h / src_w);
    } else {
        out_h = dst_h;
        out_w = static_cast<UINT32>(static_cast<UINT64>(dst_h) * src_w / src_h);
    }
    if (out_w == 0) out_w = 1;
    if (out_h == 0) out_h = 1;
    if (out_w > dst_w) out_w = dst_w;
    if (out_h > dst_h) out_h = dst_h;
    const UINT32 off_x = (dst_w - out_w) / 2;
    const UINT32 off_y = (dst_h - out_h) / 2;
    const bool same_size = out_w == src_w && out_h == src_h && out_w == dst_w && out_h == dst_h;

    for (UINT32 vy = 0; vy < out_h; ++vy) {
        const UINT32 sy =
            same_size ? vy : static_cast<UINT32>(static_cast<UINT64>(vy) * src_h / out_h);
        const BYTE* src_row = src + static_cast<size_t>(sy) * src_w * 4;
        BYTE* dst_row = dst + static_cast<size_t>(dst_h - 1 - (off_y + vy)) * dst_w * 4 +
                        static_cast<size_t>(off_x) * 4;
        if (same_size) {
            memcpy(dst_row, src_row, static_cast<size_t>(src_w) * 4);
            continue;
        }
        for (UINT32 vx = 0; vx < out_w; ++vx) {
            const UINT32 sx =
                static_cast<UINT32>(static_cast<UINT64>(vx) * src_w / out_w);
            memcpy(dst_row + static_cast<size_t>(vx) * 4, src_row + static_cast<size_t>(sx) * 4,
                   4);
        }
    }
}

// Static frame shown while the app is not publishing (ADR-0002).
std::vector<BYTE> RenderNotRunning(UINT32 width, UINT32 height) {
    std::vector<BYTE> buffer(mb_payload_size(width, height), 0);
    BITMAPINFO bmi;
    ZeroMemory(&bmi, sizeof(bmi));
    bmi.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
    bmi.bmiHeader.biWidth = static_cast<LONG>(width);
    bmi.bmiHeader.biHeight = -static_cast<LONG>(height);
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;

    void* bits = nullptr;
    HDC screen = GetDC(nullptr);
    HDC dc = CreateCompatibleDC(screen);
    HBITMAP bitmap = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &bits, nullptr, 0);
    if (bitmap && bits) {
        HGDIOBJ old_bitmap = SelectObject(dc, bitmap);
        RECT rect = {0, 0, static_cast<LONG>(width), static_cast<LONG>(height)};
        HBRUSH brush = CreateSolidBrush(RGB(0x0f, 0x11, 0x15));
        FillRect(dc, &rect, brush);
        DeleteObject(brush);
        SetBkMode(dc, TRANSPARENT);
        SetTextColor(dc, RGB(0x8b, 0x93, 0xa7));
        const int font_height = -static_cast<int>(height / 10);
        HFONT font = CreateFontW(font_height, 0, 0, 0, FW_SEMIBOLD, FALSE, FALSE, FALSE,
                                 DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS,
                                 CLEARTYPE_QUALITY, DEFAULT_PITCH | FF_SWISS, L"Segoe UI");
        HGDIOBJ old_font = SelectObject(dc, font);
        const wchar_t* message = L"MeetBurner\n\nCamera not running";
        DrawTextW(dc, message, -1, &rect, DT_CENTER | DT_VCENTER | DT_WORDBREAK | DT_NOPREFIX);
        SelectObject(dc, old_font);
        DeleteObject(font);
        GdiFlush();
        // The DIB is top-down for GDI; flip it so the buffer is bottom-up like the video.
        const auto* top_down = static_cast<const BYTE*>(bits);
        const size_t stride = static_cast<size_t>(width) * 4;
        for (UINT32 y = 0; y < height; ++y) {
            memcpy(buffer.data() + static_cast<size_t>(y) * stride,
                   top_down + static_cast<size_t>(height - 1 - y) * stride, stride);
        }
        SelectObject(dc, old_bitmap);
    }
    if (bitmap) DeleteObject(bitmap);
    DeleteDC(dc);
    ReleaseDC(nullptr, screen);
    return buffer;
}

namespace {

class MediaTypeEnumerator final : public IEnumMediaTypes {
public:
    MediaTypeEnumerator(UINT32 width, UINT32 height) : ref_(1), position_(0) {
        media_type_ = MakeVideoType(width, height);
        object_created();
    }

    STDMETHODIMP QueryInterface(REFIID riid, void** ppv) override {
        if (!ppv) return E_POINTER;
        if (riid == IID_IUnknown || riid == IID_IEnumMediaTypes) {
            *ppv = static_cast<IEnumMediaTypes*>(this);
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

    STDMETHODIMP Next(ULONG count, AM_MEDIA_TYPE** types, ULONG* fetched) override {
        if (!types) return E_POINTER;
        ULONG done = 0;
        while (done < count && position_ == 0) {
            auto* copy = static_cast<AM_MEDIA_TYPE*>(CoTaskMemAlloc(sizeof(AM_MEDIA_TYPE)));
            if (!copy) break;
            if (!CopyMediaType(*copy, media_type_)) {
                CoTaskMemFree(copy);
                break;
            }
            types[done++] = copy;
            position_ = 1;
        }
        if (fetched) *fetched = done;
        return done == count ? S_OK : S_FALSE;
    }
    STDMETHODIMP Skip(ULONG count) override {
        if (position_ + count <= 1) {
            position_ = 1;
            return S_OK;
        }
        position_ = 1;
        return S_FALSE;
    }
    STDMETHODIMP Reset() override {
        position_ = 0;
        return S_OK;
    }
    STDMETHODIMP Clone(IEnumMediaTypes** out) override {
        if (!out) return E_POINTER;
        auto* clone = new (std::nothrow) MediaTypeEnumerator(kDefaultWidth, kDefaultHeight);
        if (!clone) return E_OUTOFMEMORY;
        clone->position_ = position_;
        FreeMediaType(clone->media_type_);
        CopyMediaType(clone->media_type_, media_type_);
        *out = clone;
        return S_OK;
    }

private:
    ~MediaTypeEnumerator() {
        FreeMediaType(media_type_);
        object_destroyed();
    }

    LONG ref_;
    ULONG position_;
    AM_MEDIA_TYPE media_type_;
};

class PinEnumerator final : public IEnumPins {
public:
    explicit PinEnumerator(CameraPin* pin) : ref_(1), position_(0), pin_(pin) {
        pin_->AddRef();
        object_created();
    }

    STDMETHODIMP QueryInterface(REFIID riid, void** ppv) override {
        if (!ppv) return E_POINTER;
        if (riid == IID_IUnknown || riid == IID_IEnumPins) {
            *ppv = static_cast<IEnumPins*>(this);
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

    STDMETHODIMP Next(ULONG count, IPin** pins, ULONG* fetched) override {
        if (!pins) return E_POINTER;
        ULONG done = 0;
        if (position_ == 0 && count > 0) {
            pin_->AddRef();
            pins[0] = pin_;
            position_ = 1;
            done = 1;
        }
        if (fetched) *fetched = done;
        return done == count ? S_OK : S_FALSE;
    }
    STDMETHODIMP Skip(ULONG count) override {
        position_ = 1;
        return count <= 1 ? S_OK : S_FALSE;
    }
    STDMETHODIMP Reset() override {
        position_ = 0;
        return S_OK;
    }
    STDMETHODIMP Clone(IEnumPins** out) override {
        if (!out) return E_POINTER;
        auto* clone = new (std::nothrow) PinEnumerator(pin_);
        if (!clone) return E_OUTOFMEMORY;
        clone->position_ = position_;
        *out = clone;
        return S_OK;
    }

private:
    ~PinEnumerator() {
        pin_->Release();
        object_destroyed();
    }

    LONG ref_;
    ULONG position_;
    CameraPin* pin_;
};

} // namespace

void object_created() { InterlockedIncrement(&g_objects); }
void object_destroyed() { InterlockedDecrement(&g_objects); }
LONG live_object_count() { return g_objects; }

CameraPin::CameraPin(CameraFilter* filter)
    : ref_(1),
      filter_(filter),
      connected_(nullptr),
      input_(nullptr),
      allocator_(nullptr),
      has_media_type_(false),
      width_(kDefaultWidth),
      height_(kDefaultHeight),
      thread_(nullptr),
      stop_event_(nullptr) {
    ZeroMemory(&media_type_, sizeof(media_type_));
    ProducerSize(&width_, &height_);
    BuildNotRunningFrame();
    object_created();
}

CameraPin::~CameraPin() {
    StopStream();
    Disconnect();
    object_destroyed();
}

STDMETHODIMP CameraPin::QueryInterface(REFIID riid, void** ppv) {
    if (!ppv) return E_POINTER;
    if (riid == IID_IUnknown || riid == IID_IPin) {
        *ppv = static_cast<IPin*>(this);
        AddRef();
        return S_OK;
    }
    if (riid == IID_IKsPropertySet) {
        *ppv = static_cast<IKsPropertySet*>(this);
        AddRef();
        return S_OK;
    }
    if (riid == IID_IAMStreamConfig) {
        *ppv = static_cast<IAMStreamConfig*>(this);
        AddRef();
        return S_OK;
    }
    *ppv = nullptr;
    return E_NOINTERFACE;
}

STDMETHODIMP_(ULONG) CameraPin::AddRef() {
    return static_cast<ULONG>(InterlockedIncrement(&ref_));
}

STDMETHODIMP_(ULONG) CameraPin::Release() {
    const LONG now = InterlockedDecrement(&ref_);
    if (now == 0) delete this;
    return static_cast<ULONG>(now);
}

STDMETHODIMP CameraPin::Connect(IPin* receive_pin, const AM_MEDIA_TYPE* pmt) {
    if (!receive_pin) return E_POINTER;
    if (connected_) return VFW_E_ALREADY_CONNECTED;
    if (pmt && QueryAccept(pmt) != S_OK) return VFW_E_TYPE_NOT_ACCEPTED;

    AM_MEDIA_TYPE proposed;
    bool owned = false;
    if (pmt) {
        proposed = *pmt;
    } else {
        ProducerSize(&width_, &height_);
        proposed = MakeVideoType(width_, height_);
        owned = true;
    }
    HRESULT hr = receive_pin->ReceiveConnection(this, &proposed);
    if (SUCCEEDED(hr)) hr = CompleteConnection(receive_pin, &proposed);
    if (owned) FreeMediaType(proposed);
    return hr;
}

HRESULT CameraPin::CompleteConnection(IPin* receive_pin, const AM_MEDIA_TYPE* pmt) {
    UINT32 width = width_;
    UINT32 height = height_;
    if (pmt && VideoFormat(pmt, &width, &height)) {
        width_ = width;
        height_ = height;
    }
    if (pmt) {
        FreeMediaType(media_type_);
        if (!CopyMediaType(media_type_, *pmt)) return E_OUTOFMEMORY;
        has_media_type_ = true;
    }
    receive_pin->AddRef();
    connected_ = receive_pin;

    HRESULT hr = receive_pin->QueryInterface(IID_PPV_ARGS(&input_));
    if (FAILED(hr)) {
        Disconnect();
        return hr;
    }
    hr = input_->GetAllocator(&allocator_);    if (hr == VFW_E_NO_ALLOCATOR) {
        // Some consumers (ffmpeg's grabber, for one) do not provide an allocator:
        // the source creates its own and tells the input pin about it.
        hr = CoCreateInstance(CLSID_MemoryAllocator, nullptr, CLSCTX_INPROC_SERVER,
                              IID_PPV_ARGS(&allocator_));
        if (SUCCEEDED(hr)) hr = input_->NotifyAllocator(allocator_, FALSE);
    }    if (FAILED(hr)) {
        Disconnect();
        return hr;
    }
    ALLOCATOR_PROPERTIES requested = {};
    requested.cBuffers = 3;
    requested.cbBuffer = static_cast<LONG>(mb_payload_size(width_, height_));
    requested.cbAlign = 1;
    requested.cbPrefix = 0;
    ALLOCATOR_PROPERTIES actual = {};
    hr = allocator_->SetProperties(&requested, &actual);
    if (FAILED(hr)) {
        Disconnect();
        return hr;
    }
    hr = allocator_->Commit();
    if (FAILED(hr)) {
        Disconnect();
        return hr;
    }
    BuildNotRunningFrame();
    return S_OK;
}

STDMETHODIMP CameraPin::ReceiveConnection(IPin*, const AM_MEDIA_TYPE*) {
    // This pin is an output: the graph never calls ReceiveConnection on it.
    return E_UNEXPECTED;
}

STDMETHODIMP CameraPin::Disconnect() {
    StopStream();
    if (allocator_) {
        allocator_->Decommit();
        allocator_->Release();
        allocator_ = nullptr;
    }
    if (input_) {
        input_->Release();
        input_ = nullptr;
    }
    if (connected_) {
        connected_->Release();
        connected_ = nullptr;
    }
    if (has_media_type_) {
        FreeMediaType(media_type_);
        has_media_type_ = false;
    }
    return S_OK;
}

STDMETHODIMP CameraPin::ConnectedTo(IPin** pin) {
    if (!pin) return E_POINTER;
    if (!connected_) {
        *pin = nullptr;
        return VFW_E_NOT_CONNECTED;
    }
    connected_->AddRef();
    *pin = connected_;
    return S_OK;
}

STDMETHODIMP CameraPin::ConnectionMediaType(AM_MEDIA_TYPE* pmt) {
    if (!pmt) return E_POINTER;
    if (!has_media_type_) {
        ZeroMemory(pmt, sizeof(*pmt));
        return VFW_E_NOT_CONNECTED;
    }
    if (!CopyMediaType(*pmt, media_type_)) return E_OUTOFMEMORY;
    return S_OK;
}

STDMETHODIMP CameraPin::QueryPinInfo(PIN_INFO* info) {
    if (!info) return E_POINTER;
    info->pFilter = static_cast<IBaseFilter*>(filter_);
    if (info->pFilter) info->pFilter->AddRef();
    info->dir = PINDIR_OUTPUT;
    wcsncpy_s(info->achName, MAX_PIN_NAME, kPinId, _TRUNCATE);
    return S_OK;
}

STDMETHODIMP CameraPin::QueryDirection(PIN_DIRECTION* direction) {
    if (!direction) return E_POINTER;
    *direction = PINDIR_OUTPUT;
    return S_OK;
}

STDMETHODIMP CameraPin::QueryId(LPWSTR* id) {
    if (!id) return E_POINTER;
    const size_t bytes = (wcslen(kPinId) + 1) * sizeof(wchar_t);
    *id = static_cast<LPWSTR>(CoTaskMemAlloc(bytes));
    if (!*id) return E_OUTOFMEMORY;
    memcpy(*id, kPinId, bytes);
    return S_OK;
}

STDMETHODIMP CameraPin::QueryAccept(const AM_MEDIA_TYPE* pmt) {
    UINT32 width = 0;
    UINT32 height = 0;
    if (!VideoFormat(pmt, &width, &height)) return S_FALSE;
    if (width > MB_CAMERA_MAX_WIDTH || height > MB_CAMERA_MAX_HEIGHT) return S_FALSE;
    return S_OK;
}

STDMETHODIMP CameraPin::EnumMediaTypes(IEnumMediaTypes** out) {
    if (!out) return E_POINTER;
    UINT32 width = 0;
    UINT32 height = 0;
    ProducerSize(&width, &height);
    auto* types = new (std::nothrow) MediaTypeEnumerator(width, height);
    if (!types) return E_OUTOFMEMORY;
    *out = types;
    return S_OK;
}

STDMETHODIMP CameraPin::QueryInternalConnections(IPin**, ULONG*) { return E_NOTIMPL; }

STDMETHODIMP CameraPin::EndOfStream() { return E_UNEXPECTED; }

STDMETHODIMP CameraPin::BeginFlush() { return E_UNEXPECTED; }

STDMETHODIMP CameraPin::EndFlush() { return E_UNEXPECTED; }

STDMETHODIMP CameraPin::NewSegment(REFERENCE_TIME, REFERENCE_TIME, double) { return E_UNEXPECTED; }

STDMETHODIMP CameraPin::Set(REFGUID, DWORD, LPVOID, DWORD, LPVOID, DWORD) {
    return E_NOTIMPL;
}

STDMETHODIMP CameraPin::Get(REFGUID guidPropSet, DWORD dwPropID, LPVOID, DWORD, LPVOID pPropData,
                            DWORD cbPropData, DWORD* pcbReturned) {
    if (IsEqualGUID(guidPropSet, AMPROPSETID_Pin) && dwPropID == AMPROPERTY_PIN_CATEGORY) {
        if (pcbReturned) *pcbReturned = sizeof(GUID);
        if (!pPropData) return S_OK;
        if (cbPropData < sizeof(GUID)) return E_FAIL;
        memcpy(pPropData, &PIN_CATEGORY_CAPTURE, sizeof(GUID));
        return S_OK;
    }
    if (pcbReturned) *pcbReturned = 0;
    return E_PROP_ID_UNSUPPORTED;
}

STDMETHODIMP CameraPin::QuerySupported(REFGUID guidPropSet, DWORD dwPropID, DWORD* pTypeSupport) {
    if (IsEqualGUID(guidPropSet, AMPROPSETID_Pin) && dwPropID == AMPROPERTY_PIN_CATEGORY) {
        if (pTypeSupport) *pTypeSupport = 0x1; // KSPROPERTY_SUPPORT_GET
        return S_OK;
    }
    if (pTypeSupport) *pTypeSupport = 0;
    return E_PROP_SET_UNSUPPORTED;
}

STDMETHODIMP CameraPin::SetFormat(AM_MEDIA_TYPE* pmt) {
    UINT32 width = 0;
    UINT32 height = 0;
    if (!VideoFormat(pmt, &width, &height)) return E_INVALIDARG;
    if (width > MB_CAMERA_MAX_WIDTH || height > MB_CAMERA_MAX_HEIGHT) return E_INVALIDARG;
    if (connected_ && (width != width_ || height != height_)) return VFW_E_INVALIDMEDIATYPE;
    width_ = width;
    height_ = height;
    FreeMediaType(media_type_);
    if (!CopyMediaType(media_type_, *pmt)) return E_OUTOFMEMORY;
    has_media_type_ = true;
    BuildNotRunningFrame();
    return S_OK;
}

STDMETHODIMP CameraPin::GetFormat(AM_MEDIA_TYPE** pmt) {
    if (!pmt) return E_POINTER;
    auto* type = static_cast<AM_MEDIA_TYPE*>(CoTaskMemAlloc(sizeof(AM_MEDIA_TYPE)));
    if (!type) return E_OUTOFMEMORY;
    if (has_media_type_) {
        if (!CopyMediaType(*type, media_type_)) {
            CoTaskMemFree(type);
            return E_OUTOFMEMORY;
        }
    } else {
        *type = MakeVideoType(width_, height_);
    }
    *pmt = type;
    return S_OK;
}

STDMETHODIMP CameraPin::GetNumberOfCapabilities(int* count, int* size) {
    if (!count || !size) return E_POINTER;
    *count = 1;
    *size = sizeof(VIDEO_STREAM_CONFIG_CAPS);
    return S_OK;
}

STDMETHODIMP CameraPin::GetStreamCaps(int index, AM_MEDIA_TYPE** pmt, BYTE* caps) {
    if (!pmt || !caps) return E_POINTER;
    if (index != 0) return E_INVALIDARG;
    auto* type = static_cast<AM_MEDIA_TYPE*>(CoTaskMemAlloc(sizeof(AM_MEDIA_TYPE)));
    if (!type) return E_OUTOFMEMORY;
    if (has_media_type_) {
        if (!CopyMediaType(*type, media_type_)) {
            CoTaskMemFree(type);
            return E_OUTOFMEMORY;
        }
    } else {
        *type = MakeVideoType(width_, height_);
    }
    auto* config = reinterpret_cast<VIDEO_STREAM_CONFIG_CAPS*>(caps);
    ZeroMemory(config, sizeof(*config));
    config->guid = FORMAT_VideoInfo;
    config->InputSize.cx = static_cast<LONG>(width_);
    config->InputSize.cy = static_cast<LONG>(height_);
    config->MinCroppingSize = config->MaxCroppingSize = config->InputSize;
    config->CropGranularityX = config->CropGranularityY = 1;
    config->MinOutputSize = config->MaxOutputSize = config->InputSize;
    config->OutputGranularityX = config->OutputGranularityY = 1;
    config->StretchTapsX = config->StretchTapsY = 1;
    config->ShrinkTapsX = config->ShrinkTapsY = 1;
    config->MinFrameInterval = config->MaxFrameInterval = kFrameDuration;
    config->MinBitsPerSecond = config->MaxBitsPerSecond =
        static_cast<LONG>(mb_payload_size(width_, height_) * 30);
    *pmt = type;
    return S_OK;
}

void CameraPin::BuildNotRunningFrame() { not_running_ = RenderNotRunning(width_, height_); }

void CameraPin::StartStream() {
    if (thread_ || !allocator_ || !input_) return;
    stop_event_ = CreateEventW(nullptr, TRUE, FALSE, nullptr);
    if (!stop_event_) return;
    if (scratch_.empty()) {
        scratch_.resize(mb_payload_size(MB_CAMERA_MAX_WIDTH, MB_CAMERA_MAX_HEIGHT));
    }
    thread_ = CreateThread(nullptr, 0, ThreadEntry, this, 0, nullptr);
    if (!thread_) {
        CloseHandle(stop_event_);
        stop_event_ = nullptr;
    }
}

void CameraPin::StopStream() {
    if (!thread_) return;
    SetEvent(stop_event_);
    WaitForSingleObject(thread_, 5000);
    CloseHandle(thread_);
    thread_ = nullptr;
    CloseHandle(stop_event_);
    stop_event_ = nullptr;
}

DWORD WINAPI CameraPin::ThreadEntry(LPVOID param) {
    static_cast<CameraPin*>(param)->DeliverLoop();
    return 0;
}

void CameraPin::DeliverLoop() {
    mb_reader reader = {};
    bool reader_open = false;
    LARGE_INTEGER frequency;
    QueryPerformanceFrequency(&frequency);
    LARGE_INTEGER started;
    QueryPerformanceCounter(&started);
    const LONGLONG period_ns = 1000000000LL / 30;
    LONGLONG frame = 0;
    // Consecutive ticks without a readable frame; ~2 s makes the placeholder honest again.
    int stale_ticks = 0;
    const int kStaleLimit = 60;

    while (WaitForSingleObject(stop_event_, 0) != WAIT_OBJECT_0) {
        if (!reader_open) {
            reader_open =
                mb_reader_open(&reader, MB_CAMERA_MAPPING_NAME, MB_CAMERA_EVENT_NAME);
        }
        LARGE_INTEGER now;
        QueryPerformanceCounter(&now);
        const LONGLONG elapsed_ns =
            ((now.QuadPart - started.QuadPart) * 1000000000LL) / frequency.QuadPart;
        if (elapsed_ns < frame * period_ns) {
            Sleep(1);
            continue;
        }

        IMediaSample* sample = nullptr;
        if (allocator_ && input_ && SUCCEEDED(allocator_->GetBuffer(&sample, nullptr, nullptr, 0))) {
            BYTE* dst = nullptr;
            if (SUCCEEDED(sample->GetPointer(&dst))) {
                const UINT32 dst_size = static_cast<UINT32>(mb_payload_size(width_, height_));
                meetburner_frame_header header = {};
                const bool have = reader_open &&
                                  mb_read_frame(&reader, scratch_.data(), scratch_.size(), &header);
                const bool live = have && header.state == MB_CAMERA_STATE_LIVE && !IsStale(header);
                if (live) {
                    BlitFit(scratch_.data(), header.width, header.height, dst, width_, height_);
                    last_.assign(dst, dst + dst_size);
                    stale_ticks = 0;
                } else {
                    if (!have && reader_open) {
                        // No readable frame: reopen the mapping on the next tick.
                        mb_reader_close(&reader);
                        reader_open = false;
                    }
                    if (have) {
                        // A readable but idle or stale frame: the app is not publishing.
                        last_.clear();
                    } else if (++stale_ticks > kStaleLimit) {
                        // Unreadable for ~2 s: stop repeating the old picture (ADR-0019).
                        last_.clear();
                    }
                    const std::vector<BYTE>& shown = last_.size() == dst_size ? last_ : not_running_;
                    if (shown.size() >= dst_size) {
                        memcpy(dst, shown.data(), dst_size);
                    } else {
                        memset(dst, 0, dst_size);
                    }
                }
                sample->SetActualDataLength(static_cast<LONG>(dst_size));
                REFERENCE_TIME start_time = frame * kFrameDuration;
                REFERENCE_TIME end_time = start_time + kFrameDuration;
                sample->SetTime(&start_time, &end_time);
                sample->SetSyncPoint(TRUE);
            }
            const HRESULT hr = input_->Receive(sample);
            sample->Release();
            if (FAILED(hr)) break;
        }
        ++frame;
    }
    if (reader_open) mb_reader_close(&reader);
}

CameraFilter::CameraFilter()
    : ref_(1), pin_(new (std::nothrow) CameraPin(this)), graph_(nullptr), state_(State_Stopped) {
    wcsncpy_s(name_, MAX_FILTER_NAME, kMeetBurnerCameraName, _TRUNCATE);
    object_created();
}

CameraFilter::~CameraFilter() {
    if (pin_) pin_->Release();
    if (graph_) graph_->Release();
    object_destroyed();
}

STDMETHODIMP CameraFilter::QueryInterface(REFIID riid, void** ppv) {
    if (!ppv) return E_POINTER;
    if (riid == IID_IUnknown || riid == IID_IBaseFilter) {
        *ppv = static_cast<IBaseFilter*>(this);
    } else if (riid == IID_IPersist) {
        *ppv = static_cast<IPersist*>(this);
    } else if (riid == IID_IMediaFilter) {
        *ppv = static_cast<IMediaFilter*>(this);
    } else {
        *ppv = nullptr;
        return E_NOINTERFACE;
    }
    AddRef();
    return S_OK;
}

STDMETHODIMP_(ULONG) CameraFilter::AddRef() {
    return static_cast<ULONG>(InterlockedIncrement(&ref_));
}

STDMETHODIMP_(ULONG) CameraFilter::Release() {
    const LONG now = InterlockedDecrement(&ref_);
    if (now == 0) delete this;
    return static_cast<ULONG>(now);
}

STDMETHODIMP CameraFilter::GetClassID(CLSID* clsid) {
    if (!clsid) return E_POINTER;
    *clsid = CLSID_MeetBurnerCamera;
    return S_OK;
}

STDMETHODIMP CameraFilter::GetState(DWORD, FILTER_STATE* state) {
    if (!state) return E_POINTER;
    *state = state_;
    return S_OK;
}

STDMETHODIMP CameraFilter::SetSyncSource(IReferenceClock*) { return S_OK; }

STDMETHODIMP CameraFilter::GetSyncSource(IReferenceClock** clock) {
    if (!clock) return E_POINTER;
    *clock = nullptr;
    return S_OK;
}

STDMETHODIMP CameraFilter::Stop() {
    if (pin_) pin_->StopStream();
    state_ = State_Stopped;
    return S_OK;
}

STDMETHODIMP CameraFilter::Pause() {
    if (pin_) pin_->StartStream();
    state_ = State_Paused;
    return S_OK;
}

STDMETHODIMP CameraFilter::Run(REFERENCE_TIME) {
    if (pin_) pin_->StartStream();
    state_ = State_Running;
    return S_OK;
}

STDMETHODIMP CameraFilter::EnumPins(IEnumPins** out) {
    if (!out) return E_POINTER;
    if (!pin_) return E_OUTOFMEMORY;
    auto* pins = new (std::nothrow) PinEnumerator(pin_);
    if (!pins) return E_OUTOFMEMORY;
    *out = pins;
    return S_OK;
}

STDMETHODIMP CameraFilter::FindPin(LPCWSTR id, IPin** pin) {
    if (!pin) return E_POINTER;
    if (!id || _wcsicmp(id, kPinId) != 0) {
        *pin = nullptr;
        return VFW_E_NOT_FOUND;
    }
    pin_->AddRef();
    *pin = pin_;
    return S_OK;
}

STDMETHODIMP CameraFilter::QueryFilterInfo(FILTER_INFO* info) {
    if (!info) return E_POINTER;
    wcsncpy_s(info->achName, MAX_FILTER_NAME, name_, _TRUNCATE);
    info->pGraph = graph_;
    if (graph_) graph_->AddRef();
    return S_OK;
}

STDMETHODIMP CameraFilter::JoinFilterGraph(IFilterGraph* graph, LPCWSTR name) {
    if (graph_) {
        graph_->Release();
        graph_ = nullptr;
    }
    if (graph) {
        graph->AddRef();
        graph_ = graph;
    }
    if (name) wcsncpy_s(name_, MAX_FILTER_NAME, name, _TRUNCATE);
    return S_OK;
}

STDMETHODIMP CameraFilter::QueryVendorInfo(LPWSTR* info) {
    if (!info) return E_POINTER;
    *info = nullptr;
    return E_NOTIMPL;
}
