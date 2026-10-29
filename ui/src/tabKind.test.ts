import { describe, expect, test } from "bun:test";
import type { Snapshot, TabDocument } from "./api";
import { documentKind, tabKind } from "./tabKind";

/** A snapshot reduced to the fields `tabKind` reads. */
const snap = (over: Partial<Pick<Snapshot, "history" | "read_only" | "theming" | "editing">>) =>
  ({ history: null, read_only: false, theming: null, editing: null, ...over }) as unknown as Snapshot;
const some = {} as never;

describe("tabKind", () => {
  test.each([
    ["a plain snapshot", snap({}), "deck"],
    ["a history view", snap({ history: some }), "history"],
    ["a read-only help deck", snap({ read_only: true }), "help"],
    ["a theme draft", snap({ theming: some }), "theme"],
    ["a pack edit", snap({ editing: some }), "pack"],
    ["history over help", snap({ history: some, read_only: true }), "history"],
    ["help over theme", snap({ read_only: true, theming: some }), "help"],
    ["theme over pack", snap({ theming: some, editing: some }), "theme"],
  ])("tabKind of %s rends %s", (_, s, kind) => expect(tabKind(s)).toBe(kind));
});

describe("documentKind", () => {
  test.each([
    ["a deck", { kind: "deck", path: null, deck_uid: null }, "deck"],
    ["a help", { kind: "help", locale: "fr" }, "help"],
    ["a pack", { kind: "pack", pack: "p" }, "pack"],
    ["a deck pack", { kind: "deck_pack", deck_tab: "d", pack: "p" }, "pack"],
    ["a theme", { kind: "theme", theme: "t" }, "theme"],
    ["an unknown kind", { kind: "future" }, "deck"],
    ["no document", undefined, "deck"],
  ] as [string, unknown, string][])("documentKind of %s rends %s", (_, doc, kind) =>
    expect(documentKind(doc as TabDocument | undefined)).toBe(kind),
  );
});
