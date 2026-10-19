# "Theme" shortcut Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Typing or dictating "thème" opens a host-built A2UI theme picker in the agent dock, with no agent turn.

**Architecture:** `Stack::handle(Prompt)` (oku-slides-agent) matches the prompt against the
localized words of `ui-shortcut-theme`. On a match, it asks the tab's `SurfaceHub` to show
`host-theme-picker` instead of sending an ACP prompt.
- **Session:** builds the cards: theme id, name, swatches and a bounded thumbnail, which is the
  current slide rendered in that theme.
- **Hub:** caches the cards, applies a pick through `Session::pick_theme`, and refreshes the active
  card from `prune`.
- **UI:** mirrors the matcher so the draft can send the word when Send is otherwise disabled.
  `ThemeChoice` previews its thumbnail on the stage through a store that only the stage overlay
  reads.

**Tech Stack:** Rust (snafu, speculoos, rstest, unicode-normalization), Fluent, React 19/TS (zod,
Bun tests, happy-dom), Playwright e2e.

**Spec:** `docs/superpowers/specs/2026-10-19-raccourci-theme-design.md` · Task: t-1071 (also closes t-1074)

## Global Constraints

- Commits: `type(scope) : description (t-1071)` (space before the colon), in French, no
  `Co-Authored-By` line. Write the message to a file in the scratchpad and run `git commit -F`.
- Code comments, test comments and log messages in French; docs in English; comments of the
  English `.ftl` file in English.
- No `unwrap`/`expect` outside tests; snafu errors with backtraces. A snafu selector
  (`.context(XSnafu)`) yields the crate's private inner error: return it through `?` (or
  `.map_err(Into::into)`), never as a tail expression of a `crate::Result`.
- No silent fallback that hides a bug: log what is dropped.
- Never hold a `std::sync::Mutex` guard across an await, nor the hub `state()` lock while calling
  `session.with`. All Typst work goes through the render worker.
- Document every pub and pub(crate) item and module; the first sentence must have **fewer than 15
  words** (count them).
- Files: aim for 500 lines, never 1000. New code goes in new files where the plan says so.
- Rust unit tests in a sibling `x_tests.rs` (`#[cfg(test)] #[path = "x_tests.rs"] mod x_tests;`),
  speculoos, names `unit_scenario_result`, one behaviour per test, rstest with named cases.
  Assert localized text through its Fluent key (`localizer.tr`), never as a literal.
- Dependency versions only in the root `[workspace.dependencies]`; crates use `.workspace = true`.
- Change `ComponentKind` (Rust) and the UI catalog in the same commit. The catalog id stays
  `urn:oku-slides:a2ui:catalog:v3`, as when `Notice` was added (t-966).
- Use `oku_slides_core::tokens::effective_tokens` for effective tokens, and
  `LiveLocalizer::current()` for hub texts (crates/AGENTS.md).
- Every visible UI string goes through `t()` / Fluent with literal keys. English is the reference.
  The 20 locales: `ar bn de en es fr hi it ja ko nl pl pt ru th tr uk vi zh-Hans zh-Hant`.
- Show SVG only in an `<img>` (data URL).
- UI tests: `*.render.test.tsx` run in their own Bun process. Run them separately from the plain
  tests, as `ui/package.json` does.
- Each `ARCHITECTURE.md` ≤ 50 lines; agent, mcp and session are **already at 50**. The edits below
  replace lines; check with `wc -l`. A rule change updates the nearest `AGENTS.md` in the same commit.
- Gate before merge: `just ci`. Per task: its targeted commands, then `just check`.

---

## File map

| File | Task | Responsibility |
|---|---|---|
| `crates/oku-slides-a2ui/src/message.rs` (+ `_tests`), `lib.rs`, `ARCHITECTURE.md` | 1 | `ComponentKind::ThemeChoice` |
| `crates/oku-slides-session/src/agent/render.rs`, `render_tests.rs`, `AGENTS.md` | 1 | agent `render` refuses it |
| `ui/src/a2ui/{svgSrc.ts, stagePreview.ts (+test), ThemeChoice.tsx (+render test), catalog.tsx, catalog.test.tsx, a2ui.css}` | 1 | UI component, preview store |
| `ui/src/agent/quick.ts` (+ test) | 1 | number `ThemeChoice` cards |
| `crates/oku-slides-session/src/deck_theme.rs`, `agent/variants.rs` | 2 | shared `render_preview` |
| `crates/oku-slides-session/src/theme_picker.rs` (+ `_tests`), `lib.rs`, `ARCHITECTURE.md` | 2 | cards, surface, `pick_theme` |
| `crates/oku-slides-session/src/agent/forms_tests.rs`, `ui/src/a2ui/fixtures/agent-surfaces.json` | 2 | UI contract |
| `crates/oku-slides-i18n/src/locales/*/agent.ftl` | 2 | `agent-surface-themes` |
| `crates/oku-slides-mcp/src/{hub.rs, hub/actions.rs, hub/themes.rs}`, `tests/themes.rs`, `ARCHITECTURE.md` | 3 | hub |
| `crates/oku-slides-i18n/src/locales/*/ui.ftl` | 4 | `ui-shortcut-theme` |
| `Cargo.toml`, `crates/oku-slides-agent/{Cargo.toml, src/shortcut.rs, src/stack.rs, tests/stack.rs, ARCHITECTURE.md, AGENTS.md}` | 4 | matcher + interception |
| `ui/src/agent/{shortcut.ts (+test), useQuickKeys.ts (+test), surfaces.ts, AgentPanel.tsx, AgentDock.render.test.tsx}`, `ui/src/DictationDraft.tsx` (+ render test) | 5 | UI matcher, send gate, keys, dock |
| `ui/src/{Stage.tsx, StagePreview.render.test.tsx, App.tsx, styles.css}` | 5 | stage overlay |
| `ui/e2e/lib/agentLog.cjs`, `ui/e2e/220-import.cjs`, `ui/e2e/230-raccourci-theme.cjs`, `docs/aide/{fr,en}.json` | 6 | e2e, help |

---

### Task 1: `ThemeChoice` catalog component and the stage preview store

**Interfaces:**
- Produces (Rust): `ComponentKind::ThemeChoice { svg: Option<String>, name: String, slide: String, swatches: Vec<String>, active: bool, action: Option<Action> }`;
  JSON `{"component":"ThemeChoice","svg"?,"name","slide","swatches","active","action"?}`.
- Produces (TS):
  - `svgSrc(svg: string): string` in `ui/src/a2ui/svgSrc.ts`.
  - In `ui/src/a2ui/stagePreview.ts`: `StagePreview { slide; url }`,
    `StagePreviewStore { get; set; subscribe }`, `createStagePreviewStore()`,
    `StagePreviewContext`, `useStagePreview(store)`.
  - `ThemeChoice` and `ThemeChoiceApi` in `ui/src/a2ui/ThemeChoice.tsx`.

- [ ] **Step 1: Failing Rust tests**

Append to `crates/oku-slides-a2ui/src/message_tests.rs` (add any missing import among `Action`,
`Component`, `ComponentKind`, `Dynamic`, `json`):

```rust
fn theme_choice() -> Component {
    Component::new(
        "c0",
        ComponentKind::ThemeChoice {
            svg: Some("<svg/>".into()),
            name: "Papier".into(),
            slide: "s-1".into(),
            swatches: vec!["#ffffff".into(), "#2f5fb3".into()],
            active: true,
            action: Some(Action::event("theme", [("theme", Dynamic::Literal(json!("id-1")))])),
        },
    )
}

#[test]
fn theme_choice_serializes_with_its_catalog_shape() {
    let v = serde_json::to_value(theme_choice()).unwrap();
    assert_that!(v).is_equal_to(json!({
        "id": "c0", "component": "ThemeChoice", "svg": "<svg/>", "name": "Papier", "slide": "s-1",
        "swatches": ["#ffffff", "#2f5fb3"], "active": true,
        "action": { "event": { "name": "theme", "context": { "theme": "id-1" } } }
    }));
}

#[test]
fn theme_choice_is_reserved_to_the_host() {
    assert_that!(theme_choice().agent_allowed()).is_false();
}

#[test]
fn theme_choice_without_action_keeps_everything_else() {
    let plain = theme_choice().kind.without_action().unwrap();
    assert_that!(plain).is_equal_to(ComponentKind::ThemeChoice {
        svg: Some("<svg/>".into()),
        name: "Papier".into(),
        slide: "s-1".into(),
        swatches: vec!["#ffffff".into(), "#2f5fb3".into()],
        active: true,
        action: None,
    });
}
```

Another test in that file shows how `Action::event` serializes. If the context shape differs,
adjust the expected JSON to it, not the code.

Add to the rstest table of `crates/oku-slides-session/src/agent/render_tests.rs`, after `case::notice`:

```rust
#[case::theme_choice(json!([{"id": "root", "component": "ThemeChoice", "name": "x", "slide": "s-1", "swatches": [], "active": false}]), "ThemeChoice is reserved")]
```

- [ ] **Step 2: Run them, expect a compile failure**

Run: `cargo nextest run -p oku-slides-a2ui theme_choice`
Expected: error, no variant `ThemeChoice`.

- [ ] **Step 3: Implement the variant**

`crates/oku-slides-a2ui/src/message.rs`, after `SlideThumb`:

```rust
    /// Un thème offert par l'hôte : miniature, pastilles, nom.
    ///
    /// `svg` absent : rendu borné, en échec ou deck vide ; la carte reste choisissable.
    /// `slide` : la slide que montre la miniature (l'aperçu de la scène ne vaut que pour elle).
    ThemeChoice {
        #[serde(skip_serializing_if = "Option::is_none")]
        svg: Option<String>,
        name: String,
        slide: String,
        /// Couleurs hexadécimales lues dans les jetons du thème.
        swatches: Vec<String>,
        /// Le thème du deck.
        active: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
```

- `children`: add `| Self::ThemeChoice { .. }` to the empty list.
- `actions`: add `| Self::ThemeChoice { action, .. }` to the `action.iter().collect()` arm.
- `without_action`: add

```rust
            Self::ThemeChoice { svg, name, slide, swatches, active, .. } => {
                Self::ThemeChoice { svg, name, slide, swatches, active, action: None }
            }
```

- `agent_allowed`: add `| ComponentKind::ThemeChoice { .. }`, and list `ThemeChoice` in its doc
  line and in the doc comment at line 82.
- `lib.rs`: add `ThemeChoice` to the crate doc list.
- `crates/oku-slides-session/src/agent/render.rs:99`: add `| ComponentKind::ThemeChoice { .. }`
  to the refused arm.
- `crates/oku-slides-a2ui/ARCHITECTURE.md` (35 lines): add `ThemeChoice` to the
  `Component { id, kind }` list and to the host-reserved list of `agent_allowed`.
- `crates/oku-slides-session/AGENTS.md`: set the rule to
  `- Keep host-only components (Prompt, Timeline, Receipt, SlideThumb, Notice, ThemeChoice) refused in the agent render tool. (spec:2026-10-03-agent-a2ui-design, t-966, t-1071)`.

- [ ] **Step 4: Rust tests pass**

Run: `cargo nextest run -p oku-slides-a2ui && cargo nextest run -p oku-slides-session render && just agents-lint`
Expected: PASS.

- [ ] **Step 5: Failing UI tests**

`ui/src/a2ui/stagePreview.test.ts`:

```ts
// Le magasin de l'aperçu de thème sur la scène.

import { describe, expect, test } from "bun:test";
import { createStagePreviewStore } from "./stagePreview";

describe("createStagePreviewStore", () => {
  test("set puis get rend la valeur posée", () => {
    const store = createStagePreviewStore();
    store.set({ slide: "s-1", url: "data:a" });
    expect(store.get()).toEqual({ slide: "s-1", url: "data:a" });
  });

  test("un changement prévient les abonnés", () => {
    const store = createStagePreviewStore();
    let calls = 0;
    store.subscribe(() => calls++);
    store.set({ slide: "s-1", url: "data:a" });
    store.set(null);
    expect(calls).toBe(2);
  });

  test("la même valeur ne prévient personne", () => {
    const store = createStagePreviewStore();
    let calls = 0;
    store.set({ slide: "s-1", url: "data:a" });
    store.subscribe(() => calls++);
    store.set({ slide: "s-1", url: "data:a" });
    expect(calls).toBe(0);
  });

  test("un abonné désabonné n'est plus prévenu", () => {
    const store = createStagePreviewStore();
    let calls = 0;
    const off = store.subscribe(() => calls++);
    off();
    store.set({ slide: "s-1", url: "data:a" });
    expect(calls).toBe(0);
  });
});
```

`ui/src/a2ui/ThemeChoice.render.test.tsx`:

```tsx
// Rendu et survol d'une carte de thème du sélecteur de l'hôte, dans un DOM happy-dom.

import { describe, expect, test } from "bun:test";
import type { ReactNode } from "react";
import "../test/dom";

// Imports dynamiques après `../test/dom` : react-dom cherche le DOM à son chargement.
const { act, createElement, StrictMode, useState } = await import("react");
const { mountSurface, messagesOf } = await import("../test/surface");
const { StagePreviewContext, createStagePreviewStore } = await import("./stagePreview");
const { svgSrc } = await import("./svgSrc");

const card = {
  component: "ThemeChoice",
  svg: "<svg><script>alert(1)</script></svg>",
  name: "Papier",
  slide: "s-1",
  swatches: ["#ffffff", "#2f5fb3"],
  active: true,
  action: { event: { name: "theme", context: { theme: "id-1" } } },
};
const other = { ...card, svg: "<svg>pro</svg>", name: "Pro", active: false, action: { event: { name: "theme", context: { theme: "id-2" } } } };

type Store = ReturnType<typeof createStagePreviewStore>;

/** Monte des composants avec un magasin d'aperçu ; `around` enveloppe la surface (StrictMode…). */
async function mountWith(components: Record<string, unknown>[], around: (n: ReactNode) => ReactNode = (n) => n) {
  const store: Store = createStagePreviewStore();
  const mounted = await mountSurface(messagesOf(components, {}), (s) =>
    createElement(StagePreviewContext.Provider, { value: store }, around(s)),
  );
  const buttons = [...mounted.host.querySelectorAll("button.theme-choice")] as HTMLButtonElement[];
  return { ...mounted, store, buttons };
}
const mount = (root: Record<string, unknown> = card) => mountWith([{ id: "root", ...root }]);
const two = () =>
  mountWith([{ id: "root", component: "Row", children: ["a", "b"] }, { id: "a", ...card }, { id: "b", ...other }]);

/** Survol au sens de React 19 : `pointerover`/`pointerout` délégués, pas `pointerenter`. */
const hover = (el: Element) =>
  act(async () => el.dispatchEvent(new PointerEvent("pointerover", { bubbles: true, relatedTarget: document.body })));
const leave = (el: Element) =>
  act(async () => el.dispatchEvent(new PointerEvent("pointerout", { bubbles: true, relatedTarget: document.body })));

describe("ThemeChoice", () => {
  test("la miniature est une image, jamais un SVG en ligne", async () => {
    const { host, buttons } = await mount();
    expect(buttons[0].querySelector("img")?.getAttribute("src")).toBe(svgSrc(card.svg));
    expect(host.querySelector("script, svg, object, iframe")).toBeNull();
  });

  test("le thème actif est marqué aria-pressed", async () => {
    const { buttons } = await mount();
    expect(buttons[0].getAttribute("aria-pressed")).toBe("true");
  });

  test("les pastilles suivent l'ordre reçu", async () => {
    const { host } = await mount();
    const colours = [...host.querySelectorAll(".theme-choice-swatches .a2ui-swatch")].map((e) => (e as HTMLElement).style.background);
    expect(colours).toEqual(["#ffffff", "#2f5fb3"]);
  });

  test("le nom est isolé dans un bdi", async () => {
    const { buttons } = await mount();
    expect(buttons[0].querySelector("bdi.caption")?.textContent).toBe("Papier");
  });

  test("un clic envoie theme avec l'id du thème", async () => {
    const { buttons, actions } = await mount();
    await act(async () => buttons[0].click());
    expect(actions.map((a) => [a.name, a.context])).toEqual([["theme", { theme: "id-1" }]]);
  });

  test("le survol pose l'aperçu de sa slide", async () => {
    const { buttons, store } = await mount();
    await hover(buttons[0]);
    expect(store.get()).toEqual({ slide: "s-1", url: svgSrc(card.svg) });
  });

  test("quitter la carte retire son aperçu", async () => {
    const { buttons, store } = await mount();
    await hover(buttons[0]);
    await leave(buttons[0]);
    expect(store.get()).toBeNull();
  });

  test("le focus pose l'aperçu", async () => {
    const { buttons, store } = await mount();
    await act(async () => buttons[0].focus());
    expect(store.get()?.url).toBe(svgSrc(card.svg));
  });

  test("la perte du focus retire l'aperçu", async () => {
    const { buttons, store } = await mount();
    await act(async () => buttons[0].focus());
    await act(async () => buttons[0].blur());
    expect(store.get()).toBeNull();
  });

  test("quitter une carte ne retire pas l'aperçu qu'une autre a posé depuis", async () => {
    const { buttons, store } = await two();
    await hover(buttons[1]);
    await act(async () => buttons[0].focus());
    await leave(buttons[1]);
    expect(store.get()?.url).toBe(svgSrc(card.svg));
  });

  test("démontée pendant le survol, la carte retire son aperçu", async () => {
    let hideSurface = () => {};
    function Shown({ child }: { child: ReactNode }) {
      const [on, setOn] = useState(true);
      hideSurface = () => setOn(false);
      return on ? child : null;
    }
    const { buttons, store } = await mountWith([{ id: "root", ...card }], (n) => createElement(Shown, { child: n }));
    await hover(buttons[0]);
    await act(async () => hideSurface());
    expect(store.get()).toBeNull();
  });

  test("sans miniature, aucun aperçu", async () => {
    const { svg: _svg, ...noSvg } = card;
    const { buttons, store } = await mount(noSvg);
    await hover(buttons[0]);
    expect(store.get()).toBeNull();
  });

  test("sous StrictMode, le survol pose encore l'aperçu", async () => {
    const { buttons, store } = await mountWith([{ id: "root", ...card }], (n) => createElement(StrictMode, null, n));
    await hover(buttons[0]);
    expect(store.get()?.slide).toBe("s-1");
  });

  test("une pastille qui n'est pas une couleur fait refuser la surface", async () => {
    const { errors, buttons } = await mount({ ...card, swatches: ["red"] });
    expect([errors.length > 0, buttons.length]).toEqual([true, 0]);
  });
});
```

In `ui/src/a2ui/catalog.test.tsx`, add `"ThemeChoice"` to the `V3` list (after `"SlideThumb"`).

In `ui/src/agent/quick.test.ts`, after the `variants` constant:

```ts
const themes = surface([
  { id: "c0", component: "ThemeChoice", name: "Papier", slide: "s-1", swatches: [], active: true, action: { event: { name: "theme", context: { theme: "id-1" } } } },
  { id: "c1", component: "ThemeChoice", name: "Pro", slide: "s-1", swatches: [], active: false, action: { event: { name: "theme", context: { theme: "id-2" } } } },
  { id: "close", component: "Button", label: "Fermer", action: { event: { name: "dismiss", context: {} } } },
]);
```

and in the `describe`:

```ts
  test("un chiffre choisit le thème correspondant", () => {
    expect(quickAction(themes, "2")).toEqual({ name: "theme", context: { theme: "id-2" } });
  });

  test("le bouton Fermer n'est pas numéroté", () => {
    expect(quickAction(themes, "3")).toBeNull();
  });
```

- [ ] **Step 6: Run them, expect failures**

Run, as two commands:
- `cd ui && bun test src/a2ui/stagePreview.test.ts src/a2ui/catalog.test.tsx src/agent/quick.test.ts`
- `cd ui && bun test src/a2ui/ThemeChoice.render.test.tsx`

Expected: FAIL (missing modules or unknown component).

- [ ] **Step 7: Implement the UI side**

`ui/src/a2ui/svgSrc.ts`:

```ts
// URL d'image d'un SVG rendu par la session : dans un <img>, il n'exécute ni script ni lien.

/** Le SVG en URL `data:` pour un `<img>`. */
export const svgSrc = (svg: string): string => `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
```

`ui/src/a2ui/stagePreview.ts`:

```ts
// Aperçu d'un thème sur la scène : le sélecteur de thème de l'hôte y pose la miniature
// survolée ; seule la couche d'aperçu de la scène s'y abonne, l'onglet ne se re-rend pas.

import { createContext, useSyncExternalStore } from "react";

/** La miniature d'un thème et la slide qu'elle montre. */
export interface StagePreview {
  slide: string;
  url: string;
}

/** Une valeur d'aperçu et ses abonnés. */
export interface StagePreviewStore {
  get(): StagePreview | null;
  set(preview: StagePreview | null): void;
  subscribe(listener: () => void): () => void;
}

/** Un magasin d'aperçu vide. */
export function createStagePreviewStore(): StagePreviewStore {
  let value: StagePreview | null = null;
  const listeners = new Set<() => void>();
  return {
    get: () => value,
    set(preview) {
      if (preview?.slide === value?.slide && preview?.url === value?.url) return;
      value = preview;
      for (const listener of listeners) listener();
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}

/** Le magasin de la fenêtre ; absent (fenêtre de l'agent) : pas d'aperçu. */
export const StagePreviewContext = createContext<StagePreviewStore | null>(null);

const noSubscribe = () => () => {};
const none = () => null;

/** L'aperçu courant de `store` ; `null` sans magasin. */
export function useStagePreview(store: StagePreviewStore | null): StagePreview | null {
  return useSyncExternalStore(store ? store.subscribe : noSubscribe, store ? store.get : none);
}
```

`ui/src/a2ui/ThemeChoice.tsx`:

```tsx
// Carte d'un thème du sélecteur de l'hôte : la slide rendue dans ce thème (image), ses pastilles,
// son nom. Survol ou focus : aperçu sur la scène ; clic : l'action `theme`.

import { useContext, useEffect, useMemo } from "react";
import { z } from "zod";
import { createComponentImplementation } from "@a2ui/react/v0_9";
import { CommonSchemas } from "@a2ui/web_core/v0_9";
import { Hex } from "./schemas";
import { type StagePreviewStore, StagePreviewContext } from "./stagePreview";
import { svgSrc } from "./svgSrc";

/** Taille maximale de la miniature (miroir de `THUMB_BYTES`, oku-slides-session). */
const SVG_MAX = 512 * 1024;

/** Schéma de `ThemeChoice` : l'hôte envoie toujours des littéraux, jamais de liaison. */
export const ThemeChoiceApi = {
  name: "ThemeChoice",
  schema: z
    .object({
      svg: z.string().max(SVG_MAX).optional(),
      name: z.string().max(200),
      slide: z.string(),
      swatches: z.array(Hex).readonly(),
      active: z.boolean(),
      action: CommonSchemas.Action.optional(),
    })
    .strict(),
};

/** Retire l'aperçu de `store` seulement s'il montre encore cette miniature. */
function clearOwn(store: StagePreviewStore | null, slide: string, url: string | null) {
  const shown = store?.get();
  if (store && url && shown?.url === url && shown.slide === slide) store.set(null);
}

/** Une carte de thème : survol ou focus posent l'aperçu, que seule elle retire. */
export const ThemeChoice = createComponentImplementation(ThemeChoiceApi, ({ props }) => {
  const store = useContext(StagePreviewContext);
  const src = useMemo(() => (props.svg ? svgSrc(props.svg) : null), [props.svg]);
  const show = () => {
    if (src && store) store.set({ slide: props.slide, url: src });
  };
  const hide = () => clearOwn(store, props.slide, src);
  // Démontée pendant le survol (surface remplacée) : son aperçu ne reste pas sur la scène.
  useEffect(() => () => clearOwn(store, props.slide, src), [store, props.slide, src]);
  return (
    <button
      type="button"
      className="theme-choice"
      aria-pressed={props.active}
      onClick={() => props.action?.()}
      onPointerEnter={show}
      onPointerLeave={hide}
      onFocus={show}
      onBlur={hide}
    >
      {src ? <img src={src} alt="" draggable={false} /> : <span className="theme-choice-blank" aria-hidden="true" />}
      <span className="theme-choice-swatches" aria-hidden="true">
        {props.swatches.map((c, i) => (
          <span key={i} className="a2ui-swatch" style={{ background: c }} />
        ))}
      </span>
      <bdi className="caption">{props.name}</bdi>
    </button>
  );
});
```

`ui/src/a2ui/catalog.tsx`:
- import `svgSrc` from `./svgSrc` and `ThemeChoice` from `./ThemeChoice`;
- in `SlideThumb`, replace the inline data URL with `svgSrc(props.svg)`;
- add `ThemeChoice` after `SlideThumb` in the `Catalog` list and in the header comment's list.

`ui/src/a2ui/a2ui.css`, after `.a2ui-swatch`. `Row` wraps each child in
`<div data-component-id>` (catalog.tsx:201-207), so the grid rule targets that wrapper:

```css
/* Carte de thème : deux par ligne au dock ; au survol seule l'ombre répond (t-1069), l'actif a un contour. */
.a2ui-row > [data-component-id]:has(> .theme-choice) { flex: 0 0 calc(50% - var(--space-1)); min-width: 0; }
.theme-choice { display: flex; flex-direction: column; gap: var(--space-1); width: 100%; border: 0; background: none; cursor: pointer; padding: 0; text-align: start; }
.theme-choice img, .theme-choice-blank { display: block; width: 100%; aspect-ratio: 16 / 9; border-radius: var(--radius-sm); background: var(--paper-200); box-shadow: 0 1px 3px rgb(0 0 0 / .2); transition: box-shadow .15s; }
.theme-choice:hover :is(img, .theme-choice-blank), .theme-choice:focus-visible :is(img, .theme-choice-blank) { box-shadow: 0 3px 10px rgb(0 0 0 / .3); }
.theme-choice[aria-pressed="true"] :is(img, .theme-choice-blank) { outline: 2px solid var(--accent-fill); outline-offset: 1px; }
.theme-choice-swatches { display: flex; gap: 2px; }
.theme-choice-swatches .a2ui-swatch { width: 16px; height: 12px; }
```

`ui/src/agent/quick.ts`, `numbered`: replace
`(c.component === "Button" || c.component === "SlideThumb")` with
`(c.component === "Button" || c.component === "SlideThumb" || c.component === "ThemeChoice")`, and
mention theme cards in its doc comment.

- [ ] **Step 8: UI tests and type check pass**

Run, as two commands, then the type check:
- `cd ui && bun test src/a2ui/stagePreview.test.ts src/a2ui/catalog.test.tsx src/agent/quick.test.ts`
- `cd ui && bun test src/a2ui/ThemeChoice.render.test.tsx src/a2ui/catalog.render.test.tsx`
- `cd ui && bunx tsc -b`

Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add crates/oku-slides-a2ui crates/oku-slides-session/src/agent/render.rs crates/oku-slides-session/src/agent/render_tests.rs crates/oku-slides-session/AGENTS.md ui/src/a2ui ui/src/agent/quick.ts ui/src/agent/quick.test.ts
git commit -F <scratchpad>/msg.txt   # feat(a2ui) : composant ThemeChoice réservé à l'hôte et magasin d'aperçu de la scène (t-1071)
```

---

### Task 2: Session builds the theme cards, the surface, and applies a pick

**Interfaces:**
- Consumes: `ComponentKind::ThemeChoice` (Task 1).
- Produces, in `oku-slides-session`:
  - In `deck_theme.rs`: `pub(crate) fn render_preview(&mut self, set: &PackSet, deck: &Deck, tokens: Option<&Map<String, Value>>) -> Result<RemoteCompiled>`.
  - In `theme_picker.rs`, re-exported from `lib.rs` unless marked crate-private:
    - `pub const HOST_THEME_PICKER: &str = "host-theme-picker";`
    - `pub const THEME_EVENT: &str = "theme";`
    - `pub(crate) const THEME_KEY: &str = "theme";`
    - `pub(crate) const THEMES_RENDERED: usize = 12;`
    - `pub(crate) const RENDER_BUDGET: Duration = Duration::from_secs(4);`
    - `pub(crate) const THUMB_BYTES: usize = 512 * 1024;`
    - `pub(crate) struct Bounds { budget: Duration, thumb_bytes: usize }` with `Bounds::DEFAULT`.
    - `pub struct ThemeCard { pub id: Uuid, pub name: String, pub slide: String, pub swatches: Vec<String>, pub svg: Option<Arc<str>> }` (`Debug, Clone, PartialEq`).
    - `pub struct ThemeChoices { pub cards: Vec<ThemeCard>, pub active: Uuid }` (`Debug, Clone, PartialEq`).
    - `pub fn theme_picker(cards: &[ThemeCard], active: Uuid, l: &Localizer) -> Surface`.
    - `pub fn picked_theme_id(context: &Map<String, Value>) -> Option<Uuid>`.
  - On `Session`:
    - `pub fn theme_choices(&mut self, slide: Option<&str>) -> Result<Option<ThemeChoices>>`
    - `pub(crate) fn theme_choices_within(&mut self, slide: Option<&str>, bounds: Bounds) -> Result<Option<ThemeChoices>>`
    - `pub fn active_theme(&self) -> Uuid`
    - `pub fn pick_theme(&mut self, id: Uuid) -> Result<()>`
    - `pub fn in_deck_mode(&self) -> bool`

- [ ] **Step 1: Extract the shared preview render (refactor, tests stay green)**

In `crates/oku-slides-session/src/deck_theme.rs`, inside `impl Session`:

```rust
    /// Rend `deck` avec `tokens` dans l'emplacement des aperçus.
    ///
    /// Variantes et thèmes ; l'emplacement est ouvert au premier appel ; `set` : les packs utilisables.
    pub(crate) fn render_preview(
        &mut self,
        set: &PackSet,
        deck: &Deck,
        tokens: Option<&Map<String, Value>>,
    ) -> Result<RemoteCompiled> {
        let renderer = match &mut self.variant_renderer {
            Some(r) => r,
            None => self.variant_renderer.insert(new_renderer(
                &self.worker,
                self.render_fonts.clone(),
                &self.localizer,
                set,
                &self.registry,
            )?),
        };
        Ok(render_themed(renderer, deck, tokens, &self.doc.sources.assets).context(RenderSnafu)?)
    }
```

Imports to add to `deck_theme.rs`: `oku_slides_render::PackSet`, `crate::error::RenderSnafu` and
`crate::new_renderer`. `ResultExt` is already imported.

In `agent/variants.rs`, `render_variants` becomes:

```rust
    fn render_variants(&mut self, spec: &ShowVariants) -> Result<Vec<Arc<str>>> {
        let set = self.usable_packs()?;
        // Après `usable_packs` : un pack écarté n'est plus au registre qui valide les variantes.
        let decks = self.variant_decks(spec)?;
        let tokens = self.theme_tokens(self.doc.sources.deck());
        let mut svgs: Vec<Arc<str>> = Vec::with_capacity(decks.len());
        for (i, deck) in decks.iter().enumerate() {
            // Le thème du deck : une variante qui en changerait garde celui-ci à l'aperçu.
            let compiled = self.render_preview(&set, deck, tokens.as_ref())?;
            let page = compiled.layout().page_of(&spec.slide).context(AgentFormSnafu {
                reason: AgentFormReason::SlideRemoved { at: format!("variants[{i}]"), slide: spec.slide.clone() },
            })?;
            svgs.push(compiled.svg(page - 1).context(RenderSnafu)?.into());
        }
        Ok(svgs)
    }
```

Remove the imports of `variants.rs` that become unused (clippy names them).

Run: `cargo nextest run -p oku-slides-session variants && cargo nextest run -p oku-slides-session --test agent`
Expected: PASS (behaviour unchanged).

- [ ] **Step 2: Failing tests**

`crates/oku-slides-session/src/theme_picker_tests.rs`:

```rust
use std::time::Duration;

use oku_slides_a2ui::{Action, ComponentKind, ServerBody};
use oku_slides_core::PackScope;
use oku_slides_i18n::Localizer;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use super::{Bounds, HOST_THEME_PICKER, THEMES_RENDERED, ThemeCard, picked_theme_id, swatches, theme_picker};
use crate::test_support::session;
use crate::{DEFAULT_THEME, PAPIER_THEME, PRO_THEME, Session};

/// Les composants `ThemeChoice` d'une surface, dans l'ordre.
fn choices(s: &crate::agent::Surface) -> Vec<ComponentKind> {
    components(s).into_iter().filter(|k| matches!(k, ComponentKind::ThemeChoice { .. })).collect()
}

fn components(s: &crate::agent::Surface) -> Vec<ComponentKind> {
    s.messages
        .iter()
        .filter_map(|m| match &m.body {
            ServerBody::UpdateComponents(u) => Some(u.components.clone()),
            _ => None,
        })
        .flatten()
        .map(|c| c.kind)
        .collect()
}

fn card(id: u128, name: &str) -> ThemeCard {
    ThemeCard { id: Uuid::from_u128(id), name: name.into(), slide: "s-1".into(), swatches: vec![], svg: None }
}

fn fr() -> Localizer {
    Localizer::new(Some("fr"))
}

fn colour(name: &str) -> String {
    crate::theme_schema::default_tokens()["color"][name].as_str().unwrap().to_owned()
}

fn chart0() -> String {
    crate::theme_schema::default_tokens()["chart"]["palette"][0].as_str().unwrap().to_owned()
}

/// Embarque un thème du deck seul nommé `name`, aux jetons de Pro recouverts par `tokens` ; rend son id.
fn embed_deck_only(s: &mut Session, name: &str, tokens: Value) -> Uuid {
    let pro = crate::theme::Themes::builtins().iter().find(|b| b.manifest.id == PRO_THEME).unwrap();
    let mut all = pro.manifest.tokens.clone();
    oku_slides_core::tokens::merge(&mut all, tokens.as_object().unwrap());
    let manifest = crate::ThemeManifest {
        id: Uuid::new_v4(),
        name: name.into(),
        scope: PackScope::Deck,
        tokens: all,
        ..pro.manifest.clone()
    };
    let id = manifest.id;
    s.doc.sources.add_theme(manifest.to_files().unwrap());
    id
}

const WIDE: Bounds = Bounds { budget: Duration::from_secs(600), thumb_bytes: super::THUMB_BYTES };

#[test]
fn swatches_read_four_colours_then_the_first_chart_colour() {
    let tokens = json!({ "color": { "accent": "#c4622d" }, "chart": { "palette": ["#112233"] } });
    assert_that!(swatches(tokens.as_object().unwrap())).is_equal_to(vec![
        colour("bg"),
        colour("surface"),
        colour("ink"),
        "#c4622d".to_owned(),
        "#112233".to_owned(),
    ]);
}

#[test]
fn swatches_resolve_a_colour_that_names_another() {
    let tokens = json!({ "color": { "accent": "ink", "ink": "#101010" } });
    assert_that!(swatches(tokens.as_object().unwrap())[3].as_str()).is_equal_to("#101010");
}

#[test]
fn swatches_skip_an_unknown_colour_name() {
    let tokens = json!({ "color": { "accent": "nowhere" } });
    assert_that!(swatches(tokens.as_object().unwrap())).is_equal_to(vec![
        colour("bg"),
        colour("surface"),
        colour("ink"),
        chart0(),
    ]);
}

#[test]
fn theme_picker_wires_each_choice_to_its_theme_id() {
    let s = theme_picker(&[card(1, "Papier"), card(2, "Pro")], Uuid::from_u128(2), &fr());
    let ids: Vec<Option<Uuid>> = choices(&s)
        .into_iter()
        .map(|k| match k {
            ComponentKind::ThemeChoice { action: Some(a), .. } => {
                let ctx = serde_json::to_value(&a).unwrap()["event"]["context"].clone();
                picked_theme_id(ctx.as_object().unwrap())
            }
            _ => None,
        })
        .collect();
    assert_that!(ids).is_equal_to(vec![Some(Uuid::from_u128(1)), Some(Uuid::from_u128(2))]);
}

#[test]
fn theme_picker_marks_only_the_active_card() {
    let s = theme_picker(&[card(1, "Papier"), card(2, "Pro")], Uuid::from_u128(2), &fr());
    let active: Vec<bool> =
        choices(&s).into_iter().map(|k| matches!(k, ComponentKind::ThemeChoice { active: true, .. })).collect();
    assert_that!(active).is_equal_to(vec![false, true]);
}

#[test]
fn theme_picker_uses_the_host_surface_id() {
    let s = theme_picker(&[card(1, "Papier")], Uuid::from_u128(1), &fr());
    assert_that!(s.id.as_str()).is_equal_to(HOST_THEME_PICKER);
}

#[test]
fn theme_picker_is_not_a_question() {
    let s = theme_picker(&[card(1, "Papier")], Uuid::from_u128(1), &fr());
    assert_that!(s.question).is_false();
}

#[test]
fn theme_picker_has_a_dismiss_button() {
    let s = theme_picker(&[card(1, "Papier")], Uuid::from_u128(1), &fr());
    let dismiss = components(&s).into_iter().any(|k| match k {
        ComponentKind::Button { action, .. } => action == Action::named("dismiss"),
        _ => false,
    });
    assert_that!(dismiss).is_true();
}

#[test]
fn picked_theme_id_refuses_a_malformed_id() {
    let ctx = json!({ "theme": "pas-un-uuid" });
    assert_that!(picked_theme_id(ctx.as_object().unwrap())).is_none();
}

#[test]
fn theme_choices_list_library_then_deck_only_themes() {
    let mut s = session();
    let only = embed_deck_only(&mut s, "Zéphyr", json!({}));
    let ids: Vec<Uuid> = s.theme_choices_within(None, WIDE).unwrap().unwrap().cards.iter().map(|c| c.id).collect();
    let mut expected: Vec<Uuid> = s.theme_list.iter().map(|t| t.id).collect();
    expected.push(only);
    assert_that!(ids).is_equal_to(expected);
}

#[test]
fn theme_choices_without_deck_theme_marks_the_default_active() {
    let mut s = session();
    assert_that!(s.theme_choices_within(None, WIDE).unwrap().unwrap().active).is_equal_to(DEFAULT_THEME);
}

#[test]
fn theme_choices_after_set_theme_marks_it_active() {
    let mut s = session();
    s.set_theme(PRO_THEME).unwrap();
    assert_that!(s.theme_choices_within(None, WIDE).unwrap().unwrap().active).is_equal_to(PRO_THEME);
}

#[test]
fn theme_choices_render_the_requested_slide() {
    let mut s = session();
    let card = s.theme_choices_within(Some("s-libre"), WIDE).unwrap().unwrap().cards.remove(0);
    assert_that!((card.slide.as_str(), card.svg.is_some())).is_equal_to(("s-libre", true));
}

#[test]
fn theme_choices_with_an_unknown_slide_show_the_first() {
    let mut s = session();
    let first = s.deck().slides[0].id.clone();
    let card = s.theme_choices_within(Some("s-nope"), WIDE).unwrap().unwrap().cards.remove(0);
    assert_that!((card.slide, card.svg.is_some())).is_equal_to((first, true));
}

#[test]
fn theme_choices_give_each_theme_its_own_thumbnail() {
    let mut s = session();
    let cards = s.theme_choices_within(None, WIDE).unwrap().unwrap().cards;
    let svg = |id| cards.iter().find(|c| c.id == id).and_then(|c| c.svg.clone());
    let (papier, pro) = (svg(PAPIER_THEME), svg(PRO_THEME));
    assert_that!(papier.is_some() && pro.is_some() && papier != pro).is_true();
}

#[test]
fn theme_choices_render_at_most_themes_rendered_thumbnails() {
    let mut s = session();
    for i in 0..THEMES_RENDERED {
        embed_deck_only(&mut s, &format!("Thème {i:02}"), json!({}));
    }
    let cards = s.theme_choices_within(None, WIDE).unwrap().unwrap().cards;
    assert_that!(cards.iter().filter(|c| c.svg.is_some()).count()).is_equal_to(THEMES_RENDERED);
}

#[test]
fn theme_choices_past_the_budget_still_render_the_active() {
    let mut s = session();
    let last = embed_deck_only(&mut s, "Zzz", json!({}));
    s.set_theme(last).unwrap();
    let none = Bounds { budget: Duration::ZERO, thumb_bytes: super::THUMB_BYTES };
    let rendered: Vec<Uuid> = s
        .theme_choices_within(None, none)
        .unwrap()
        .unwrap()
        .cards
        .into_iter()
        .filter(|c| c.svg.is_some())
        .map(|c| c.id)
        .collect();
    assert_that!(rendered).is_equal_to(vec![last]);
}

#[test]
fn theme_choices_drop_an_oversized_thumbnail() {
    let mut s = session();
    let tiny = Bounds { budget: Duration::from_secs(600), thumb_bytes: 1 };
    let cards = s.theme_choices_within(None, tiny).unwrap().unwrap().cards;
    assert_that!(cards.iter().all(|c| c.svg.is_none()) && !cards.is_empty()).is_true();
}

#[test]
fn theme_choices_keep_a_theme_that_fails_to_render_without_thumbnail() {
    let mut s = session();
    let broken = embed_deck_only(&mut s, "Cassé", json!({ "color": { "bg": 12 } }));
    let cards = s.theme_choices_within(None, WIDE).unwrap().unwrap().cards;
    let card = cards.iter().find(|c| c.id == broken).unwrap();
    let others_rendered = cards.iter().filter(|c| c.id != broken).all(|c| c.svg.is_some());
    assert_that!((card.svg.is_none(), others_rendered)).is_equal_to((true, true));
}

#[test]
fn theme_choices_on_an_empty_deck_keep_every_card_without_thumbnail() {
    let mut s = session();
    let ops = s.deck().slides.iter().map(|x| oku_slides_core::Op::RemoveSlide { slide: x.id.clone() }).collect();
    s.apply("vider", ops, None).unwrap();
    let cards = s.theme_choices_within(None, WIDE).unwrap().unwrap().cards;
    assert_that!((cards.len(), cards.iter().all(|c| c.svg.is_none()))).is_equal_to((s.theme_list.len(), true));
}

#[test]
fn theme_choices_outside_deck_mode_are_none() {
    let mut s = session();
    s.edit_templates(crate::pack::DEFAULT_PACK).unwrap();
    assert_that!(s.theme_choices(None).unwrap()).is_none();
}

#[test]
fn theme_choices_leave_the_deck_unchanged() {
    let mut s = session();
    let before = (s.deck().clone(), s.snapshot().undo_label);
    s.theme_choices_within(None, WIDE).unwrap();
    assert_that!((s.deck().clone(), s.snapshot().undo_label)).is_equal_to(before);
}

#[test]
fn pick_theme_sets_the_deck_theme() {
    let mut s = session();
    s.pick_theme(PRO_THEME).unwrap();
    assert_that!(s.deck().theme.as_ref().map(|r| r.id)).is_equal_to(Some(PRO_THEME));
}

#[test]
fn pick_theme_is_one_undo_step() {
    let mut s = session();
    let before = s.deck().theme.clone();
    s.pick_theme(PRO_THEME).unwrap();
    s.undo().unwrap();
    assert_that!(s.deck().theme.clone()).is_equal_to(before);
}

#[test]
fn pick_theme_notes_the_quoted_name_for_the_agent() {
    let mut s = session();
    s.pick_theme(PRO_THEME).unwrap();
    let note = format!("theme picked: \"Pro\" ({PRO_THEME})");
    assert_that!(s.agent_context(&Default::default()).contains(&note)).is_true();
}

#[test]
fn pick_theme_of_an_unknown_id_is_refused() {
    let mut s = session();
    assert_that!(s.pick_theme(Uuid::from_u128(7))).is_err();
}

#[test]
fn pick_theme_in_templates_mode_is_refused() {
    let mut s = session();
    s.edit_templates(crate::pack::DEFAULT_PACK).unwrap();
    assert_that!(s.pick_theme(PRO_THEME)).is_err();
}
```

Check these before running, and adapt the test, not the code:
- **Failing render.** `{"color": {"bg": 12}}` must make the Typst render fail; check it once with
  `render_preview`. If it renders, use another token type error the Typst theme reads (for
  example `{"font": {"size": "x"}}`).
- **`Action` comparison.** `Action` must implement `PartialEq` for the `theme_picker_has_a_dismiss_button`
  comparison. If it does not, compare `serde_json::to_value(action)["event"]["name"]` with `"dismiss"`.
- **`Themes::builtins()`.** Its path must match `test_support.rs:299`.

`Map` and `Value` are used by `embed_deck_only`; drop any import that clippy flags as unused.

- [ ] **Step 3: Run them, expect a compile failure**

Run: `cargo nextest run -p oku-slides-session theme_picker`
Expected: error, module `theme_picker` not found.

- [ ] **Step 4: Implement**

`crates/oku-slides-session/src/theme_picker.rs`:

```rust
//! Sélecteur de thème de l'hôte : cartes, surface et choix.
//!
//! Ouvert par le raccourci « thème » de l'agent, sans tour. Chaque carte porte la
//! slide montrée rendue dans son thème (clone d'une seule slide : numéro de page
//! et sommaire peuvent différer du deck entier), ses pastilles et son nom. Les
//! rendus sont bornés (`Bounds`) : l'actif d'abord, puis au plus
//! `THEMES_RENDERED` miniatures dans `RENDER_BUDGET`, chacune sous `THUMB_BYTES` ;
//! au-delà, en échec ou sur un deck vide, une carte reste sans miniature. Le
//! verrou de session (celui de l'espace de travail dans l'app) est tenu pendant
//! les rendus. Le hub garde les cartes : un choix ne rend rien.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use oku_slides_a2ui::{Action, Component, ComponentKind, TextVariant};
use oku_slides_core::tokens::effective_tokens;
use oku_slides_core::{Deck, is_hex_color};
use oku_slides_i18n::Localizer;
use oku_slides_render::PackSet;
use serde_json::{Map, Value, json};
use snafu::ResultExt;
use uuid::Uuid;

use crate::agent::clip;
use crate::agent::forms::{Place, Surface, lit, surface};
use crate::error::RenderSnafu;
use crate::theme::DEFAULT_THEME;
use crate::theme_schema::{default_tokens, lookup};
use crate::{Mode, Result, Session};

/// Surface du sélecteur de thème de l'hôte.
pub const HOST_THEME_PICKER: &str = "host-theme-picker";
/// Évènement d'un choix de thème.
pub const THEME_EVENT: &str = "theme";
/// Clé du contexte d'un choix : l'id du thème.
pub(crate) const THEME_KEY: &str = "theme";
/// Miniatures rendues au plus, l'actif compris.
pub(crate) const THEMES_RENDERED: usize = 12;
/// Temps de rendu des miniatures autres que l'actif.
pub(crate) const RENDER_BUDGET: Duration = Duration::from_secs(4);
/// Taille maximale d'une miniature (SVG) ; miroir de `SVG_MAX` dans l'interface.
pub(crate) const THUMB_BYTES: usize = 512 * 1024;
/// Longueur maximale d'un nom de thème montré.
const NAME_CHARS: usize = 80;

/// Jetons montrés en pastilles, dans l'ordre ; puis la première couleur de `chart.palette`.
const SWATCH_NAMES: [&str; 4] = ["bg", "surface", "ink", "accent"];

/// Les bornes des miniatures : temps total et taille de chacune.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Bounds {
    pub(crate) budget: Duration,
    pub(crate) thumb_bytes: usize,
}

impl Bounds {
    /// Les bornes de l'application.
    pub(crate) const DEFAULT: Self = Self { budget: RENDER_BUDGET, thumb_bytes: THUMB_BYTES };
}

/// Un thème offert par le sélecteur.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeCard {
    /// L'id du thème.
    pub id: Uuid,
    /// Son nom, coupé à 80 caractères.
    pub name: String,
    /// La slide que montre la miniature.
    pub slide: String,
    /// Couleurs hexadécimales de ses jetons.
    pub swatches: Vec<String>,
    /// La slide rendue dans ce thème ; absente au-delà des bornes ou en échec.
    pub svg: Option<Arc<str>>,
}

/// Les cartes du sélecteur et le thème actif du deck.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeChoices {
    /// Bibliothèque d'abord, puis thèmes du deck seul.
    pub cards: Vec<ThemeCard>,
    /// Le thème du deck, ou celui par défaut.
    pub active: Uuid,
}

/// La couleur hexadécimale de `value`, ou celle de la couleur qu'elle nomme.
fn colour(all: &Map<String, Value>, value: &Value) -> Option<String> {
    let s = value.as_str()?;
    let hex = if s.starts_with('#') { Some(s) } else { lookup(all, &["color".to_owned(), s.to_owned()])?.as_str() };
    hex.filter(|h| is_hex_color(h)).map(str::to_owned)
}

/// Les pastilles des jetons propres `tokens`, posés sur les défauts.
pub(crate) fn swatches(tokens: &Map<String, Value>) -> Vec<String> {
    let all = effective_tokens(default_tokens(), tokens);
    let named = SWATCH_NAMES.iter().filter_map(|n| lookup(&all, &["color".to_owned(), (*n).to_owned()]));
    let chart = lookup(&all, &["chart".to_owned(), "palette".to_owned()]).and_then(|v| v.get(0));
    named.chain(chart).filter_map(|v| colour(&all, v)).collect()
}

/// La surface : un titre, un `ThemeChoice` par carte, Fermer.
pub fn theme_picker(cards: &[ThemeCard], active: Uuid, l: &Localizer) -> Surface {
    let ids: Vec<String> = (0..cards.len()).map(|i| format!("c{i}")).collect();
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let mut components = vec![
        Component::column("root", &["title", "row", "close"]),
        Component::text("title", l.tr("agent-surface-themes"), Some(TextVariant::H4)),
        Component::row("row", &refs),
        Component::button("close", l.tr("agent-surface-close"), false, Action::named("dismiss")),
    ];
    for (cid, card) in ids.iter().zip(cards) {
        components.push(Component::new(
            cid,
            ComponentKind::ThemeChoice {
                svg: card.svg.as_deref().map(str::to_owned),
                name: card.name.clone(),
                slide: card.slide.clone(),
                swatches: card.swatches.clone(),
                active: card.id == active,
                action: Some(Action::event(THEME_EVENT, [(THEME_KEY, lit(json!(card.id.to_string())))])),
            },
        ));
    }
    surface(HOST_THEME_PICKER, Place::Thread, None, components, json!({}))
}

/// L'id du thème d'un choix ; `None` s'il manque ou n'est pas un uuid.
pub fn picked_theme_id(context: &Map<String, Value>) -> Option<Uuid> {
    context.get(THEME_KEY).and_then(Value::as_str).and_then(|s| Uuid::parse_str(s).ok())
}

impl Session {
    /// Vrai en mode deck : ni modèles, ni historique, ni aide.
    pub fn in_deck_mode(&self) -> bool {
        matches!(self.mode, Mode::Deck)
    }

    /// Le thème avec lequel le deck se rend.
    pub fn active_theme(&self) -> Uuid {
        self.deck_theme().map_or(DEFAULT_THEME, |r| r.id)
    }

    /// Les cartes du sélecteur sur `slide` ; `None` hors du mode deck.
    ///
    /// `slide` absente ou inconnue : la première. Rend dans le worker, verrou de session tenu.
    pub fn theme_choices(&mut self, slide: Option<&str>) -> Result<Option<ThemeChoices>> {
        self.theme_choices_within(slide, Bounds::DEFAULT)
    }

    /// [`theme_choices`](Self::theme_choices) avec ces bornes.
    pub(crate) fn theme_choices_within(&mut self, slide: Option<&str>, bounds: Bounds) -> Result<Option<ThemeChoices>> {
        if !self.in_deck_mode() {
            return Ok(None);
        }
        let active = self.active_theme();
        let shown = self.one_slide_deck(slide);
        let shown_id = shown.as_ref().and_then(|d| d.slides.first()).map(|s| s.id.clone()).unwrap_or_default();
        let offered: Vec<(Uuid, String)> = self
            .theme_list
            .iter()
            .map(|t| (t.id, t.name.clone()))
            .chain(self.deck_only_themes().into_iter().map(|t| (t.id, t.name)))
            .collect();
        let tokens: HashMap<Uuid, Option<Map<String, Value>>> =
            offered.iter().map(|(id, _)| (*id, self.offered_tokens(*id))).collect();
        let svgs = match &shown {
            Some(deck) => self.thumbnails(deck, active, &offered, &tokens, bounds)?,
            None => HashMap::new(),
        };
        let cards = offered
            .into_iter()
            .map(|(id, name)| ThemeCard {
                id,
                name: clip(&name, NAME_CHARS),
                slide: shown_id.clone(),
                swatches: tokens.get(&id).and_then(Option::as_ref).map(swatches).unwrap_or_default(),
                svg: svgs.get(&id).cloned(),
            })
            .collect();
        Ok(Some(ThemeChoices { cards, active }))
    }

    /// Applique le thème choisi au deck et le note pour l'agent.
    ///
    /// Refusé hors du mode deck ou pour un thème inconnu ; une étape d'annulation.
    pub fn pick_theme(&mut self, id: Uuid) -> Result<()> {
        self.ensure_deck_mode()?;
        let name = self.theme_info(id)?.name;
        self.set_theme(id)?;
        self.note(format!("theme picked: {:?} ({id})", clip(&name, NAME_CHARS)));
        Ok(())
    }

    /// Les miniatures : l'actif d'abord, puis les autres dans l'ordre, dans les bornes.
    fn thumbnails(
        &mut self,
        deck: &Deck,
        active: Uuid,
        offered: &[(Uuid, String)],
        tokens: &HashMap<Uuid, Option<Map<String, Value>>>,
        bounds: Bounds,
    ) -> Result<HashMap<Uuid, Arc<str>>> {
        let set = self.usable_packs()?;
        let mut order: Vec<Uuid> = vec![active];
        order.extend(offered.iter().map(|(id, _)| *id).filter(|id| *id != active));
        order.truncate(THEMES_RENDERED);
        let deadline = Instant::now() + bounds.budget;
        let mut svgs = HashMap::new();
        for id in order {
            if id != active && Instant::now() >= deadline {
                break;
            }
            let Some(Some(theme)) = tokens.get(&id) else { continue };
            if let Some(svg) = self.thumbnail(&set, deck, theme, id, bounds.thumb_bytes) {
                svgs.insert(id, svg);
            }
        }
        Ok(svgs)
    }

    /// Un clone du deck réduit à `slide` (sinon à la première) ; `None` si le deck est vide.
    fn one_slide_deck(&self, slide: Option<&str>) -> Option<Deck> {
        let deck = self.deck();
        let keep = slide.and_then(|id| deck.slide(id)).or_else(|| deck.slides.first())?.clone();
        Some(Deck {
            version: deck.version,
            theme: deck.theme.clone(),
            params: deck.params.clone(),
            vars: deck.vars.clone(),
            tokens: deck.tokens.clone(),
            widgets: deck.widgets.clone(),
            slides: vec![keep],
        })
    }

    /// Les jetons propres du thème `id` dans la version que prendrait `set_theme`.
    ///
    /// `None` (journalisé) si sa version a disparu entre la liste et la lecture.
    fn offered_tokens(&self, id: Uuid) -> Option<Map<String, Value>> {
        self.theme_tokens_of(id)
            .inspect_err(|e| tracing::warn!(theme = %id, error = %e.one_line(), "jetons du thème illisibles"))
            .ok()
    }

    /// La miniature du thème `id`, sous `max` octets ; `None` (journalisé) sinon.
    fn thumbnail(&mut self, set: &PackSet, deck: &Deck, tokens: &Map<String, Value>, id: Uuid, max: usize) -> Option<Arc<str>> {
        match self.thumbnail_svg(set, deck, tokens) {
            Ok(Some(svg)) if svg.len() <= max => Some(svg.into()),
            Ok(Some(svg)) => {
                tracing::warn!(theme = %id, bytes = svg.len(), "miniature du thème trop lourde : écartée");
                None
            }
            Ok(None) => {
                tracing::warn!(theme = %id, "miniature du thème : slide absente du rendu");
                None
            }
            Err(e) => {
                tracing::warn!(theme = %id, error = %e.one_line(), "miniature du thème impossible");
                None
            }
        }
    }

    /// Le SVG de l'unique slide de `deck` rendue avec `tokens` ; `None` si elle n'a pas de page.
    fn thumbnail_svg(&mut self, set: &PackSet, deck: &Deck, tokens: &Map<String, Value>) -> Result<Option<String>> {
        let Some(slide) = deck.slides.first().map(|s| s.id.clone()) else { return Ok(None) };
        let compiled = self.render_preview(set, deck, Some(tokens))?;
        let Some(page) = compiled.layout().page_of(&slide) else { return Ok(None) };
        Ok(Some(compiled.svg(page - 1).context(RenderSnafu)?))
    }
}

#[cfg(test)]
#[path = "theme_picker_tests.rs"]
mod theme_picker_tests;
```

Notes for the implementer:
- `theme_tokens_of` (`deck_theme_edit.rs:115`) and `theme_info` (`:99`) are public and return `Result`.
- `clip` is re-exported in `agent/mod.rs`.
- `mode` is a crate-visible field of `Session`.

Add `agent-surface-themes` to the 20 `crates/oku-slides-i18n/src/locales/<locale>/agent.ftl` files,
next to `agent-surface-close`:
- `en` `Themes`, `fr` `Thèmes`, `ar` `السمات`, `bn` `থিম`, `de` `Themen`, `es` `Temas`, `hi` `थीम`
- `it` `Temi`, `ja` `テーマ`, `ko` `테마`, `nl` `Thema's`, `pl` `Motywy`, `pt` `Temas`, `ru` `Темы`
- `th` `ธีม`, `tr` `Temalar`, `uk` `Теми`, `vi` `Chủ đề`, `zh-Hans` `主题`, `zh-Hant` `主題`

`lib.rs`: `mod theme_picker;` and
`pub use theme_picker::{HOST_THEME_PICKER, THEME_EVENT, ThemeCard, ThemeChoices, picked_theme_id, theme_picker};`.

`crates/oku-slides-session/ARCHITECTURE.md`: replace the three "Theme editing" lines (18–20) with
these three lines (the file stays at 50):

```
- Theme editing: `theme_edit.rs` (templates-mode draft, `save_theme_edit`), `deck_theme_edit.rs` (`edit_theme` in deck mode),
  `theme_schema.rs` (controls, bounds), `theme_panel.rs` + `theme_controls.rs` (`host-theme`), `theme_picker.rs` (`host-theme-picker`:
  bounded thumbnails, swatches, `pick_theme`), `appearance.rs`; `deck_themes.rs`: deck-only themes (`Snapshot.deck_themes`), after the library.
```

- [ ] **Step 5: Tests pass**

Run: `cargo nextest run -p oku-slides-session theme_picker && cargo nextest run -p oku-slides-i18n && wc -l crates/oku-slides-session/ARCHITECTURE.md`
Expected: PASS, `50`.

- [ ] **Step 6: Contract fixture**

In `every_form()` (`crates/oku-slides-session/src/agent/forms_tests.rs`), before the final
`pack_notice_banner(...)` entry:

```rust
        crate::theme_picker(
            &[
                crate::ThemeCard {
                    id: uuid::Uuid::from_u128(1),
                    name: "Papier".into(),
                    slide: "s-4".into(),
                    swatches: vec!["#ffffff".into(), "#2f5fb3".into()],
                    svg: Some("<svg xmlns=\"http://www.w3.org/2000/svg\"/>".into()),
                },
                crate::ThemeCard {
                    id: uuid::Uuid::from_u128(2),
                    name: "Pro".into(),
                    slide: "s-4".into(),
                    swatches: vec!["#101418".into()],
                    svg: None,
                },
            ],
            uuid::Uuid::from_u128(1),
            &l,
        ),
```

Run: `OKU_UPDATE_FIXTURES=1 cargo nextest run -p oku-slides-session every_form_matches_the_ui_fixture`
then `cargo nextest run -p oku-slides-session forms && cd ui && bun test src/a2ui/catalog.test.tsx`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/oku-slides-session crates/oku-slides-i18n/src/locales ui/src/a2ui/fixtures/agent-surfaces.json
git commit -F <scratchpad>/msg.txt   # feat(session) : cartes du sélecteur de thème, miniatures bornées et choix noté pour l'agent (t-1071)
```

---

### Task 3: Hub shows the picker, applies a pick, refreshes it

**Interfaces:**
- Consumes (Task 2): `Session::{theme_choices, pick_theme, active_theme, in_deck_mode}`,
  `theme_picker`, `picked_theme_id`, `ThemeCard`, `HOST_THEME_PICKER`, `THEME_EVENT`.
- Produces: `impl SurfaceHub { pub fn show_themes(&self, slide: Option<String>) -> Result<bool, HubError> }`.
  It returns `Ok(false)` outside deck mode and `Ok(true)` once the box is shown. It renders under
  the session lock, so call it off the runtime threads.

- [ ] **Step 1: Failing tests**

Create `crates/oku-slides-mcp/tests/themes.rs`:

```rust
//! Sélecteur de thème de l'hôte : montré, choisi, rafraîchi par `prune`.

mod common;

use common::{action, next_surface, start, wait_event};
use oku_slides_a2ui::{ComponentKind, ServerBody};
use oku_slides_mcp::{HubErrorKind, McpEvent};
use oku_slides_session::agent::Surface;
use oku_slides_session::{HOST_THEME_PICKER, PRO_THEME};
use serde_json::json;
use speculoos::prelude::*;

/// Les `ThemeChoice` d'une surface : (nom, actif, a une miniature).
fn cards(s: &Surface) -> Vec<(String, bool, bool)> {
    s.messages
        .iter()
        .filter_map(|m| match &m.body {
            ServerBody::UpdateComponents(u) => Some(u.components.clone()),
            _ => None,
        })
        .flatten()
        .filter_map(|c| match c.kind {
            ComponentKind::ThemeChoice { name, active, svg, .. } => Some((name, active, svg.is_some())),
            _ => None,
        })
        .collect()
}

fn active_names(s: &Surface) -> Vec<String> {
    cards(s).into_iter().filter(|(_, active, _)| *active).map(|(n, ..)| n).collect()
}

fn show(h: &common::Harness) -> bool {
    tokio::task::block_in_place(|| h.hub.show_themes(Some("s-libre".into()))).unwrap()
}

fn pick(h: &common::Harness, theme: &str) -> Result<(), oku_slides_mcp::HubError> {
    tokio::task::block_in_place(|| h.hub.act(action(HOST_THEME_PICKER, "theme", json!({ "theme": theme }))))
}

fn open(h: &common::Harness) -> bool {
    h.hub.surfaces().iter().any(|s| s.id == HOST_THEME_PICKER)
}

#[tokio::test(flavor = "multi_thread")]
async fn show_themes_puts_the_picker_live_outside_any_turn() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    assert_that!(show(&h)).is_true();
    let surface = next_surface(&mut rx).await;
    assert_that!((surface.id.as_str(), surface.live, surface.turn)).is_equal_to((HOST_THEME_PICKER, true, None));
}

#[tokio::test(flavor = "multi_thread")]
async fn show_themes_twice_replaces_the_picker_without_closing_it() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    show(&h);
    next_surface(&mut rx).await;
    show(&h);
    let event = wait_event(&mut rx, |e| match e {
        McpEvent::CloseSurface { id } => Some(format!("close {id}")),
        McpEvent::Surface { surface } => Some(format!("show {}", surface.id)),
        _ => None,
    })
    .await;
    assert_that!(event).is_equal_to(format!("show {HOST_THEME_PICKER}"));
}

#[tokio::test(flavor = "multi_thread")]
async fn show_themes_outside_deck_mode_shows_nothing() {
    let h = start().await;
    h.session.lock().unwrap().edit_templates(oku_slides_session::pack::DEFAULT_PACK).unwrap();
    assert_that!((show(&h), open(&h))).is_equal_to((false, false));
}

#[tokio::test(flavor = "multi_thread")]
async fn picking_a_theme_sets_the_deck_theme() {
    let h = start().await;
    show(&h);
    pick(&h, &PRO_THEME.to_string()).unwrap();
    assert_that!(h.session.lock().unwrap().deck().theme.as_ref().map(|r| r.id)).is_equal_to(Some(PRO_THEME));
}

#[tokio::test(flavor = "multi_thread")]
async fn picking_a_theme_is_labelled_as_a_theme_change() {
    let h = start().await;
    show(&h);
    pick(&h, &PRO_THEME.to_string()).unwrap();
    let label = h.localizer.tr("session-undo-theme");
    assert_that!(h.session.lock().unwrap().snapshot().undo_label).is_equal_to(Some(label));
}

#[tokio::test(flavor = "multi_thread")]
async fn picking_a_theme_emits_changed_then_the_picker_with_it_active() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    show(&h);
    next_surface(&mut rx).await;
    pick(&h, &PRO_THEME.to_string()).unwrap();
    wait_event(&mut rx, |e| matches!(e, McpEvent::Changed).then_some(())).await;
    let rebuilt = next_surface(&mut rx).await;
    assert_that!(active_names(&rebuilt)).is_equal_to(vec!["Pro".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn picking_a_theme_keeps_the_cached_thumbnails() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    show(&h);
    let first = next_surface(&mut rx).await;
    pick(&h, &PRO_THEME.to_string()).unwrap();
    let rebuilt = next_surface(&mut rx).await;
    let thumbs = |s: &Surface| cards(s).into_iter().map(|(n, _, svg)| (n, svg)).collect::<Vec<_>>();
    assert_that!(thumbs(&rebuilt)).is_equal_to(thumbs(&first));
}

#[tokio::test(flavor = "multi_thread")]
async fn picking_a_malformed_id_is_incomplete() {
    let h = start().await;
    show(&h);
    assert_that!(pick(&h, "pas-un-uuid").unwrap_err().kind()).is_equal_to(HubErrorKind::Incomplete);
}

#[tokio::test(flavor = "multi_thread")]
async fn picking_a_malformed_id_keeps_the_picker() {
    let h = start().await;
    show(&h);
    let _ = pick(&h, "pas-un-uuid");
    assert_that!(open(&h)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn picking_an_unknown_theme_is_a_session_error() {
    let h = start().await;
    show(&h);
    let refused = pick(&h, &uuid::Uuid::from_u128(7).to_string()).unwrap_err();
    assert_that!(refused.kind()).is_equal_to(HubErrorKind::Session);
}

#[tokio::test(flavor = "multi_thread")]
async fn dismiss_closes_the_picker() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    show(&h);
    h.hub.act(action(HOST_THEME_PICKER, "dismiss", json!({}))).unwrap();
    wait_event(&mut rx, |e| matches!(e, McpEvent::CloseSurface { id } if id == HOST_THEME_PICKER).then_some(())).await;
    assert_that!(open(&h)).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn prune_after_an_outside_theme_change_marks_it_active() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    show(&h);
    next_surface(&mut rx).await;
    h.session.lock().unwrap().set_theme(PRO_THEME).unwrap();
    tokio::task::block_in_place(|| h.hub.prune());
    let refreshed = next_surface(&mut rx).await;
    assert_that!(active_names(&refreshed)).is_equal_to(vec!["Pro".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn prune_in_templates_mode_closes_the_picker() {
    let h = start().await;
    show(&h);
    h.session.lock().unwrap().edit_templates(oku_slides_session::pack::DEFAULT_PACK).unwrap();
    tokio::task::block_in_place(|| h.hub.prune());
    assert_that!(open(&h)).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn prune_after_dismiss_does_not_reopen_the_picker() {
    let h = start().await;
    show(&h);
    h.hub.act(action(HOST_THEME_PICKER, "dismiss", json!({}))).unwrap();
    h.session.lock().unwrap().set_theme(PRO_THEME).unwrap();
    tokio::task::block_in_place(|| h.hub.prune());
    assert_that!(open(&h)).is_false();
}
```

The harness exports `localizer: Arc<Localizer>` (`common/mod.rs`). If `Harness` is not public in
`common`, name the type `start()` returns.

- [ ] **Step 2: Run them, expect a compile failure**

Run: `cargo nextest run -p oku-slides-mcp --test themes`
Expected: error, no method `show_themes`.

- [ ] **Step 3: Implement**

`hub.rs`:
- In `enum Shown`, add:

```rust
    /// Sélecteur de thème de l'hôte : ses cartes rendues et l'actif montré.
    Themes(ThemePicker),
```

- Next to `Shown`, add:

```rust
/// Le sélecteur de thème montré : un choix ou `prune` le refait sans rien rendre.
#[derive(Debug, Clone)]
struct ThemePicker {
    cards: Vec<oku_slides_session::ThemeCard>,
    active: uuid::Uuid,
}
```

  `oku-slides-mcp` already depends on `uuid`.
- In `show_alone`, do not close the surface being replaced by itself. Use
  `for id in old.into_iter().filter(|id| *id != surface.id)`.
  Variants use a new id each time, so they are unaffected.
- After `mod actions;`, add `mod themes;`.

`crates/oku-slides-mcp/src/hub/themes.rs`:

```rust
//! Sélecteur de thème de l'hôte : montré, choisi, rafraîchi.
//!
//! Le seul `show_*` que le hub construit lui-même : il le refait après un choix ou
//! un changement du deck (`prune`), à partir des cartes gardées, sans rien rendre.

use oku_slides_session::{HOST_THEME_PICKER, picked_theme_id, theme_picker};
use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt};

use super::error::{HubError, HubInner, IncompleteSnafu, SessionSnafu};
use super::{McpEvent, Shown, SurfaceHub, ThemePicker};

impl SurfaceHub {
    /// Montre le sélecteur de thème sur `slide` ; `false` hors du mode deck.
    ///
    /// Rend les miniatures sous le verrou de session : hors des fils du runtime.
    pub fn show_themes(&self, slide: Option<String>) -> Result<bool, HubError> {
        let choices = self
            .session
            .with(|s| s.theme_choices(slide.as_deref()))
            .and_then(|r| r)
            .context(SessionSnafu)?;
        let Some(choices) = choices else { return Ok(false) };
        let surface = theme_picker(&choices.cards, choices.active, &self.localizer.current());
        let picker = ThemePicker { cards: choices.cards, active: choices.active };
        self.show_alone(surface, Shown::Themes(picker), |s| matches!(s, Shown::Themes(_)));
        Ok(true)
    }

    /// Applique le thème choisi, puis refait le sélecteur avec lui actif.
    pub(super) fn apply_theme(&self, picker: &ThemePicker, context: &Map<String, Value>) -> Result<ThemePicker, HubInner> {
        let id = picked_theme_id(context).context(IncompleteSnafu { key: oku_slides_session::THEME_EVENT })?;
        self.session.with(|s| s.pick_theme(id)).and_then(|r| r).context(SessionSnafu)?;
        self.emit(McpEvent::Changed);
        let next = ThemePicker { cards: picker.cards.clone(), active: id };
        self.emit(McpEvent::Surface { surface: theme_picker(&next.cards, id, &self.localizer.current()) });
        Ok(next)
    }

    /// Ferme le sélecteur hors du mode deck ; le refait si l'actif a changé.
    ///
    /// Appelée par `prune`. Relit `state` après la session : un sélecteur fermé
    /// entre-temps n'est jamais rouvert.
    pub(super) fn refresh_themes(&self) {
        let shown = matches!(self.state().shows.get(HOST_THEME_PICKER), Some(Shown::Themes(_)));
        if !shown {
            return;
        }
        let Ok((deck_mode, active)) = self.session.with(|s| (s.in_deck_mode(), s.active_theme())) else { return };
        let mut state = self.state();
        let Some(Shown::Themes(picker)) = state.shows.get(HOST_THEME_PICKER).cloned() else { return };
        if !deck_mode {
            state.shows.remove(HOST_THEME_PICKER);
            self.record_and_send(&mut state, McpEvent::CloseSurface { id: HOST_THEME_PICKER.to_owned() });
            return;
        }
        if active == picker.active {
            return;
        }
        let surface = theme_picker(&picker.cards, active, &self.localizer.current());
        state.shows.insert(HOST_THEME_PICKER.to_owned(), Shown::Themes(ThemePicker { active, ..picker }));
        self.record_and_send(&mut state, McpEvent::Surface { surface });
    }
}
```

Notes:
- `Shown` needs `Clone` for `.cloned()`. If deriving it on `Shown` is too much, clone the
  `ThemePicker` inside a `match` instead.
- If clippy flags `HubError` as unused, drop it from the `use`: `.context(SessionSnafu)?`
  converts through `From<HubInner>`.

`hub/actions.rs`:
- Add a variant to `After`:

```rust
    /// La surface reste, avec cet état (sélecteur de thème refait).
    Replace(Shown),
```

- In `act`'s match, before `(_, "dismiss")`:

```rust
            (Shown::Themes(picker), name) if name == oku_slides_session::THEME_EVENT => {
                self.apply_theme(picker, &action.context).map(|p| After::Replace(Shown::Themes(p)))
            }
```

- In the match on `result`, add:

```rust
            Ok(After::Replace(next)) => {
                self.state().shows.insert(action.surface, next);
                Ok(())
            }
```

- In `prune`:
  - First line: `self.refresh_themes();`.
  - In the slide filter, the exhaustive match: `Shown::Static { .. } | Shown::Themes(_) => return false,`.
  - Leave the early-return guard on `Params | Variants` as it is.
  - Update `prune`'s doc: it also refreshes the theme picker.

`crates/oku-slides-mcp/ARCHITECTURE.md`: replace the four `hub` lines (23–26) with these four:

```
- `hub` : `SurfaceHub`, open surfaces and events to the UI (`McpEvent`); `emit` alone changes
  the open list; `resync` resends it whole. `hub/asks` (questions, confirmations, permissions),
  `hub/actions` (`SurfaceAction`, `act`, `prune`), `hub/themes` (`show_themes`: the only box the
  hub builds itself; cached cards, `theme` picks, `prune` refreshes), `hub/error` (`HubError`, `HubErrorKind`).
```

- [ ] **Step 4: Tests pass**

Run: `cargo nextest run -p oku-slides-mcp && wc -l crates/oku-slides-mcp/ARCHITECTURE.md`
Expected: PASS, `50`.

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-mcp
git commit -F <scratchpad>/msg.txt   # feat(mcp) : le hub montre le sélecteur de thème, applique un choix et le rafraîchit (t-1071)
```

---

### Task 4: Shortcut words, matcher and `Stack` interception

**Interfaces:**
- Consumes: `SurfaceHub::show_themes` (Task 3).
- Produces, in `crates/oku-slides-agent/src/shortcut.rs`:
  - `pub(crate) const THEME_SHORTCUT: &str = "ui-shortcut-theme";`
  - `pub(crate) fn normalize(text: &str) -> String`
  - `pub(crate) fn matches_shortcut(text: &str, key: &str, l: &Localizer) -> bool`
- Produces the Fluent key `ui-shortcut-theme` in `ui.ftl`. The Rust `Localizer` loads `ui.ftl`
  too (`oku-slides-i18n/src/lib.rs:44`), so this is the one place the words live.

- [ ] **Step 1: Dependency and words**

Root `Cargo.toml`, under the `unicode-properties` line:

```toml
# Repli des diacritiques (NFD) pour reconnaître les raccourcis de l'agent (« Thème. » = « theme »).
unicode-normalization = "0.1"
```

`crates/oku-slides-agent/Cargo.toml` `[dependencies]`: `unicode-normalization.workspace = true`.
`Cargo.lock` must only gain the crate in oku-slides-agent's dependency list; 0.1.25 is already
locked.

In each `crates/oku-slides-i18n/src/locales/<locale>/ui.ftl`, add `ui-shortcut-theme`.
- **Comments:** in `en`, `# Words that, alone in an agent prompt, open the theme picker; comma-separated, the English ones included.`
  In `fr`, `# Mots qui, seuls dans une consigne, ouvrent le sélecteur de thème ; séparés par des virgules, anglais compris.`
- **Values:** every list includes `theme, themes`:
  - `en` `theme, themes`
  - `fr` `thème, thèmes, theme, themes`
  - `ar` `سمة, السمة, ثيم, theme, themes`
  - `bn` `থিম, theme, themes`
  - `de` `Thema, Themen, Design, theme, themes`
  - `es` `tema, temas, theme, themes`
  - `hi` `थीम, theme, themes`
  - `it` `tema, temi, theme, themes`
  - `ja` `テーマ, theme, themes`
  - `ko` `테마, theme, themes`
  - `nl` `thema, thema's, theme, themes`
  - `pl` `motyw, motywy, theme, themes`
  - `pt` `tema, temas, theme, themes`
  - `ru` `тема, темы, theme, themes`
  - `th` `ธีม, theme, themes`
  - `tr` `tema, temalar, theme, themes`
  - `uk` `тема, теми, theme, themes`
  - `vi` `chủ đề, giao diện, theme, themes`
  - `zh-Hans` `主题, theme, themes`
  - `zh-Hant` `主題, theme, themes`

`ui/src/i18n-keys.test.ts` requires every `ui-` key to be used in UI code. Task 5 uses it, so
this test may fail until Task 5 lands. Run Task 4 and Task 5 back to back, or move this key's
introduction to Task 5's commit if `just check` runs in between.

- [ ] **Step 2: Failing tests**

`crates/oku-slides-agent/src/shortcut_tests.rs`:

```rust
use oku_slides_i18n::Localizer;
use rstest::rstest;
use speculoos::prelude::*;

use super::{THEME_SHORTCUT, matches_shortcut, normalize};

#[rstest]
#[case::accent_and_period("Thème.", "theme")]
#[case::caps_and_spaces("  THÈMES ! ", "themes")]
#[case::guillemets("«thème»", "theme")]
#[case::curly_quote("thema’s", "thema's")]
#[case::inner_spaces("chủ   đề", "chu đe")]
#[case::devanagari_danda("थीम।", "थीम")]
fn normalize_folds_case_accents_quotes_and_edges(#[case] text: &str, #[case] expected: &str) {
    assert_that!(normalize(text)).is_equal_to(expected.to_owned());
}

#[rstest]
#[case::french("fr", "Thème.", true)]
#[case::french_plural("fr", "thèmes", true)]
#[case::french_bare("fr", "THEME", true)]
#[case::sentence("fr", "change le thème en sombre", false)]
#[case::empty("fr", "", false)]
#[case::blank("fr", "   ", false)]
#[case::ellipsis("fr", "…", false)]
#[case::german("de", "Themen", true)]
#[case::japanese("ja", "テーマ。", true)]
#[case::hindi("hi", "थीम", true)]
#[case::hindi_other_word("hi", "थम", false)]
#[case::thai("th", "ธีม", true)]
#[case::dutch_curly_quote("nl", "thema’s", true)]
#[case::english_in_russian("ru", "Theme", true)]
fn matches_shortcut_accepts_only_the_whole_localized_word(
    #[case] locale: &str,
    #[case] text: &str,
    #[case] expected: bool,
) {
    assert_that!(matches_shortcut(text, THEME_SHORTCUT, &Localizer::new(Some(locale)))).is_equal_to(expected);
}

#[test]
fn matches_shortcut_over_long_prompt_never_matches() {
    let long = format!("theme{}", " ".repeat(100));
    assert_that!(matches_shortcut(&long, THEME_SHORTCUT, &Localizer::new(Some("en")))).is_false();
}
```

`đ` (U+0111) has no NFD decomposition, so `"chủ   đề"` gives `"chu đe"`. If the real output
differs only on that letter, fix the expected value, never the matcher.

Append to `crates/oku-slides-agent/tests/stack.rs` (add the `common` helpers used to its
`use common::{…}` list):

```rust
/// Relayé : une surface `host-theme-picker`.
fn is_picker(v: &(&'static str, serde_json::Value)) -> bool {
    v.0 == "mcp" && v.1["event"]["kind"] == "surface" && v.1["event"]["surface"]["id"] == "host-theme-picker"
}

/// Le `slide` de la première carte d'une surface relayée.
fn first_card_slide(v: &serde_json::Value) -> Option<String> {
    v["event"]["surface"]["messages"].as_array()?.iter().find_map(|m| {
        m["updateComponents"]["components"].as_array()?.iter().find_map(|c| {
            (c["component"] == "ThemeChoice").then(|| c["slide"].as_str().map(str::to_owned)).flatten()
        })
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_theme_shows_the_picker_on_the_slide_shown() {
    let (stack, rx, _log) = stack_with_sink_and_log(json!({ "turns": [{ "chunks": ["ok"] }] }), &["t-1"]).await;
    let prompt = json!({ "cmd": "prompt", "tab": "t-1", "text": "Thème.", "ui": { "slide": "s-libre" } });
    tokio::task::block_in_place(|| stack.handle(cmd(prompt))).unwrap();
    let shown = wait_for(&rx, is_picker).await;
    assert_that!(first_card_slide(&shown.1)).is_equal_to(Some("s-libre".to_owned()));
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_theme_sends_nothing_to_the_agent() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "chunks": ["ok"] }] }), &["t-1"]).await;
    tokio::task::block_in_place(|| stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "Thème." })))).unwrap();
    wait_for(&rx, is_picker).await;
    // Témoin : les commandes d'un onglet passent dans l'ordre, une consigne envoyée à tort serait la première.
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "salut" }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let prompts = common::logged(&log, "session/prompt");
    assert_that!((prompts.len(), prompts[0].to_string().contains("salut"), common::logged(&log, "session/cancel").len()))
        .is_equal_to((1, true, 0));
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_theme_answering_a_waiting_question_answers_it() {
    let ask = json!({ "tool": "ask_choice", "args": { "question": "Laquelle ?", "options": [
        { "id": "a", "label": "A" }, { "id": "b", "label": "B" } ] } });
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "mcp": [ask] }] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "demande-moi" }))).unwrap();
    let asked = wait_for(&rx, |(_, v)| is_tool_surface(v, "t-1", "ask-")).await;
    let id = asked.1["event"]["surface"]["id"].clone();
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "thème", "answer": id }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let outs: Vec<String> = common::logged(&log, "mcp/result").iter().map(|r| r["out"].to_string()).collect();
    assert_that!((outs.len(), outs.first().is_some_and(|o| o.contains("thème")))).is_equal_to((1, true));
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_theme_during_a_turn_leaves_the_turn_running() {
    let ask = json!({ "tool": "ask_choice", "args": { "question": "Laquelle ?", "options": [
        { "id": "a", "label": "A" }, { "id": "b", "label": "B" } ] } });
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "mcp": [ask] }] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "demande-moi" }))).unwrap();
    let asked = wait_for(&rx, |(_, v)| is_tool_surface(v, "t-1", "ask-")).await;
    let id = asked.1["event"]["surface"]["id"].as_str().unwrap().to_owned();
    tokio::task::block_in_place(|| stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "thème" })))).unwrap();
    wait_for(&rx, is_picker).await;
    // La question attend toujours : y répondre finit le tour, sans annulation.
    let answer = json!({ "cmd": "surface_action", "tab": "t-1", "surface": id, "name": "choose", "context": { "choice": "a" } });
    stack.handle(cmd(answer)).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let outs = common::logged(&log, "mcp/result");
    assert_that!((outs.len(), common::logged(&log, "session/cancel").len(), common::logged(&log, "session/prompt").len()))
        .is_equal_to((1, 0, 1));
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_theme_outside_deck_mode_goes_to_the_agent() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "chunks": ["ok"] }] }), &["t-1"]).await;
    stack.on_session("t-1", |s| s.edit_templates(oku_slides_session::pack::DEFAULT_PACK).unwrap());
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "thème" }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    assert_that!(common::logged(&log, "session/prompt")).has_length(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_containing_theme_in_a_sentence_goes_to_the_agent() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "chunks": ["ok"] }] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "change le thème" }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    assert_that!(common::logged(&log, "session/prompt")).has_length(1);
}
```

For the answer action in `prompt_theme_during_a_turn_leaves_the_turn_running`, copy how
`ask_choice` is answered elsewhere in `tests/stack.rs` or `tests/turns.rs`: the action name
(`choose`) and its context key (`choice`). Also check that the fake agent ends the turn once the
tool result arrives.

- [ ] **Step 3: Run them, expect failures**

Run: `cargo nextest run -p oku-slides-agent shortcut && cargo nextest run -p oku-slides-agent --test stack theme`
Expected: compile error (no module `shortcut`), then the stack tests fail.

- [ ] **Step 4: Implement**

`crates/oku-slides-agent/src/shortcut.rs`:

```rust
//! Raccourcis de l'agent : un mot seul ouvre une surface de l'hôte.
//!
//! Aucun tour, aucun jeton dépensé. Les mots d'un raccourci sont une clé Fluent de
//! `ui.ftl` (lue aussi par l'interface, `ui/src/agent/shortcut.ts`, même règle),
//! séparés par des virgules ; chaque langue y met aussi les mots anglais. La
//! consigne entière doit être l'un d'eux, une fois normalisée.

use oku_slides_i18n::Localizer;
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// Clé Fluent des mots qui ouvrent le sélecteur de thème.
pub(crate) const THEME_SHORTCUT: &str = "ui-shortcut-theme";

/// Au-delà, une consigne n'est jamais un raccourci (et n'est pas normalisée).
const SHORTCUT_BYTES: usize = 64;

/// Vrai pour un diacritique latin, grec ou cyrillique.
fn is_accent(c: char) -> bool {
    ('\u{0300}'..='\u{036F}').contains(&c)
}

/// `text` en minuscules, sans accents, espaces resserrés, bords nettoyés.
pub(crate) fn normalize(text: &str) -> String {
    let folded: String =
        text.to_lowercase().nfd().filter(|c| !is_accent(*c)).map(|c| if c == '’' { '\'' } else { c }).collect();
    let edges = |c: char| !(c.is_alphanumeric() || is_combining_mark(c));
    folded.split_whitespace().collect::<Vec<_>>().join(" ").trim_matches(edges).nfc().collect()
}

/// Vrai si `text` entier est l'un des mots de `key`.
pub(crate) fn matches_shortcut(text: &str, key: &str, l: &Localizer) -> bool {
    if text.len() > SHORTCUT_BYTES {
        return false;
    }
    let said = normalize(text);
    !said.is_empty() && l.tr(key).split(',').any(|word| normalize(word) == said)
}

#[cfg(test)]
#[path = "shortcut_tests.rs"]
mod shortcut_tests;
```

In `crates/oku-slides-agent/src/lib.rs`: `mod shortcut;`.

`stack.rs`, `AgentCommand::Prompt` arm:

```rust
            AgentCommand::Prompt { tab, scope, text, ui, answer } => {
                // Fermé entre-temps : ignorée, comme ci-dessus.
                let Some(t) = self.open(&tab) else { return Ok(()) };
                // Réponse à la question désignée, jamais à une autre. Si elle n'attend plus (répondue
                // ou annulée entre-temps), la consigne n'est pas perdue : elle part comme toute
                // consigne (nouveau tour ; pendant un tour, elle l'interrompt puis le remplace).
                if answer.is_some_and(|id| t.hub.answer(&id, &text)) {
                    return Ok(());
                }
                // Raccourci « thème » : le sélecteur de l'hôte, sans tour ni agent ; un tour en cours
                // continue. Hors du mode deck, ou s'il échoue (journalisé), la consigne part à l'agent.
                if matches_shortcut(&text, THEME_SHORTCUT, &self.platform.live_localizer.current()) {
                    match t.hub.show_themes(ui.slide.clone()) {
                        Ok(true) => return Ok(()),
                        Ok(false) => {}
                        Err(e) => tracing::warn!(tab, error = %snafu::Report::from_error(&e), "sélecteur de thème impossible"),
                    }
                }
                t.send(Command::Prompt(Instruction { scope, text, ui, tune: false }));
            }
```

Import: `use crate::shortcut::{THEME_SHORTCUT, matches_shortcut};`. In `handle`'s doc, after
"Peut bloquer (…)", add: "; un raccourci « thème » rend jusqu'à 12 miniatures, 4 s au plus".

`crates/oku-slides-agent/ARCHITECTURE.md`: replace lines 9–11, the start of the `stack` bullet,
with these three lines (the file stays at 50):

```
- `stack` : `Stack`, the table of tabs and the entry point; `handle` takes an `AgentCommand`; a
  `Prompt` that is a shortcut word (`shortcut`: `matches_shortcut` on a `ui.ftl` key, mirrored by
  the UI) shows the hub's theme picker instead of a turn in deck mode. `tune` starts an
```

`crates/oku-slides-agent/AGENTS.md`, `## Invariants`:
`- Check a prompt for an answer first, then for a shortcut word, before any agent launch; a shortcut never interrupts a turn. (spec:2026-10-19-raccourci-theme-design, t-1071)`

- [ ] **Step 5: Tests pass**

Run: `cargo nextest run -p oku-slides-agent && cargo nextest run -p oku-slides-i18n && wc -l crates/oku-slides-agent/ARCHITECTURE.md && just agents-lint`
Expected: PASS, `50`, PASS.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock crates/oku-slides-agent crates/oku-slides-i18n/src/locales
git commit -F <scratchpad>/msg.txt   # feat(agent) : « thème » seul ouvre le sélecteur de l'hôte sans lancer l'agent (t-1071)
```

---

### Task 5: UI matcher, send gate, keys, dock and stage overlay

**Interfaces:**
- Consumes (Task 1): `StagePreviewStore`, `useStagePreview`, `createStagePreviewStore`,
  `StagePreviewContext`. Consumes the `host-theme-picker` fixture (Task 2) and
  `ui-shortcut-theme` (Task 4).
- Produces:
  - In `ui/src/agent/shortcut.ts`: `normalizeShortcut(text: string): string` and
    `isShortcut(text: string): boolean`.
  - `quickTarget` gives the keys to `host-theme-picker`.
  - `isHostSettings` excludes the picker.
  - `Stage` gets the `DeckProps` prop `themePreview?: StagePreviewStore`.

- [ ] **Step 1: Failing tests**

`ui/src/agent/shortcut.test.ts`:

```ts
// Reconnaissance du raccourci « thème » par l'interface : mêmes cas que shortcut_tests.rs.

import { afterAll, describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { applyMessages } from "../i18n";
import { resetMessages } from "../test/messages";
import { isShortcut, normalizeShortcut } from "./shortcut";

/** Charge `ui.ftl` de `locale` (anglais d'abord, comme le cœur). */
const load = (locale: string) => {
  const ftl = (l: string) => readFileSync(`${import.meta.dir}/../../../crates/oku-slides-i18n/src/locales/${l}/ui.ftl`, "utf8");
  applyMessages({ locale, resources: locale === "en" ? [ftl("en")] : [ftl("en"), ftl(locale)], chosen: null });
};
afterAll(resetMessages);

describe("normalizeShortcut", () => {
  test.each([
    ["Thème.", "theme"],
    ["  THÈMES ! ", "themes"],
    ["«thème»", "theme"],
    ["thema’s", "thema's"],
    ["chủ   đề", "chu đe"],
    ["थीम।", "थीम"],
  ])("normalizeShortcut(%p) gives %p", (text, expected) => {
    expect(normalizeShortcut(text)).toBe(expected);
  });
});

describe("isShortcut", () => {
  test.each([
    ["fr", "Thème.", true],
    ["fr", "thèmes", true],
    ["fr", "THEME", true],
    ["fr", "change le thème en sombre", false],
    ["fr", "", false],
    ["fr", "   ", false],
    ["fr", "…", false],
    ["de", "Themen", true],
    ["ja", "テーマ。", true],
    ["hi", "थीम", true],
    ["hi", "थम", false],
    ["th", "ธีม", true],
    ["nl", "thema’s", true],
    ["ru", "Theme", true],
  ])("in %p, isShortcut(%p) is %p", (locale, text, expected) => {
    load(locale as string);
    expect(isShortcut(text as string)).toBe(expected as boolean);
  });

  test("a prompt over 64 bytes is never a shortcut", () => {
    load("en");
    expect(isShortcut(`theme${" ".repeat(100)}`)).toBe(false);
  });
});
```

`ui/src/DictationDraft.render.test.tsx`: the file needs messages for `t("ui-shortcut-theme")`.
Load the French `ui.ftl` once in that file with the same `load` code as above, and add
`afterAll(resetMessages)`. Then, in the `describe`:

```tsx
  test("DictationDraft sends the shortcut word with no agent ready", async () => {
    const d = await mount("Thème.", { ready: false });
    expect(d.send().disabled).toBe(false);
  });

  test("DictationDraft sends the shortcut word during a turn", async () => {
    const d = await mount("thème", { busy: true });
    expect(d.send().disabled).toBe(false);
  });

  test("DictationDraft sends the shortcut word with a notice", async () => {
    const d = await mount("thème", { notice: "L'agent répond dans l'atelier…" });
    expect(d.send().disabled).toBe(false);
  });

  test("DictationDraft keeps other text gated during a turn", async () => {
    const d = await mount("change le thème", { busy: true });
    expect(d.send().disabled).toBe(true);
  });
```

`ui/src/agent/useQuickKeys.test.ts`, in `describe("quickTarget")`:

```ts
  test("le sélecteur de thème du dock prend les touches", () => {
    const picker = real("host-theme-picker");
    expect(quickTarget({ dock: [settings, picker] })).toEqual({ surface: picker, kind: "dock" });
  });

  test("une question du dock passe avant le sélecteur de thème", () => {
    const q = ask("ask-8");
    expect(quickTarget({ dock: [real("host-theme-picker"), q] })).toEqual({ surface: q, kind: "dock" });
  });

  test("le sélecteur de thème passe avant le tiroir des variantes", () => {
    const picker = real("host-theme-picker");
    expect(quickTarget({ drawer: real("show-6"), dock: [picker] })).toEqual({ surface: picker, kind: "dock" });
  });
```

In the `describe("answerTarget")` block of the same file:

```ts
  test("le sélecteur de thème ne reçoit jamais une consigne", () => {
    expect(answerTarget(quickTarget({ dock: [real("host-theme-picker")] }))).toBeNull();
  });
```

`ui/src/agent/surfaces.test.ts`:

```ts
test("isHostSettings ne compte pas le sélecteur de thème", () => {
  expect(isHostSettings(real("host-theme-picker"))).toBe(false);
});

test("isHostSettings compte les réglages de l'hôte", () => {
  expect(isHostSettings(real("host-widget"))).toBe(true);
});
```

Import `isHostSettings` and `realSurface as real` there, as `useQuickKeys.test.ts` does.

`ui/src/StagePreview.render.test.tsx` (new file: `Stage.render.test.tsx` is 809 lines):

```tsx
// L'aperçu du thème survolé sur la scène du deck.

import { afterEach, describe, expect, test } from "bun:test";
import type { Root } from "react-dom/client";
import "./test/dom";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { default: Stage } = await import("./Stage");
const { createStagePreviewStore } = await import("./a2ui/stagePreview");
const { fieldKey } = await import("./stageFields");

type StageProps = Parameters<typeof Stage>[0];

// La vue de Stage.render.test.tsx (lignes 36-47) : une slide « s », un élément c1 avec un champ title.
const view = {
  slide: { id: "s" }, page: 1, width: 1280, height: 720,
  targets: [{ id: "c1", kind: "acme/card", free: false, x: 0, y: 0, w: 400, h: 200 }],
  fields: [{ owner: "c1", field: "title", text: "Titre", x: 0, y: 60, w: 400, h: 60, size: 60 }],
  items: [], lists: [],
} as unknown as StageProps["view"];

let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function mount(more: Partial<StageProps>) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    root.render(
      createElement(Stage, {
        view, svgUrl: null, selected: null, editing: null, palette: [],
        onSelect: () => {}, onEdit: () => {}, onApply: () => {}, onNotice: () => {},
        ...more,
      } as StageProps),
    );
  });
  return host;
}

describe("aperçu du thème sur la scène", () => {
  test("l'aperçu de la slide montrée se pose par-dessus elle", async () => {
    const store = createStagePreviewStore();
    const host = await mount({ themePreview: store });
    await act(async () => store.set({ slide: "s", url: "data:image/svg+xml;utf8,%3Csvg%2F%3E" }));
    expect(host.querySelector(".stage-theme-preview")?.getAttribute("src")).toBe("data:image/svg+xml;utf8,%3Csvg%2F%3E");
  });

  test("l'aperçu d'une autre slide n'est pas montré", async () => {
    const store = createStagePreviewStore();
    const host = await mount({ themePreview: store });
    await act(async () => store.set({ slide: "s-2", url: "data:a" }));
    expect(host.querySelector(".stage-theme-preview")).toBeNull();
  });

  test("pendant l'édition d'un texte, l'aperçu n'est pas montré", async () => {
    const store = createStagePreviewStore();
    store.set({ slide: "s", url: "data:a" });
    const host = await mount({ themePreview: store, editing: fieldKey({ owner: "c1", field: "title" }) });
    expect(host.querySelector(".stage-theme-preview")).toBeNull();
  });

  test("retirer l'aperçu l'enlève de la scène", async () => {
    const store = createStagePreviewStore();
    const host = await mount({ themePreview: store });
    await act(async () => store.set({ slide: "s", url: "data:a" }));
    await act(async () => store.set(null));
    expect(host.querySelector(".stage-theme-preview")).toBeNull();
  });
});
```

Copy the field entry from `Stage.render.test.tsx`'s `fv("c1", "title", "Titre", 60)` helper so
its shape is exact; the literal above stands for it.

- [ ] **Step 2: Run them, expect failures**

Run, as two commands:
- `cd ui && bun test src/agent/shortcut.test.ts src/agent/useQuickKeys.test.ts src/agent/surfaces.test.ts`
- `cd ui && bun test src/DictationDraft.render.test.tsx src/StagePreview.render.test.tsx`

Expected: FAIL.

- [ ] **Step 3: Implement**

`ui/src/agent/shortcut.ts`:

```ts
// Raccourcis de l'agent reconnus par l'interface : le brouillon envoie « thème » même sans
// agent prêt, pendant un tour ou un tour ailleurs. Même règle et mêmes mots (`ui-shortcut-theme`)
// que l'hôte (oku-slides-agent, shortcut.rs), qui seul décide.

import { t } from "../i18n";

/** Au-delà, une consigne n'est jamais un raccourci. */
const SHORTCUT_BYTES = 64;
const encoder = new TextEncoder();
/** Diacritiques latins, grecs et cyrilliques ; les voyelles indiennes et thaïes restent. */
const ACCENTS = /[̀-ͯ]/g;
/** Bords qui ne sont ni lettre, ni chiffre, ni signe combinant. */
const EDGES = /^[^\p{L}\p{N}\p{M}]+|[^\p{L}\p{N}\p{M}]+$/gu;

/** `text` en minuscules, sans accents, espaces resserrés, bords nettoyés. */
export function normalizeShortcut(text: string): string {
  const folded = text.toLowerCase().normalize("NFD").replace(ACCENTS, "").replaceAll("’", "'");
  return folded.split(/\s+/).filter(Boolean).join(" ").replace(EDGES, "").normalize("NFC");
}

/** Vrai si `text` entier est un mot du raccourci « thème » de la langue de l'interface. */
export function isShortcut(text: string): boolean {
  if (encoder.encode(text).length > SHORTCUT_BYTES) return false;
  const said = normalizeShortcut(text);
  return said !== "" && t("ui-shortcut-theme").split(",").some((word) => normalizeShortcut(word) === said);
}
```

`ui/src/DictationDraft.tsx`, line 67:

```ts
  // Un mot-raccourci (« thème ») part toujours : l'hôte l'ouvre sans agent ni tour.
  const canSend = said !== "" && (isShortcut(said) || (!busy && !notice && ready));
```

with `import { isShortcut } from "./agent/shortcut";`.

`ui/src/agent/AgentPanel.tsx`, `promptAgent`: change the refusal guard to
`if (elsewhereNotice && answer === null && !isShortcut(text)) {`, add a French comment saying a
shortcut never starts a turn, and import `isShortcut` from `./shortcut`.

`ui/src/agent/surfaces.ts`:

```ts
/** Le sélecteur de thème de l'hôte : une boîte demandée, annoncée, jamais repliée. */
export const HOST_THEME_PICKER = "host-theme-picker";
/** Une surface de réglages de l'hôte (`host-template`, `host-widget`) : reconstruite à chaque relecture. */
export const isHostSettings = (s: Surface) => s.id.startsWith("host-") && s.id !== HOST_THEME_PICKER;
```

Check `AgentDock.tsx`'s folding: it must fold only `isHostSettings` / non-question surfaces that
are settings. If it folds every non-question, use `isHostSettings` there instead, and add a case
to `AgentDock.render.test.tsx` (or the existing dock test): with a question and the picker in the
dock, the picker is not inside the folded "Réglages" part, and it has no `aria-live="off"`.

`ui/src/agent/useQuickKeys.ts`:
- import `HOST_THEME_PICKER` from `./surfaces`;
- in `quickTarget`, after the `question` line:

```ts
  const picker = dock.find((s) => s.id === HOST_THEME_PICKER);
  if (picker) return { surface: picker, kind: "dock" };
```

- `QuickTarget.kind` doc: "`dock` : une question du fil ou le sélecteur de thème de l'hôte".
- `quickTarget` doc: "…sinon une question du dock, sinon le sélecteur de thème de l'hôte (1–4,
  Échap avec le focus dans le dock), sinon le tiroir. Les autres réglages du dock n'en reçoivent pas."

`ui/src/Stage.tsx`:
- In `DeckProps`:

```ts
  /** Aperçu du thème survolé dans le sélecteur de l'hôte ; absent : aucun. */
  themePreview?: StagePreviewStore;
```

- Add `themePreview` to the destructuring from `deck ?? ({} as Partial<DeckProps>)`.
- Below `Stage`, add:

```tsx
/** L'aperçu d'un thème sur la slide montrée ; seul ce composant suit le survol. */
function ThemePreviewLayer({ store, slide }: { store: StagePreviewStore; slide: string }) {
  const preview = useStagePreview(store);
  if (!preview || preview.slide !== slide) return null;
  return <img className="stage-theme-preview" src={preview.url} alt="" draggable={false} />;
}
```

- In the `.stage` div, as its last child:
  `{themePreview && editing === null && <ThemePreviewLayer store={themePreview} slide={view.slide.id} />}`.
- Imports: `type StagePreviewStore, useStagePreview` from `./a2ui/stagePreview`.

`ui/src/styles.css`, after the `.stage > img` rule (line ~285, which already sets position, size
and `pointer-events: none`):

```css
/* Thème survolé dans le sélecteur de l'hôte : couvre la slide, ses cibles et la sélection. */
.stage > .stage-theme-preview { z-index: 7; background: var(--paper-100); }
```

`ui/src/App.tsx`:
- import `StagePreviewContext, createStagePreviewStore` from `./a2ui/stagePreview`;
- with the other `useState` calls, **before** the early return (line ~300):
  `const [previewStore] = useState(createStagePreviewStore);`;
- wrap the returned tree, just inside `NumberLangContext.Provider`, in
  `<StagePreviewContext.Provider value={previewStore}>` … `</StagePreviewContext.Provider>`;
- pass `themePreview={readOnly || snap.editing ? undefined : previewStore}` to `<Stage>`.

- [ ] **Step 4: Tests and type check pass**

Run, as two commands, then the type check:
- `cd ui && bun test --path-ignore-patterns='**/*.render.test.tsx'`
- `cd ui && bun test .render.test.`
- `cd ui && bunx tsc -b`

Expected: PASS, `i18n-keys.test.ts` included (`ui-shortcut-theme` is now used).

- [ ] **Step 5: Commit**

```bash
git add ui/src/agent ui/src/DictationDraft.tsx ui/src/DictationDraft.render.test.tsx ui/src/Stage.tsx ui/src/StagePreview.render.test.tsx ui/src/App.tsx ui/src/styles.css
git commit -F <scratchpad>/msg.txt   # feat(ui) : raccourci « thème » envoyable sans agent, touches 1–4 et aperçu sur la scène (t-1071, t-1074)
```

---

### Task 6: End-to-end spec and help

- [ ] **Step 1: Share the agent log reader**

Move `logOffset` and `logSince` out of `ui/e2e/220-import.cjs` into `ui/e2e/lib/agentLog.cjs`.
Keep their code and comments **verbatim**; the only change is the log path, which becomes a
parameter:

```js
// Journal du faux agent (E2E_AGENT_LOG) : ce qu'il a reçu, ligne JSON par message.
const { readFileSync, statSync } = require('node:fs');

/** La taille du journal `log` : ce qui s'écrit ensuite vient de l'action qui suit. */
const logOffset = (log) => statSync(log).size;

/**
 * Les messages notés dans `log` depuis `from` (octets), dans l'ordre. La dernière ligne,
 * peut-être en cours d'écriture (sans saut de ligne final), n'est pas lue ; une ligne illisible est ignorée.
 */
const logSince = (log, from) => {
  const lines = readFileSync(log).subarray(from).toString('utf8').split('\n');
  lines.pop();
  return lines.flatMap((l) => {
    try {
      return [JSON.parse(l)];
    } catch {
      return [];
    }
  });
};

module.exports = { logOffset, logSince };
```

In `220-import.cjs`:
- `require` it;
- replace its local definitions with `const logOffset = () => agentLog.logOffset(LOG);` and
  `const logSince = (from) => agentLog.logSince(LOG, from);`;
- drop the `fs` imports that become unused.

- [ ] **Step 2: Write the e2e spec**

`ui/e2e/230-raccourci-theme.cjs`:

```js
// Raccourci « thème » : la consigne seule ouvre le sélecteur de l'hôte au dock, sans agent ;
// survoler un thème le montre sur la scène, cliquer ou presser son chiffre l'applique, annuler
// revient au précédent. Contre `oku-slides serve` lancé par `just e2e` (après 220 : journal créé).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post, activeSnapshot } = require('./lib/http.cjs');
const { until } = require('./lib/until.cjs');
const { logOffset, logSince } = require('./lib/agentLog.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const LOG = process.env.E2E_AGENT_LOG;

const prompts = (from) => logSince(LOG, from).filter((m) => m.method === 'session/prompt');
const themeId = async (p) => (await activeSnapshot(p)).theme?.id ?? null;
const picker = (p) => p.locator('.agent-dock [data-surface-id="host-theme-picker"]');
const cards = (p) => picker(p).locator('.theme-choice');
/** Les noms des cartes actives ; jamais d'exception, une liste vide s'il n'y en a pas. */
const activeNames = (p) => picker(p).locator('.theme-choice[aria-pressed="true"] bdi').allInnerTexts();
/** Le rang de la première carte non active parmi les quatre premières (touches 1–4). */
const firstInactive = async (p) =>
  (await cards(p).evaluateAll((els) => els.map((e) => e.getAttribute('aria-pressed')))).slice(0, 4).indexOf('false');
/** Écrit une consigne dans le brouillon de l'agent et l'envoie. */
const say = async (p, text) => {
  await p.keyboard.press('ControlOrMeta+k');
  await p.locator('.draft-text').fill(text);
  await p.locator('.draft-send').click();
  await p.waitForSelector('.draft-text', { state: 'detached' });
};
/** Rend le focus au document : les touches vont à l'app, pas au dock ni au brouillon. */
const blur = (p) => p.evaluate(() => (document.activeElement instanceof HTMLElement ? document.activeElement.blur() : undefined));

(async () => {
  assert.ok(LOG, 'E2E_AGENT_LOG manquant : lancer par just e2e');
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  p.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  await p.goto(URL);
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await p.reload();
  await p.waitForSelector('.stage img');
  const from = logOffset(LOG);
  const before = await themeId(p);

  // 1. « Thème. » tapé dans le brouillon ouvre le sélecteur au dock ; un seul thème est actif.
  await say(p, 'Thème.');
  await cards(p).first().waitFor();
  assert.ok((await cards(p).count()) >= 2, 'au moins deux thèmes');
  const startActive = await activeNames(p);
  assert.equal(startActive.length, 1);

  // 2. Survoler un thème non actif montre sa miniature sur la scène ; la quitter la retire.
  const i = await firstInactive(p);
  assert.ok(i >= 0, 'une carte non active parmi les quatre premières');
  const other = cards(p).nth(i);
  const name = await other.locator('bdi').innerText();
  const thumb = await other.locator('img').getAttribute('src');
  await other.hover();
  await p.waitForSelector('.stage .stage-theme-preview');
  assert.equal(await p.locator('.stage .stage-theme-preview').getAttribute('src'), thumb);
  await p.mouse.move(0, 0);
  await p.waitForSelector('.stage .stage-theme-preview', { state: 'detached' });

  // 3. Cliquer l'applique : le thème du deck change, la carte cliquée devient l'active.
  await other.click();
  await until(p, async () => (await themeId(p)) !== before, 'le thème du deck doit changer');
  await until(p, async () => JSON.stringify(await activeNames(p)) === JSON.stringify([name]), 'la carte choisie doit être active');

  // 4. ⌘Z (focus hors du dock) rend le thème précédent, et l'active redevient celle du début.
  await blur(p);
  await p.keyboard.press('ControlOrMeta+z');
  await until(p, async () => (await themeId(p)) === before, "l'annulation doit rendre le thème précédent");
  await until(p, async () => JSON.stringify(await activeNames(p)) === JSON.stringify(startActive), "l'active doit suivre l'annulation");

  // 5. Le chiffre d'une carte non active l'applique (touches 1–4 du sélecteur).
  await blur(p);
  await p.keyboard.press(`Digit${i + 1}`);
  await until(p, async () => JSON.stringify(await activeNames(p)) === JSON.stringify([name]), 'la touche doit appliquer le thème');
  await blur(p);
  await p.keyboard.press('ControlOrMeta+z');
  await until(p, async () => (await themeId(p)) === before, "l'annulation doit rendre le thème précédent");

  // 6. Fermer le sélecteur.
  await picker(p).getByRole('button', { name: 'Fermer', exact: true }).click();
  await picker(p).waitFor({ state: 'detached' });

  // 7. Aucun prompt n'est parti vers l'agent : un témoin envoyé ensuite est le seul noté.
  await post(p, '/api/agent', { cmd: 'prompt', tab: 't-1', text: 'témoin', ui: {} });
  await until(p, async () => prompts(from).length > 0, 'le témoin doit atteindre le faux agent', 15000);
  const sent = prompts(from);
  assert.equal(sent.length, 1, JSON.stringify(sent));
  assert.ok(JSON.stringify(sent[0]).includes('témoin'));

  assert.deepEqual(errors, []);
  console.log('raccourci-theme : ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
```

Before running:
- **Mode after spec 200.** Spec 200 (theme editor) must leave t-1 in deck mode. If it does not,
  leave templates mode at the start. Take the request name from `ui/src/api.ts`.
- **Digits.** If `Digit2` does not reach `quickKey` (it reads `code`), use
  `p.keyboard.press(String(i + 1))`.

- [ ] **Step 3: Run it**

Run: `just e2e 4999`
Expected: every spec passes, including `220-import` (log helpers moved) and `230-raccourci-theme`.

- [ ] **Step 4: Help content**

In `docs/aide/fr.json`, slide "Grille, modèles, thème", the run at line 261 becomes:

`" : Papier, Pro ou les vôtres. Dites ou tapez « thème » seul : chaque thème s'affiche sur votre slide, sans attendre l'agent ; survolez pour prévisualiser, cliquez pour appliquer. Il est indépendant de l'ensemble de modèles. En mode modèles, l'onglet Thème règle ses jetons un par un ; « Terminer » l'enregistre, en copie pour Papier ou Pro."`

In `docs/aide/en.json`, line 261:

`": Papier, Pro or your own. Say or type “theme” alone: each theme shows on your slide, without waiting for the agent; hover to preview, click to apply. It is independent of the template set. In template mode, the Theme tab sets its tokens one by one; “Done” saves it, as a copy for Papier or Pro."`

Run: `cargo nextest run -p oku-slides-session help`
Expected: PASS.

- [ ] **Step 5: Full gate and commit**

Run: `just ci`
Expected: PASS.

```bash
git add ui/e2e/lib/agentLog.cjs ui/e2e/220-import.cjs ui/e2e/230-raccourci-theme.cjs docs/aide/fr.json docs/aide/en.json
git commit -F <scratchpad>/msg.txt   # test(ui) : e2e du raccourci « thème », lecteur du journal partagé, aide (t-1071)
```

Then mark t-1074 done (folded into this work) and t-1071 done after merge.
