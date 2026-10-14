# Template and widget settings under their preview — design

Date: 2026-10-04 · Task: t-488

## Goal

In templates mode, the host settings (t-423 phase B) are shown in the
agent dock, far from what they set: "Ce modèle" (This template) (name,
description, grid…) and "Paramètres de <widget>" (Parameters of <widget>) (trial values).
We place them **under their preview**:

- on the Templates sub-tab, "Ce modèle" under the slide;
- on a widget sub-tab, "Paramètres de <widget>" under the widget's
  preview, in the same area (they scroll with it); the code keeps its
  position (below or on the right, t-474) and its size.

The agent dock keeps only the agent's surfaces (questions).

Out of scope: the Scene / Board view and on-slide editing (t-497);
the content of the surfaces, built by the session, unchanged.

## Decisions

1. **Same surfaces, different place**: `useHostSettings` is unchanged
   (reading, action routing, re-read after a refusal). `App` no longer
   adds them to the dock (`dockWithHost` goes away).
2. **A `HostSettings` component** (`ui/src/HostSettings.tsx`) renders a
   host surface with `SurfaceView`, in a `.host-settings` container
   that carries `data-surface-id` (the e2e and the tests
   still target `[data-surface-id="host-…"]`). No announcement
   (`aria-live="off"`), as in the dock.
3. **Templates**: `.templates-panel` becomes a column; the stage takes the
   space, the settings below, at most 40 % of the height, scrolling
   on their own beyond that.
4. **Widget**: `WidgetPane` passes a `settings` node to the active
   sub-tab; `WidgetTab` places it in `.widget-tab-side`, between the preview and
   the diagnostics. Bounded width (`min(100%, 560px)`), centred under
   the preview. The preview keeps at least 96 px of height; beyond that, the area
   scrolls. With the code below, the preview's default share goes from 1/4
   (t-474) to 2/5: it now carries the settings; a height set
   with the splitter still takes priority.
5. **Style**: resting card, no floating glass (see t-486): background
   `--paper-100`, medium radius, object shadow.

## Tests

- `HostSettings.render.test.tsx`: `data-surface-id`, action routed to
  `onAction` with the surface.
- `WidgetTab.render.test.tsx`: the `settings` node is in
  `.widget-tab-side`, after the preview; absent from an inactive tab.
- e2e `panel.cjs`: "Ce modèle" is in `.templates-panel`, not in
  `.notes`; `widgets.cjs`: "Paramètres de" in `.widget-tab-side`,
  not in `.notes`.
