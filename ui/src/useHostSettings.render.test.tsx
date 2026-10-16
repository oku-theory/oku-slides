// Réglages de l'hôte : relus sur leurs clés, actions adressées par clé.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Snapshot, Surface, TabApi } from "./api";

const { act, createElement, useLayoutEffect } = await import("react");
const { createRoot } = await import("react-dom/client");
const { HOST_TEMPLATE, HOST_THEME, HOST_WIDGET, useHostSettings } = await import("./useHostSettings");
const { deferred, memory, stub, wait } = await import("./test/widgetApi");
const { AgentThread } = await import("./agent/AgentThread");
const { CATALOG_ID } = await import("./a2ui/catalog");
const { EditorView } = await import("@codemirror/view");

const surface = (id: string): Surface => ({ id, place: "thread", live: true, turn: null, messages: [] }) as unknown as Surface;
interface Snap { trials?: number; history?: boolean; hash?: string; undo?: string | null }
const snapshot = (o: Snap = {}): Snapshot =>
  ({
    editing: { set: "p", base: "b" }, templates: { version: "v" }, history: o.history ? {} : null,
    widget_trials: o.trials ?? 0, undo_label: o.undo ?? null, redo_label: null,
    slides: [{ id: "t-1", hash: o.hash ?? "h", name: "Libre" }],
  }) as unknown as Snapshot;
const action = (context: Record<string, unknown>): A2uiClientAction =>
  ({ name: "param", surfaceId: HOST_WIDGET, sourceComponentId: "f0", timestamp: new Date(0).toISOString(), context }) as unknown as A2uiClientAction;

interface Props { snap: Snapshot; current: string; widget: string | null }
let unmount: (() => Promise<void>) | null = null;
afterEach(async () => {
  await unmount?.();
  unmount = null;
});

async function mountHost(api: TabApi, first: Props) {
  const out: { h: ReturnType<typeof useHostSettings> | null; toasts: string[]; runs: number } = { h: null, toasts: [], runs: 0 };
  // Comme `run` de l'App : attend, et dit l'erreur en toast.
  const run = async (p: Promise<Snapshot>) => {
    out.runs += 1;
    try {
      return await p;
    } catch (e) {
      out.toasts.push(String(e));
      return null;
    }
  };
  function Probe(p: Props) {
    out.h = useHostSettings(api, p.snap, p.current, p.widget, run, (m) => out.toasts.push(m));
    return null;
  }
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  unmount = async () => {
    await act(async () => root.unmount());
    host.remove();
  };
  const render = async (p: Props) => {
    await act(async () => root.render(createElement(Probe, p)));
    await wait(10);
  };
  await render(first);
  return { out, render };
}

function counted(id: string) {
  const { fake } = memory({});
  const reads = { n: 0 };
  stub(fake, "hostSettings", async () => (reads.n++, surface(id)));
  return { fake, reads };
}

describe("useHostSettings", () => {
  test("montre le modèle sans widget", async () => {
    const { fake } = memory({});
    stub(fake, "hostSettings", async (_s: string, w: string | null) => surface(w ? HOST_WIDGET : HOST_TEMPLATE));
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: null });
    expect(m.out.h!.surface?.id).toBe(HOST_TEMPLATE);
  });

  test("montre le widget du sous-onglet actif", async () => {
    const { fake } = memory({});
    stub(fake, "hostSettings", async (_s: string, w: string | null) => surface(w ? HOST_WIDGET : HOST_TEMPLATE));
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: "carte" });
    expect(m.out.h!.surface?.id).toBe(HOST_WIDGET);
  });

  test("un instantané identique ne relit pas la surface", async () => {
    const { fake, reads } = counted(HOST_TEMPLATE);
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: null });
    await m.render({ snap: snapshot(), current: "t-1", widget: null });
    expect(reads.n).toBe(1);
  });

  test("la révision des valeurs d'exemple relit la surface", async () => {
    const { fake, reads } = counted(HOST_WIDGET);
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: "carte" });
    await m.render({ snap: snapshot({ trials: 1 }), current: "t-1", widget: "carte" });
    expect(reads.n).toBe(2);
  });

  test("une annulation relit la surface du modèle", async () => {
    const { fake, reads } = counted(HOST_TEMPLATE);
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: null });
    await m.render({ snap: snapshot({ undo: "Grille 6 × 6" }), current: "t-1", widget: null });
    expect(reads.n).toBe(2);
  });

  test("pendant l'historique, aucune surface", async () => {
    const { fake } = counted(HOST_TEMPLATE);
    const m = await mountHost(fake, { snap: snapshot({ history: true }), current: "t-1", widget: null });
    expect(m.out.h!.surface).toBeNull();
  });

  test("une réponse en retard ne remplace pas la surface plus récente", async () => {
    const { fake } = memory({});
    const slow = deferred<Surface>();
    stub(fake, "hostSettings", (_s: string, w: string | null) => (w === "lent" ? slow.promise : Promise.resolve(surface(HOST_TEMPLATE))));
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: "lent" });
    await m.render({ snap: snapshot(), current: "t-1", widget: null });
    slow.resolve(surface(HOST_WIDGET));
    await wait(10);
    expect(m.out.h!.surface?.id).toBe(HOST_TEMPLATE);
  });

  test("un réglage du widget part dans ses valeurs d'exemple, avec le widget de la surface montrée", async () => {
    const { fake } = memory({});
    const sent: unknown[] = [];
    const later = deferred<Surface>();
    stub(fake, "hostSettings", (_s: string, w: string | null) => (w === "carte" ? Promise.resolve(surface(HOST_WIDGET)) : later.promise));
    stub(fake, "setExampleValue", async (...a: unknown[]) => (sent.push(a), snapshot()));
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: "carte" });
    const shown = m.out.h!.surface!;
    await m.render({ snap: snapshot(), current: "t-1", widget: "autre" });
    m.out.h!.onAction(shown, action({ key: "title", value: "Météo" }));
    await wait(10);
    expect(sent).toEqual([["carte", "title", "Météo"]]);
  });

  test("la surface du widget est lue pour la slide et le widget seuls", async () => {
    const { fake } = memory({});
    const asked: unknown[][] = [];
    stub(fake, "hostSettings", async (...a: unknown[]) => (asked.push(a), surface(HOST_WIDGET)));
    await mountHost(fake, { snap: snapshot(), current: "t-1", widget: "carte" });
    expect(asked).toEqual([["t-1", "carte"]]);
  });

  test("un changement de liste du widget part en edit_example_list", async () => {
    const { fake } = memory({});
    const sent: unknown[] = [];
    stub(fake, "hostSettings", async () => surface(HOST_WIDGET));
    stub(fake, "editExampleList", async (...a: unknown[]) => (sent.push(a), snapshot()));
    stub(fake, "setExampleValue", async (...a: unknown[]) => (sent.push(["value", ...a]), snapshot()));
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: "frise" });
    m.out.h!.onAction(m.out.h!.surface!, action({ key: "etapes", field: 0, change: { op: "remove", item: "d0" } }));
    await wait(10);
    expect(sent).toEqual([["frise", "etapes", { op: "remove", item: "d0" }]]);
  });

  test("un changement d'une liste imbriquée du widget part en edit_example_list sur son chemin", async () => {
    const { fake } = memory({});
    const sent: unknown[] = [];
    stub(fake, "hostSettings", async () => surface(HOST_WIDGET));
    stub(fake, "editExampleList", async (...a: unknown[]) => (sent.push(a), snapshot()));
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: "plan" });
    const change = { op: "nested", path: "d0/taches", change: { op: "remove", item: "t1" } };
    m.out.h!.onAction(m.out.h!.surface!, action({ key: "etapes", field: 0, change }));
    await wait(10);
    expect(sent).toEqual([["plan", "etapes/d0/taches", { op: "remove", item: "t1" }]]);
  });

  test("un collage dans une liste du widget remplace la liste entière", async () => {
    const { fake } = memory({});
    const sent: unknown[] = [];
    const items = [{ id: "d0", date: "2020" }];
    stub(fake, "hostSettings", async () => surface(HOST_WIDGET));
    stub(fake, "setExampleValue", async (...a: unknown[]) => (sent.push(a), snapshot()));
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: "frise" });
    m.out.h!.onAction(m.out.h!.surface!, action({ key: "etapes", field: 0, change: { op: "replace", items } }));
    await wait(10);
    expect(sent).toEqual([["frise", "etapes", items]]);
  });

  test("un réglage du modèle part en host_param avec sa clé", async () => {
    const { fake } = memory({});
    const sent: unknown[] = [];
    stub(fake, "hostSettings", async () => surface(HOST_TEMPLATE));
    stub(fake, "hostParam", async (...a: unknown[]) => (sent.push(a), snapshot()));
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: null });
    m.out.h!.onAction(m.out.h!.surface!, action({ key: "columns", value: 6 }));
    await wait(10);
    expect(sent).toEqual([["t-1", "columns", 6]]);
  });

  test("une surface d'une autre origine n'est pas traitée", async () => {
    const { fake } = counted(HOST_TEMPLATE);
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: null });
    expect(m.out.h!.onAction(surface("show-1"), action({ key: "name", value: "x" }))).toBe(false);
  });

  test("une action de la surface des jetons est prise, et rien ne part (routée par useThemeSettings)", async () => {
    const { fake } = counted(HOST_TEMPLATE);
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: null });
    expect(m.out.h!.onAction(surface(HOST_THEME), action({ key: "x", value: "y" }))).toBe(true);
    expect(m.out.runs).toBe(0);
  });

  test("useHostSettings ignores host-target", async () => {
    const { fake } = counted(HOST_TEMPLATE);
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: null });
    expect(m.out.h!.onAction(surface("host-target"), action({ key: "titre", value: "x" }))).toBe(false);
    expect(m.out.runs).toBe(0);
  });

  test("une action sans clé n'envoie rien", async () => {
    const { fake } = counted(HOST_TEMPLATE);
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: null });
    m.out.h!.onAction(m.out.h!.surface!, action({ value: "x" }));
    expect(m.out.runs).toBe(0);
  });

  test("un refus de la session est dit en toast", async () => {
    const { fake } = memory({});
    stub(fake, "hostSettings", async () => surface(HOST_WIDGET));
    stub(fake, "setExampleValue", async () => {
      throw new Error("refusé");
    });
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: "carte" });
    m.out.h!.onAction(m.out.h!.surface!, action({ key: "count", value: 99 }));
    await wait(10);
    expect(m.out.toasts).toEqual(["Error: refusé"]);
  });

  test("l'issue d'un réglage est posée pendant l'envoi : fausse si la session refuse, vraie sinon", async () => {
    const { fake } = memory({});
    let refuse = true;
    stub(fake, "hostSettings", async () => surface(HOST_WIDGET));
    stub(fake, "editExampleList", async () => {
      if (refuse) throw new Error("refusé");
      return snapshot();
    });
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: "frise" });
    const change = action({ key: "etapes", field: 0, change: { op: "remove", item: "d0" } });
    m.out.h!.onAction(m.out.h!.surface!, change);
    expect(await m.out.h!.outcome.current).toBe(false);
    refuse = false;
    m.out.h!.onAction(m.out.h!.surface!, change);
    expect(await m.out.h!.outcome.current).toBe(true);
  });

  test("une action d'une surface remplacée a une issue fausse", async () => {
    const { fake } = counted(HOST_WIDGET);
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: "frise" });
    m.out.h!.onAction(surface(HOST_WIDGET), action({ key: "title", value: "x" }));
    expect(await m.out.h!.outcome.current).toBe(false);
  });

  test("un refus de la session relit la surface et la reconstruit", async () => {
    const { fake, reads } = counted(HOST_WIDGET);
    stub(fake, "setExampleValue", async () => {
      throw new Error("refusé");
    });
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: "carte" });
    const before = m.out.h!.surface!;
    m.out.h!.onAction(before, action({ key: "count", value: 99 }));
    await wait(10);
    expect(reads.n).toBe(2);
    expect(m.out.h!.surface).not.toBe(before);
  });

  test("une relecture au contenu identique garde la même surface", async () => {
    const { fake, reads } = counted(HOST_WIDGET);
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: "carte" });
    const before = m.out.h!.surface!;
    await m.render({ snap: snapshot({ trials: 1 }), current: "t-1", widget: "carte" });
    expect(reads.n).toBe(2);
    expect(m.out.h!.surface).toBe(before);
  });

  test("une action d'une surface de l'hôte remplacée est prise, et rien ne part", async () => {
    const { fake } = memory({});
    const sent: unknown[] = [];
    stub(fake, "hostSettings", async (_s: string, w: string | null) => surface(w ? HOST_WIDGET : HOST_TEMPLATE));
    stub(fake, "setExampleValue", async (...a: unknown[]) => (sent.push(a), snapshot()));
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: "carte" });
    const old = m.out.h!.surface!;
    await m.render({ snap: snapshot(), current: "t-1", widget: "autre" });
    expect(m.out.h!.surface).not.toBe(old);
    expect(m.out.h!.onAction(old, action({ key: "title", value: "x" }))).toBe(true);
    await wait(10);
    expect(sent).toEqual([]);
  });

  test("sans surface montrée, une action de l'hôte est prise, et rien ne part", async () => {
    const { fake } = counted(HOST_TEMPLATE);
    const m = await mountHost(fake, { snap: snapshot({ history: true }), current: "t-1", widget: null });
    expect(m.out.h!.onAction(surface(HOST_TEMPLATE), action({ key: "name", value: "x" }))).toBe(true);
    expect(m.out.runs).toBe(0);
  });

  test("un texte riche validé pendant le remplacement de sa surface ne part pas vers le widget suivant", async () => {
    const { fake } = memory({});
    const sent: unknown[] = [];
    // La surface du widget : un champ riche dont la valeur porte le nom du widget.
    const rich = (w: string): Surface =>
      ({
        id: HOST_WIDGET, place: "dock", live: true, turn: null,
        messages: [
          { version: "v0.9", createSurface: { surfaceId: HOST_WIDGET, catalogId: CATALOG_ID } },
          { version: "v0.9", updateComponents: { surfaceId: HOST_WIDGET, components: [
            { id: "root", component: "RichTextField", label: "Corps", value: { path: "/v" }, source: { path: "/s" }, palette: [],
              action: { event: { name: "param", context: { key: "body", value: { path: "/s" } } } } },
          ] } },
          { version: "v0.9", updateDataModel: { surfaceId: HOST_WIDGET, value: { v: w, s: [{ runs: [{ t: w }] }] } } },
        ],
      }) as unknown as Surface;
    stub(fake, "hostSettings", async (_s: string, w: string | null) => rich(w ?? ""));
    stub(fake, "setExampleValue", async (...a: unknown[]) => (sent.push(a), snapshot()));
    const run = async (p: Promise<Snapshot>) => p.catch(() => null);
    function Dock({ widget }: { widget: string }) {
      const h = useHostSettings(fake, snapshot(), "t-1", widget, run, () => {});
      const dock = h.surface ? [h.surface] : [];
      // L'éditeur perd le focus pendant le rendu qui remplace la surface (avant que l'ancienne soit libérée).
      useLayoutEffect(() => {
        document.activeElement?.dispatchEvent(new FocusEvent("blur"));
      }, [h.surface]);
      return createElement(AgentThread, {
        state: "idle", turns: [], loose: [], dock,
        onAction: (s: Surface, a: A2uiClientAction) => void h.onAction(s, a),
        onNewConversation: () => {}, onError: () => {},
      });
    }
    const host = document.body.appendChild(document.createElement("div"));
    const root = createRoot(host);
    unmount = async () => {
      await act(async () => root.unmount());
      host.remove();
    };
    await act(async () => root.render(createElement(Dock, { widget: "carte" })));
    await wait(20);
    const content = host.querySelector(".cm-content") as HTMLElement;
    const view = EditorView.findFromDOM(content)!;
    content.focus();
    await act(async () => view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: "tapé" } }));
    await act(async () => root.render(createElement(Dock, { widget: "autre" })));
    await wait(20);
    expect(host.querySelector(".cm-content")?.textContent).toBe("autre");
    expect(sent.map((a) => (a as unknown[])[0])).not.toContain("autre");
  });
});
