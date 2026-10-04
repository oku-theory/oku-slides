// Sous-onglet d'un fichier de widget : source, diagnostics, aperçu, Commun.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { TabApi, WidgetStatus } from "./api";
import { codeTrack } from "./codeSplit";
import type { CodeLayout } from "./useViewOptions";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { EditorView } = await import("@codemirror/view");
const { PREVIEW_DELAY_MS } = await import("./debounce");
const { useWidgetFiles } = await import("./useWidgetFiles");

const BELOW: CodeLayout = { code_place: "below", code_height: null, code_width: null };
const WidgetTab = (await import("./WidgetTab")).default;
const { deferred, memory, stub, wait } = await import("./test/widgetApi");

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

interface Shown { name: string; active?: boolean; revision?: string; trials?: number; code?: boolean; layout?: CodeLayout; look?: string }

async function mount(fake: TabApi, first: Shown) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const opened: string[] = [];
  const layouts: CodeLayout[] = [];
  const files: { current: ReturnType<typeof useWidgetFiles> | null } = { current: null };
  function Probe({ name, active = true, revision = "r", trials = 0, code = true, layout = BELOW, look = "clair" }: Shown) {
    const f = useWidgetFiles(fake, "p", revision, { changed: () => {}, error: () => {}, renamed: () => {} });
    files.current = f;
    return createElement(WidgetTab, {
      api: fake, files: f, id: 1, name, revision, active, trials, code, layout, look, onLayout: (l: CodeLayout) => layouts.push(l),
      onOpen: (n: string) => opened.push(n), onCommands: () => {}, onHistory: () => {},
    });
  }
  const render = (p: Shown) => act(async () => root.render(createElement(Probe, p)));
  await render(first);
  await wait(10);
  const view = () => EditorView.findFromDOM(host.querySelector(".cm-content") as HTMLElement);
  const typeText = (insert: string) =>
    act(async () => {
      const v = view()!;
      v.dispatch({ changes: { from: 0, to: v.state.doc.length, insert }, userEvent: "input.type" });
    });
  const text = () => host.querySelector(".cm-content")?.textContent;
  return { host, render, view, typeText, text, opened, files, layouts };
}
const later = () => wait(PREVIEW_DELAY_MS + 50);
const status = (name: string, error = false): WidgetStatus => ({
  name,
  schema: null,
  diagnostics: error ? [{ file: name, line: 1, column: 1, error: true, message: "x" }] : [],
});

describe("WidgetTab, source", () => {
  test("affiche la source du fichier", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi" });
    expect(m.text()).toBe("k");
  });

  test("les diagnostics de _commun restent visibles sur le widget qui l'importe, avec leur fichier", async () => {
    const { fake } = memory({ kpi: "k" });
    const kpi: WidgetStatus = {
      name: "kpi",
      schema: null,
      diagnostics: [{ file: "_commun", line: 1, column: 1, error: true, message: "x" }],
    };
    stub(fake, "widgetStatus", async () => [kpi]);
    const m = await mount(fake, { name: "kpi" });
    const items = [...m.host.querySelectorAll(".widget-tab-diagnostics li")].map((li) => li.textContent);
    expect(items).toEqual(["_commun:1:1 x"]);
  });

  test("une frappe est écrite après le délai", async () => {
    const { fake, log } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi" });
    await m.typeText("k2");
    await later();
    expect(log).toEqual(["set kpi:k2"]);
  });

  test("une réponse de source arrivée après une frappe ne ramène pas l'ancien texte", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi" });
    const slow = deferred<string>();
    stub(fake, "widgetSource", () => slow.promise);
    await m.render({ name: "kpi", revision: "r1" });
    await m.typeText("k2");
    await later();
    await act(async () => slow.resolve("k"));
    await wait(10);
    expect(m.text()).toBe("k2");
  });

  test("une réponse de source lente, sans écriture partie mais après une frappe, est ignorée", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi" });
    const slow = deferred<string>();
    stub(fake, "widgetSource", () => slow.promise);
    await m.render({ name: "kpi", revision: "r1" });
    await m.typeText("k2");
    await act(async () => slow.resolve("k"));
    await wait(10);
    expect(m.text()).toBe("k2");
    await later();
    expect(m.text()).toBe("k2");
  });

  test("le pack changé ailleurs (l'agent) : la source affichée est relue", async () => {
    const files = { kpi: "k" };
    const { fake, calls } = memory(files);
    const m = await mount(fake, { name: "kpi" });
    files.kpi = "agent";
    const previews = calls.filter((c) => c === "preview").length;
    await m.render({ name: "kpi", revision: "r1" });
    await wait(10);
    expect(m.text()).toBe("agent");
    expect(calls.filter((c) => c === "preview").length).toBe(previews + 1);
  });

  test("le style du deck changé (thème) : l'aperçu est redemandé", async () => {
    const { fake, calls } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi" });
    const previews = calls.filter((c) => c === "preview").length;
    await m.render({ name: "kpi", look: "sombre" });
    await wait(10);
    expect(calls.filter((c) => c === "preview").length).toBe(previews + 1);
  });

  test("une écriture refusée : le texte tapé reste, même après relecture", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi" });
    stub(fake, "setWidget", async () => {
      throw new Error("refusé");
    });
    await m.typeText("k2");
    await later();
    await m.render({ name: "kpi", revision: "r1" });
    await wait(10);
    expect(m.text()).toBe("k2");
  });

  test("une frappe pas encore envoyée n'est pas écrasée par une relecture demandée après elle", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi" });
    await m.typeText("k2");
    await m.render({ name: "kpi", revision: "r1" });
    await wait(10);
    expect(m.text()).toBe("k2");
  });

  test("désactivé pendant une relecture : la lecture reprend à l'activation", async () => {
    const files = { kpi: "k" };
    const { fake } = memory(files);
    const m = await mount(fake, { name: "kpi" });
    const slow = deferred<string>();
    stub(fake, "widgetSource", () => slow.promise);
    files.kpi = "agent";
    await m.render({ name: "kpi", revision: "r1" });
    await m.render({ name: "kpi", revision: "r1", active: false });
    await act(async () => slow.resolve("agent"));
    stub(fake, "widgetSource", async () => files.kpi);
    await m.render({ name: "kpi", revision: "r1", active: true });
    await wait(10);
    expect(m.text()).toBe("agent");
  });

  test("le panneau porte les attributs ARIA du sous-onglet", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi" });
    const panel = m.host.querySelector("#widget-panel-1");
    expect(panel?.getAttribute("role")).toBe("tabpanel");
    expect(panel?.getAttribute("aria-labelledby")).toBe("widget-tab-1");
  });

  test("nom changé (renommage) : même vue CodeMirror, remplacement appliqué, pas de relecture", async () => {
    const { fake, calls } = memory({ kpi: 'widget("kpi", a)' });
    const m = await mount(fake, { name: "kpi" });
    const before = m.view();
    await m.typeText('widget("kpi", b)');
    const sources = calls.filter((c) => c === "source").length;
    await act(async () => {
      await m.files.current!.rename("kpi", "kpi2");
    });
    await m.render({ name: "kpi2" });
    await wait(10);
    expect(m.view()).toBe(before);
    expect(m.text()).toContain('widget("kpi2"');
    expect(calls.filter((c) => c === "source").length).toBe(sources);
  });
});

describe("WidgetTab, aperçu", () => {
  test("un aperçu ancien et lent n'écrase pas un plus récent", async () => {
    const { fake } = memory({ kpi: "k" });
    const first = deferred<{ svg: string; diagnostics: never[] }>();
    let n = 0;
    stub(fake, "previewWidget", () => {
      n += 1;
      return n === 1 ? first.promise : Promise.resolve({ svg: "<svg id='new'/>", diagnostics: [] });
    });
    const m = await mount(fake, { name: "kpi" });
    await m.typeText("k2");
    await later();
    const src = m.host.querySelector("img")?.getAttribute("src");
    expect(src).toBeTruthy();
    await act(async () => first.resolve({ svg: "<svg id='old'/>", diagnostics: [] }));
    await wait(10);
    expect(m.host.querySelector("img")?.getAttribute("src")).toBe(src);
  });

  test("un widget d'élément est cadré sur sa boîte", async () => {
    const blobs: Blob[] = [];
    const create = URL.createObjectURL;
    URL.createObjectURL = (b: Blob | MediaSource) => {
      blobs.push(b as Blob);
      return "blob:apercu";
    };
    try {
      const { fake } = memory({ kpi: "k" });
      stub(fake, "previewWidget", async () => ({
        svg: '<svg viewBox="0 0 960 540" width="960pt" height="540pt"></svg>',
        bbox: { x: 100, y: 100, w: 200, h: 100 },
        diagnostics: [],
      }));
      const m = await mount(fake, { name: "kpi" });
      expect(m.host.querySelector("img")?.getAttribute("src")).toBe("blob:apercu");
      const svg = await blobs.at(-1)?.text();
      // Au plus près : la vue est la boîte, sans marge.
      expect(/viewBox="([^"]*)"/.exec(svg ?? "")?.[1]).toBe("100 100 200 100");
    } finally {
      URL.createObjectURL = create;
    }
  });

  test("après une écriture : un seul aperçu redemandé", async () => {
    const { fake, calls } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi" });
    const before = calls.filter((c) => c === "preview").length;
    const statusBefore = calls.filter((c) => c === "status").length;
    await m.typeText("k2");
    await later();
    await m.render({ name: "kpi", revision: "r1" });
    await wait(10);
    expect(calls.filter((c) => c === "preview").length).toBe(before + 1);
    expect(calls.filter((c) => c === "status").length).toBe(statusBefore + 1);
  });

  test("inactif : ni aperçu, ni relecture ; activé : relecture puis aperçu", async () => {
    const { fake, calls } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi", active: false });
    expect(calls).not.toContain("preview");
    const count = (c: string) => calls.filter((x) => x === c).length;
    const sources = count("source");
    await m.render({ name: "kpi", active: false, revision: "r1" });
    await wait(10);
    expect(count("source")).toBe(sources);
    expect(count("preview")).toBe(0);
    await m.render({ name: "kpi", active: true, revision: "r1" });
    await wait(10);
    expect(count("source")).toBe(sources + 1);
    expect(count("preview")).toBe(1);
  });
});

describe("WidgetTab, Commun", () => {
  test("liste les widgets en erreur ; un clic appelle onOpen", async () => {
    const { fake } = memory({ _commun: "c", kpi: "k", quote: "q" });
    stub(fake, "widgetStatus", async () => [status("_commun"), status("kpi", true), status("quote")]);
    const m = await mount(fake, { name: "_commun" });
    const buttons = [...m.host.querySelectorAll<HTMLButtonElement>(".widget-tab-broken button")];
    expect(buttons.map((b) => b.textContent)).toEqual(["kpi"]);
    await act(async () => buttons[0]?.click());
    expect(m.opened).toEqual(["kpi"]);
    expect(m.host.querySelector("img")).toBeNull();
  });

  test("sans widget en erreur : le texte ui-widgets-none-in-error", async () => {
    const { fake } = memory({ _commun: "c", kpi: "k" });
    const m = await mount(fake, { name: "_commun" });
    expect(m.host.querySelector(".widget-tab-broken button")).toBeNull();
    expect(m.host.querySelector(".widget-tab-broken .caption")?.textContent).toBe("ui-widgets-none-in-error");
  });
});

describe("WidgetTab, disposition", () => {
  const order = (host: HTMLElement) =>
    [...host.querySelectorAll(".widget-tab > *")].map((el) => el.className);

  test("un widget : l'aperçu, le séparateur, puis le code", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi" });
    expect(order(m.host)).toEqual(["widget-tab-side", "code-divider", "widget-tab-code"]);
  });

  test("Commun : les widgets en erreur, le séparateur, puis le code", async () => {
    const { fake } = memory({ _commun: "c" });
    const m = await mount(fake, { name: "_commun" });
    expect(order(m.host)).toEqual(["widget-tab-side", "code-divider", "widget-tab-code"]);
  });

  test("code replié : pas de séparateur", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi", code: false });
    expect(order(m.host)).toEqual(["widget-tab-side", "widget-tab-code"]);
  });

  test("la place choisie est posée sur le sous-onglet", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi", layout: { ...BELOW, code_place: "right" } });
    expect(m.host.querySelector<HTMLElement>(".widget-tab")!.dataset.place).toBe("right");
  });

  test("les tailles choisies sont posées en pistes bornées sur le sous-onglet", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi", layout: { code_place: "below", code_height: 300, code_width: null } });
    const style = m.host.querySelector<HTMLElement>(".widget-tab")!.style;
    expect([style.getPropertyValue("--code-h"), style.getPropertyValue("--code-w")]).toEqual([codeTrack(300), ""]);
  });

  test("double-clic sur le séparateur, code dessous : hauteur par défaut", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi", layout: { code_place: "below", code_height: 300, code_width: 500 } });
    await act(async () => {
      m.host.querySelector(".code-divider")!.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
    });
    expect(m.layouts).toEqual([{ code_place: "below", code_height: null, code_width: 500 }]);
  });
});

describe("WidgetTab, code source masquable", () => {
  const editor = (host: HTMLElement) => host.querySelector<HTMLElement>(".widget-tab-code")!;

  test("masqué : l'éditeur reste monté mais inerte", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi", code: false });
    expect([editor(m.host).hasAttribute("inert"), m.text()]).toEqual([true, "k"]);
  });

  test("affiché : l'éditeur est actif", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi", code: true });
    expect(editor(m.host).hasAttribute("inert")).toBe(false);
  });

  test("l'éditeur porte l'id que l'interrupteur de la barre désigne", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi" });
    expect(editor(m.host).id).toBe("widget-code-1");
  });
});
