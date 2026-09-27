import { describe, expect, test } from "bun:test";
import { dictationShortcut, escapeAction } from "./keys";

const k = (
  key: string,
  mods: { meta?: boolean; ctrl?: boolean; shift?: boolean; alt?: boolean; repeat?: boolean; code?: string } = {},
) => ({
  key,
  code: mods.code ?? `Key${key.toUpperCase()}`,
  metaKey: !!mods.meta,
  ctrlKey: !!mods.ctrl,
  shiftKey: !!mods.shift,
  altKey: !!mods.alt,
  repeat: !!mods.repeat,
});
const idle = { agentBusy: false, recording: false };
const busy = { agentBusy: true, recording: false };

describe("raccourcis de la dictée", () => {
  test("⌘D dicte là où l'on écrit", () => expect(dictationShortcut(k("d", { meta: true }), idle)).toBe("auto"));
  test("Ctrl+D aussi", () => expect(dictationShortcut(k("d", { ctrl: true }), idle)).toBe("auto"));
  test("⌘⇧D dicte dans le brouillon", () => expect(dictationShortcut(k("D", { meta: true, shift: true }), idle)).toBe("draft"));
  test("Ctrl⇧D aussi", () => expect(dictationShortcut(k("d", { ctrl: true, shift: true }), idle)).toBe("draft"));
  test("⌘⇧D pendant un tour arrête l'agent", () =>
    expect(dictationShortcut(k("d", { meta: true, shift: true }), busy)).toBe("agent-cancel"));
  test("⌘D pendant un tour dicte toujours", () =>
    expect(dictationShortcut(k("d", { meta: true }), busy)).toBe("auto"));
  test("pendant une écoute, ⌘⇧D l'arrête au lieu d'arrêter l'agent", () =>
    expect(dictationShortcut(k("d", { meta: true, shift: true }), { agentBusy: true, recording: true })).toBe("draft"));
  test("une touche maintenue ne répète pas le raccourci", () =>
    expect(dictationShortcut(k("d", { meta: true, repeat: true }), idle)).toBeNull());
  test("⌘⌥D n'est pas un raccourci de dictée", () =>
    expect(dictationShortcut(k("d", { meta: true, alt: true }), idle)).toBeNull());
  test("sur un clavier cyrillique, ⌘D est lu par sa position", () =>
    expect(dictationShortcut(k("в", { meta: true, code: "KeyD" }), idle)).toBe("auto"));
  test("D seul, ⇧D ou ⌘S ne font rien", () => {
    expect(dictationShortcut(k("d"), idle)).toBeNull();
    expect(dictationShortcut(k("D", { shift: true }), idle)).toBeNull();
    expect(dictationShortcut(k("s", { meta: true }), idle)).toBeNull();
  });
});

describe("Échap", () => {
  test("désélectionne d'abord", () => expect(escapeAction({ selected: true, agentBusy: true })).toBe("deselect"));
  test("arrête l'agent quand rien n'est sélectionné", () =>
    expect(escapeAction({ selected: false, agentBusy: true })).toBe("agent-cancel"));
  test("ne fait rien sinon", () => expect(escapeAction({ selected: false, agentBusy: false })).toBeNull());
});
