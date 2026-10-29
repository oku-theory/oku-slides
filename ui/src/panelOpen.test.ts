// The packs panel's "Open" buttons: which actions the interface answers itself.

import { describe, expect, test } from "bun:test";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Surface } from "./api";
import { panelOpen } from "./panelOpen";

const ID = "2a7d4c91-5e3b-4f08-b6a1-8c9e0d1f2a31";
const surface = (id: string): Surface => ({ id, place: "thread", live: true, question: false, messages: [] });
const action = (name: string, context: unknown): A2uiClientAction => ({ name, context }) as unknown as A2uiClientAction;

describe("panelOpen", () => {
  test("panelOpen with open_theme on the packs panel answers the theme id", () => {
    expect(panelOpen(surface("host-packs"), action("open_theme", { theme: ID }))).toEqual({ kind: "theme", id: ID });
  });

  test("panelOpen with open_pack on the packs panel answers the pack id", () => {
    expect(panelOpen(surface("host-packs"), action("open_pack", { pack: ID }))).toEqual({ kind: "pack", id: ID });
  });

  test("panelOpen on another surface answers null", () => {
    expect(panelOpen(surface("host-data"), action("open_pack", { pack: ID }))).toBeNull();
    expect(panelOpen(surface("sheet-1"), action("open_theme", { theme: ID }))).toBeNull();
  });

  test("panelOpen with another event answers null", () => {
    expect(panelOpen(surface("host-packs"), action("use_pack", { pack: ID }))).toBeNull();
  });

  test("panelOpen with a non-UUID id answers null", () => {
    expect(panelOpen(surface("host-packs"), action("open_pack", { pack: "not-a-uuid" }))).toBeNull();
    expect(panelOpen(surface("host-packs"), action("open_pack", { pack: `${ID}x` }))).toBeNull();
  });

  test("panelOpen with a non-string id answers null", () => {
    expect(panelOpen(surface("host-packs"), action("open_pack", { pack: 12 }))).toBeNull();
    expect(panelOpen(surface("host-packs"), action("open_pack", {}))).toBeNull();
    expect(panelOpen(surface("host-packs"), action("open_pack", undefined))).toBeNull();
  });

  test("panelOpen with an inherited __proto__ context answers null", () => {
    const context = JSON.parse(`{"__proto__": {"pack": "${ID}"}}`);
    expect(panelOpen(surface("host-packs"), action("open_pack", context))).toBeNull();
    expect(panelOpen(surface("host-packs"), action("open_pack", Object.create({ pack: ID })))).toBeNull();
  });
});
