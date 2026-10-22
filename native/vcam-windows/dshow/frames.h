#pragma once
// Frame helpers shared by the filter and its tests (ADR-0019).
//
// The producer publishes top-down BGRA frames, but RGB32 in DirectShow/Media Foundation is
// bottom-up (positive `biHeight`). Media Foundation — and therefore Chrome, Teams and Meet —
// ignores a negative height and shows a top-down buffer upside down, so the filter always
// hands out bottom-up frames.
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <dshow.h>
#include <strmif.h>

#include <vector>

// RGB32 media type for the given size. `biHeight` is positive: bottom-up rows.
AM_MEDIA_TYPE MakeVideoType(UINT32 width, UINT32 height);
void FreeMediaType(AM_MEDIA_TYPE& mt);
bool CopyMediaType(AM_MEDIA_TYPE& dst, const AM_MEDIA_TYPE& src);

// Reads the size out of an RGB32 `FORMAT_VideoInfo` type; accepts either row order.
bool VideoFormat(const AM_MEDIA_TYPE* mt, UINT32* width, UINT32* height);

// Size the producer is publishing right now, or the default before it starts.
void ProducerSize(UINT32* width, UINT32* height);

// Copies a top-down BGRA frame into a bottom-up destination buffer, preserving the aspect
// ratio and filling the unused area with the theme background (0f1115). Any producer size
// fits any destination size, so a resolution change never breaks the connected device.
void BlitFit(const BYTE* src, UINT32 src_w, UINT32 src_h, BYTE* dst, UINT32 dst_w,
             UINT32 dst_h);

// Bottom-up BGRA frame shown while the app is not publishing.
std::vector<BYTE> RenderNotRunning(UINT32 width, UINT32 height);

// Version of this build (`MB_CAMERA_DRIVER_VERSION`), exported by the DLL so the app can
// re-register an older installation on start (ADR-0020).
extern "C" unsigned int __stdcall MeetBurnerCameraVersion();
