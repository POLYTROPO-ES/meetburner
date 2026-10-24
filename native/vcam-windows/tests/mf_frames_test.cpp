// Unit tests for the Media Foundation frame helpers (ADR-0019, ADR-0031). They pin the two
// things that broke real meeting apps: RGB32 reaches consumers bottom-up, and a producer
// resolution that differs from the negotiated media type is letterboxed, not stretched.
#include <cstdio>
#include <vector>

#include "../mf/frames.h"

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

DWORD Pixel(const std::vector<BYTE>& buffer, UINT32 pitch, UINT32 x, UINT32 y) {
    const BYTE* p = buffer.data() + static_cast<size_t>(y) * pitch + static_cast<size_t>(x) * 4;
    return static_cast<DWORD>(p[0] | (p[1] << 8) | (p[2] << 16) | (p[3] << 24));
}

bool IsThemeBackground(const std::vector<BYTE>& buffer, UINT32 pitch, UINT32 x, UINT32 y) {
    // GDI leaves the alpha byte at 0 and RGB32 consumers ignore it: compare the colour.
    return (Pixel(buffer, pitch, x, y) & 0x00FFFFFFu) == (kBackground & 0x00FFFFFFu);
}

void flips_top_down_into_bottom_up() {
    // Top-down source: row 0 is the top of the picture.
    const std::vector<BYTE> source = {
        0x00, 0x00, 0xFF, 0xFF, 0x00, 0xFF, 0x00, 0xFF, // red, green
        0xFF, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, // blue, white
    };
    std::vector<BYTE> destination(2 * 2 * 4, 0);
    FitFlipBgra(source.data(), 2, 2, destination.data(), 2 * 4, 2, 2);
    // Bottom-up destination: row 0 holds the bottom of the picture.
    CHECK(Pixel(destination, 8, 0, 0) == Bgra(0, 0, 255));
    CHECK(Pixel(destination, 8, 1, 0) == Bgra(255, 255, 255));
    CHECK(Pixel(destination, 8, 0, 1) == Bgra(255, 0, 0));
    CHECK(Pixel(destination, 8, 1, 1) == Bgra(0, 255, 0));
}

void honours_the_destination_pitch() {
    // A padded sample buffer (stride > width * 4) must not smear rows into the padding.
    const std::vector<BYTE> source = {
        0x00, 0x00, 0xFF, 0xFF, // red (top)
        0x00, 0xFF, 0x00, 0xFF, // green (bottom)
    };
    const UINT32 pitch = 2 * 4 + 8;
    std::vector<BYTE> destination(static_cast<size_t>(pitch) * 2, 0xAB);
    FitFlipBgra(source.data(), 1, 2, destination.data(), pitch, 1, 2);
    CHECK(Pixel(destination, pitch, 0, 0) == Bgra(0, 255, 0));
    CHECK(Pixel(destination, pitch, 0, 1) == Bgra(255, 0, 0));
    CHECK(destination[4] == 0xAB && destination[12] == 0xAB);
}

void letterboxes_to_keep_the_aspect_ratio() {
    // A 2:1 source into a square destination: full width, centred band, background bars.
    const std::vector<BYTE> wide = {
        0x00, 0x00, 0xFF, 0xFF, 0x00, 0xFF, 0x00, 0xFF, // red, green
    };
    std::vector<BYTE> destination(2 * 2 * 4, 0);
    FitFlipBgra(wide.data(), 2, 1, destination.data(), 2 * 4, 2, 2);
    CHECK(Pixel(destination, 8, 0, 1) == Bgra(255, 0, 0));
    CHECK(Pixel(destination, 8, 1, 1) == Bgra(0, 255, 0));
    CHECK(Pixel(destination, 8, 0, 0) == kBackground);
    CHECK(Pixel(destination, 8, 1, 0) == kBackground);

    // A 1:2 portrait source into the same square: full height, centred column.
    const std::vector<BYTE> tall = {
        0x00, 0x00, 0xFF, 0xFF, // red on top
        0x00, 0xFF, 0x00, 0xFF, // green at the bottom
    };
    FitFlipBgra(tall.data(), 1, 2, destination.data(), 2 * 4, 2, 2);
    // (2 - 1) / 2 == 0, so the single column sits at x = 0; the right half is background.
    CHECK(Pixel(destination, 8, 0, 1) == Bgra(255, 0, 0));
    CHECK(Pixel(destination, 8, 0, 0) == Bgra(0, 255, 0));
    CHECK(Pixel(destination, 8, 1, 1) == kBackground);
    CHECK(Pixel(destination, 8, 1, 0) == kBackground);
}

void not_running_frame_is_painted() {
    const std::vector<BYTE> frame = RenderNotRunning(320, 180);
    CHECK(frame.size() == 320u * 180u * 4u);
    if (frame.size() != 320u * 180u * 4u) return;
    // Theme background at the corners and something painted in the middle.
    CHECK(IsThemeBackground(frame, 320 * 4, 0, 0));
    CHECK(IsThemeBackground(frame, 320 * 4, 319, 179));
    size_t painted = 0;
    for (size_t i = 0; i < frame.size(); i += 4) {
        if (frame[i] != 0x15 || frame[i + 1] != 0x11 || frame[i + 2] != 0x0f) ++painted;
    }
    CHECK(painted > 0);
}

} // namespace

int main() {
    printf("meetburner mf frame helpers test\n");
    flips_top_down_into_bottom_up();
    honours_the_destination_pitch();
    letterboxes_to_keep_the_aspect_ratio();
    not_running_frame_is_painted();
    if (g_failures != 0) {
        printf("%d failure(s)\n", g_failures);
        return 1;
    }
    printf("mf frame helpers ok\n");
    return 0;
}
