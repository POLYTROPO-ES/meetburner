# 0034. The setup page is a shell: editor on the left, the counter always in view

- Status: accepted
- Date: 2026-10-25

## Context

Everything lived in one vertical column: header, meeting, participants, style, camera, footer. Editing a
font meant scrolling the counter out of view, and at 1280×720 the page never fit. The chosen direction
(after reviewing three mockups) is a compact studio: the counter visible at all times, the controls in a
column of their own, and a single primary action always reachable.

## Decision

```
┌ header: brand · people · €/h · €/min · language ┐
├───────────────┬──┬───────────────────────────────┤
│ editor column │⇔ │ preview: mode switch + camera  │
│  Meeting      │  │ frame with the counter card    │
│  Participants │  │ (drag, resize, turn)           │
│  Counter      │  │                               │
│  Camera       │  │                               │
├───────────────┴──┴───────────────────────────────┤
│ footer: totals + Reset · Share · Start/Stop      │
└──────────────────────────────────────────────────┘
```

- The shell is a three-row grid (header, workspace, footer) that fills the viewport. Only the editor
  column scrolls; the preview never leaves the screen.
- The editor column holds the four groups as native `<details>`: keyboard support, no JavaScript state,
  and collapsed groups still show their title. `Counter style` and `Camera` keep everything that used to
  be on the page; nothing is dropped.
- The splitter between the columns is a `role="separator"` with arrow keys (2 %, Shift 5 %, Home resets),
  and the chosen fraction is remembered per device (`meetburner.pane`, 22–50 %).
- The **Only counter / Add video background** switch stays directly above the preview (ADR-0033); the
  camera options (source, resolution, mirrors, driver) live in the **Camera** group.
- Below 1100 px the workspace stacks and the page scrolls: the target is 1280×720 and up, as agreed.

## Consequences

- The counter is visible while any control is edited, which was the point.
- One scrolling region instead of a scrolling page: the header and the actions never move.
- The style groups are still laid out for a wide page; the follow-up compacts them into a table so the
  editor fits a 1280×720 window without scrolling.
- Three pure helpers (default, clamp, stored value) are unit tested; the shell itself is verified by
  building and screenshotting the app.

## Amendment (2026-10-25): camera options under the preview, wider editor

Reviewing the built shell, the camera group made the editor column scroll and sat away from the
picture it changes. Changes:

- The camera options moved to a panel **under the preview**: resolution, card mirror and the
  driver are always there; the webcam and the image mirror appear with video; the long notes fold
  under "Camera notes". Statuses and errors stay visible.
- The stage is a size container capped at 48 % of the window height and never taller than its
  width allows, so the panel always fits below it at any window size.
- Without video the card always fills the frame, so its move/resize handle and the Fill frame /
  Back to card buttons are hidden; the turn handle stays.
- The editor column defaults to 40 % (30–60 %, at least 380 px), stored under a new key so the old
  narrow width does not stick. Its rows are dense: meeting fields in two rows, one row per
  participant, a Show / Font / Size / Colour table for the six texts, the background in one row and
  its sliders in another. Below 440 px of column width the rows fold (container queries).
- The desktop window opens at 1280×720 (minimum 960×600) and the workspace only stacks below
  900 px.

Measured at 1280×720: the page never scrolls and the editor overflows by its last 10 px of padding;
at 1536×816 it fits entirely.
