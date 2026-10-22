#pragma once
/*
 * MeetBurner camera frame protocol, shared by the Rust producer and the DirectShow
 * filter (ADR-0013). Layout: a 48-byte header followed by tightly packed BGRA
 * pixels, guarded by a seqlock so readers never copy a torn frame.
 *
 * Keep this file in sync with apps/desktop/src-tauri/src/camera/ipc.rs.
 */
#include <windows.h>
#include <stdint.h>
#include <string.h>

#define MB_CAMERA_MAPPING_NAME L"Local\\MeetBurnerCamera"
#define MB_CAMERA_EVENT_NAME L"Local\\MeetBurnerCameraFrame"

#define MB_CAMERA_MAGIC 0x4D42434Du /* "MBCM" */
#define MB_CAMERA_VERSION 1u
#define MB_CAMERA_STATE_IDLE 0u
#define MB_CAMERA_STATE_LIVE 1u

/* Driver build: bump it whenever the filter behaves differently, so the app can
 * re-register an older installation on start (ADR-0020).
 *   1: first release (top-down frames, mapping re-created on resize)
 *   2: bottom-up frames, one mapping, stale-frame handling (ADR-0019) */
#define MB_CAMERA_DRIVER_VERSION 2u
#define MB_CAMERA_MAX_WIDTH 1920u
#define MB_CAMERA_MAX_HEIGHT 1920u
/* The mapping is always created with room for this many pixels (ADR-0019), so a
 * resolution change never re-creates it and readers keep one valid view. */
#define MB_CAMERA_CAPACITY_PIXELS (1920u * 1080u)

#pragma pack(push, 1)
typedef struct meetburner_frame_header {
    uint32_t magic;         /*  0 */
    uint32_t version;       /*  4 */
    uint32_t width;         /*  8 */
    uint32_t height;        /* 12 */
    uint32_t fps;           /* 16 */
    uint32_t state;         /* 20: 0 idle, 1 live */
    uint64_t frame_counter; /* 24 */
    uint64_t timestamp_ms;  /* 32 */
    uint32_t sequence;      /* 40: odd while the producer writes */
    uint32_t reserved;      /* 44: must be 0 */
} meetburner_frame_header;
#pragma pack(pop)

#if defined(__cplusplus)
static_assert(sizeof(meetburner_frame_header) == 48, "header must stay 48 bytes");
#else
_Static_assert(sizeof(meetburner_frame_header) == 48, "header must stay 48 bytes");
#endif

typedef struct mb_reader {
    HANDLE mapping;
    HANDLE frame_event;
    const uint8_t* view;
    size_t len;
} mb_reader;

static inline size_t mb_payload_size(uint32_t width, uint32_t height) {
    return (size_t)width * (size_t)height * 4u;
}

static inline void mb_reader_close(mb_reader* reader) {
    if (!reader) return;
    if (reader->view) UnmapViewOfFile(reader->view);
    if (reader->mapping) CloseHandle(reader->mapping);
    if (reader->frame_event) CloseHandle(reader->frame_event);
    memset(reader, 0, sizeof(*reader));
}

/* Opens the producer mapping. Returns false while the app is not publishing frames. */
static inline bool mb_reader_open(mb_reader* reader, const wchar_t* mapping_name,
                                  const wchar_t* event_name) {
    memset(reader, 0, sizeof(*reader));
    HANDLE mapping = OpenFileMappingW(FILE_MAP_READ, FALSE, mapping_name);
    if (!mapping) return false;
    const uint8_t* view = (const uint8_t*)MapViewOfFile(mapping, FILE_MAP_READ, 0, 0, 0);
    if (!view) {
        CloseHandle(mapping);
        return false;
    }
    MEMORY_BASIC_INFORMATION info;
    size_t len = 0;
    if (VirtualQuery(view, &info, sizeof(info)) == sizeof(info)) len = info.RegionSize;
    reader->mapping = mapping;
    reader->view = view;
    reader->len = len;
    /* The event only exists while the producer is alive; a missing event is fine. */
    reader->frame_event = OpenEventW(SYNCHRONIZE, FALSE, event_name);
    return true;
}

/* Reads the header when the sequence is stable. */
static inline bool mb_peek_header(const mb_reader* reader, meetburner_frame_header* out) {
    if (!reader || !reader->view || !out || reader->len < sizeof(*out)) return false;
    for (unsigned attempt = 0; attempt < 8; ++attempt) {
        meetburner_frame_header start;
        memcpy(&start, reader->view, sizeof(start));
        if (start.magic != MB_CAMERA_MAGIC || start.version != MB_CAMERA_VERSION) return false;
        if (start.sequence & 1u) {
            SwitchToThread();
            continue;
        }
        meetburner_frame_header end;
        memcpy(&end, reader->view, sizeof(end));
        if (end.sequence == start.sequence && (end.sequence & 1u) == 0) {
            *out = end;
            return true;
        }
        SwitchToThread();
    }
    return false;
}

/* Copies the payload into dst while validating the seqlock. */
static inline bool mb_read_frame(const mb_reader* reader, void* dst, size_t dst_size,
                                 meetburner_frame_header* out) {
    if (!reader || !reader->view || !dst || reader->len < sizeof(meetburner_frame_header)) return false;
    for (unsigned attempt = 0; attempt < 8; ++attempt) {
        meetburner_frame_header start;
        memcpy(&start, reader->view, sizeof(start));
        if (start.magic != MB_CAMERA_MAGIC || start.version != MB_CAMERA_VERSION) return false;
        if (start.sequence & 1u) {
            SwitchToThread();
            continue;
        }
        if (start.width == 0 || start.height == 0 || start.width > MB_CAMERA_MAX_WIDTH ||
            start.height > MB_CAMERA_MAX_HEIGHT ||
            start.width * start.height > MB_CAMERA_CAPACITY_PIXELS) {
            return false;
        }
        const size_t payload = mb_payload_size(start.width, start.height);
        if (sizeof(start) + payload > reader->len || payload > dst_size) return false;
        MemoryBarrier();
        memcpy(dst, reader->view + sizeof(start), payload);
        MemoryBarrier();
        meetburner_frame_header end;
        memcpy(&end, reader->view, sizeof(end));
        if (end.sequence == start.sequence && (end.sequence & 1u) == 0) {
            if (out) *out = end;
            return true;
        }
        SwitchToThread();
    }
    return false;
}
