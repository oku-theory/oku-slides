// Jetons du thème édité : surface relue sur ses clés, actions routées en host_token.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Snapshot, Surface, TabApi } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useThemeSettings } = await import("./useThemeSettings");
const { HOST_THEME } = await import("./useHostSettings");
const { memory, stub, wait } = await import("./test/widgetApi");

const PAPIER = "2a7d4c91-5e3b-4f08-b6a1-8c9e0d1f2a31";
const surface = (id = HOST_THEME): Surface => ({ id, place: "thread", live: true, turn: null, messages: [] }) as unknown as Surface;
interface Snap { history?: boolean; undo?: string | null; dirty?: boolean; name?: string; editing?: boolean }
const snapshot = (o: Snap = {}): Snapshot =>
  ({
    editing: o.editing === false ? null : {
      set: "p", base: "b",
      theme: { base: { id: PAPIER, version: "v1" }, name: o.name ?? "Papier", builtin: true, dirty: o.dirty ?? false, revision: "r1" },
    },
    history: o.history ? {} : null, undo_label: o.undo ?? null, redo_label: null, slides: [],
  }) as unknown as Snapshot;
const action = (name: string, context: Record<string, unknown>): A2uiClientAction =>
  ({ name, surfaceId: HOST_THEME, sourceComponentId: "f0", timestamp: new Date(0).toISOString(), context }) as unknown as A2uiClientAction;

interface Props { snap: Snapshot; filter: string }
let unmount: (() => Promise<void>) | null = null;
afterEach(async () => {
  await unmount?.();
  unmount = null;
});

async function mountTheme(api: TabApi, first: Props) {
  const out: { h: ReturnType<typeof useThemeSettings> | null; toasts: string[]; runs: number; filters: string[] } = {
    h: null, toasts: [], runs: 0, filters: [],
  };
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
    out.h = useThemeSettings(api, p.snap, p.filter, run, (m) => out.toasts.push(m), (f) => out.filters.push(f));
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

function counted() {
  const { fake } = memory({});
  const asked: string[] = [];
  stub(fake, "hostTheme", async (filter: string) => (asked.push(filter), surface()));
  return { fake, asked };
}

describe("useThemeSettings", () => {
  test("lit la surface du thème avec le filtre", async () => {
    const { fake, asked } = counted();
    const m = await mountTheme(fake, { snap: snapshot(), filter: "acc" });
    expect(asked).toEqual(["acc"]);
    expect(m.out.h!.surface?.id).toBe(HOST_THEME);
  });

  test("un instantané identique ne relit pas ; filtre, annulation, nom et état relisent", async () => {
    const { fake, asked } = counted();
    const m = await mountTheme(fake, { snap: snapshot(), filter: "" });
    await m.render({ snap: snapshot(), filter: "" });
    expect(asked).toHaveLength(1);
    await m.render({ snap: snapshot(), filter: "taille" });
    await m.render({ snap: snapshot({ undo: "Thème : accent" }), filter: "taille" });
    await m.render({ snap: snapshot({ undo: "Thème : accent", name: "Maison", dirty: true }), filter: "taille" });
    expect(asked).toEqual(["", "taille", "taille", "taille"]);
  });

  test("hors mode modèles ou pendant l'historique : aucune surface, aucune lecture", async () => {
    const { fake, asked } = counted();
    const m = await mountTheme(fake, { snap: snapshot({ history: true }), filter: "" });
    expect(m.out.h!.surface).toBeNull();
    await m.render({ snap: snapshot({ editing: false }), filter: "" });
    expect(m.out.h!.surface).toBeNull();
    expect(asked).toEqual([]);
  });

  test("une valeur part en host_token set", async () => {
    const { fake } = counted();
    const sent: unknown[] = [];
    stub(fake, "hostToken", async (...a: unknown[]) => (sent.push(a), snapshot()));
    const m = await mountTheme(fake, { snap: snapshot(), filter: "" });
    m.out.h!.onAction(m.out.h!.surface!, action("token", { token: ["color", "accent"], action: "set", value: "#112233" }));
    await wait(10);
    expect(sent).toEqual([[["color", "accent"], "set", "#112233"]]);
  });

  test("un changement de liste part comme valeur du jeton", async () => {
    const { fake } = counted();
    const sent: unknown[] = [];
    stub(fake, "hostToken", async (...a: unknown[]) => (sent.push(a), snapshot()));
    const m = await mountTheme(fake, { snap: snapshot(), filter: "" });
    const change = { op: "remove", item: "1" };
    m.out.h!.onAction(m.out.h!.surface!, action("token", { token: ["font", "body"], action: "set", value: null, change }));
    await wait(10);
    expect(sent).toEqual([[["font", "body"], "set", change]]);
  });

  test("Ajouter la couleur : le nom prolonge le chemin", async () => {
    const { fake } = counted();
    const sent: unknown[] = [];
    stub(fake, "hostToken", async (...a: unknown[]) => (sent.push(a), snapshot()));
    const m = await mountTheme(fake, { snap: snapshot(), filter: "" });
    m.out.h!.onAction(m.out.h!.surface!, action("token", { token: ["color"], action: "set", name: "brand", value: "#abcdef" }));
    await wait(10);
    expect(sent).toEqual([[["color", "brand"], "set", "#abcdef"]]);
  });

  test("↺, Retirer, Vider partent sans valeur", async () => {
    const { fake } = counted();
    const sent: unknown[] = [];
    stub(fake, "hostToken", async (...a: unknown[]) => (sent.push(a), snapshot()));
    const m = await mountTheme(fake, { snap: snapshot(), filter: "" });
    for (const verb of ["reset", "remove", "clear"]) {
      m.out.h!.onAction(m.out.h!.surface!, action("token", { token: ["size", "title"], action: verb }));
    }
    await wait(10);
    expect(sent).toEqual([
      [["size", "title"], "reset", undefined],
      [["size", "title"], "remove", undefined],
      [["size", "title"], "clear", undefined],
    ]);
  });

  test("set puis reset du même jeton, libellés et état inchangés : la surface est relue après chaque action acceptée", async () => {
    const { fake } = memory({});
    let version = 0;
    const asked: string[] = [];
    // Chaque lecture rend un contenu différent (la valeur du jeton a changé côté session).
    stub(fake, "hostTheme", async (filter: string) => {
      asked.push(filter);
      return { ...surface(), messages: [{ v: version }] } as unknown as Surface;
    });
    // Instantané identique avant et après : même libellé d'annulation, toujours `dirty`.
    const same = snapshot({ undo: "Thème : Accent", dirty: true });
    stub(fake, "hostToken", async () => (version++, same));
    const m = await mountTheme(fake, { snap: same, filter: "" });
    m.out.h!.onAction(m.out.h!.surface!, action("token", { token: ["color", "accent"], action: "set", value: "#112233" }));
    await wait(10);
    expect(asked).toHaveLength(2);
    m.out.h!.onAction(m.out.h!.surface!, action("token", { token: ["color", "accent"], action: "reset" }));
    expect(await m.out.h!.outcome.current).toBe(true);
    await wait(10);
    expect(asked).toHaveLength(3);
    expect(m.out.h!.surface!.messages).toEqual([{ v: 2 }] as never);
  });

  test("le champ de filtre de la surface passe par onFilter, sans requête", async () => {
    const { fake } = counted();
    const m = await mountTheme(fake, { snap: snapshot(), filter: "" });
    expect(m.out.h!.onAction(m.out.h!.surface!, action("filter", { filter: "Taille " }))).toBe(true);
    expect(m.out.filters).toEqual(["Taille "]);
    expect(m.out.runs).toBe(0);
  });

  test("un refus de la session relit la surface et l'issue est fausse", async () => {
    const { fake, asked } = counted();
    stub(fake, "hostToken", async () => {
      throw new Error("refusé");
    });
    const m = await mountTheme(fake, { snap: snapshot(), filter: "" });
    const before = m.out.h!.surface!;
    m.out.h!.onAction(before, action("token", { token: ["color", "accent"], action: "clear" }));
    expect(await m.out.h!.outcome.current).toBe(false);
    await wait(10);
    expect(m.out.toasts).toEqual(["Error: refusé"]);
    expect(asked).toHaveLength(2);
    expect(m.out.h!.surface).not.toBe(before);
  });

  test("une surface d'une autre origine n'est pas traitée ; une surface remplacée n'envoie rien", async () => {
    const { fake } = counted();
    const m = await mountTheme(fake, { snap: snapshot(), filter: "" });
    expect(m.out.h!.onAction(surface("show-1"), action("token", { token: ["a"], action: "set" }))).toBe(false);
    expect(m.out.h!.onAction(surface(), action("token", { token: ["a"], action: "set" }))).toBe(true);
    expect(await m.out.h!.outcome.current).toBe(false);
    expect(m.out.runs).toBe(0);
  });
});
