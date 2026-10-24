#pragma once
// Frame helpers for the Media Foundation camera (ADR-0019, ADR-0031).
//
// The producer publishes top-down BGRA pixels (ADR-0013), and MFVideoFormat_RGB32 is
// bottom-up: MF ignores a negative stride, so the rows are flipped here while the sample
// is filled. Both cameras read the same mapping, so the flip rules must not drift.
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cstdint>
#include <vector>

// Copies a top-down BGRA frame into a bottom-up RGB32 sample buffer, preserving the
// aspect ratio: the picture is centred and the bars use the theme background (0f1115).
// Any producer size fits any negotiated size, so a resolution change never breaks the
// device (ADR-0019). `dst_pitch` is in bytes and is `dst_w * 4` for a packed buffer.
void FitFlipBgra(const uint8_t* src, uint32_t src_w, uint32_t src_h, uint8_t* dst,
                 uint32_t dst_pitch, uint32_t dst_w, uint32_t dst_h);

// Top-down BGRA placeholder shown while the app publishes nothing ("MeetBurner – not
// running"). Callers flip it like any other frame.
std::vector<uint8_t> RenderNotRunning(uint32_t width, uint32_t height);
