// Champ en édition : quitter la slide valide la saisie ouverte d'abord.

import { afterEach, expect, test } from "bun:test";
import "./test/dom";
import type { SlideView } from "./api";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useFieldEditing } = await import("./useFieldEditing");

type Hook = ReturnType<typeof useFieldEditing>;
let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const view = { slide: { id: "s1" }, fields: [{ owner: "c1", field: "title", empty: false, kind: "plain" }] } as unknown as SlideView;

/** Monte le hook ; `commits` compte les validations demandées à la scène. */
async function mount() {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const out: { hook: Hook | null; commits: number } = { hook: null, commits: 0 };
  function Probe() {
    const hook = useFieldEditing(view);
    hook.stage.current = { commitEditing: () => { out.commits += 1; } };
    out.hook = hook;
    return null;
  }
  await act(async () => root.render(createElement(Probe)));
  await act(async () => out.hook!.setEditing("c1\u001ftitle"));
  return out;
}

test("useFieldEditing leaveSlide to another slide commits the open field and closes editing", async () => {
  const out = await mount();
  await act(async () => out.hook!.leaveSlide("s2"));
  expect(out.commits).toBe(1);
  expect(out.hook!.editing).toBeNull();
});

test("useFieldEditing leaveSlide to the shown slide keeps editing", async () => {
  const out = await mount();
  await act(async () => out.hook!.leaveSlide("s1"));
  expect(out.commits).toBe(0);
  expect(out.hook!.editing).toBe("c1\u001ftitle");
});

/** Monte le hook sur une vue qu'on peut changer (`show`). */
async function mountOn(v: SlideView) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const out: { hook: Hook | null } = { hook: null };
  function Probe({ v }: { v: SlideView }) {
    out.hook = useFieldEditing(v);
    return null;
  }
  const show = (next: SlideView) => act(async () => root.render(createElement(Probe, { v: next })));
  await show(v);
  return { out, show };
}

const itemKey = "f\u001fetapes/i-1/date";
const withItem = { ...view, fields: [...view.fields, { owner: "f", field: "etapes/i-1/date", empty: true, kind: "plain" }] } as unknown as SlideView;

test("useFieldEditing editKeyWhenShown opens the field once a view shows it", async () => {
  const { out, show } = await mountOn(view);
  await act(async () => out.hook!.editKeyWhenShown(itemKey));
  expect(out.hook!.editing).toBeNull();
  await show(withItem);
  expect(out.hook!.editing).toBe(itemKey);
});

test("useFieldEditing setEditing forgets a pending field key", async () => {
  const { out, show } = await mountOn(view);
  await act(async () => out.hook!.editKeyWhenShown(itemKey));
  await act(async () => out.hook!.setEditing(null));
  await show(withItem);
  expect(out.hook!.editing).toBeNull();
});
