// Copies the Windows camera drivers where the app and the bundles expect them
// (ADR-0017, ADR-0031). Runs before `tauri build` and again after it (postbuild:desktop).
import { copyFileSync, existsSync, mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
// The DirectShow filter (M6) and the Windows 11 Media Foundation camera (M8). Both read the
// same shared memory, so both ship together.
const drivers = ['MeetBurnerCamera.dll', 'MeetBurnerMfCamera.dll'];

// The drivers only exist on Windows; the macOS and Linux bundles carry none.
if (process.platform !== 'win32') {
  process.exit(0);
}

for (const driver of drivers) {
  const source = resolve(root, 'native/vcam-windows/build/Release', driver);
  // Bundled by Tauri (`bundle.resources`) into <install>/resources/.
  const bundle = resolve(root, 'apps/desktop/src-tauri/resources', driver);
  // Next to the portable exe, for target/release/meetburner.exe.
  const portable = resolve(root, 'apps/desktop/src-tauri/target/release/resources', driver);

  if (!existsSync(source)) {
    console.warn(`[copy-driver] ${driver} not found, run the CMake build in native/vcam-windows first`);
    console.warn(`[copy-driver] expected: ${source}`);
    continue;
  }

  for (const target of [bundle, portable]) {
    mkdirSync(dirname(target), { recursive: true });
    try {
      copyFileSync(source, target);
      console.log(`[copy-driver] ${target}`);
    } catch (error) {
      // Browser/meeting apps keep the DLL loaded while they list cameras, which locks the
      // copy on Windows. The build continues with the previous copy and warns instead.
      if (error && (error.code === 'EBUSY' || error.code === 'EPERM' || error.code === 'EACCES')) {
        console.warn(`[copy-driver] locked by another process, keeping the current copy: ${target}`);
        continue;
      }
      throw error;
    }
  }
}
