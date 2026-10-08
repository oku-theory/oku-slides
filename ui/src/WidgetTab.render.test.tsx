// Sous-onglet d'un fichier de widget : source, diagnostics, aperçu, Commun.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { FieldView, ItemView, ListView, SlideView, Snapshot, TabApi, WidgetStatus } from "./api";
import { codeTrack, SIDE_MIN, SPLIT } from "./codeSplit";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { EditorView } = await import("@codemirror/view");
const { PREVIEW_DELAY_MS } = await import("./debounce");
const { useWidgetFiles } = await import("./useWidgetFiles");

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

interface Shown { name: string; active?: boolean; revision?: string; trials?: number; code?: boolean; codeWidth?: number | null; look?: string; settings?: boolean }

async function mount(fake: TabApi, first: Shown) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const opened: string[] = [];
  const widths: (number | null)[] = [];
  const files: { current: ReturnType<typeof useWidgetFiles> | null } = { current: null };
  function Probe({ name, active = true, revision = "r", trials = 0, code = true, codeWidth = null, look = "clair", settings = false }: Shown) {
    const f = useWidgetFiles(fake, "p", revision, { changed: () => {}, error: () => {}, renamed: () => {} });
    files.current = f;
    return createElement(WidgetTab, {
      api: fake, files: f, id: 1, name, revision, active, trials, code, codeWidth, look, settings: settings ? createElement("div", { className: "probe-settings" }) : null, onCodeWidth: (w: number | null) => widths.push(w),
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
  return { host, render, view, typeText, text, opened, files, widths };
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

  test("un widget : l'aperçu, puis les réglages, le séparateur et le code", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi" });
    expect(order(m.host)).toEqual(["widget-tab-preview", "widget-tab-side", "code-divider", "widget-tab-code"]);
  });

  test("Commun : les widgets en erreur à la place de l'aperçu", async () => {
    const { fake } = memory({ _commun: "c" });
    const m = await mount(fake, { name: "_commun" });
    expect(m.host.querySelector(".widget-tab-preview > .widget-tab-broken")).not.toBeNull();
  });

  test("les réglages passés sont sous l'aperçu, avant les diagnostics", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi", settings: true });
    const side = [...m.host.querySelectorAll(".widget-tab-side > *")].map((el) => el.className);
    expect(side).toEqual(["probe-settings", "widget-tab-diagnostics"]);
  });

  test("code replié : pas de séparateur", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi", code: false });
    expect(order(m.host)).toEqual(["widget-tab-preview", "widget-tab-side", "widget-tab-code"]);
  });

  test("code déplié : le séparateur entre les réglages et le code est vertical", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi" });
    expect(m.host.querySelector(".code-divider")!.getAttribute("aria-orientation")).toBe("vertical");
  });

  test("la largeur choisie est posée en piste bornée sur le sous-onglet", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi", codeWidth: 300 });
    expect(m.host.querySelector<HTMLElement>(".widget-tab")!.style.getPropertyValue("--code-w")).toBe(codeTrack(300)!);
  });

  /** Sous-onglet de 1000 px dont 400 au code (happy-dom ne mesure rien), et un geste de pointeur sur le séparateur. */
  function measured(host: HTMLElement) {
    const tab = host.querySelector<HTMLElement>(".widget-tab")!;
    Object.defineProperty(tab, "clientWidth", { configurable: true, value: 1000 });
    Object.defineProperty(host.querySelector(".widget-tab-code")!, "offsetWidth", { configurable: true, value: 400 });
    const divider = host.querySelector<HTMLElement>(".code-divider")!;
    divider.setPointerCapture = () => {};
    const pointer = (type: string, at: number) =>
      act(async () => {
        divider.dispatchEvent(new PointerEvent(type, { bubbles: true, button: 0, pointerId: 1, clientX: at, clientY: 0 }));
      });
    return { tab, pointer };
  }

  test("pendant le glisser, la largeur n'est posée que sur le sous-onglet, sans être retenue", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi" });
    const { tab, pointer } = measured(m.host);
    await pointer("pointerdown", 500);
    await pointer("pointermove", 460);
    expect([tab.style.getPropertyValue("--code-w"), m.widths]).toEqual([codeTrack(440)!, []]);
  });

  test("au relâcher, la largeur bornée est retenue une fois", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi" });
    const { pointer } = measured(m.host);
    await pointer("pointerdown", 500);
    await pointer("pointermove", 0);
    await pointer("pointerup", -1000);
    expect(m.widths).toEqual([1000 - SPLIT - SIDE_MIN]);
  });

  test("sans largeur mesurée, le glisser ne règle rien", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi" });
    const divider = m.host.querySelector<HTMLElement>(".code-divider")!;
    divider.setPointerCapture = () => {};
    for (const [type, at] of [["pointerdown", 500], ["pointermove", 460], ["pointerup", 400]] as const)
      await act(async () => {
        divider.dispatchEvent(new PointerEvent(type, { bubbles: true, button: 0, pointerId: 1, clientX: at, clientY: 0 }));
      });
    const tab = m.host.querySelector<HTMLElement>(".widget-tab")!;
    expect([tab.style.getPropertyValue("--code-w"), m.widths]).toEqual(["", []]);
  });

  test("double-clic sur le séparateur : largeur par défaut", async () => {
    const { fake } = memory({ kpi: "k" });
    const m = await mount(fake, { name: "kpi", codeWidth: 500 });
    await act(async () => {
      m.host.querySelector(".code-divider")!.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
    });
    expect(m.widths).toEqual([null]);
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

describe("WidgetTab, valeurs d'exemple", () => {
  test("l'aperçu est demandé au seul nom du widget, ses valeurs d'exemple prises par la session", async () => {
    const { fake } = memory({ kpi: "k" });
    const asked: unknown[][] = [];
    stub(fake, "previewWidget", async (...a: never[]) => (asked.push(a), { svg: null, diagnostics: [] }));
    const m = await mount(fake, { name: "kpi" });
    expect(asked).toEqual([["kpi"]]);
    // Rien avant l'aperçu dans sa zone.
    expect(m.host.querySelector(".widget-tab-preview [role=group]")).toBeNull();
  });
});

describe("WidgetTab, scène", () => {
  /** Un champ de l'élément d'aperçu du widget « kpi », en pt de la slide. */
  const field = (name: string, kind: "plain" | "rich", text: string, x: number, y: number, extra: Partial<FieldView> = {}): FieldView => ({
    owner: "apercu-el", owner_kind: "element", widget: "kpi", field: name, kind, label: null, text, empty: false, bound: false,
    side: null, x, y, w: 100, h: 20, ax: x, aw: 300, style: null, ...extra,
  });
  const slideView = (fields: FieldView[], items: ItemView[] = [], lists: ListView[] = []) => ({
    slide: { id: "apercu", kind: "blank", params: { elements: [{ id: "apercu-el", kind: "kpi", params: { body: [{ runs: [{ t: "Corps", italic: true }] }] } }] } },
    page: 1, hash: "h", width: 1920, height: 1080, targets: [], zone: null, fields, items, lists,
  }) as unknown as SlideView;
  const FIELDS = [field("title", "plain", "Titre", 150, 230), field("body", "rich", "Corps", 150, 300)];

  /** L'api en mémoire : un aperçu recadré avec ses champs ; chaque écriture d'une valeur d'exemple notée dans `written`. */
  function scene(fields: FieldView[] = FIELDS, items: ItemView[] = [], lists: ListView[] = []) {
    const { fake, calls } = memory({ kpi: "k" });
    const written: unknown[][] = [];
    stub(fake, "previewWidget", async () => ({
      svg: '<svg viewBox="0 0 1920 1080" width="1920pt" height="1080pt"></svg>',
      bbox: { x: 100, y: 200, w: 600, h: 300 },
      view: slideView(fields, items, lists),
      diagnostics: [],
    }));
    stub(fake, "setExampleValue", async (...a: never[]) => (written.push(a), {} as Snapshot));
    return { fake, calls, written };
  }
  const fieldsOf = (host: HTMLElement) => [...host.querySelectorAll<HTMLElement>(".widget-tab-preview .stage-preview .field")];
  const open = (el: Element) => act(async () => void el.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
  const blur = (host: HTMLElement) =>
    act(async () => void host.querySelector(".editor .cm-content")!.dispatchEvent(new FocusEvent("blur")));
  const typeIn = (host: HTMLElement, insert: string) =>
    act(async () => {
      const v = EditorView.findFromDOM(host.querySelector(".editor .cm-content") as HTMLElement)!;
      v.dispatch({ changes: { from: 0, to: v.state.doc.length, insert }, userEvent: "input.type" });
    });

  test("un aperçu avec sa vue : la scène sur l'image recadrée, champs placés dans le repère de la boîte", async () => {
    const { fake } = scene();
    const m = await mount(fake, { name: "kpi" });
    expect(m.host.querySelector(".widget-tab-preview > img")).toBeNull();
    expect(m.host.querySelector(".widget-tab-preview .stage-preview .stage > img")).not.toBeNull();
    // Scène non mesurée (happy-dom) : échelle 0,1 ; le titre est à (150 − 100, 230 − 200) pt de la boîte.
    const [title] = fieldsOf(m.host);
    expect([title.style.left, title.style.top]).toEqual(["5px", "3px"]);
  });

  test("un aperçu sans vue : l'image seule", async () => {
    const { fake } = memory({ kpi: "k" });
    stub(fake, "previewWidget", async () => ({ svg: "<svg/>", diagnostics: [] }));
    const m = await mount(fake, { name: "kpi" });
    expect(m.host.querySelector(".widget-tab-preview > img")).not.toBeNull();
    expect(m.host.querySelector(".stage-wrap")).toBeNull();
  });

  test("un champ simple édité sur place : sa valeur écrite dans les valeurs d'exemple", async () => {
    const { fake, written } = scene();
    const m = await mount(fake, { name: "kpi" });
    await open(fieldsOf(m.host)[0]);
    expect(m.host.querySelector(".editor .cm-content")?.textContent).toBe("Titre");
    await typeIn(m.host, "Météo");
    await blur(m.host);
    expect(written).toEqual([["kpi", "title", "Météo"]]);
    expect(m.host.querySelector(".editor")).toBeNull();
  });

  test("un champ d'item de liste édité sur place : sa colonne changée dans la liste des valeurs d'exemple", async () => {
    const item = field("etapes/d0/date", "plain", "2019", 150, 230, { list: "etapes", item: "d0", column: "date" });
    const { fake, written } = scene([item]);
    const lists: unknown[][] = [];
    stub(fake, "editExampleList", async (...a: never[]) => (lists.push(a), {} as Snapshot));
    const m = await mount(fake, { name: "kpi" });
    await open(fieldsOf(m.host)[0]);
    await typeIn(m.host, "2020");
    await blur(m.host);
    expect(lists).toEqual([["kpi", "etapes", { op: "set_field", item: "d0", field: "date", value: "2020" }]]);
    expect(written).toEqual([]);
  });

  test("un champ riche : ses paragraphes, les lignes inchangées gardant leurs runs", async () => {
    const { fake, written } = scene();
    const m = await mount(fake, { name: "kpi" });
    await open(fieldsOf(m.host)[1]);
    await typeIn(m.host, "Corps\n- **Point**");
    await blur(m.host);
    expect(written).toEqual([
      ["kpi", "body", [{ runs: [{ t: "Corps", italic: true }] }, { runs: [{ t: "Point", bold: true }], list: "bullet" }]],
    ]);
  });

  test("Tab écrit le champ et passe au suivant ; l'aperçu rechargé garde le brouillon", async () => {
    const { fake, written } = scene();
    const m = await mount(fake, { name: "kpi", trials: 0 });
    await open(fieldsOf(m.host)[0]);
    await typeIn(m.host, "Neuf");
    await act(async () =>
      void m.host.querySelector(".editor .cm-content")!.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab", bubbles: true, cancelable: true })),
    );
    expect(written).toEqual([["kpi", "title", "Neuf"]]);
    expect(m.host.querySelector(".editor .cm-content")?.textContent).toBe("Corps");
    await typeIn(m.host, "En cours");
    // L'écriture change la révision des valeurs d'exemple : l'aperçu est redemandé.
    await m.render({ name: "kpi", trials: 1 });
    await wait(10);
    expect(m.host.querySelector(".editor .cm-content")?.textContent).toBe("En cours");
  });

  /** Un item de la liste `etapes` de l'élément d'aperçu, en pt de la slide. */
  const itemOf = (id: string, index: number, x: number, more: Partial<ItemView> = {}): ItemView => ({
    owner: "apercu-el", owner_kind: "element", list: "etapes", item: id, index, len: 2, min: 0, max: 8,
    item_label: "étape", x, y: 250, w: 100, h: 50, ...more,
  });
  const listOf = (more: Partial<ListView> = {}): ListView => ({
    owner: "apercu-el", owner_kind: "element", list: "etapes", len: 2, min: 0, max: 8, item_label: "étape", ...more,
  });
  const ITEMS = [itemOf("d0", 0, 150), itemOf("d1", 1, 300)];
  test("WidgetTab a preview list shows no item handles", async () => {
    const { fake } = scene([], ITEMS, [listOf()]);
    const m = await mount(fake, { name: "kpi" });
    expect(m.host.querySelector(".stage-preview .items, .stage-preview [data-item-remove]")).toBeNull();
  });

  test("Cmd/Ctrl+Alt+flèche dans un champ d'item de l'aperçu déplace l'item dans les valeurs d'exemple", async () => {
    const dates = ITEMS.map((it) =>
      field(`etapes/${it.item}/date`, "plain", it.item, it.x, 250, { list: "etapes", item: it.item, column: "date" }));
    const sc = scene(dates, ITEMS, [listOf()]);
    const edits: unknown[][] = [];
    stub(sc.fake, "editExampleList", async (...a: never[]) => (edits.push(a), {} as Snapshot));
    const m = await mount(sc.fake, { name: "kpi" });
    await open(fieldsOf(m.host)[0]);
    const mac = /Mac/.test(navigator.platform);
    await act(async () => void m.host.querySelector(".editor .cm-content")!.dispatchEvent(new KeyboardEvent("keydown", {
      key: "ArrowRight", altKey: true, metaKey: mac, ctrlKey: !mac, bubbles: true, cancelable: true,
    })));
    expect(edits).toEqual([["kpi", "etapes", { op: "move", item: "d0", to: 1 }]]);
  });

  test("un champ lié au deck ne s'ouvre pas", async () => {
    const { fake } = scene([field("title", "plain", "Deck", 150, 230, { bound: true })]);
    const m = await mount(fake, { name: "kpi" });
    await open(fieldsOf(m.host)[0]);
    expect(m.host.querySelector(".editor")).toBeNull();
  });
});
