// Planche des cas d'un widget : une vignette par cas, alertes, clair / sombre, 16:9 / 4:3.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { ExampleAlert, PreviewLook, Snapshot, TabApi, WidgetPreview } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const ExampleBoard = (await import("./ExampleBoard")).default;
const { setMessages } = await import("./i18n");
const { resetMessages } = await import("./test/messages");
const { deferred, memory, stub, wait } = await import("./test/widgetApi");
const { PREVIEW_DELAY_MS } = await import("./debounce");

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
// Les autres fichiers de la passe attendent les identifiants bruts.
afterEach(resetMessages);
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const SVG = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1920 1080" width="1920pt" height="1080pt"></svg>';
const preview = (alerts: ExampleAlert[]): WidgetPreview => ({ svg: SVG, bbox: { x: 10, y: 20, w: 300, h: 100 }, alerts, diagnostics: [] });

/** Une api dont chaque aperçu est noté dans `asked` et rendu par `answer`. */
function boardApi(answer: (example: string, look: PreviewLook) => Promise<WidgetPreview> | WidgetPreview = () => preview([])) {
  const { fake } = memory({ kpi: "k" });
  const asked: [string, string | null, unknown, PreviewLook][] = [];
  stub(fake, "previewWidget", (async (name: string, example: string | null, params: unknown, look: PreviewLook) => {
    asked.push([name, example, params, look]);
    return answer(example ?? "", look);
  }) as never);
  const sent: unknown[][] = [];
  stub(fake, "addExample", (async (...a: unknown[]) => (sent.push(a), { examples: {} } as unknown as Snapshot)) as never);
  return { fake, asked, sent };
}

interface Shown { names: string[]; current?: string; trials?: number; look?: string; scheme?: "light" | "dark" }

async function mount(fake: TabApi, first: Shown) {
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  mounted = { root, host };
  const opened: string[] = [];
  const run = async (p: Promise<Snapshot>) => {
    try {
      return await p;
    } catch {
      return null;
    }
  };
  const render = ({ names, current = names[0], trials = 0, look = "l", scheme }: Shown) =>
    act(async () =>
      root.render(
        createElement(ExampleBoard, { api: fake, widget: "kpi", names, current, trials, look, scheme, onOpen: (n: string) => opened.push(n), run }),
      ),
    );
  await render(first);
  await wait(10);
  const cards = () => [...host.querySelectorAll<HTMLButtonElement>(".board-card:not(.board-new)")];
  const card = (name: string) => cards().find((c) => c.querySelector(".board-name")?.textContent === name)!;
  const badges = (name: string) => [...card(name).querySelectorAll(".board-badge")].map((b) => b.textContent);
  const button = (label: string) => [...host.querySelectorAll<HTMLButtonElement>(".board-toggles button")].find((b) => b.textContent === label)!;
  const click = (b: HTMLElement) => act(async () => b.click());
  return { host, render, cards, card, badges, button, click, opened };
}

describe("ExampleBoard, vignettes", () => {
  test("une vignette par cas, avec son nom et son image recadrée", async () => {
    const { fake } = boardApi();
    const m = await mount(fake, { names: ["Court", "Long", "Vide"] });
    expect(m.cards().map((c) => c.querySelector(".board-name")?.textContent)).toEqual(["Court", "Long", "Vide"]);
    expect(m.cards().every((c) => !!c.querySelector("img.board-thumb")?.getAttribute("src"))).toBe(true);
  });

  test("les badges suivent les alertes : vide, déborde, ou OK", async () => {
    const alerts: Record<string, ExampleAlert[]> = {
      Court: [],
      Long: [{ kind: "overflow", field: "body" }],
      Vide: [{ kind: "empty", field: "title" }, { kind: "empty", field: "body" }],
    };
    const { fake } = boardApi((example) => preview(alerts[example]));
    const m = await mount(fake, { names: ["Court", "Long", "Vide"] });
    expect(m.badges("Court")).toEqual(["ui-board-ok"]);
    expect(m.badges("Long")).toEqual(["ui-board-overflow"]);
    // Une alerte du même genre sur deux champs : un seul badge.
    expect(m.badges("Vide")).toEqual(["ui-board-empty"]);
  });

  test("le bilan compte les cas et les alertes", async () => {
    setMessages("fr", ["ui-board-summary = { $cases } cas · { $alerts } alertes"]);
    const alerts: Record<string, ExampleAlert[]> = {
      Court: [],
      Long: [{ kind: "overflow", field: "body" }],
      Vide: [{ kind: "empty", field: "title" }, { kind: "empty", field: "body" }],
    };
    const { fake } = boardApi((example) => preview(alerts[example]));
    const m = await mount(fake, { names: ["Court", "Long", "Vide"] });
    expect(m.host.querySelector(".board-summary")?.textContent).toBe("3 cas · 3 alertes");
  });

  test("le cas courant est marqué", async () => {
    const { fake } = boardApi();
    const m = await mount(fake, { names: ["Court", "Long"], current: "Long" });
    expect(m.cards().map((c) => c.getAttribute("aria-current"))).toEqual([null, "true"]);
  });
});

describe("ExampleBoard, aperçus", () => {
  test("un aperçu par cas, en clair et en 16:9 d'abord", async () => {
    const { fake, asked } = boardApi();
    await mount(fake, { names: ["Court", "Long"] });
    expect(asked).toEqual([
      ["kpi", "Court", {}, { scheme: "light", ratio: "16:9" }],
      ["kpi", "Long", {}, { scheme: "light", ratio: "16:9" }],
    ]);
  });

  test("un deck sombre : la planche part en sombre", async () => {
    const { fake, asked } = boardApi();
    await mount(fake, { names: ["Court"], scheme: "dark" });
    expect(asked.map((a) => a[3])).toEqual([{ scheme: "dark", ratio: "16:9" }]);
  });

  test("au plus 12 aperçus", async () => {
    const { fake, asked } = boardApi();
    const names = Array.from({ length: 14 }, (_, i) => `Cas ${i + 1}`);
    const m = await mount(fake, { names });
    expect(asked.length).toBe(12);
    expect(m.cards().length).toBe(12);
  });

  test("Sombre redemande tous les aperçus en sombre", async () => {
    const { fake, asked } = boardApi();
    const m = await mount(fake, { names: ["Court", "Long"] });
    await m.click(m.button("ui-board-dark"));
    await wait(10);
    expect(asked.slice(2).map((a) => [a[1], a[3]])).toEqual([
      ["Court", { scheme: "dark", ratio: "16:9" }],
      ["Long", { scheme: "dark", ratio: "16:9" }],
    ]);
    expect(m.button("ui-board-dark").getAttribute("aria-pressed")).toBe("true");
    expect(m.button("ui-board-light").getAttribute("aria-pressed")).toBe("false");
  });

  test("4:3 redemande tous les aperçus en 4:3", async () => {
    const { fake, asked } = boardApi();
    const m = await mount(fake, { names: ["Court"] });
    await m.click(m.button("4:3"));
    await wait(10);
    expect(asked.at(-1)).toEqual(["kpi", "Court", {}, { scheme: "light", ratio: "4:3" }]);
    expect(m.button("4:3").getAttribute("aria-pressed")).toBe("true");
  });

  test("une nouvelle révision des cas ou un autre style du deck : les aperçus sont redemandés après un délai", async () => {
    const { fake, asked } = boardApi();
    const m = await mount(fake, { names: ["Court"] });
    await m.render({ names: ["Court"], trials: 1 });
    await wait(PREVIEW_DELAY_MS + 50);
    await m.render({ names: ["Court"], trials: 1, look: "autre" });
    await wait(PREVIEW_DELAY_MS + 50);
    expect(asked.length).toBe(3);
  });

  test("une rafale de révisions (frappes dans le code) ne redemande les aperçus qu'une fois", async () => {
    const { fake, asked } = boardApi();
    const m = await mount(fake, { names: ["Court", "Long"] });
    for (let r = 1; r <= 5; r++) {
      await m.render({ names: ["Court", "Long"], trials: r });
      await wait(20);
    }
    expect(asked.length).toBe(2);
    await wait(PREVIEW_DELAY_MS + 50);
    expect(asked.length).toBe(4);
  });

  test("une réponse périmée ne remplace pas la plus récente", async () => {
    const slow = deferred<WidgetPreview>();
    let first = true;
    const { fake } = boardApi((_, look) => {
      if (first) {
        first = false;
        return slow.promise;
      }
      return preview(look.scheme === "dark" ? [{ kind: "overflow", field: "body" }] : []);
    });
    const m = await mount(fake, { names: ["Court"] });
    await m.click(m.button("ui-board-dark"));
    await wait(10);
    expect(m.badges("Court")).toEqual(["ui-board-overflow"]);
    await act(async () => slow.resolve(preview([])));
    await wait(10);
    expect(m.badges("Court")).toEqual(["ui-board-overflow"]);
  });

  test("un aperçu refusé n'empêche pas les autres", async () => {
    const { fake } = boardApi((example) => (example === "Long" ? Promise.reject(new Error("x")) : preview([])));
    const m = await mount(fake, { names: ["Court", "Long"] });
    expect(m.badges("Court")).toEqual(["ui-board-ok"]);
    expect(m.badges("Long")).toEqual([]);
  });
});

describe("ExampleBoard, ouvrir et ajouter", () => {
  test("un clic sur une vignette ouvre son cas", async () => {
    const { fake } = boardApi();
    const m = await mount(fake, { names: ["Court", "Long"] });
    await m.click(m.card("Long"));
    expect(m.opened).toEqual(["Long"]);
  });

  test("« Nouveau cas » ajoute une copie du cas courant et l'ouvre", async () => {
    const { fake, sent } = boardApi();
    const m = await mount(fake, { names: ["Court", "Long"], current: "Long" });
    await m.click(m.host.querySelector<HTMLButtonElement>(".board-new")!);
    await wait(10);
    expect(sent).toEqual([["kpi", "ui-examples-new-name", "Long"]]);
    expect(m.opened).toEqual(["ui-examples-new-name"]);
  });

  test("« Nouveau cas » refusé n'ouvre rien", async () => {
    const { fake } = boardApi();
    stub(fake, "addExample", (async () => Promise.reject(new Error("refus"))) as never);
    const m = await mount(fake, { names: ["Court"] });
    await m.click(m.host.querySelector<HTMLButtonElement>(".board-new")!);
    await wait(10);
    expect(m.opened).toEqual([]);
  });

  test("« Nouveau cas » est grisé à 12 cas", async () => {
    const { fake } = boardApi();
    const names = Array.from({ length: 12 }, (_, i) => `Cas ${i + 1}`);
    const m = await mount(fake, { names });
    expect(m.host.querySelector<HTMLButtonElement>(".board-new")!.disabled).toBe(true);
  });
});
