// Copies the DirectShow driver where the app and the bundles expect it
// (ADR-0017). Runs before `tauri build` and again after it (postbuild:desktop).
import { copyFileSync, existsSync, mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const source = resolve(root, 'native/vcam-windows/build/Release/MeetBurnerCamera.dll');
// Bundled by Tauri (`bundle.resources`) into <install>/resources/.
const bundle = resolve(root, 'apps/desktop/src-tauri/resources/MeetBurnerCamera.dll');
// Next to the portable exe, for target/release/meetburner.exe.
const portable = resolve(root, 'apps/desktop/src-tauri/target/release/resources/MeetBurnerCamera.dll');

if (!existsSync(source)) {
  console.warn('[copy-driver] driver not found, run the CMake build in native/vcam-windows first');
  console.warn(`[copy-driver] expected: ${source}`);
  process.exit(0);
}

for (const target of [bundle, portable]) {
  mkdirSync(dirname(target), { recursive: true });
  copyFileSync(source, target);
  console.log(`[copy-driver] ${target}`);
}
