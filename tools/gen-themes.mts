// Writes the Rust mirror of packages/core/src/themes.ts (ADR-0014).
// Usage: npm run gen:themes
import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { THEMES } from '../packages/core/src/themes.ts';
import { renderRustThemes } from '../packages/core/src/themes.rust.ts';

const target = fileURLToPath(new URL('../apps/desktop/src-tauri/src/camera/themes_data.rs', import.meta.url));
writeFileSync(target, renderRustThemes(THEMES));
console.log(`wrote ${target}`);
