// Rendu et comportement des composants du catalogue v2, dans un DOM happy-dom.

import { afterEach, describe, expect, spyOn, test } from "bun:test";
import type { A2uiClientAction, A2uiMessage } from "@a2ui/web_core/v0_9";
import "../test/dom";
import { CATALOG_ID } from "./catalog";

// react-dom cherche le DOM à son chargement, et Bun n'évalue pas les imports
// statiques dans l'ordre écrit : React et la surface se chargent donc après
// `../test/dom` (plus haut), par des imports dynamiques.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { SurfaceView } = await import("./SurfaceView");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const messages = (components: unknown[], data: unknown = {}) =>
  [
    { version: "v0.9", createSurface: { surfaceId: "s", catalogId: CATALOG_ID } },
    { version: "v0.9", updateComponents: { surfaceId: "s", components } },
    { version: "v0.9", updateDataModel: { surfaceId: "s", value: data } },
  ] as unknown as A2uiMessage[];

/** Monte une surface d'un composant racine ; renvoie son hôte et les actions reçues. */
async function mount(root: Record<string, unknown>, data: unknown = {}) {
  const actions: A2uiClientAction[] = [];
  const errors: string[] = [];
  const host = document.createElement("div");
  document.body.append(host);
  const r = createRoot(host);
  mounted = { root: r, host };
  await act(async () => {
    r.render(
      createElement(SurfaceView, {
        messages: messages([{ id: "root", ...root }], data),
        surfaceId: "s",
        onAction: (a: A2uiClientAction) => actions.push(a),
        onError: (m: string) => errors.push(m),
      }),
    );
  });
  return { host, actions, errors };
}

/**
 * Change la valeur d'un champ comme le navigateur, puis émet `type`.
 * Le setter du prototype contourne le suivi de valeur de React, qui ignore les
 * affectations faites par le setter de l'instance.
 */
async function fire(el: HTMLInputElement | HTMLSelectElement, type: "input" | "change", value?: string) {
  await act(async () => {
    if (value !== undefined) {
      const proto = el instanceof HTMLSelectElement ? HTMLSelectElement.prototype : HTMLInputElement.prototype;
      Object.getOwnPropertyDescriptor(proto, "value")?.set?.call(el, value);
    }
    el.dispatchEvent(new Event(type, { bubbles: true }));
  });
}

const click = (el: Element) => act(async () => (el as HTMLElement).click());
const event = (name: string) => ({ event: { name, context: {} } });

/** Un sélecteur de couleur lié à `/c`, dont l'action envoie la valeur. */
const colorField = {
  component: "ColorField",
  label: "Accent",
  value: { path: "/c" },
  palette: ["#c2573a", "#1f4e79"],
  action: { event: { name: "param", context: { value: { path: "/c" } } } },
};

describe("rendu du catalogue v2", () => {
  test("DiffText barre le retiré et marque l'ajouté", async () => {
    const { host } = await mount({
      component: "DiffText",
      segments: [{ kind: "keep", text: "Nos " }, { kind: "del", text: "objectifs" }, { kind: "ins", text: "paris" }],
    });
    expect(host.querySelector("del")?.textContent).toBe("objectifs");
    expect(host.querySelector("ins")?.textContent).toBe("paris");
  });

  test("SlideThumb montre le SVG en image, jamais en ligne", async () => {
    const { host } = await mount({ component: "SlideThumb", svg: "<svg><script>alert(1)</script></svg>", label: "A" });
    // Seule une image isole le SVG : <object>, <embed>, <iframe> ou le SVG en ligne exécuteraient son script.
    const shown = [...host.querySelectorAll(".slide-thumb img, .slide-thumb object, .slide-thumb embed, .slide-thumb iframe")];
    expect(shown.map((e) => e.tagName.toLowerCase())).toEqual(["img"]);
    expect(shown[0].getAttribute("src")).toStartWith("data:image/svg+xml;utf8,%3Csvg%3E");
    expect(host.querySelector("script, svg")).toBeNull();
  });

  test("Button : type button, un clic envoie son action", async () => {
    const { host, actions } = await mount({ component: "Button", label: "OK", primary: true, action: event("confirm") });
    const button = host.querySelector("button");
    expect(button?.getAttribute("type")).toBe("button");
    expect(button?.hasAttribute("data-primary")).toBe(true);
    await click(button as Element);
    expect(actions.map((a) => a.name)).toEqual(["confirm"]);
  });

  test("Button sans primary n'est pas l'action principale", async () => {
    const { host } = await mount({ component: "Button", label: "Annuler", action: event("dismiss") });
    expect(host.querySelector("button")?.hasAttribute("data-primary")).toBe(false);
  });

  test("Button danger porte la classe danger, sans devenir principal", async () => {
    const { host } = await mount({ component: "Button", label: "Toujours", danger: true, action: event("x") });
    const button = host.querySelector("button");
    expect(button?.classList.contains("danger")).toBe(true);
    expect(button?.classList.contains("primary")).toBe(false);
    expect(button?.hasAttribute("data-primary")).toBe(false);
  });

  test("Button principal et danger porte les deux classes (confirmation non annulable)", async () => {
    const { host } = await mount({ component: "Button", label: "Oui", primary: true, danger: true, action: event("x") });
    const button = host.querySelector("button");
    expect(button?.classList.contains("primary")).toBe(true);
    expect(button?.classList.contains("danger")).toBe(true);
    expect(button?.hasAttribute("data-primary")).toBe(true);
  });

  test("ChoiceChips : groupe radio ; la valeur est écrite avant l'envoi de l'action", async () => {
    const { host, actions } = await mount(
      {
        component: "ChoiceChips",
        value: { path: "/choice" },
        options: [{ id: "a", label: "A" }, { id: "b", label: "B" }],
        action: { event: { name: "choose", context: { choice: { path: "/choice" } } } },
      },
      { choice: "" },
    );
    expect(host.querySelector('[role="radiogroup"]')).not.toBeNull();
    const chips = [...host.querySelectorAll('[role="radio"]')];
    expect(chips.map((c) => c.getAttribute("aria-checked"))).toEqual(["false", "false"]);
    expect(chips[0].querySelector(".chip-key")?.getAttribute("aria-hidden")).toBe("true");
    await click(chips[1]);
    expect(actions.map((a) => a.context)).toEqual([{ choice: "b" }]);
    expect(host.querySelectorAll('[role="radio"]')[1].getAttribute("aria-checked")).toBe("true");
  });

  test("ColorField : glisser dans le sélecteur n'envoie qu'une action, à sa fermeture", async () => {
    const { host, actions } = await mount(colorField, { c: "#c2573a" });
    const picker = host.querySelector('input[type="color"]') as HTMLInputElement;
    for (const v of ["#101010", "#202020", "#303030"]) await fire(picker, "input", v);
    expect(picker.value).toBe("#303030");
    expect(actions).toHaveLength(0);
    await fire(picker, "change", "#303030");
    expect(actions.map((a) => a.context)).toEqual([{ value: "#303030" }]);
  });

  test("ColorField : une pastille déjà choisie ne renvoie rien", async () => {
    const { host, actions } = await mount(colorField, { c: "#c2573a" });
    const swatches = [...host.querySelectorAll(".swatch")];
    expect(swatches[0].getAttribute("aria-pressed")).toBe("true");
    await click(swatches[0]);
    expect(actions).toHaveLength(0);
    await click(swatches[1]);
    await click(host.querySelectorAll(".swatch")[1]);
    expect(actions.map((a) => a.context)).toEqual([{ value: "#1f4e79" }]);
    expect(host.querySelectorAll(".swatch")[1].getAttribute("aria-pressed")).toBe("true");
  });

  test("Slider : la valeur suit le glissé, l'action part une fois à la validation", async () => {
    const { host, actions } = await mount(
      {
        component: "Slider",
        label: "Espacement",
        value: { path: "/f" },
        min: 0,
        max: 64,
        step: 4,
        action: { event: { name: "param", context: { value: { path: "/f" } } } },
      },
      { f: 16 },
    );
    const range = host.querySelector('input[type="range"]') as HTMLInputElement;
    await fire(range, "input", "24");
    await fire(range, "input", "32");
    expect(host.textContent).toContain("Espacement · 32");
    expect(actions).toHaveLength(0);
    await fire(range, "change", "32");
    // React écoute `focusout` (qui remonte) pour onBlur, pas `blur`.
    await act(async () => range.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
    expect(actions.map((a) => a.context)).toEqual([{ value: 32 }]);
  });

  test("Slider : quitter le champ après un glissé envoie la valeur, une fois", async () => {
    const { host, actions } = await mount(
      {
        component: "Slider",
        label: "Espacement",
        value: { path: "/f" },
        min: 0,
        max: 64,
        step: 4,
        action: { event: { name: "param", context: { value: { path: "/f" } } } },
      },
      { f: 16 },
    );
    const range = host.querySelector('input[type="range"]') as HTMLInputElement;
    await fire(range, "input", "40");
    await act(async () => range.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
    await act(async () => range.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
    expect(actions.map((a) => a.context)).toEqual([{ value: 40 }]);
  });

  test("Select : une valeur inconnue montre une option vide, choisir envoie l'action", async () => {
    const { host, actions } = await mount(
      {
        component: "Select",
        label: "Thème",
        value: { path: "/t" },
        options: [{ id: "papier", label: "Papier" }, { id: "pro", label: "Pro" }],
        action: { event: { name: "param", context: { value: { path: "/t" } } } },
      },
      { t: "ancien" },
    );
    const select = host.querySelector("select") as HTMLSelectElement;
    expect(select.value).toBe("");
    expect(select.options).toHaveLength(3);
    expect(select.options[0].value).toBe("");
    expect(select.options[0].disabled).toBe(true);
    await fire(select, "change", "papier");
    expect(actions.map((a) => a.context)).toEqual([{ value: "papier" }]);
  });

  test("Select : une valeur connue n'a pas d'option vide", async () => {
    const { host } = await mount(
      { component: "Select", label: "Thème", value: "pro", options: [{ id: "papier", label: "Papier" }, { id: "pro", label: "Pro" }] },
    );
    const select = host.querySelector("select") as HTMLSelectElement;
    expect(select.value).toBe("pro");
    expect(select.options).toHaveLength(2);
  });

  test("SurfaceView : une surface invalide va à onError, rien n'est affiché", async () => {
    // @a2ui journalise l'erreur de validation : attendue ici, elle ne doit pas encombrer la sortie.
    const logged = spyOn(console, "error").mockImplementation(() => {});
    try {
      const { host, errors } = await mount({ component: "Text" });
      expect(errors).toHaveLength(1);
      expect(errors[0]).toContain("Text");
      expect(errors[0]).toContain("text");
      expect(host.querySelector(".a2ui-surface")).toBeNull();
    } finally {
      logged.mockRestore();
    }
  });

  test("SurfaceView : la surface est enveloppée d'un .a2ui-surface", async () => {
    const { host } = await mount({ component: "Text", text: "Bonjour" });
    expect(host.querySelector(".a2ui-surface")?.textContent).toBe("Bonjour");
  });
});
