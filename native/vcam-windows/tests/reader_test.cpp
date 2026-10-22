// CTest for the shared-memory reader (ADR-0013). Runs without COM or the app:
// it plays the Rust producer with a real named mapping and checks the seqlock rules.
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

#include "meetburner_camera.h"

static int g_failures = 0;

#define CHECK(condition)                                                     \
    do {                                                                     \
        if (!(condition)) {                                                  \
            printf("FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition);      \
            ++g_failures;                                                    \
        }                                                                    \
    } while (0)

namespace {

struct TestMapping {
    HANDLE mapping = nullptr;
    uint8_t* view = nullptr;
    size_t len = 0;
    std::wstring name;

    explicit TestMapping(const wchar_t* tag) {
        name = std::wstring(L"Local\\MeetBurnerReaderTest-") + tag + L"-" +
               std::to_wstring(GetCurrentProcessId());
        len = sizeof(meetburner_frame_header) + mb_payload_size(4, 2);
        mapping = CreateFileMappingW(INVALID_HANDLE_VALUE, nullptr, PAGE_READWRITE, 0, (DWORD)len,
                                     name.c_str());
        if (!mapping) return;
        view = (uint8_t*)MapViewOfFile(mapping, FILE_MAP_ALL_ACCESS, 0, 0, len);
    }

    ~TestMapping() {
        if (view) UnmapViewOfFile(view);
        if (mapping) CloseHandle(mapping);
    }

    meetburner_frame_header* header() { return (meetburner_frame_header*)view; }

    void write_frame(uint32_t width, uint32_t height, uint8_t value, uint64_t counter) {
        meetburner_frame_header* h = header();
        h->sequence |= 1u; // odd: writing
        h->magic = MB_CAMERA_MAGIC;
        h->version = MB_CAMERA_VERSION;
        h->width = width;
        h->height = height;
        h->fps = 30;
        h->state = MB_CAMERA_STATE_LIVE;
        h->frame_counter = counter;
        h->timestamp_ms = 42;
        h->reserved = 0;
        memset(view + sizeof(*h), value, mb_payload_size(width, height));
        h->sequence = (uint32_t)(counter * 2); // even: stable
    }
};

void runs_with_an_empty_mapping(TestMapping& mapping) {
    (void)mapping;
    mb_reader reader;
    CHECK(mb_reader_open(&reader, mapping.name.c_str(), L"Local\\MeetBurnerReaderTestEvent-none"));
    meetburner_frame_header header;
    CHECK(!mb_peek_header(&reader, &header));
    std::vector<uint8_t> pixels(1024);
    CHECK(!mb_read_frame(&reader, pixels.data(), pixels.size(), &header));
    mb_reader_close(&reader);
}

void reads_a_frame(TestMapping& mapping) {
    mapping.write_frame(4, 2, 0x5A, 1);
    mb_reader reader;
    CHECK(mb_reader_open(&reader, mapping.name.c_str(), L"Local\\MeetBurnerReaderTestEvent-none"));
    meetburner_frame_header header;
    CHECK(mb_peek_header(&reader, &header));
    CHECK(header.width == 4 && header.height == 2 && header.frame_counter == 1);

    std::vector<uint8_t> pixels(mb_payload_size(4, 2));
    CHECK(mb_read_frame(&reader, pixels.data(), pixels.size(), &header));
    CHECK(header.timestamp_ms == 42 && header.state == MB_CAMERA_STATE_LIVE);
    bool all_copied = true;
    for (uint8_t pixel : pixels) all_copied = all_copied && pixel == 0x5A;
    CHECK(all_copied);
    mb_reader_close(&reader);
}

void rejects_a_torn_frame(TestMapping& mapping) {
    mapping.write_frame(4, 2, 0x11, 2);
    mapping.header()->sequence |= 1u; // producer died mid-copy
    mb_reader reader;
    CHECK(mb_reader_open(&reader, mapping.name.c_str(), L"Local\\MeetBurnerReaderTestEvent-none"));
    std::vector<uint8_t> pixels(mb_payload_size(4, 2));
    meetburner_frame_header header;
    CHECK(!mb_read_frame(&reader, pixels.data(), pixels.size(), &header));
    mb_reader_close(&reader);
}

void rejects_a_buffer_that_is_too_small(TestMapping& mapping) {
    mapping.write_frame(4, 2, 0x22, 3);
    mb_reader reader;
    CHECK(mb_reader_open(&reader, mapping.name.c_str(), L"Local\\MeetBurnerReaderTestEvent-none"));
    std::vector<uint8_t> pixels(8);
    meetburner_frame_header header;
    CHECK(!mb_read_frame(&reader, pixels.data(), pixels.size(), &header));
    mb_reader_close(&reader);
}

void reports_a_missing_producer(TestMapping& mapping) {
    mb_reader reader;
    CHECK(!mb_reader_open(&reader, L"Local\\MeetBurnerReaderTest-does-not-exist",
                          L"Local\\MeetBurnerReaderTestEvent-none"));
    (void)mapping;
}

} // namespace

int main() {
    printf("meetburner reader test\n");
    TestMapping mapping(L"main");
    if (!mapping.view) {
        printf("FAIL: cannot create the test mapping (error %lu)\n", GetLastError());
        return 1;
    }
    runs_with_an_empty_mapping(mapping);
    reads_a_frame(mapping);
    rejects_a_torn_frame(mapping);
    rejects_a_buffer_that_is_too_small(mapping);
    reports_a_missing_producer(mapping);
    if (g_failures == 0) printf("OK\n");
    return g_failures == 0 ? 0 : 1;
}
