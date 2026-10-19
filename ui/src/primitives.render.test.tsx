// Les primitives partagées par le catalogue A2UI et l'UI maison, dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import type { ReactElement } from "react";
import "./test/dom";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { Button, Field, IconButton, IconToggle, Select } = await import("./primitives");
const { expectIconOnly } = await import("./test/iconOnly");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

async function mount(el: ReactElement) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => root.render(el));
  return host;
}

const icon = createElement("svg", null);

describe("Button", () => {
  test("Button defaults to a plain btn of type button", async () => {
    const b = (await mount(createElement(Button, null, "OK"))).querySelector("button");
    expect([b?.className, b?.getAttribute("type")]).toEqual(["btn", "button"]);
  });

  test("Button raised primary danger orders its classes like the catalog", async () => {
    const host = await mount(createElement(Button, { raised: true, primary: true, danger: true }, "OK"));
    expect(host.querySelector("button")?.className).toBe("object btn primary danger");
  });

  test("Button ghost with an extra class appends it last", async () => {
    const host = await mount(createElement(Button, { ghost: true, className: "add-slide" }, "OK"));
    expect(host.querySelector("button")?.className).toBe("btn ghost add-slide");
  });

  test("Button passes its click through", async () => {
    let clicks = 0;
    const host = await mount(createElement(Button, { onClick: () => clicks++ }, "OK"));
    await act(async () => host.querySelector("button")?.click());
    expect(clicks).toBe(1);
  });
});

describe("IconButton", () => {
  test("IconButton shows its icon only, named by its label", async () => {
    const host = await mount(createElement(IconButton, { label: "Fermer", icon }));
    expectIconOnly(host.querySelector("button"), "Fermer");
  });

  test("IconButton has no pressed state", async () => {
    const host = await mount(createElement(IconButton, { label: "Fermer", icon }));
    expect(host.querySelector("button")?.hasAttribute("aria-pressed")).toBe(false);
  });

  test("IconButton is never a submit button", async () => {
    const host = await mount(createElement(IconButton, { label: "Fermer", icon }));
    expect(host.querySelector("button")?.getAttribute("type")).toBe("button");
  });
});

describe("IconToggle", () => {
  test("IconToggle shows its icon only, named by its label", async () => {
    const host = await mount(createElement(IconToggle, { label: "Empiler", pressed: false, onToggle: () => {}, icon }));
    expectIconOnly(host.querySelector("button"), "Empiler");
  });

  test("IconToggle title is exactly its label", async () => {
    const host = await mount(createElement(IconToggle, { label: "Empiler", pressed: false, onToggle: () => {}, icon }));
    expect(host.querySelector("button")?.getAttribute("title")).toBe("Empiler");
  });

  test("IconToggle disabled disables the button", async () => {
    const host = await mount(createElement(IconToggle, { label: "x", pressed: false, onToggle: () => {}, icon, disabled: true }));
    expect(host.querySelector("button")?.disabled).toBe(true);
  });

  test("IconToggle pressed sets aria-pressed and the on class", async () => {
    const host = await mount(createElement(IconToggle, { label: "x", pressed: true, onToggle: () => {}, icon, className: "token ghost" }));
    const b = host.querySelector("button");
    expect([b?.getAttribute("aria-pressed"), b?.className]).toEqual(["true", "token ghost on"]);
  });

  test("IconToggle not pressed has no class when none is given", async () => {
    const host = await mount(createElement(IconToggle, { label: "x", pressed: false, onToggle: () => {}, icon }));
    expect(host.querySelector("button")?.hasAttribute("class")).toBe(false);
  });

  test("IconToggle click calls onToggle", async () => {
    let toggles = 0;
    const host = await mount(createElement(IconToggle, { label: "x", pressed: false, onToggle: () => toggles++, icon }));
    await act(async () => host.querySelector("button")?.click());
    expect(toggles).toBe(1);
  });
});

describe("Select", () => {
  const options = [{ value: "a", label: "A" }, { value: "b", label: "B", disabled: true }];

  test("Select renders a select.select with its options", async () => {
    const host = await mount(createElement(Select, { value: "a", options, onChange: () => {} }));
    const s = host.querySelector("select");
    expect([s?.className, [...(s?.options ?? [])].map((o) => [o.value, o.textContent, o.disabled])]).toEqual([
      "select",
      [["a", "A", false], ["b", "B", true]],
    ]);
  });

  test("Select passes other attributes to the select", async () => {
    const host = await mount(createElement(Select, { value: "a", options, onChange: () => {}, "aria-label": "Silence", disabled: true }));
    const s = host.querySelector("select");
    expect([s?.getAttribute("aria-label"), s?.disabled]).toEqual(["Silence", true]);
  });

  test("Select sends the chosen value", async () => {
    const chosen: string[] = [];
    const host = await mount(createElement(Select, { value: "a", options: [...options, { value: "c", label: "C" }], onChange: (v: string) => chosen.push(v) }));
    const s = host.querySelector("select") as HTMLSelectElement;
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")?.set?.call(s, "c");
      s.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(chosen).toEqual(["c"]);
  });
});

describe("Field", () => {
  test.each([
    ["note", "note-field", "a2ui-hint"],
    ["row", "settings-row", "caption"],
    ["menu", "menu-theme", "caption"],
  ] as const)("Field %s wraps its label and control", async (variant, outer, hint) => {
    const host = await mount(createElement(Field, { variant, label: "Langue" }, createElement("input")));
    const f = host.firstElementChild;
    expect([f?.tagName, f?.className, f?.firstElementChild?.className, f?.firstElementChild?.textContent, f?.lastElementChild?.tagName]).toEqual([
      "LABEL", outer, hint, "Langue", "INPUT",
    ]);
  });

  test("Field as div keeps its classes in a div", async () => {
    const host = await mount(createElement(Field, { variant: "note", label: "Couleur", as: "div" }, createElement("input")));
    const f = host.firstElementChild;
    expect([f?.tagName, f?.className, f?.firstElementChild?.className, f?.firstElementChild?.textContent]).toEqual([
      "DIV", "note-field", "a2ui-hint", "Couleur",
    ]);
  });
});
