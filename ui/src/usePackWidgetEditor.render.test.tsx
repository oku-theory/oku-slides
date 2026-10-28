// Sous-onglets de widgets du mode pack, assemblés pour `App`.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { Snapshot, TabApi } from "./api";
import type { PackKind } from "./packTypes";

const { act, createElement, Fragment, StrictMode, useState } = await import("react");
const { createRoot } = await import("react-dom/client");
const { EditorView } = await import("@codemirror/view");
const { isMac } = await import("./api");
const { PREVIEW_DELAY_MS } = await import("./debounce");
const { usePackWidgetEditor } = await import("./usePackWidgetEditor");
const { runCloseGuard, useCloseGuard } = await import("./closeGuards");
const { t } = await import("./i18n");
const WidgetPane = (await import("./WidgetPane")).default;
const PackTabs = (await import("./PackTabs")).default;
const { deferred, memory, stub, wait } = await import("./test/widgetApi");

type Widgets = ReturnType<typeof usePackWidgetEditor>;
let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
const listeners: ((e: KeyboardEvent) => void)[] = [];
afterEach(async () => {
  listeners.splice(0).forEach((l) => window.removeEventListener("keydown", l));
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

/** Un snapshot réduit à ce que lit le hook. */
function snapOf(o: { set?: string | null; version?: string; base?: string; widgets?: string[]; widget?: string | null; history?: boolean; kind?: PackKind }): Snapshot {
  const { set = "p", version = "v", base = "b", widgets = [], widget = null, history = false, kind = "widgets" } = o;
  return {
    editing: set === null ? null : { pack: set, base, kind },
    widgets: { deck: { id: "p", version } },
    history: history ? {} : null,
    ui: { slide: null, grid: true, ghosts: false, pile: null, notes: null, widgets, widget },
  } as unknown as Snapshot;
}

interface Out {
  w: Widgets | null;
  setSnap: ((s: Snapshot) => void) | null;
  /** À chaque rendu : en mode pack, et ce qui serait gardé. */
  renders: { editing: boolean; widgets: string[] }[];
  blocked: boolean;
  notices: string[];
}

/** Each mount's close guard tab, unique to it (App registers `widgets.files.settle` the same way). */
let guardTabs = 0;

async function mount(fake: TabApi, first: Snapshot, strict = false) {
  const guardTab = `probe-${++guardTabs}`;
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const out: Out = { w: null, setSnap: null, renders: [], blocked: false, notices: [] };
  function Probe() {
    const [snap, setSnap] = useState(first);
    out.setSnap = setSnap;
    const w = usePackWidgetEditor(fake, snap, {
      changed: () => {},
      error: () => {},
      notice: (m: string) => out.notices.push(m),
      blocked: () => out.blocked,
    });
    out.w = w;
    useCloseGuard(guardTab, () => w.files.settle());
    out.renders.push({ editing: !!snap.editing, widgets: w.kept.widgets });
    return createElement(
      Fragment,
      null,
      createElement(PackTabs, { kind: snap.editing?.kind ?? "widgets", tabs: w.tabs.tabs, active: w.tabs.active, unsaved: w.files.unsaved, onActivate: w.activate, onClose: w.close }),
      createElement(WidgetPane, { api: fake, widgets: w, trials: snap.widget_trials ?? 0, look: "", code: false, codeWidth: null, onCodeWidth: () => {}, run: async () => null, palette: [] }),
    );
  }
  const probe = createElement(Probe);
  await act(async () => root.render(strict ? createElement(StrictMode, null, probe) : probe));
  await wait(10);
  const render = async (s: Snapshot) => {
    await act(async () => out.setSnap!(s));
    await wait(10);
  };
  const names = () => out.w!.tabs.tabs.map((t) => t.name);
  const idOf = (name: string) => out.w!.tabs.tabs.find((t) => t.name === name)!.id;
  const content = (name: string) => host.querySelector(`#widget-panel-${idOf(name)} .cm-content`) as HTMLElement;
  const view = (name: string) => EditorView.findFromDOM(content(name))!;
  const typeIn = (name: string, insert: string) =>
    act(async () => {
      const v = view(name);
      v.dispatch({ changes: { from: 0, to: v.state.doc.length, insert }, userEvent: "input.type" });
    });
  const text = (name: string) => content(name)?.textContent;
  return { host, out, render, names, idOf, view, typeIn, text, guardTab };
}

/** Une touche avec ⌘ (Ctrl hors macOS) ; vrai si un écouteur `window` en bulle l'a reçue. */
async function press(target: EventTarget, key: string, shift = false) {
  let seen = false;
  const l = () => {
    seen = true;
  };
  listeners.push(l);
  window.addEventListener("keydown", l);
  await act(async () => {
    target.dispatchEvent(new KeyboardEvent("keydown", { key, metaKey: isMac, ctrlKey: !isMac, shiftKey: shift, bubbles: true, cancelable: true }));
  });
  return seen;
}

const later = () => wait(PREVIEW_DELAY_MS + 50);

describe("usePackWidgetEditor, revision", () => {
  test("usePackWidgetEditor layout pin change gives a new revision", async () => {
    const { fake } = memory({});
    const snap = snapOf({});
    const m = await mount(fake, snap);
    const before = m.out.w!.revision;
    await act(async () => m.out.setSnap!({ ...snap, layouts: { deck: { id: "p", version: "v2" } } } as Snapshot));
    expect(m.out.w!.revision).not.toBe(before);
  });
});

describe("usePackWidgetEditor, reprise", () => {
  test("reprise : les sous-onglets gardés sont rouverts, l'actif aussi, une fois les statuts lus", async () => {
    const { fake } = memory({ kpi: "k", chart: "c" });
    const gate = deferred<void>();
    const real = fake.widgetStatus;
    stub(fake, "widgetStatus", async () => {
      await gate.promise;
      return real();
    });
    const m = await mount(fake, snapOf({ widgets: ["kpi", "chart"], widget: "chart" }));
    expect(m.names()).toEqual([]);
    gate.resolve();
    await wait(10);
    expect(m.names()).toEqual(["kpi", "chart"]);
    expect(m.out.w!.activeName).toBe("chart");
    expect(m.out.w!.kept).toEqual({ widgets: ["kpi", "chart"], widget: "chart" });
  });

  test("reprise : un snapshot sans sous-onglets arrivé avant les statuts ne l'empêche pas", async () => {
    const { fake } = memory({ kpi: "k" });
    const gate = deferred<void>();
    const real = fake.widgetStatus;
    stub(fake, "widgetStatus", async () => {
      await gate.promise;
      return real();
    });
    const m = await mount(fake, snapOf({ widgets: ["kpi"], widget: "kpi" }));
    // L'hôte a été relu entre-temps (par exemple après une sauvegarde de l'état d'interface).
    await m.render(snapOf({ widgets: [], widget: null }));
    gate.resolve();
    await wait(10);
    expect(m.names()).toEqual(["kpi"]);
    expect(m.out.w!.activeName).toBe("kpi");
  });

  test("avant les statuts, kept rend les sous-onglets de l'hôte (jamais vide)", async () => {
    const { fake } = memory({ kpi: "k" });
    const gate = deferred<void>();
    const real = fake.widgetStatus;
    stub(fake, "widgetStatus", async () => {
      await gate.promise;
      return real();
    });
    const m = await mount(fake, snapOf({ widgets: ["kpi"], widget: "kpi" }));
    expect(m.names()).toEqual([]);
    expect(m.out.w!.kept).toEqual({ widgets: ["kpi"], widget: "kpi" });
    expect(m.out.renders.every((r) => r.widgets.length === 1)).toBe(true);
    gate.resolve();
    await wait(10);
    expect(m.out.w!.kept).toEqual({ widgets: ["kpi"], widget: "kpi" });
  });

  test("reprise faite une fois par pack : un sous-onglet fermé ne revient pas", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, snapOf({ widgets: ["kpi"], widget: "kpi" }));
    await act(async () => m.out.w!.close(m.idOf("kpi")));
    await m.render(snapOf({ version: "v2", widgets: ["kpi"], widget: "kpi" }));
    expect(m.names()).toEqual([]);
  });

  test("reprise : un nom absent des statuts est retiré ; aucune source n'est lue pour lui", async () => {
    const { fake } = memory({ kpi: "k" });
    const asked: string[] = [];
    stub(fake, "widgetSource", async (name: string) => {
      asked.push(name);
      return name === "kpi" ? "k" : "";
    });
    const m = await mount(fake, snapOf({ widgets: ["kpi", "gone"], widget: "gone" }));
    expect(m.names()).toEqual(["kpi"]);
    expect(m.out.w!.activeName).toBeNull();
    expect(asked).toEqual(["kpi"]);
  });

  test("des statuts d'une autre revision n'élaguent pas", async () => {
    const files: Record<string, string> = {};
    const { fake } = memory(files);
    const m = await mount(fake, snapOf({}));
    // L'agent crée `kpi` : nouvelle revision, ses statuts tardent.
    files.kpi = "k";
    const gate = deferred<void>();
    const real = fake.widgetStatus;
    stub(fake, "widgetStatus", async () => {
      await gate.promise;
      return real();
    });
    await m.render(snapOf({ version: "v2" }));
    await act(async () => m.out.w!.open("kpi"));
    expect(m.names()).toEqual(["kpi"]);
    gate.resolve();
    await wait(10);
    expect(m.names()).toEqual(["kpi"]);
    expect(m.out.w!.activeName).toBe("kpi");
  });

  test("sortie du mode pack : kept vide dès le même rendu ; plus de sous-onglets", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, snapOf({ widgets: ["kpi"], widget: "kpi" }));
    expect(m.names()).toEqual(["kpi"]);
    await m.render(snapOf({ set: null }));
    const outside = m.out.renders.filter((r) => !r.editing);
    expect(outside.length).toBeGreaterThan(0);
    expect(outside.every((r) => r.widgets.length === 0)).toBe(true);
    expect(m.names()).toEqual([]);
    expect(m.out.w!.activeName).toBeNull();
  });

  test("autre pack : sous-onglets remis à zéro", async () => {
    const { fake } = memory({ kpi: "k", chart: "c" });
    const m = await mount(fake, snapOf({ widgets: ["kpi"], widget: "kpi" }));
    expect(m.names()).toEqual(["kpi"]);
    await m.render(snapOf({ set: "q", base: "b2", widgets: ["chart"], widget: null }));
    // La reprise est réarmée pour le nouveau pack.
    expect(m.names()).toEqual(["chart"]);
    expect(m.out.w!.activeName).toBeNull();
  });
});

describe("usePackWidgetEditor, a pack the host opened", () => {
  test("entering a pack whose host opened a widget sub-tab shows that sub-tab", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, snapOf({ set: null }));
    await m.render(snapOf({ widgets: ["kpi"], widget: "kpi" }));
    expect(m.names()).toEqual(["kpi"]);
    expect(m.out.w!.activeName).toBe("kpi");
  });

  test("settle writes the pending typing of a hidden sub-tab", async () => {
    const { fake, log } = memory({ kpi: "k", chart: "c" });
    const m = await mount(fake, snapOf({ widgets: ["kpi", "chart"], widget: "kpi" }));
    await m.typeIn("kpi", "k2");
    await act(async () => m.out.w!.activate(m.idOf("chart")));
    expect(m.out.w!.activeName).toBe("chart");
    expect(log).toEqual([]);
    await act(async () => m.out.w!.files.settle());
    expect(log).toEqual(["set kpi:k2"]);
  });
});

describe("usePackWidgetEditor, fermer, supprimer, renommer", () => {
  test("close envoie la frappe en attente du fichier", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const m = await mount(fake, snapOf({ widgets: ["kpi"], widget: "kpi" }));
    await m.typeIn("kpi", "k2");
    expect(log).toEqual([]);
    await act(async () => m.out.w!.close(m.idOf("kpi")));
    await wait(0);
    expect(log).toEqual(["set kpi:k2"]);
    expect(m.names()).toEqual([]);
  });

  test("removed ferme le sous-onglet", async () => {
    const { fake } = memory({ kpi: "k", chart: "c" });
    const m = await mount(fake, snapOf({ widgets: ["kpi", "chart"], widget: "chart" }));
    await act(async () => m.out.w!.removed("chart"));
    expect(m.names()).toEqual(["kpi"]);
    expect(m.out.w!.activeName).toBe("kpi");
  });

  test("un renommage garde l'id et le sous-onglet n'est pas élagué", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, snapOf({ widgets: ["kpi"], widget: "kpi" }), true);
    const id = m.idOf("kpi");
    const from = m.out.renders.length;
    await act(async () => {
      await m.out.w!.files.rename("kpi", "kpi2");
    });
    await wait(10);
    expect(m.out.w!.tabs.tabs).toEqual([{ id, name: "kpi2" }]);
    expect(m.out.renders.slice(from).every((r) => r.widgets.length === 1)).toBe(true);
  });
});

describe("usePackWidgetEditor, annuler et rétablir", () => {
  test("undo/redo appellent les commandes de l'actif seulement ; history suit l'actif", async () => {
    const { fake } = memory({ kpi: "k", chart: "c" });
    const m = await mount(fake, snapOf({ widgets: ["kpi", "chart"], widget: "kpi" }));
    await m.typeIn("kpi", "k2");
    expect(m.out.w!.history).toEqual({ undo: 1, redo: 0 });
    await act(async () => m.out.w!.activate(m.idOf("chart")));
    expect(m.out.w!.history).toEqual({ undo: 0, redo: 0 });
    await act(async () => m.out.w!.undo());
    expect(m.text("kpi")).toBe("k2");
    expect(m.text("chart")).toBe("c");
    await act(async () => m.out.w!.activate(m.idOf("kpi")));
    expect(m.out.w!.history).toEqual({ undo: 1, redo: 0 });
    await act(async () => m.out.w!.undo());
    expect(m.text("kpi")).toBe("k");
    expect(m.out.w!.history).toEqual({ undo: 0, redo: 1 });
    await act(async () => m.out.w!.redo());
    expect(m.text("kpi")).toBe("k2");
    await later();
  });
});

describe("usePackWidgetEditor, raccourcis", () => {
  test("⌘W sur un sous-onglet widget le ferme et ne se propage pas", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, snapOf({ widgets: ["kpi"], widget: "kpi" }));
    expect(await press(document.body, "w")).toBe(false);
    expect(m.names()).toEqual([]);
  });

  test("⌘W sur Modèles se propage", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, snapOf({ widgets: ["kpi"], widget: null }));
    expect(await press(document.body, "w")).toBe(true);
    expect(m.names()).toEqual(["kpi"]);
  });

  test("⌘W avec on.blocked() : se propage", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, snapOf({ widgets: ["kpi"], widget: "kpi" }));
    m.out.blocked = true;
    expect(await press(document.body, "w")).toBe(true);
    expect(m.names()).toEqual(["kpi"]);
  });

  test("⌘W dans un champ hors de l'éditeur se propage", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, snapOf({ widgets: ["kpi"], widget: "kpi" }));
    const input = document.createElement("input");
    m.host.append(input);
    expect(await press(input, "w")).toBe(true);
    expect(m.names()).toEqual(["kpi"]);
  });

  test("⌘W avec le focus dans l'éditeur ferme aussi le sous-onglet", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, snapOf({ widgets: ["kpi"], widget: "kpi" }));
    const editor = m.view("kpi").contentDOM;
    expect(await press(editor, "w")).toBe(false);
    expect(m.names()).toEqual([]);
  });

  test("⌘Z hors de l'éditeur, sous-onglet widget actif : undo de l'éditeur, ne se propage pas", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, snapOf({ widgets: ["kpi"], widget: "kpi" }));
    await m.typeIn("kpi", "k2");
    expect(await press(document.body, "z")).toBe(false);
    expect(m.text("kpi")).toBe("k");
    expect(await press(document.body, "z", true)).toBe(false);
    expect(m.text("kpi")).toBe("k2");
    await later();
  });

  test("⌘Z sur Modèles se propage", async () => {
    const { fake } = memory({ kpi: "k" });
    await mount(fake, snapOf({ widgets: ["kpi"], widget: null }));
    expect(await press(document.body, "z")).toBe(true);
  });
});

describe("usePackWidgetEditor, close guard", () => {
  test("regression: a hidden sub-tab's pending typing is written before the close guard answers", async () => {
    const { fake, log } = memory({ kpi: "k", chart: "c" });
    const m = await mount(fake, snapOf({ widgets: ["kpi", "chart"], widget: "kpi" }));
    await m.typeIn("kpi", "k2");
    await act(async () => m.out.w!.activate(m.idOf("chart")));
    await act(async () => {
      expect(await runCloseGuard(m.guardTab)).toBe("done");
      log.push("close");
    });
    expect(log).toEqual(["set kpi:k2", "close"]);
  });
});

describe("usePackWidgetEditor, revue finale", () => {
  test("relecture en vol puis renommage : le sous-onglet renommé n'est pas élagué", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, snapOf({ widgets: ["kpi"], widget: "kpi" }));
    const id = m.idOf("kpi");
    const real = fake.widgetStatus;
    const before = await real();
    const slow = deferred<void>();
    let first = true;
    stub(fake, "widgetStatus", async () => {
      if (!first) return real();
      first = false;
      await slow.promise;
      return before;
    });
    await m.render(snapOf({ version: "v2", widgets: ["kpi"], widget: "kpi" }));
    await act(async () => {
      await m.out.w!.files.rename("kpi", "kpi2");
    });
    await act(async () => slow.resolve());
    await wait(10);
    expect(m.out.w!.tabs.tabs).toEqual([{ id, name: "kpi2" }]);
    expect(m.out.w!.activeName).toBe("kpi2");
  });

  test("Suppr sur un sous-onglet : le focus va au sous-onglet devenu actif", async () => {
    const { fake } = memory({ kpi: "k", chart: "c" });
    const m = await mount(fake, snapOf({ widgets: ["kpi", "chart"], widget: "chart" }));
    const tab = m.host.querySelector<HTMLElement>(`#widget-tab-${m.idOf("chart")}`)!;
    const kpi = m.idOf("kpi");
    tab.focus();
    await act(async () => {
      tab.dispatchEvent(new KeyboardEvent("keydown", { key: "Delete", bubbles: true, cancelable: true }));
    });
    expect(m.names()).toEqual(["kpi"]);
    expect(document.activeElement?.id).toBe(`widget-tab-${kpi}`);
  });

  test("usePackWidgetEditor closing the last widget sub-tab of a widget pack leaves the focus alone", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, snapOf({ widgets: ["kpi"], widget: "kpi" }));
    const editor = m.view("kpi").contentDOM;
    editor.focus();
    await press(editor, "w");
    expect(m.names()).toEqual([]);
    expect(document.activeElement?.id ?? "").not.toMatch(/^widget-tab-/);
  });

  test("⌘W depuis l'éditeur du dernier sous-onglet d'un pack de layouts : le focus va à « Layouts »", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, snapOf({ widgets: ["kpi"], widget: "kpi", kind: "layouts" }));
    const editor = m.view("kpi").contentDOM;
    editor.focus();
    await press(editor, "w");
    expect(document.activeElement?.id).toBe("widget-tab-0");
  });
});

describe("usePackWidgetEditor, l'agent écrit", () => {
  test("touched ouvre et active le widget écrit", async () => {
    const { fake } = memory({ kpi: "k", quote: "q" });
    const m = await mount(fake, snapOf({}));
    await act(async () => m.out.w!.touched("quote", false));
    expect(m.out.w!.activeName).toBe("quote");
  });

  test("touched ouvre derrière si l'actif a des frappes en attente", async () => {
    const { fake } = memory({ kpi: "k", quote: "q" });
    const m = await mount(fake, snapOf({}));
    await act(async () => m.out.w!.open("kpi"));
    await wait(10);
    await m.typeIn("kpi", "k2"); // < PREVIEW_DELAY_MS (400 ms) : la frappe attend encore
    await act(async () => m.out.w!.touched("quote", false));
    expect(m.out.w!.activeName).toBe("kpi");
    expect(m.names()).toEqual(["kpi", "quote"]);
  });

  test("après Changed puis touched, les frappes en attente l'emportent sur l'agent", async () => {
    const mem = memory({ kpi: "k" });
    const m = await mount(mem.fake, snapOf({}));
    await act(async () => m.out.w!.open("kpi"));
    await wait(10);
    await m.typeIn("kpi", "tapé"); // < PREVIEW_DELAY_MS : en attente au moment de touched
    mem.files.kpi = "agent";
    await m.render(snapOf({ version: "v2" }));
    await act(async () => m.out.w!.touched("kpi", false));
    await later();
    expect(mem.files.kpi).toBe("tapé");
  });

  test("touched renvoie la dernière frappe refusée par-dessus l'écriture de l'agent", async () => {
    const mem = memory({ kpi: "k" });
    const real = mem.fake.setWidget;
    let refuse = true;
    stub(mem.fake, "setWidget", async (name: string, text: string) => {
      if (!refuse) return real(name, text);
      refuse = false;
      return { status: { name, schema: null, diagnostics: [] }, broken: [], saved: false };
    });
    const m = await mount(mem.fake, snapOf({}));
    await act(async () => m.out.w!.open("kpi"));
    await wait(10);
    await m.typeIn("kpi", "tapé");
    await later(); // la frappe part et est refusée : non enregistrée
    mem.files.kpi = "agent";
    await act(async () => m.out.w!.touched("kpi", false));
    await later();
    expect(mem.files.kpi).toBe("tapé");
  });

  test("après touched, la relecture ne remet pas le texte de l'agent", async () => {
    const mem = memory({ kpi: "k" });
    const m = await mount(mem.fake, snapOf({}));
    await act(async () => m.out.w!.open("kpi"));
    await wait(10);
    await m.typeIn("kpi", "tapé"); // < PREVIEW_DELAY_MS : en attente au moment de touched
    mem.files.kpi = "agent";
    await act(async () => m.out.w!.touched("kpi", false));
    await later();
    await m.render(snapOf({ version: "v3" }));
    expect(m.text("kpi")).toBe("tapé");
  });

  test("touched d'un widget nouveau l'ouvre une fois la liste relue", async () => {
    const mem = memory({ kpi: "k" });
    const m = await mount(mem.fake, snapOf({}));
    mem.files.carte = "c";
    await act(async () => m.out.w!.touched("carte", false));
    await m.render(snapOf({ version: "v2" }));
    expect(m.names()).toEqual(["carte"]);
  });

  test("touched removed ferme le sous-onglet", async () => {
    const mem = memory({ kpi: "k" });
    const m = await mount(mem.fake, snapOf({}));
    await act(async () => m.out.w!.open("kpi"));
    await wait(10);
    delete mem.files.kpi;
    await act(async () => m.out.w!.touched("kpi", true));
    expect(m.names()).toEqual([]);
  });

  test("touched removed abandonne les frappes en attente", async () => {
    const mem = memory({ kpi: "k" });
    const m = await mount(mem.fake, snapOf({}));
    await act(async () => m.out.w!.open("kpi"));
    await wait(10);
    await m.typeIn("kpi", "tapé"); // < PREVIEW_DELAY_MS : en attente au moment de touched
    delete mem.files.kpi;
    await act(async () => m.out.w!.touched("kpi", true));
    await later();
    expect(mem.log.some((l) => l.startsWith("set kpi:"))).toBe(false);
  });

  test("touched removed prévient l'utilisateur", async () => {
    const mem = memory({ kpi: "k" });
    const m = await mount(mem.fake, snapOf({}));
    await act(async () => m.out.w!.touched("kpi", true));
    expect(m.out.notices).toEqual([t("ui-widgets-removed-by-agent", { name: "kpi" })]);
  });

  test("hors mode pack, touched ne laisse rien à ouvrir plus tard", async () => {
    const mem = memory({ kpi: "k" });
    const m = await mount(mem.fake, snapOf({ set: null }));
    await act(async () => m.out.w!.touched("kpi", false));
    await m.render(snapOf({}));
    expect(m.names()).toEqual([]);
  });

  test("l'aperçu actif est redemandé quand la révision des essais change", async () => {
    const { fake, calls } = memory({ kpi: "k" });
    const m = await mount(fake, snapOf({}));
    await act(async () => m.out.w!.open("kpi"));
    await wait(10);
    const before = calls.filter((c) => c === "preview").length;
    await m.render({ ...snapOf({}), widget_trials: 1 } as Snapshot);
    expect(calls.filter((c) => c === "preview").length).toBe(before + 1);
  });
});
