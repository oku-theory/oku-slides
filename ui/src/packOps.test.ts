import { afterAll, expect, test } from "bun:test";
import type { PackInfo, PackPin, Snapshot } from "./api";
import { setMessages } from "./i18n";
import { addPackOp, firstPackId, packOptionLabel, pinsKey, splitKind, widgetToOpen } from "./packOps";
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
  count: 3,
  ...over,
});

test("addPackOp pins the pack under its namespace", () => {
  const op = addPackOp(pack({ namespace: "acme", requires: ["base"] }));
  expect(op).toEqual({ op: "add_pack", namespace: "acme", pin: { id: pack({}).id, version: pack({}).version, requires: ["base"] } });
});

test("pinsKey is independent of key order", () => {
  const pin = (version: string): PackPin => ({ id: "x", version });
  expect(pinsKey({ b: pin("2"), a: pin("1") })).toBe(pinsKey({ a: pin("1"), b: pin("2") }));
  expect(pinsKey({ b: pin("2"), a: pin("1") })).toBe("a@1,b@2");
  expect(pinsKey({})).toBe("");
});

test("packOptionLabel tells a received pack from mine and leaves a built-in one bare", () => {
  setMessages("en", ["ui-picker-set-mine = { $name } · mine\nui-picker-set-received = { $name } · received"]);
  expect(packOptionLabel(pack({}))).toBe("Essentiel");
  expect(packOptionLabel(pack({ builtin: false, name: "Charte" }))).toBe("Charte · mine");
  expect(packOptionLabel(pack({ builtin: false, received: true, name: "Charte" }))).toBe("Charte · received");
});

const snapOf = (widgets: Record<string, PackPin>) => ({ widgets }) as unknown as Snapshot;

test("firstPackId prefers a library pack over the deck pack", () => {
  expect(firstPackId(snapOf({ deck: { id: "d", version: "1" }, essentiel: { id: "e", version: "1" } }))).toBe("e");
});

test("firstPackId falls back to the deck pack", () => {
  expect(firstPackId(snapOf({ deck: { id: "d", version: "1" } }))).toBe("d");
});

const pin = (id: string): PackPin => ({ id, version: "1" });

test("firstPackId saute base pour essentiel", () => expect(firstPackId(snapOf({ base: pin("b"), essentiel: pin("e") }))).toBe("e"));

test("firstPackId saute base pour le pack du deck", () => expect(firstPackId(snapOf({ base: pin("b"), deck: pin("d") }))).toBe("d"));

test("firstPackId with base alone returns base", () => expect(firstPackId(snapOf({ base: pin("b") }))).toBe("b"));

test("firstPackId of no pin is null", () => {
  expect(firstPackId(snapOf({}))).toBeNull();
  expect(firstPackId(null)).toBeNull();
});

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
