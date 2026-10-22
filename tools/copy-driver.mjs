// Copies the DirectShow driver next to the packaged app so the portable exe can
// register it (ADR-0017). Runs automatically after `npm run build:desktop`.
import { copyFileSync, existsSync, mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const source = resolve(root, 'native/vcam-windows/build/Release/MeetBurnerCamera.dll');
const target = resolve(root, 'apps/desktop/src-tauri/target/release/resources/MeetBurnerCamera.dll');

if (!existsSync(source)) {
  console.warn('[copy-driver] driver not found, run the CMake build in native/vcam-windows first');
  console.warn(`[copy-driver] expected: ${source}`);
  process.exit(0);
}

mkdirSync(dirname(target), { recursive: true });
copyFileSync(source, target);
console.log(`[copy-driver] ${target}`);
