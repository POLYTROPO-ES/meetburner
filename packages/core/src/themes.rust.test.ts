import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { THEMES } from './themes';
import { RUST_THEMES_PATH, renderRustThemes } from './themes.rust';

describe('Rust theme mirror', () => {
  it('keeps the generated Rust file in sync with themes.ts', () => {
    const generated = readFileSync(new URL(RUST_THEMES_PATH, import.meta.url), 'utf8');
    expect(generated).toBe(renderRustThemes(THEMES));
  });

  it('renders every theme with its colours, font, weight and radius', () => {
    const rust = renderRustThemes({
      classic: {
        background: '#0f1115',
        backgroundAlt: '#181b22',
        text: '#e8eaf0',
        muted: '#8b93a7',
        accent: '#ff5d5d',
        font: 'system',
        weight: 700,
        radius: 12,
        decoration: '',
      },
    });
    expect(rust).toContain('"classic"');
    expect(rust).toContain('(0x0f, 0x11, 0x15)');
    expect(rust).toContain('FontId::System');
    expect(rust).toContain('weight: 700.0');
    expect(rust).toContain('radius: 12.0');
  });
});
