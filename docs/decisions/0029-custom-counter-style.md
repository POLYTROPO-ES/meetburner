# 0029. Every style is customisable, and the customisation follows the style

- Status: accepted
- Date: 2026-10-23

## Context

The seven presets in `THEMES` are closed: colours, radius and font come from one table, and the
only user controls were a font and a size for the cost and for the time. Users asked to paint the
counter their own way without giving up the presets, and to have the web preview and the Windows
camera agree.

ADR-0024 already established that the camera renderer reads the same theme data as the web
(`themes_data.rs` is generated from `themes.ts`), so any new visual property has to exist as data.

## Decision

Customisation is stored **per style id**, not per element globally: switching from Classic to
Halloween and back returns exactly the values you left on each one. A **Reset** button removes the
stored tweaks for the selected style and restores the preset.

```ts
type CounterElement = 'label' | 'cost' | 'time' | 'status';

interface ElementStyle {
  font: FontChoice;      // 'theme' keeps the preset font
  scale: number;         // 50..250 %, 100 = the preset size
  color: string | null;  // null keeps the preset colour
}

interface BackgroundStyle {
  mode: 'solid' | 'gradient';
  color: string;         // solid fill, or the gradient start
  colorAlt: string;      // gradient end
  direction: 'horizontal' | 'vertical';
  opacity: number;       // 0 = fully transparent, 100 = solid
  radius: number;        // 0..40 px, 0 = square
}

interface DisplaySettings {
  theme: ThemeId;
  layout: Layout;
  overflow: Overflow;
  mirror: MirrorSettings;
  overlay: OverlayRect | null;
  styles: Partial<Record<ThemeId, StyleOverride>>; // absent = the preset
}
```

- `styles` is keyed by the selected entry of `THEME_IDS`, so `seasons` keeps one set of tweaks
  across the four seasons; the defaults come from the season in force when the style is opened.
- `costFont`, `timeFont`, `costScale` and `timeScale` disappear from `DisplaySettings`: they were
  a partial version of `ElementStyle` and two sources of truth are worse than one. Stored settings
  from older builds fall back to the presets (`normalizeDisplay` ignores the old keys).
- The presets keep their look. `mode` defaults to `gradient` (the web already painted
  `linear-gradient(135deg, background, backgroundAlt)`), `direction` to `vertical`, `opacity` to
  100, `radius` to the preset radius.
- The decorations (`🎄❄️`, `🌸`, `☀️`, `🍂`, `❄️`) are removed from Christmas and the four
  seasons: at the size the counter renders them they read as noise, not as decoration. Halloween
  keeps its `🎃🦇`.
- The Windows renderer implements the same properties (gradient, opacity over the webcam, radius,
  per-element font/size/colour) so the camera output matches the setup preview.

## Consequences

- `themes_data.rs` keeps carrying the presets; the per-style tweaks travel in the `camera_start` /
  `camera_update` payload, exactly like `costFont` does today.
- A saved style survives theme switches and reloads; there is no "unnamed custom style" object to
  manage, and no migration or naming UI.
- Text elements that the preset draws with one colour (`muted`) can now differ from each other;
  `color: null` must keep meaning "whatever the preset says", including the `accent` colour of a
  running cost.
- The setup page grows a dense block of controls; they stay inside the existing "Counter style"
  section, collapsed behind the style selector.

## Amendment (2026-10-24): visibility and rotation

Two fields joined the model without changing its shape:

- `ElementStyle.visible` (default `true`) hides one of the six texts while keeping its font, size
  and colour, so showing it again does not need to be re-configured. A hidden element also stops
  reserving space in the camera layout: the label band and the time/footer lines give their room
  to the others.
- `StyleOverride.rotation` (0..360 degrees, default `0`) turns the counter. It rotates the card
  and its text only — never the webcam image — around the card centre (widget) or the frame
  centre (fullscreen). A full turn (`360`) draws exactly like `0`.

Both follow the per-style rule above: each preset remembers whether a text is shown and how far
it is turned.
