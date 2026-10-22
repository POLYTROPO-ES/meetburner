# vcam-windows

Owner: `vcam-windows` agent. See [ADR-0002](../../docs/decisions/0002-virtual-camera.md).

Planned layout:

```
common/   shared-memory IPC header (C), mirrored in Rust with #[repr(C)]
dshow/    DirectShow source filter DLL – stage 1 (M6)
mf/       Media Foundation virtual camera – Windows 11 (M8)
CMakeLists.txt
```
