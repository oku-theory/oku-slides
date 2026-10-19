// Le focus demandé par l'agent : différé pendant l'édition d'un texte, ignoré s'il vise l'inconnu (t-210).

import { afterEach, expect, test } from "bun:test";
import "../test/dom";
import type { SlideSummary, SlideView } from "../api";

const { act, createElement, useMemo, useState } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useAgentFocus } = await import("./useAgentFocus");

let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

const slides = [{ id: "s1" }, { id: "s2" }, { id: "s3" }] as SlideSummary[];
/** La vue d'une slide : son élément `<id>-e1`, plus `added`. */
const viewOf = (id: string, added: string[]) =>
  ({ slide: { id, kind: "content", params: { body: [`${id}-e1`, ...added].map((e) => ({ id: e, kind: "base/text" })) } } }) as unknown as SlideView;

/** Un onglet réduit : slide courante (sa vue suit), sélection, édition ; rend le rappel `focus`. */
async function mount() {
  let out: {
    focus: (slide: string, element: string | null) => void;
    current: string;
    selected: string | null;
    setEditing: (k: string | null) => void;
    setCurrent: (id: string) => void;
    setSelected: (id: string | null) => void;
    /** Une nouvelle vue de la slide affichée, avec ces éléments en plus. */
    setAdded: (ids: string[]) => void;
  } | null = null;
  const Probe = () => {
    const [current, setCurrent] = useState("s1");
    const [selected, setSelected] = useState<string | null>(null);
    const [editing, setEditing] = useState<string | null>(null);
    const [added, setAdded] = useState<string[]>([]);
    // Une vue par slide affichée et par contenu, comme celle qu'App relit.
    const view = useMemo(() => viewOf(current, added), [current, added]);
    const focus = useAgentFocus({ slides, view, current, editing, show: setCurrent, select: setSelected });
    out = { focus, current, selected, setEditing, setCurrent, setSelected, setAdded };
    return null;
  };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(createElement(Probe)));
  cleanup = async () => {
    await act(async () => root.unmount());
    host.remove();
  };
  return { state: () => out! };
}

test("useAgentFocus hors édition montre la slide et sélectionne l'élément", async () => {
  const h = await mount();
  await act(async () => h.state().focus("s2", "s2-e1"));
  expect(h.state().current).toBe("s2");
  expect(h.state().selected).toBe("s2-e1");
});

test("useAgentFocus pendant l'édition attend la fin de l'édition, puis applique le dernier focus reçu", async () => {
  const h = await mount();
  await act(async () => h.state().setEditing("s1-e1/body"));
  await act(async () => h.state().focus("s1", "s1-e1"));
  await act(async () => h.state().focus("s2", "s2-e1"));
  // L'édition continue sur la slide affichée.
  expect(h.state().current).toBe("s1");
  expect(h.state().selected).toBeNull();
  await act(async () => h.state().setEditing(null));
  expect(h.state().current).toBe("s2");
  expect(h.state().selected).toBe("s2-e1");
});

test("useAgentFocus ignore une slide inconnue", async () => {
  const h = await mount();
  await act(async () => h.state().focus("s9", null));
  expect(h.state().current).toBe("s1");
});

test("useAgentFocus montre la slide mais ne sélectionne pas un élément inconnu", async () => {
  const h = await mount();
  await act(async () => h.state().focus("s2", "ghost"));
  expect(h.state().current).toBe("s2");
  expect(h.state().selected).toBeNull();
});

test("useAgentFocus un focus inconnu reçu pendant l'édition est ignoré à la fin", async () => {
  const h = await mount();
  await act(async () => h.state().setEditing("s1-e1/body"));
  await act(async () => h.state().focus("s9", "x"));
  await act(async () => h.state().setEditing(null));
  expect(h.state().current).toBe("s1");
  expect(h.state().selected).toBeNull();
});

test("useAgentFocus un élément que l'agent vient de créer sur la slide affichée est choisi à la vue suivante", async () => {
  const h = await mount();
  await act(async () => h.state().focus("s1", "s1-new"));
  // La vue affichée date d'avant la création : l'élément n'y est pas encore.
  await act(async () => h.state().setAdded(["s1-new"]));
  expect(h.state().selected).toBe("s1-new");
});

test("useAgentFocus un élément inconnu garde la sélection en place", async () => {
  const h = await mount();
  await act(async () => h.state().setSelected("s1-e1"));
  await act(async () => h.state().focus("s1", "ghost"));
  await act(async () => h.state().setAdded(["autre"]));
  expect(h.state().selected).toBe("s1-e1");
});

test("useAgentFocus un focus différé est abandonné si l'utilisateur change de slide pendant l'édition", async () => {
  const h = await mount();
  await act(async () => h.state().setEditing("s1-e1/body"));
  await act(async () => h.state().focus("s2", "s2-e1"));
  await act(async () => h.state().setCurrent("s3"));
  await act(async () => h.state().setEditing(null));
  expect(h.state().current).toBe("s3");
  expect(h.state().selected).toBeNull();
});
