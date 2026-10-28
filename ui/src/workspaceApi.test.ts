import { describe, expect, test } from "bun:test";
import { closeStays, linkedTabs, type TabDocument, type TabInfo, tabPath, type WindowView, type WorkspaceView } from "./workspaceApi";

const tab = (document: TabDocument): TabInfo => ({
  id: "t-1",
  title: "essai",
  dirty: false,
  pristine: false,
  theme: null,
  document,
});

describe("tabPath", () => {
  test("a deck with a file: its path", () => {
    expect(tabPath(tab({ kind: "deck", path: "/d/a.oku-slides.pdf", deck_uid: "u" }))).toBe("/d/a.oku-slides.pdf");
  });

  test("a deck without a file: null", () => {
    expect(tabPath(tab({ kind: "deck", path: null, deck_uid: null }))).toBeNull();
  });

  test("a help tab: null", () => {
    expect(tabPath(tab({ kind: "help", locale: "fr" }))).toBeNull();
  });

  test("a pack tab: null", () => {
    expect(tabPath(tab({ kind: "pack", pack: "a7e11e40-0000-4000-8000-000000000001" }))).toBeNull();
  });

  test("tabPath of a deck_pack tab is null", () => {
    expect(tabPath(tab({ kind: "deck_pack", deck_tab: "t-1", pack: "a7e11e40-0000-4000-8000-000000000001" }))).toBeNull();
  });

  test("an unknown kind (a newer host): null, not a throw", () => {
    const unknown = { kind: "slides", path: "/x" } as unknown as TabDocument;
    expect(tabPath(tab(unknown))).toBeNull();
  });
});

const PACK = "a7e11e40-0000-4000-8000-000000000001";
/** Tab `id` editing `document`. */
const tabIn = (id: string, document: TabDocument = { kind: "deck", path: null, deck_uid: null }): TabInfo => ({ ...tab(document), id, title: id });
const linked = (id: string, owner: string) => tabIn(id, { kind: "deck_pack", deck_tab: owner, pack: PACK });
const win = (label: string, tabs: TabInfo[]): WindowView => ({ label, active: tabs[0]?.id ?? "", tabs });
const viewOf = (...windows: WindowView[]): WorkspaceView => ({ generation: 1, focused: "w-1", agent_detached: false, import_pick: null, windows });

describe("linkedTabs", () => {
  test("linkedTabs of a deck tab lists its deck_pack tabs in every window", () => {
    const windows = [win("w-1", [tabIn("t-1"), linked("t-2", "t-1"), linked("t-3", "t-9")]), win("w-2", [linked("t-4", "t-1")])];
    expect(linkedTabs(windows, "t-1").map((x) => x.id)).toEqual(["t-2", "t-4"]);
  });
});

describe("closeStays", () => {
  test("closeStays of an owner whose linked tab is the only other tab is true in the browser", () => {
    expect(closeStays(viewOf(win("w-1", [tabIn("t-1"), linked("t-2", "t-1")])), "w-1", "t-1", false)).toBe(true);
  });

  test("closeStays in Tauri is false", () => {
    expect(closeStays(viewOf(win("w-1", [tabIn("t-1")])), "w-1", "t-1", true)).toBe(false);
  });

  test("closeStays of a lone plain tab is true in the browser", () => {
    expect(closeStays(viewOf(win("w-1", [tabIn("t-1")])), "w-1", "t-1", false)).toBe(true);
  });

  test("closeStays of a deck-pack tab beside its owner is false", () => {
    expect(closeStays(viewOf(win("w-1", [tabIn("t-1"), linked("t-2", "t-1")])), "w-1", "t-2", false)).toBe(false);
  });

  test("closeStays of an owner with another plain tab is false", () => {
    expect(closeStays(viewOf(win("w-1", [tabIn("t-1"), linked("t-2", "t-1"), tabIn("t-3")])), "w-1", "t-1", false)).toBe(false);
  });
});
