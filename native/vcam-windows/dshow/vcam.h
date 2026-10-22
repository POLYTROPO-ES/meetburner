#pragma once
// Minimal DirectShow source filter ("MeetBurner Camera") that publishes the frames
// the Rust app writes into shared memory (ADR-0013, ADR-0016).
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <dshow.h>
#include <strmif.h>

#include <vector>

extern const CLSID CLSID_MeetBurnerCamera;
extern const wchar_t* const kMeetBurnerCameraName;

// Keeps DllCanUnloadNow honest while any filter object is alive.
void object_created();
void object_destroyed();
LONG live_object_count();

class CameraFilter;

class CameraPin final : public IPin, public IKsPropertySet, public IAMStreamConfig {
public:
    explicit CameraPin(CameraFilter* filter);

    // IUnknown
    STDMETHODIMP QueryInterface(REFIID riid, void** ppv) override;
    STDMETHODIMP_(ULONG) AddRef() override;
    STDMETHODIMP_(ULONG) Release() override;

    // IPin
    STDMETHODIMP Connect(IPin* pReceivePin, const AM_MEDIA_TYPE* pmt) override;
    STDMETHODIMP ReceiveConnection(IPin* pConnector, const AM_MEDIA_TYPE* pmt) override;
    STDMETHODIMP Disconnect() override;
    STDMETHODIMP ConnectedTo(IPin** pPin) override;
    STDMETHODIMP ConnectionMediaType(AM_MEDIA_TYPE* pmt) override;
    STDMETHODIMP QueryPinInfo(PIN_INFO* pInfo) override;
    STDMETHODIMP QueryDirection(PIN_DIRECTION* pPinDir) override;
    STDMETHODIMP QueryId(LPWSTR* Id) override;
    STDMETHODIMP QueryAccept(const AM_MEDIA_TYPE* pmt) override;
    STDMETHODIMP EnumMediaTypes(IEnumMediaTypes** ppEnum) override;
    STDMETHODIMP QueryInternalConnections(IPin** apPin, ULONG* nPin) override;
    STDMETHODIMP EndOfStream() override;
    STDMETHODIMP BeginFlush() override;
    STDMETHODIMP EndFlush() override;
    STDMETHODIMP NewSegment(REFERENCE_TIME tStart, REFERENCE_TIME tStop, double dRate) override;

    // IKsPropertySet: reports PIN_CATEGORY_CAPTURE, which is how consumers pick the
    // capture pin of a video device (ffmpeg, OBS, meeting apps).
    STDMETHODIMP Set(REFGUID guidPropSet, DWORD dwPropID, LPVOID pInstanceData,
                     DWORD cbInstanceData, LPVOID pPropData, DWORD cbPropData) override;
    STDMETHODIMP Get(REFGUID guidPropSet, DWORD dwPropID, LPVOID pInstanceData,
                     DWORD cbInstanceData, LPVOID pPropData, DWORD cbPropData,
                     DWORD* pcbReturned) override;
    STDMETHODIMP QuerySupported(REFGUID guidPropSet, DWORD dwPropID,
                                DWORD* pTypeSupport) override;

    // IAMStreamConfig: consumers (ffmpeg, OBS, meeting apps) read and set the format here.
    STDMETHODIMP SetFormat(AM_MEDIA_TYPE* pmt) override;
    STDMETHODIMP GetFormat(AM_MEDIA_TYPE** pmt) override;
    STDMETHODIMP GetNumberOfCapabilities(int* count, int* size) override;
    STDMETHODIMP GetStreamCaps(int index, AM_MEDIA_TYPE** pmt, BYTE* caps) override;

    // Stream control, called by the filter.
    void StartStream();
    void StopStream();

private:
    ~CameraPin();

    HRESULT CompleteConnection(IPin* receive_pin, const AM_MEDIA_TYPE* pmt);
    void BuildNotRunningFrame();
    void DeliverLoop();
    static DWORD WINAPI ThreadEntry(LPVOID param);

    LONG ref_;
    CameraFilter* filter_;
    IPin* connected_;
    IMemInputPin* input_;
    IMemAllocator* allocator_;
    AM_MEDIA_TYPE media_type_;
    bool has_media_type_;
    UINT32 width_;
    UINT32 height_;
    HANDLE thread_;
    HANDLE stop_event_;
    std::vector<BYTE> scratch_;
    std::vector<BYTE> not_running_;
};

class CameraFilter final : public IBaseFilter {
public:
    CameraFilter();

    // IUnknown
    STDMETHODIMP QueryInterface(REFIID riid, void** ppv) override;
    STDMETHODIMP_(ULONG) AddRef() override;
    STDMETHODIMP_(ULONG) Release() override;

    // IPersist
    STDMETHODIMP GetClassID(CLSID* pClassID) override;

    // IMediaFilter
    STDMETHODIMP GetState(DWORD dwMilliSecsTimeout, FILTER_STATE* State) override;
    STDMETHODIMP SetSyncSource(IReferenceClock* pClock) override;
    STDMETHODIMP GetSyncSource(IReferenceClock** pClock) override;
    STDMETHODIMP Stop() override;
    STDMETHODIMP Pause() override;
    STDMETHODIMP Run(REFERENCE_TIME tStart) override;

    // IBaseFilter
    STDMETHODIMP EnumPins(IEnumPins** ppEnum) override;
    STDMETHODIMP FindPin(LPCWSTR Id, IPin** ppPin) override;
    STDMETHODIMP QueryFilterInfo(FILTER_INFO* pInfo) override;
    STDMETHODIMP JoinFilterGraph(IFilterGraph* pGraph, LPCWSTR pName) override;
    STDMETHODIMP QueryVendorInfo(LPWSTR* pVendorInfo) override;

private:
    ~CameraFilter();

    LONG ref_;
    CameraPin* pin_;
    IFilterGraph* graph_;
    FILTER_STATE state_;
    WCHAR name_[MAX_FILTER_NAME];
};
