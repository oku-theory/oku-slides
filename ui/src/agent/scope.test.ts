import { describe, expect, test } from "bun:test";
import { agentScope, busyElsewhere, sameScope, scopeKey, scopePlace } from "./scope";

const view = { packMode: true, history: false, pack: "p-1", widget: null };
const slides = { kind: "slides" } as const;
const atelier = { kind: "atelier", pack: "p-1" } as const;
const kpi = { kind: "widget", pack: "p-1", name: "kpi" } as const;

describe("agentScope", () => {
  test("agentScope on the deck gives slides", () => expect(agentScope({ ...view, packMode: false })).toEqual(slides));
  test("agentScope in the history gives slides", () => expect(agentScope({ ...view, history: true, widget: "kpi" })).toEqual(slides));
  test("agentScope in pack mode without widget gives the pack atelier", () => expect(agentScope(view)).toEqual(atelier));
  test("agentScope on a widget tab gives that widget of the pack", () => expect(agentScope({ ...view, widget: "kpi" })).toEqual(kpi));
  test("agentScope in pack mode without pack gives slides", () => expect(agentScope({ ...view, pack: null })).toEqual(slides));
});

describe("scopeKey", () => {
  test("scopeKey of equal scopes is equal", () => expect(scopeKey(kpi)).toBe(scopeKey({ ...kpi })));
  test("scopeKey of another pack differs", () => expect(scopeKey(atelier)).not.toBe(scopeKey({ ...atelier, pack: "p-2" })));
});

describe("sameScope", () => {
  test("sameScope with the same widget is true", () => expect(sameScope(kpi, { ...kpi })).toBe(true));
  test("sameScope with another widget name is false", () => expect(sameScope(kpi, { ...kpi, name: "frise" })).toBe(false));
  test("sameScope of atelier and widget is false", () => expect(sameScope(atelier, kpi)).toBe(false));
});

describe("busyElsewhere", () => {
  test("busyElsewhere when idle is null", () => expect(busyElsewhere({ state: "idle" }, slides)).toBeNull());
  test("busyElsewhere without scope is null", () => expect(busyElsewhere({ state: "starting" }, slides)).toBeNull());
  test("busyElsewhere in the same scope is null", () => expect(busyElsewhere({ state: "working", scope: slides }, slides)).toBeNull());
  test("busyElsewhere in another scope names it", () =>
    expect(busyElsewhere({ state: "working", scope: atelier }, slides)).toEqual(atelier));
});

describe("scopePlace", () => {
  const tr = (key: string) => `[${key}]`;
  test("scopePlace of a widget is its name", () => expect(scopePlace(kpi, tr)).toBe("kpi"));
  test("scopePlace of the atelier is its label", () => expect(scopePlace(atelier, tr)).toBe("[ui-agent-scope-atelier]"));
  test("scopePlace of the slides is their label", () => expect(scopePlace(slides, tr)).toBe("[ui-agent-scope-slides]"));
});
