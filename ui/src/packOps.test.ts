import { afterAll, expect, test } from "bun:test";
import type { Snapshot } from "./api";
import type { PackInfo, PackPin } from "./packTypes";
import { setMessages } from "./i18n";
import { deckPinsKey, packOptionLabel, pinsKey, splitKind, widgetToOpen } from "./packOps";
import { resetMessages } from "./test/messages";

afterAll(resetMessages);

const pack = (over: Partial<PackInfo>): PackInfo => ({
  id: "6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10",
  version: "sha256:" + "a".repeat(64),
  name: "Essentiel",
  namespace: "essentiel",
  requires: [],
  description: "",
  builtin: true,
  default: false,
  received: false,
  kind: "layouts",
  widgets: 6,
  profiles: 10,
  ...over,
});

test("pinsKey is independent of key order", () => {
  const pin = (version: string): PackPin => ({ id: "x", version });
  expect(pinsKey({ b: pin("2"), a: pin("1") })).toBe(pinsKey({ a: pin("1"), b: pin("2") }));
  expect(pinsKey({ b: pin("2"), a: pin("1") })).toBe("a@1,b@2");
  expect(pinsKey({})).toBe("");
});

test("deckPinsKey changes when a layout pin changes", () => {
  const pin = (version: string): PackPin => ({ id: "x", version });
  const widgets = { base: pin("1") };
  expect(deckPinsKey({ widgets, layouts: { essentiel: pin("1") } })).not.toBe(deckPinsKey({ widgets, layouts: { essentiel: pin("2") } }));
  expect(deckPinsKey({ widgets, layouts: {} })).not.toBe(deckPinsKey({ widgets: {}, layouts: widgets }));
  expect(deckPinsKey(null)).toBe("|");
});

test("packOptionLabel tells a received pack from mine and leaves a built-in one bare", () => {
  setMessages("en", ["ui-picker-set-mine = { $name } · mine\nui-picker-set-received = { $name } · received"]);
  expect(packOptionLabel(pack({}))).toBe("Essentiel");
  expect(packOptionLabel(pack({ builtin: false, name: "Charte" }))).toBe("Charte · mine");
  expect(packOptionLabel(pack({ builtin: false, received: true, name: "Charte" }))).toBe("Charte · received");
});

const snapOf = (widgets: Record<string, PackPin>, layouts: Record<string, PackPin> = {}) => ({ widgets, layouts }) as unknown as Snapshot;

test.each([
  ["acme/card", { ns: "acme", name: "card" }],
  ["acme.charte/card", { ns: "acme.charte", name: "card" }],
  ["deck/kpi-2", { ns: "deck", name: "kpi-2" }],
])("splitKind reads the namespace and name of %s", (kind, parts) => {
  expect(splitKind(kind)).toEqual(parts);
});

test.each(["card", "Acme/card", "acme/", "/card", "acme/a/b", "a.b.c/card", "1acme/card", `${"a".repeat(65)}/card`])(
  "splitKind of %s is null, as split_kind",
  (kind) => {
    expect(splitKind(kind)).toBeNull();
  },
);

test("widgetToOpen targets the pack pinned under the kind's namespace, not the first pack", () => {
  const snap = snapOf({ essentiel: { id: "e", version: "1" }, acme: { id: "a", version: "1" } });
  expect(widgetToOpen(snap, "acme/card")).toEqual({ pack: "a", name: "card" });
});

test("widgetToOpen of an unpinned namespace has no pack, and of a core kind is null", () => {
  const snap = snapOf({ essentiel: { id: "e", version: "1" } });
  expect(widgetToOpen(snap, "acme/card")).toEqual({ pack: null, name: "card" });
  expect(widgetToOpen(snap, "card")).toBeNull();
});

test("widgetToOpen targets the layout pack pinned under the kind's namespace", () => {
  const snap = snapOf({ base: { id: "b", version: "1" } }, { essentiel: { id: "e", version: "1" } });
  expect(widgetToOpen(snap, "essentiel/title")).toEqual({ pack: "e", name: "title" });
});
