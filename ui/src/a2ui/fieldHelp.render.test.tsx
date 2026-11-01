// A field's help line: shown under the control and tied to it with aria-describedby (t-1417).

import { describe, expect, test } from "bun:test";
import "../test/dom";

// Bun does not evaluate static imports in written order: the surface loads after `../test/dom`.
const { mountSurface, messagesOf } = await import("../test/surface");

const HELP = "Shown above the title";
const action = { event: { name: "param", context: {} } };

/** Mounts a surface whose root is `root`; returns its host. */
const mount = async (root: Record<string, unknown>, data: unknown = {}) =>
  (await mountSurface(messagesOf([{ id: "root", ...root }], data))).host;

/** The text of the element `el` cites in aria-describedby, or null. */
function describedText(host: Element, el: Element | null): string | null {
  const id = el?.getAttribute("aria-describedby");
  if (!id) return null;
  return host.ownerDocument.getElementById(id)?.textContent ?? null;
}

/** Each field kind with help, and the selector of the control the help describes. */
const FIELDS: [string, Record<string, unknown>, string][] = [
  ["TextField", { component: "TextField", label: "Kicker", value: { path: "/v" }, action }, "input"],
  ["Switch", { component: "Switch", label: "Highlight", value: { path: "/on" }, action }, 'input[role="switch"]'],
  ["Select", { component: "Select", label: "Theme", value: { path: "/v" }, options: [{ id: "a", label: "A" }], action }, "select"],
  ["Slider", { component: "Slider", label: "Opacity", value: { path: "/n" }, min: 0, max: 10, action }, 'input[type="range"]'],
  ["Stepper", { component: "Stepper", label: "Columns", value: { path: "/n" }, min: 1, max: 4, action }, ".stepper button"],
  ["ColorField", { component: "ColorField", label: "Accent", value: { path: "/v" }, palette: ["#112233"], action }, ".swatches"],
  [
    "ListTable",
    {
      component: "ListTable", label: "Steps", rows: { path: "/rows" }, change: { path: "/change" }, markdown: { path: "/md" },
      columns: [{ key: "name", label: "Name", kind: "text", options: [] }], min: 0, palette: [], action,
    },
    "table",
  ],
];

const DATA = { v: "a", on: true, n: 2, rows: [{ id: "r0", name: "Intro" }], change: null, md: {} };

describe("field help", () => {
  test.each(FIELDS)("%s: its control is described by the help", async (_name, field, selector) => {
    const host = await mount({ ...field, help: HELP }, DATA);
    expect(describedText(host, host.querySelector(selector))).toBe(HELP);
  });

  test.each(FIELDS)("%s without help: its control has no description", async (_name, field, selector) => {
    const host = await mount(field, DATA);
    expect(host.querySelector(selector)?.hasAttribute("aria-describedby")).toBe(false);
    expect(host.querySelector(".field-help")).toBeNull();
  });

  test("TextField: the help sits outside the label, so it is not part of the field's name", async () => {
    const host = await mount({ ...FIELDS[0][1], help: HELP }, DATA);
    expect(host.querySelector("label")?.textContent).not.toContain(HELP);
    expect(host.querySelector(".field-help")?.textContent).toBe(HELP);
  });

  test("Switch: the help sits outside its label", async () => {
    const host = await mount({ ...FIELDS[1][1], help: HELP }, DATA);
    expect(host.querySelector("label.toggle")?.textContent).not.toContain(HELP);
    expect(host.querySelector(".field-help")?.textContent).toBe(HELP);
  });

  test("RichTextField: the editor is described by the help", async () => {
    const rich = {
      component: "RichTextField", label: "Body", help: HELP, value: { path: "/body" }, source: { path: "/src" }, palette: [], action,
    };
    const host = await mount(rich, { body: "x", src: [{ runs: [{ t: "x" }] }] });
    expect(describedText(host, host.querySelector(".cm-content"))).toBe(HELP);
  });
});
