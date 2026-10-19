// Rendu et comportement des composants du catalogue v3, dans un DOM happy-dom.

import { describe, expect, spyOn, test } from "bun:test";
import "../test/dom";

// react-dom cherche le DOM à son chargement, et Bun n'évalue pas les imports
// statiques dans l'ordre écrit : React et la surface se chargent donc après
// `../test/dom` (plus haut), par des imports dynamiques.
const { act, createElement } = await import("react");
const { mountSurface, messagesOf } = await import("../test/surface");
const { SlideThumbsContext } = await import("./slideThumbs");
const { QuickHintsContext } = await import("./quickHints");
const { t } = await import("../i18n");
const { expectIconOnly } = await import("../test/iconOnly");

/** Monte une surface d'un composant racine ; renvoie son hôte et les actions reçues. `hints` : elle reçoit les touches rapides. */
const mount = (root: Record<string, unknown>, data: unknown = {}, hints = false) =>
  mountSurface(messagesOf([{ id: "root", ...root }], data), (s) =>
    createElement(QuickHintsContext.Provider, { value: hints }, s),
  );

/** Monte une surface de plusieurs composants (dont `root`) ; renvoie son hôte et les erreurs reçues. */
const mountAll = (components: Record<string, unknown>[], data: unknown = {}) => mountSurface(messagesOf(components, data));

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

describe("rendu du catalogue v3", () => {
  /** Un bandeau : son texte, un bouton enfant, sa croix. */
  const notice = [
    { id: "root", component: "Notice", text: "Une autre version", children: ["mine"], close: event("close"), close_label: "Fermer" },
    { id: "mine", component: "Button", label: "Ma version", primary: true, action: event("use_mine") },
  ];

  test("Notice is announced as a status", async () => {
    const { host } = await mountAll(notice);
    expect(host.querySelector(".notice-bar")?.getAttribute("role")).toBe("status");
  });

  test("Notice shows its text, then its buttons, then its close", async () => {
    const { host } = await mountAll(notice);
    expect([...(host.querySelector(".notice-bar")?.children ?? [])].map((c) => c.className)).toEqual([
      "notice-bar-text",
      "notice-bar-actions",
      "token ghost notice-bar-close",
    ]);
    expect(host.querySelector(".notice-bar-text")?.textContent).toBe("Une autre version");
  });

  test("Notice close shows an icon only, named by close_label", async () => {
    const { host } = await mountAll(notice);
    expectIconOnly(host.querySelector(".notice-bar-close"), "Fermer");
  });

  test("Button with an icon shows it alone, named by its label", async () => {
    const { host } = await mount({ component: "Button", label: "Fermer", icon: "close", action: event("dismiss") });
    expectIconOnly(host.querySelector("button"), "Fermer");
  });

  test("Button with an icon still sends its action", async () => {
    const { host, actions } = await mount({ component: "Button", label: "Fermer", icon: "close", action: event("dismiss") });
    await click(host.querySelector("button")!);
    expect(actions.map((a) => a.name)).toEqual(["dismiss"]);
  });

  test("Button with an icon keeps its primary mark", async () => {
    const { host } = await mount({ component: "Button", label: "OK", icon: "close", primary: true, action: event("ok") });
    expect(host.querySelector("button")?.hasAttribute("data-primary")).toBe(true);
  });

  test("Button with an icon keeps its danger style", async () => {
    const { host } = await mount({ component: "Button", label: "Non", icon: "close", danger: true, action: event("no") });
    expect(host.querySelector("button")?.classList.contains("danger")).toBe(true);
  });

  test("Notice sends its buttons' actions, then close", async () => {
    const { host, actions } = await mountAll(notice);
    await click(host.querySelector(".notice-bar-actions button") as Element);
    await click(host.querySelector(".notice-bar-close") as Element);
    expect(actions.map((a) => a.name)).toEqual(["use_mine", "close"]);
  });

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

  test("Button a le style des boutons de l'app", async () => {
    const { host } = await mount({ component: "Button", label: "OK", action: event("confirm") });
    expect(host.querySelector("button")?.classList.contains("object")).toBe(true);
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
      true,
    );
    expect(host.querySelector('[role="radiogroup"]')).not.toBeNull();
    const chips = [...host.querySelectorAll('[role="radio"]')];
    expect(chips.map((c) => c.getAttribute("aria-checked"))).toEqual(["false", "false"]);
    expect(chips[0].querySelector(".chip-key")?.getAttribute("aria-hidden")).toBe("true");
    await click(chips[1]);
    expect(actions.map((a) => a.context)).toEqual([{ choice: "b" }]);
    expect(host.querySelectorAll('[role="radio"]')[1].getAttribute("aria-checked")).toBe("true");
  });

  test("ChoiceChips : sans fournisseur des touches rapides, aucun chiffre 1–4", async () => {
    const { host } = await mountAll([
      { id: "root", component: "ChoiceChips", value: { path: "/choice" }, options: [{ id: "a", label: "A" }, { id: "b", label: "B" }] },
    ]);
    expect(host.querySelectorAll('[role="radio"]')).toHaveLength(2);
    expect(host.querySelectorAll(".chip-key")).toHaveLength(0);
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

  test("ColorField : un contexte d'action avec un tableau littéral (chemin d'un jeton) part tel quel", async () => {
    // Régression t-554 : sous la clé `path`, @a2ui y voyait une liaison de données et levait.
    const context = { token: ["color", "accent"], action: "set", value: { path: "/c" } };
    const field = { ...colorField, action: { event: { name: "token", context } } };
    const { host, actions } = await mount(field, { c: "#c2573a" });
    await fire(host.querySelector('input[type="color"]') as HTMLInputElement, "change", "#ff0000");
    expect(actions.map((a) => a.context)).toEqual([{ token: ["color", "accent"], action: "set", value: "#ff0000" }]);
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

  test("ColorField : un nom de couleur est montré tel quel ; une pastille écrit l'hex", async () => {
    const { host, actions } = await mount(colorField, { c: "accent" });
    expect(host.querySelector(".color-name")?.textContent).toBe("accent");
    expect(host.querySelectorAll('.swatch[aria-pressed="true"]')).toHaveLength(0);
    await click(host.querySelectorAll(".swatch")[1]);
    expect(actions.map((a) => a.context)).toEqual([{ value: "#1f4e79" }]);
  });

  test("ColorField : un hex ne montre pas de nom", async () => {
    const { host } = await mount(colorField, { c: "#c2573a" });
    expect(host.querySelector(".color-name")).toBeNull();
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

  test("Column et Row : chaque emplacement porte l'id de son composant", async () => {
    const { host } = await mountAll([
      { id: "root", component: "Column", children: ["color.accent:row"] },
      { id: "color.accent:row", component: "Row", children: ["color.accent"] },
      { id: "color.accent", component: "Text", text: "Accent" },
    ]);
    const row = host.querySelector('[data-component-id="color.accent:row"]');
    expect(row?.querySelector('[data-component-id="color.accent"]')?.textContent).toBe("Accent");
  });
});

describe("composants du fil (v3)", () => {
  test("Prompt rend la consigne dans une bulle", async () => {
    const { host } = await mount({ component: "Prompt", text: "Change le fond" });
    expect(host.querySelector(".a2ui-prompt")?.textContent).toBe("Change le fond");
  });

  test("Timeline repliée ne montre que son résumé jusqu'au clic", async () => {
    const data = { steps: [{ label: "Slide 4 lue", status: "completed" }], collapsed: true, summary: "1 outil · 4 s" };
    const { host } = await mount(
      { component: "Timeline", steps: { path: "/steps" }, collapsed: { path: "/collapsed" }, summary: { path: "/summary" } },
      data,
    );
    const summary = host.querySelector(".a2ui-timeline-summary");
    expect(summary?.textContent).toContain("1 outil · 4 s");
    expect(host.querySelector(".a2ui-step")).toBeNull();
    await click(summary!);
    expect(host.querySelector(".a2ui-step-label")?.textContent).toBe("Slide 4 lue");
  });

  test("Timeline ignore des étapes mal formées dans le modèle de données", async () => {
    const data = { steps: [{ label: "ok", status: "completed" }, { label: "x", status: "<b>" }, 3], collapsed: false, summary: "" };
    const { host, errors } = await mount(
      { component: "Timeline", steps: { path: "/steps" }, collapsed: { path: "/collapsed" }, summary: { path: "/summary" } },
      data,
    );
    expect(errors).toEqual([]);
    expect(host.querySelectorAll(".a2ui-step").length).toBe(1);
  });

  test("Timeline en cours liste ses étapes et leur nombre d'outils", async () => {
    const data = { steps: [{ label: "Fond posé", status: "in_progress", count: 3 }], collapsed: false, summary: "" };
    const { host } = await mount(
      { component: "Timeline", steps: { path: "/steps" }, collapsed: { path: "/collapsed" }, summary: { path: "/summary" } },
      data,
    );
    expect(host.querySelector(".a2ui-step.in_progress .a2ui-step-label")?.textContent).toBe("Fond posé");
    expect(host.querySelector(".a2ui-step .a2ui-badge")?.textContent).toBe("3");
  });

  const timeline = { component: "Timeline", steps: { path: "/steps" }, collapsed: { path: "/collapsed" }, summary: { path: "/summary" } };

  test("Timeline : une étape d'outils se déplie sur chacun de ses appels", async () => {
    const calls = [
      { label: "widgets", status: "completed", input: '{\n  "slide": 4\n}' },
      { label: "widgets", status: "failed", output: "index hors bornes" },
    ];
    const data = { steps: [{ label: "widgets", status: "failed", count: 2, calls }], collapsed: false, summary: "" };
    const { host } = await mount(timeline, data);
    expect(host.querySelector(".a2ui-call")).toBeNull();
    await click(host.querySelector(".a2ui-step-toggle")!);
    const rows = host.querySelectorAll(".a2ui-call");
    expect(rows.length).toBe(2);
    expect(rows[1].classList.contains("failed")).toBe(true);
    // Replié, un appel résume ses arguments sur une ligne.
    expect(rows[0].querySelector(".a2ui-call-args")?.textContent).toBe('{ "slide": 4 }');
  });

  test("Timeline : un appel se déplie sur ses arguments et son résultat", async () => {
    const calls = [{ label: "widgets", status: "completed", input: '{\n  "slide": 4\n}', output: "3 widgets" }];
    const data = { steps: [{ label: "widgets", status: "completed", count: 1, calls }], collapsed: false, summary: "" };
    const { host } = await mount(timeline, data);
    await click(host.querySelector(".a2ui-step-toggle")!);
    expect(host.querySelector(".a2ui-call-io")).toBeNull();
    await click(host.querySelector(".a2ui-call-toggle")!);
    const blocks = [...host.querySelectorAll(".a2ui-call-io pre")].map((x) => x.textContent);
    expect(blocks).toEqual(['{\n  "slide": 4\n}', "3 widgets"]);
    expect(host.querySelector(".a2ui-call-io")?.textContent).toContain(t("ui-agent-tool-input"));
    expect(host.querySelector(".a2ui-call-io")?.textContent).toContain(t("ui-agent-tool-output"));
  });

  test("Timeline : un appel sans arguments ni résultat n'a rien à déplier", async () => {
    const data = { steps: [{ label: "slides", status: "completed", count: 1, calls: [{ label: "slides", status: "completed" }] }], collapsed: false, summary: "" };
    const { host } = await mount(timeline, data);
    await click(host.querySelector(".a2ui-step-toggle")!);
    expect(host.querySelector(".a2ui-call .a2ui-call-label")?.textContent).toBe("slides");
    expect(host.querySelector(".a2ui-call-toggle")).toBeNull();
  });

  test("Timeline : un appel aux arguments vides n'a rien à déplier", async () => {
    const data = { steps: [{ label: "slides", status: "completed", count: 1, calls: [{ label: "slides", status: "completed", input: "" }] }], collapsed: false, summary: "" };
    const { host } = await mount(timeline, data);
    await click(host.querySelector(".a2ui-step-toggle")!);
    expect(host.querySelector(".a2ui-call-toggle")).toBeNull();
    expect(host.querySelector(".a2ui-call-args")).toBeNull();
  });

  test("Timeline : une note coupée se déplie sur son texte entier", async () => {
    const detail = "Corrections en une transaction.\n\nPuis **la grille**.";
    const data = { steps: [{ label: "Corrections en une transaction.", status: "completed", detail }], collapsed: false, summary: "" };
    const { host } = await mount(timeline, data);
    expect(host.querySelector(".a2ui-step-detail")).toBeNull();
    await click(host.querySelector(".a2ui-step-toggle")!);
    expect(host.querySelector(".a2ui-step-detail strong")?.textContent).toBe("la grille");
  });

  test("Timeline : une étape sans détail n'est pas un bouton", async () => {
    const data = { steps: [{ label: "Lire la slide", status: "completed" }], collapsed: false, summary: "" };
    const { host } = await mount(timeline, data);
    expect(host.querySelector(".a2ui-step-toggle")).toBeNull();
    expect(host.querySelector(".a2ui-step-label")?.textContent).toBe("Lire la slide");
  });

  test("Receipt montre titre, faits et pastille de couleur", async () => {
    const { host } = await mount({
      component: "Receipt",
      title: "Slide 4 : fond bleu nuit",
      facts: ["titre en blanc", "surtitre bleu clair"],
      swatch: "#00154e",
    });
    expect(host.querySelector(".a2ui-receipt .a2ui-title")?.textContent).toBe("Slide 4 : fond bleu nuit");
    expect(host.querySelector(".a2ui-receipt .a2ui-hint")?.textContent).toBe("titre en blanc · surtitre bleu clair");
    // happy-dom garde la valeur telle qu'écrite (pas de conversion en rgb()).
    expect((host.querySelector(".a2ui-swatch") as HTMLElement).style.background).toBe("#00154e");
  });

  /** Monte un reçu sur la slide `s-4` (empreinte `h1`), avec ces miniatures. */
  async function mountReceipt(thumbs: import("./slideThumbs").SlideThumbs | null) {
    const { host } = await mountSurface(
      messagesOf([{ id: "root", component: "Receipt", title: "Fond", facts: ["a"], slide: { id: "s-4", hash: "h1" }, swatch: "#00154e" }]),
      (surface) => createElement(SlideThumbsContext.Provider, { value: thumbs }, surface),
    );
    return host;
  }

  test("Receipt avec une slide montre sa miniature résolue par empreinte, sans SVG dans le message", async () => {
    const host = await mountReceipt({ cached: (h) => (h === "h1" ? "blob:h1" : null), svg: () => null, current: () => "h2" });
    const img = host.querySelector<HTMLImageElement>(".a2ui-receipt-thumb");
    expect(img?.getAttribute("src")).toBe("blob:h1");
    expect(img?.classList.contains("changed")).toBe(false);
    expect(host.querySelector(".a2ui-swatch")).toBeNull();
  });

  test("Receipt dont la slide a changé depuis montre son état actuel, marqué « modifiée depuis »", async () => {
    const host = await mountReceipt({ cached: () => null, svg: (id, h) => `blob:${id}@${h}`, current: () => "h2" });
    const img = host.querySelector<HTMLImageElement>(".a2ui-receipt-thumb");
    expect(img?.getAttribute("src")).toBe("blob:s-4@h2");
    expect(img?.classList.contains("changed")).toBe(true);
    expect(img?.getAttribute("title")).toBe(t("ui-history-changed-since"));
  });

  test("Receipt dont la slide est introuvable montre sa pastille de couleur à la place", async () => {
    const host = await mountReceipt(null);
    expect(host.querySelector(".a2ui-receipt-thumb")).toBeNull();
    expect(host.querySelector(".a2ui-swatch")).not.toBeNull();
  });

  test("Disclosure cache son enfant jusqu'au clic", async () => {
    const { host } = await mountSurface(messagesOf([
      { id: "root", component: "Disclosure", summary: "Pourquoi", child: "why" },
      { id: "why", component: "Markdown", text: "Le modèle **content** n'a pas de fond." },
    ]));
    expect(host.querySelector(".a2ui-md")).toBeNull();
    await click(host.querySelector(".a2ui-disclosure > button")!);
    expect(host.querySelector(".a2ui-md strong")?.textContent).toBe("content");
  });

  test("Disclosure open : dépliée au premier rendu, se replie au clic", async () => {
    const { host } = await mountSurface(messagesOf([
      { id: "root", component: "Disclosure", summary: "Couleurs", child: "c", open: true },
      { id: "c", component: "Markdown", text: "**accent**" },
    ]));
    expect(host.querySelector(".a2ui-md strong")?.textContent).toBe("accent");
    await click(host.querySelector(".a2ui-disclosure > button")!);
    expect(host.querySelector(".a2ui-md")).toBeNull();
  });

  test("Disclosure open: false reste repliée", async () => {
    const { host } = await mountSurface(messagesOf([
      { id: "root", component: "Disclosure", summary: "Couleurs", child: "c", open: false },
      { id: "c", component: "Markdown", text: "x" },
    ]));
    expect(host.querySelector(".a2ui-md")).toBeNull();
  });

  test("Badge porte la classe de son ton", async () => {
    const { host } = await mount({ component: "Badge", text: "annulé", tone: "danger" });
    expect(host.querySelector(".a2ui-badge.tone-danger")?.textContent).toBe("annulé");
  });

  test("Text note prend la classe des notes", async () => {
    const { host } = await mount({ component: "Text", text: "Je lis la slide", variant: "note" });
    expect(host.querySelector(".a2ui-note")?.textContent).toBe("Je lis la slide");
  });

  test("Receipt avec une propriété inconnue est refusé en la nommant", async () => {
    // @a2ui journalise l'erreur de validation : attendue ici (même précaution que le test d'erreur existant).
    const logged = spyOn(console, "error").mockImplementation(() => {});
    try {
      const receipt = { id: "root", component: "Receipt", title: "x", facts: [], extra: 1 };
      const { host, errors } = await mountSurface(messagesOf([receipt]));
      expect(host.querySelector(".a2ui-receipt")).toBeNull();
      expect(errors.join("\n")).toContain("extra");
    } finally {
      logged.mockRestore();
    }
  });
});
