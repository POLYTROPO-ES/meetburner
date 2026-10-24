cmake_minimum_required(VERSION 3.20)
project(meetburner_vcam LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
# Static CRT: the DLL is loaded inside other applications, so it must not depend
# on the Visual C++ redistributable being installed.
set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>")

add_library(MeetBurnerCamera SHARED
    dshow/dll.cpp
    dshow/vcam.cpp
    dshow/meetburner_camera.def
)
target_include_directories(MeetBurnerCamera PRIVATE common)
target_compile_definitions(MeetBurnerCamera PRIVATE
    UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX _WIN32_WINNT=0x0601)
target_compile_options(MeetBurnerCamera PRIVATE /W4 /permissive- /utf-8)
target_link_libraries(MeetBurnerCamera PRIVATE strmiids uuid ole32 oleaut32 user32 gdi32)

# Media Foundation virtual camera for Windows 11 (M8, ADR-0031). It reads the same shared
# memory as the DirectShow filter and is registered per user by the app. The target builds
# on any Windows SDK: MFCreateVirtualCamera is resolved at run time, never linked.
add_library(MeetBurnerMfCamera SHARED
    mf/dll.cpp
    mf/frames.cpp
    mf/meetburner_mf_camera.def
)
target_include_directories(MeetBurnerMfCamera PRIVATE common mf)
target_compile_definitions(MeetBurnerMfCamera PRIVATE
    UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX _WIN32_WINNT=0x0A00)
target_compile_options(MeetBurnerMfCamera PRIVATE /W4 /permissive- /utf-8)
target_link_libraries(MeetBurnerMfCamera PRIVATE mfplat mfuuid ole32 oleaut32 uuid user32 gdi32)

enable_testing()

add_executable(reader_test tests/reader_test.cpp)
target_include_directories(reader_test PRIVATE common)
target_compile_definitions(reader_test PRIVATE UNICODE _UNICODE _WIN32_WINNT=0x0601)
target_compile_options(reader_test PRIVATE /W4 /utf-8)
add_test(NAME meetburner_reader COMMAND reader_test)

# Unit tests for the frame helpers: DIB row order and the aspect-fit blit (ADR-0019).
add_executable(frame_test tests/frame_test.cpp dshow/vcam.cpp)
target_include_directories(frame_test PRIVATE common dshow)
target_compile_definitions(frame_test PRIVATE
    UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX _WIN32_WINNT=0x0601)
target_compile_options(frame_test PRIVATE /W4 /permissive- /utf-8)
target_link_libraries(frame_test PRIVATE strmiids uuid ole32 oleaut32 user32 gdi32)
add_test(NAME meetburner_frame COMMAND frame_test)

# Unit tests for the Media Foundation frame helpers: top-down RGB32 copy and aspect-fit
# (ADR-0031 amendment).
add_executable(mf_frames_test tests/mf_frames_test.cpp mf/frames.cpp)
target_include_directories(mf_frames_test PRIVATE common mf)
target_compile_definitions(mf_frames_test PRIVATE
    UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX _WIN32_WINNT=0x0A00)
target_compile_options(mf_frames_test PRIVATE /W4 /permissive- /utf-8)
target_link_libraries(mf_frames_test PRIVATE user32 gdi32)
add_test(NAME meetburner_mf_frames COMMAND mf_frames_test)

# Diagnostic: prints what a DirectShow consumer sees for the registered device.
add_executable(dshow_probe tests/dshow_probe.cpp)
target_compile_definitions(dshow_probe PRIVATE UNICODE _UNICODE _WIN32_WINNT=0x0601)
target_compile_options(dshow_probe PRIVATE /W4 /utf-8)
target_link_libraries(dshow_probe PRIVATE strmiids uuid ole32 oleaut32)

# Diagnostic: builds a minimal graph and reports every HRESULT.
add_executable(graph_test tests/graph_test.cpp)
target_compile_definitions(graph_test PRIVATE UNICODE _UNICODE _WIN32_WINNT=0x0601)
target_compile_options(graph_test PRIVATE /W4 /utf-8)
target_link_libraries(graph_test PRIVATE strmiids uuid ole32 oleaut32)
