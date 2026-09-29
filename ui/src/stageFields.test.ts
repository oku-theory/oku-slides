import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import type { FieldView, SlideView, TextStyle } from "./api";
import { setMessages } from "./i18n";
import { anchorIds, fieldHitBox, fieldKey, fieldLabel, findField, lineHeight, nextField, primaryFieldOf } from "./stageFields";

const ftl = (lang: string) =>
  Bun.file(new URL(`../../crates/oku-slides-i18n/src/locales/${lang}/ui.ftl`, import.meta.url)).text();

beforeAll(async () => {
  setMessages("fr", [await ftl("en"), await ftl("fr")]);
});
// Les autres fichiers de la passe attendent les identifiants bruts.
afterAll(() => setMessages("en", []));

const style: TextStyle = { size: 20, leading: 6, tracking: 0, font: ["Inter"], weight: 400, fill: null, align: "left", case: null, bg: null };
const f = (owner: string, field: string, extra: Partial<FieldView> = {}): FieldView => ({
  owner, owner_kind: owner === "s" ? "slide" : "element", widget: "card", field, kind: "plain", label: null,
  text: "x", empty: false, side: null, x: 10, y: 100, w: 50, h: 20, ax: 10, aw: 200, style, ...extra,
});
const view = (fields: FieldView[]) => ({ fields } as unknown as SlideView);
const v = view([f("s", "title"), f("c1", "kicker"), f("c1", "title"), f("c2", "title")]);

describe("nextField", () => {
  test("nextField forward returns the following field", () => {
    expect(nextField(v, fieldKey({ owner: "c1", field: "kicker" }), 1)?.field).toBe("title");
  });
  test("nextField backward from the first returns null", () => {
    expect(nextField(v, fieldKey({ owner: "s", field: "title" }), -1)).toBeNull();
  });
  test("nextField forward from the last returns null", () => {
    expect(nextField(v, fieldKey({ owner: "c2", field: "title" }), 1)).toBeNull();
  });
  test("nextField with a vanished key returns null", () => {
    expect(nextField(v, fieldKey({ owner: "zz", field: "title" }), 1)).toBeNull();
  });
});

test("findField by key survives a reordered view", () => {
  const k = fieldKey({ owner: "c1", field: "title" });
  expect(findField(view([...v.fields].reverse()), k)?.owner).toBe("c1");
});

test("findField with no key returns null", () => {
  expect(findField(v, null)).toBeNull();
});

describe("primaryFieldOf", () => {
  const card = (kicker: Partial<FieldView>, title: Partial<FieldView>, body: Partial<FieldView>) =>
    view([f("c1", "kicker", kicker), f("c1", "title", title), f("c1", "body", { kind: "rich", ...body })]);
  test("primaryFieldOf returns the first non-empty field of the element", () => {
    const empty = { empty: true, text: "", h: 0, side: "before" as const };
    expect(primaryFieldOf(card(empty, {}, {}), "c1")?.field).toBe("title");
  });
  test("primaryFieldOf with every field empty returns the first rich field", () => {
    const empty = { empty: true, text: "" };
    expect(primaryFieldOf(card(empty, empty, empty), "c1")?.field).toBe("body");
  });
  test("primaryFieldOf with every plain field empty and no rich one returns the first field", () => {
    const empty = { empty: true, text: "" };
    expect(primaryFieldOf(view([f("c1", "kicker", empty), f("c1", "title", empty)]), "c1")?.field).toBe("kicker");
  });
  test("primaryFieldOf of an object without fields returns null", () => {
    expect(primaryFieldOf(v, "zz")).toBeNull();
  });
});

test("fieldLabel of a known field uses the translation, else the schema label", () => {
  expect(fieldLabel({ field: "title", label: null })).not.toBe("title");
  expect(fieldLabel({ field: "kicker", label: null })).toBe("Surtitre");
  expect(fieldLabel({ field: "zz-inconnu", label: "Légende" })).toBe("Légende");
  expect(fieldLabel({ field: "zz-inconnu", label: null })).toBe("zz-inconnu");
});

test("lineHeight without layout is a 0.7 em cap height plus leading, scaled", () => {
  expect(lineHeight(style, 1)).toBeCloseTo(20);
  expect(lineHeight({ ...style, leading: 20 }, 2)).toBeCloseTo(68);
});

describe("fieldHitBox", () => {
  const alone = (x: FieldView) => [x];
  test("fieldHitBox of an empty field before its anchor ends at the anchor", () => {
    const k = f("c1", "kicker", { empty: true, side: "before", h: 0, w: 0 });
    const b = fieldHitBox(k, 1, alone(k));
    expect(b.top + b.height).toBeCloseTo(100);
    expect(b.height).toBeGreaterThan(0);
    expect(b.width).toBeCloseTo(200);
  });
  test("fieldHitBox of an empty field after its anchor starts at the anchor", () => {
    const sub = f("c1", "sub", { empty: true, side: "after", h: 0, w: 0 });
    const b = fieldHitBox(sub, 2, alone(sub));
    expect(b.top).toBeCloseTo(200);
  });
  test("fieldHitBox of a filled field is its rendered box", () => {
    const t = f("c1", "title");
    expect(fieldHitBox(t, 2, alone(t))).toEqual({ left: 20, top: 200, width: 100, height: 40 });
  });
  test("fieldHitBox of a rendered but empty field keeps its measured box", () => {
    const body = f("c1", "body", { empty: true, text: "" });
    expect(fieldHitBox(body, 1, alone(body))).toEqual({ left: 10, top: 100, width: 50, height: 20 });
  });
  test("fieldHitBox stacks consecutive empty fields before one anchor, top to bottom in tab order", () => {
    // Carte sans surtitre ni titre : les deux marqueurs sont au haut du corps.
    const empty = { empty: true, text: "", side: "before" as const, h: 0, w: 0 };
    const kicker = f("c1", "kicker", { ...empty, style: { ...style, size: 10 } });
    const title = f("c1", "title", empty);
    const fields = [kicker, title, f("c1", "body", { kind: "rich" })];
    const k = fieldHitBox(kicker, 2, fields);
    const t = fieldHitBox(title, 2, fields);
    expect(t.top + t.height).toBeCloseTo(200);
    expect(k.top + k.height).toBeCloseTo(t.top);
    expect(k.height).toBeCloseTo(lineHeight(kicker.style!, 2));
    expect(t.height).toBeCloseTo(lineHeight(style, 2));
  });
  test("fieldHitBox stacks consecutive empty fields after one anchor, top to bottom in tab order", () => {
    const empty = { empty: true, text: "", side: "after" as const, h: 0, w: 0 };
    const sub = f("s", "subtitle", empty);
    const note = f("s", "note", empty);
    const fields = [f("s", "title"), sub, note];
    const a = fieldHitBox(sub, 1, fields);
    const b = fieldHitBox(note, 1, fields);
    expect(a.top).toBeCloseTo(100);
    expect(b.top).toBeCloseTo(a.top + a.height);
  });
  test("fieldHitBox does not stack empty fields of another object at the same height", () => {
    const empty = { empty: true, text: "", side: "before" as const, h: 0, w: 0 };
    const mine = f("c1", "kicker", empty);
    const fields = [mine, f("c2", "kicker", empty)];
    const b = fieldHitBox(mine, 1, fields);
    expect(b.top + b.height).toBeCloseTo(100);
  });
});

test("anchorIds lists element targets and slide fields as @param", () => {
  const withTargets = { ...v, targets: [{ id: "c1" }, { id: "c2" }] } as unknown as SlideView;
  expect(anchorIds(withTargets)).toEqual(["c1", "c2", "@title"]);
});
