# 0038. Accept the glib `VariantStrIter` advisory instead of forking the GTK stack

- Status: accepted
- Date: 2026-10-25

## Context

[RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429) (alias
GHSA-wrw7-89jp-8q8g): `glib::VariantStrIter`'s `Iterator`/`DoubleEndedIterator` implementations pass
`&p` — an immutable reference to a `*mut libc::c_char` out-parameter — to the variadic C function
`g_variant_get_child()`, which writes through it. With recent rustc versions those writes are
optimized away, so `CStr::from_ptr()` receives the `NULL` the pointer started as: undefined
behaviour and a crash. Affected: `glib >= 0.15.0, < 0.20.0`. Fixed in `glib 0.20.0`. There is no
backport to the 0.18 line — 0.18.5 (2023-12-30) is the last 0.18 release and is still affected.

### Why the vulnerable crate is in our tree

`apps/desktop/src-tauri/Cargo.lock` pins:

```
tauri 2.12.5 -> tauri-runtime-wry -> tao 0.37.1 -> gtk 0.18.2 -> glib 0.18.5
```

`gtk`/`glib` are Linux-only dependencies of `tao` and `tauri` (their `[target.'cfg(any(target_os =
"linux", ...))'.dependencies]` tables). The Windows and macOS builds never compile or link them, so
only the Linux `.deb`/AppImage contains glib. Our own Rust code (`tauri`, `serde`, `tiny-skia`,
`ab_glyph`, `nokhwa`, `libc`, `windows-sys`) never depends on glib directly.

### Why we cannot just bump it

Checked on 2026-10-25:

| Fact | Value |
|------|-------|
| `glib` fixed version | `0.20.0` (2024-07-10); the 0.19 line is affected too |
| `gtk 0.18.2` (ours, 2024-12-09) | requires `glib 0.18`, last release of its line |
| `gtk 0.19.0` (2026-09-08, revived gtk3-rs) | moves to `atk 0.19` / `cairo-rs 0.22` and its sys crate (`ffi/*` features instead of `gtk-sys/*`), i.e. a newer glib |
| `tao 0.37.2` (2026-10-23, newest) | still requires `gtk ^0.18` |

Two glib majors cannot coexist inside one gtk stack (their types are not interchangeable), so adding
`glib = "0.20"` to `Cargo.toml` does not upgrade anything: it links a second, unused copy of glib
and leaves the unsound one in place. The real fix is a gtk 0.19+ `tao`/`wry` release, and none
exists yet.

### Is the unsound code reachable?

No code path in our graph calls it. The only entry point is
`glib::Variant::array_iter_str()` (`glib-0.18.5/src/variant.rs:843`), which constructs the
`VariantStrIter`. Searching the sources of every crate in the lock — `tao`, `wry`,
`tauri`/`tauri-runtime-wry`/`tauri-utils`, `gtk`, `gdk*`, `gio`, `atk`, `pango*`, `gdk-pixbuf` —
finds `VariantStrIter` only inside `glib` itself. The only GVariant use in the tree is `gio`'s
`VariantDict` for `GApplication` command-line options, which looks values up by key and never
iterates a string array.

## Decision

Accept the advisory and keep the current GTK stack; do not fork, vendor or pin glib.

1. Stay on `tauri 2` / `tao` / `gtk 0.18` until upstream moves to gtk 0.19+ (glib 0.20+), then do
   that upgrade in one dedicated PR with the full verify suite (Linux build included).
2. Do not add a direct `glib` dependency or a `[patch]` for it: it cannot change what `gtk` links.
3. Never call `glib::Variant::array_iter_str()` (or iterate a GVariant string array by any other
   route) from our code. A future feature that needs it must raise the GTK stack first.
4. Report the situation as: *not reachable in MeetBurner; Windows and macOS are unaffected; the
   Linux build picks up the fix when the upstream GTK stack moves.*
5. If we ever add a dependency audit to CI (`cargo audit`, `cargo deny`), this advisory appears in
   its output and must be ignored with a comment pointing here — never with a bare
   `--ignore RUSTSEC-2024-0429` and no reason.

## Consequences

- The Linux binary keeps linking `glib 0.18.5`. The residual risk is accepted because nothing in
  the process can reach the unsound iterator: it needs an API call we do not make, on a crate we do
  not use.
- Windows (the primary target, with the DirectShow and Media Foundation cameras) and macOS are not
  affected at all: glib is not in their dependency graphs.
- The gtk 0.19 upgrade is not a version bump: it renames the sys crate and drags atk/cairo/pango
  one major ahead. It is tracked as its own roadmap item, not smuggled into a security fix.
- If glib/Variant string iteration ever shows up in our code or in a dependency we add, this ADR is
  superseded by an upgrade ADR.
