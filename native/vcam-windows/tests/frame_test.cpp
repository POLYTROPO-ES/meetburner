// Unit tests for the frame helpers (ADR-0019). They pin the two things that broke real
// meeting apps: the DIB row order (Media Foundation shows top-down frames upside down) and
// the fit used when the producer resolution differs from the negotiated media type.
#include <algorithm>
#include <cstdio>
#include <cstring>
#include <vector>

#include "../common/meetburner_camera.h"
#include "../dshow/frames.h"

static int g_failures = 0;

#define CHECK(condition)                                                \
    do {                                                                \
        if (!(condition)) {                                             \
            printf("FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
            ++g_failures;                                               \
        }                                                               \
    } while (0)

namespace {

const DWORD kBackground = 0xFF0F1115u;

DWORD Bgra(BYTE r, BYTE g, BYTE b) {
    return static_cast<DWORD>(0xFF000000u | (static_cast<DWORD>(r) << 16) |
                              (static_cast<DWORD>(g) << 8) | static_cast<DWORD>(b));
}

DWORD Pixel(const std::vector<BYTE>& buffer, UINT32 width, UINT32 x, UINT32 y) {
    const BYTE* p = buffer.data() + (static_cast<size_t>(y) * width + x) * 4;
    return static_cast<DWORD>(p[0] | (p[1] << 8) | (p[2] << 16) | (p[3] << 24));
}

bool IsThemeBackground(const std::vector<BYTE>& buffer, UINT32 width, UINT32 x, UINT32 y) {
    // GDI leaves the alpha byte at 0 and RGB32 consumers ignore it, so check the colour only.
    const BYTE* p = buffer.data() + (static_cast<size_t>(y) * width + x) * 4;
    return p[0] == 0x15 && p[1] == 0x11 && p[2] == 0x0f;
}

void frame_type_is_bottom_up() {
    AM_MEDIA_TYPE mt = MakeVideoType(1280, 720);
    CHECK(mt.cbFormat == sizeof(VIDEOINFOHEADER));
    CHECK(mt.lSampleSize == mb_payload_size(1280, 720));
    const auto* vih = reinterpret_cast<const VIDEOINFOHEADER*>(mt.pbFormat);
    CHECK(vih != nullptr);
    if (vih) {
        // Positive height means bottom-up, which is what Media Foundation expects. A
        // negative height made Meet show the counter rotated 180 degrees.
        CHECK(vih->bmiHeader.biHeight == 720);
        CHECK(vih->bmiHeader.biWidth == 1280);
        CHECK(vih->bmiHeader.biBitCount == 32);
        CHECK(vih->bmiHeader.biCompression == BI_RGB);
    }
    UINT32 width = 0;
    UINT32 height = 0;
    CHECK(VideoFormat(&mt, &width, &height));
    CHECK(width == 1280 && height == 720);
    FreeMediaType(mt);
}

void blit_flips_top_down_into_bottom_up() {
    // Top-down source: row 0 is the top of the picture.
    const std::vector<BYTE> source = {
        0x00, 0x00, 0xFF, 0xFF, 0x00, 0xFF, 0x00, 0xFF, // red, green
        0xFF, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, // blue, white
    };
    std::vector<BYTE> destination(mb_payload_size(2, 2), 0);
    BlitFit(source.data(), 2, 2, destination.data(), 2, 2);
    // Bottom-up destination: row 0 holds the bottom of the picture.
    CHECK(Pixel(destination, 2, 0, 0) == Bgra(0, 0, 255));
    CHECK(Pixel(destination, 2, 1, 0) == Bgra(255, 255, 255));
    CHECK(Pixel(destination, 2, 0, 1) == Bgra(255, 0, 0));
    CHECK(Pixel(destination, 2, 1, 1) == Bgra(0, 255, 0));
}

void blit_letterboxes_to_keep_the_aspect_ratio() {
    // A 2:1 source into a square destination: full width, centred band, bars above and below.
    const std::vector<BYTE> wide = {
        0x00, 0x00, 0xFF, 0xFF, 0x00, 0xFF, 0x00, 0xFF, // red, green
    };
    std::vector<BYTE> destination(mb_payload_size(2, 2), 0);
    BlitFit(wide.data(), 2, 1, destination.data(), 2, 2);
    CHECK(Pixel(destination, 2, 0, 1) == Bgra(255, 0, 0));
    CHECK(Pixel(destination, 2, 1, 1) == Bgra(0, 255, 0));
    CHECK(Pixel(destination, 2, 0, 0) == kBackground);
    CHECK(Pixel(destination, 2, 1, 0) == kBackground);

    // A 1:2 source into the same square: full height, centred column.
    const std::vector<BYTE> tall = {
        0x00, 0x00, 0xFF, 0xFF, // red on top
        0x00, 0xFF, 0x00, 0xFF, // green at the bottom
    };
    std::fill(destination.begin(), destination.end(), static_cast<BYTE>(0));
    BlitFit(tall.data(), 1, 2, destination.data(), 2, 2);
    CHECK(Pixel(destination, 2, 0, 1) == Bgra(255, 0, 0));
    CHECK(Pixel(destination, 2, 0, 0) == Bgra(0, 255, 0));
    CHECK(Pixel(destination, 2, 1, 1) == kBackground);
    CHECK(Pixel(destination, 2, 1, 0) == kBackground);
}

void not_running_frame_is_painted_with_the_theme_background() {
    const std::vector<BYTE> frame = RenderNotRunning(320, 180);
    CHECK(frame.size() == mb_payload_size(320, 180));
    if (frame.size() != mb_payload_size(320, 180)) return;
    // Background plus the centred message: no uninitialised rows, no all-black frame.
    CHECK(IsThemeBackground(frame, 320, 0, 0));
    CHECK(IsThemeBackground(frame, 320, 319, 179));
    size_t painted = 0;
    for (size_t i = 0; i < frame.size(); i += 4) {
        if (frame[i] != 0x15 || frame[i + 1] != 0x11 || frame[i + 2] != 0x0f) ++painted;
    }
    CHECK(painted > 0);
}

void driver_reports_its_build_version() {
    // The app compares this with the version stored at registration (ADR-0020).
    CHECK(MeetBurnerCameraVersion() == MB_CAMERA_DRIVER_VERSION);
    CHECK(MeetBurnerCameraVersion() >= 2u);
}

} // namespace

int main() {
    printf("meetburner frame helpers test\n");
    frame_type_is_bottom_up();
    blit_flips_top_down_into_bottom_up();
    blit_letterboxes_to_keep_the_aspect_ratio();
    not_running_frame_is_painted_with_the_theme_background();
    driver_reports_its_build_version();
    if (g_failures != 0) {
        printf("%d failure(s)\n", g_failures);
        return 1;
    }
    printf("frame helpers ok\n");
    return 0;
}
