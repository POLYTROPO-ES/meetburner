// Frame helpers for the Media Foundation camera (ADR-0019, ADR-0031).
#include "frames.h"

#include <algorithm>
#include <cstring>

namespace {

// BGRA of RGB(0f, 11, 15), the theme background used by the DirectShow filter too.
const uint32_t kBackground = 0xFF0F1115u;
const wchar_t* const kNotRunningText = L"MeetBurner \u2013 not running";

} // namespace

void FitFlipBgra(const uint8_t* src, uint32_t src_w, uint32_t src_h, uint8_t* dst,
                 uint32_t dst_pitch, uint32_t dst_w, uint32_t dst_h) {
    if (!src || !dst || src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0) return;

    // Background first: the bars around a letterboxed picture.
    for (uint32_t y = 0; y < dst_h; ++y) {
        auto* row = reinterpret_cast<uint32_t*>(dst + static_cast<size_t>(y) * dst_pitch);
        std::fill(row, row + dst_w, kBackground);
    }

    uint32_t out_w;
    uint32_t out_h;
    const uint64_t src_cross = static_cast<uint64_t>(src_w) * dst_h;
    const uint64_t dst_cross = static_cast<uint64_t>(dst_w) * src_h;
    if (src_cross >= dst_cross) {
        out_w = dst_w;
        out_h = static_cast<uint32_t>(static_cast<uint64_t>(dst_w) * src_h / src_w);
    } else {
        out_h = dst_h;
        out_w = static_cast<uint32_t>(static_cast<uint64_t>(dst_h) * src_w / src_h);
    }
    if (out_w == 0) out_w = 1;
    if (out_h == 0) out_h = 1;
    if (out_w > dst_w) out_w = dst_w;
    if (out_h > dst_h) out_h = dst_h;

    const uint32_t off_x = (dst_w - out_w) / 2;
    const uint32_t off_y = (dst_h - out_h) / 2;
    const bool same_size = out_w == src_w && out_h == src_h && out_w == dst_w && out_h == dst_h;

    for (uint32_t vy = 0; vy < out_h; ++vy) {
        const uint32_t sy =
            same_size ? vy : static_cast<uint32_t>(static_cast<uint64_t>(vy) * src_h / out_h);
        const uint8_t* src_row = src + static_cast<size_t>(sy) * src_w * 4;
        // Row 0 of a bottom-up buffer is the bottom of the picture.
        uint8_t* dst_row =
            dst + static_cast<size_t>(dst_h - 1 - (off_y + vy)) * dst_pitch +
            static_cast<size_t>(off_x) * 4;
        if (same_size) {
            std::memcpy(dst_row, src_row, static_cast<size_t>(src_w) * 4);
            continue;
        }
        for (uint32_t vx = 0; vx < out_w; ++vx) {
            const uint32_t sx =
                static_cast<uint32_t>(static_cast<uint64_t>(vx) * src_w / out_w);
            std::memcpy(dst_row + static_cast<size_t>(vx) * 4,
                        src_row + static_cast<size_t>(sx) * 4, 4);
        }
    }
}

std::vector<uint8_t> RenderNotRunning(uint32_t width, uint32_t height) {
    std::vector<uint8_t> buffer(static_cast<size_t>(width) * height * 4, 0);
    if (width == 0 || height == 0) return buffer;

    BITMAPINFO bmi;
    ZeroMemory(&bmi, sizeof(bmi));
    bmi.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
    bmi.bmiHeader.biWidth = static_cast<LONG>(width);
    // Negative height: a top-down DIB, the order the producer uses and `FitFlipBgra` expects.
    bmi.bmiHeader.biHeight = -static_cast<LONG>(height);
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;

    void* bits = nullptr;
    HDC screen = GetDC(nullptr);
    HDC dc = CreateCompatibleDC(screen);
    HBITMAP bitmap = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &bits, nullptr, 0);
    if (bitmap && bits) {
        HGDIOBJ previous = SelectObject(dc, bitmap);
        HBRUSH brush = CreateSolidBrush(RGB(0x0f, 0x11, 0x15));
        RECT rect = {0, 0, static_cast<LONG>(width), static_cast<LONG>(height)};
        FillRect(dc, &rect, brush);
        DeleteObject(brush);

        SetBkMode(dc, TRANSPARENT);
        SetTextColor(dc, RGB(0xe6, 0xe8, 0xeb));
        HFONT font = CreateFontW(-28, 0, 0, 0, FW_SEMIBOLD, FALSE, FALSE, FALSE, DEFAULT_CHARSET,
                                 OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY,
                                 DEFAULT_PITCH | FF_DONTCARE, L"Segoe UI");
        HGDIOBJ previous_font = font ? SelectObject(dc, font) : nullptr;
        DrawTextW(dc, kNotRunningText, -1, &rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        GdiFlush();
        if (previous_font) SelectObject(dc, previous_font);
        if (font) DeleteObject(font);
        SelectObject(dc, previous);
        std::memcpy(buffer.data(), bits, buffer.size());
    }
    if (bitmap) DeleteObject(bitmap);
    DeleteDC(dc);
    ReleaseDC(nullptr, screen);
    return buffer;
}
