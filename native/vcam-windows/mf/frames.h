#pragma once
// Frame helpers for the Media Foundation camera (ADR-0019, ADR-0031).
//
// The producer publishes top-down BGRA pixels (ADR-0013) and the media type declares a
// positive MF_MT_DEFAULT_STRIDE, which means top-down to Media Foundation, so rows are
// copied in order. This differs from the DirectShow filter: a DIB with biHeight > 0 is
// bottom-up there and `BlitFit` flips (ADR-0019). Negative strides are not honoured by MF,
// so the fix for a flip here is the row mapping, never a negative stride.
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cstdint>
#include <vector>

// Copies a top-down BGRA frame into a top-down RGB32 sample buffer, preserving the aspect
// ratio: the picture is centred and the bars use the theme background (0f1115). Row 0 of
// the source stays row 0 of the destination band (never mirrored). Any producer size fits
// any negotiated size, so a resolution change never breaks the device (ADR-0019).
// `dst_pitch` is in bytes and is `dst_w * 4` for a packed buffer.
void FitBgra(const uint8_t* src, uint32_t src_w, uint32_t src_h, uint8_t* dst,
             uint32_t dst_pitch, uint32_t dst_w, uint32_t dst_h);

// Top-down BGRA placeholder shown while the app publishes nothing ("MeetBurner – not
// running"). It is already in the sample's row order, so callers copy it like any other
// frame.
std::vector<uint8_t> RenderNotRunning(uint32_t width, uint32_t height);
