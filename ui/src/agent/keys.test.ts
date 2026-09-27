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
  test("⌘D dicte du texte", () => expect(dictationShortcut(k("d", { meta: true }), idle)).toBe("text"));
  test("Ctrl+D aussi", () => expect(dictationShortcut(k("d", { ctrl: true }), idle)).toBe("text"));
  test("⌘⇧D parle à l'agent", () => expect(dictationShortcut(k("D", { meta: true, shift: true }), idle)).toBe("agent"));
  test("Ctrl⇧D aussi", () => expect(dictationShortcut(k("d", { ctrl: true, shift: true }), idle)).toBe("agent"));
  test("⌘⇧D pendant un tour arrête l'agent", () =>
    expect(dictationShortcut(k("d", { meta: true, shift: true }), busy)).toBe("agent-cancel"));
  test("⌘D pendant un tour dicte toujours du texte", () =>
    expect(dictationShortcut(k("d", { meta: true }), busy)).toBe("text"));
  test("pendant une écoute, ⌘⇧D l'arrête au lieu d'arrêter l'agent", () =>
    expect(dictationShortcut(k("d", { meta: true, shift: true }), { agentBusy: true, recording: true })).toBe("agent"));
  test("une touche maintenue ne répète pas le raccourci", () =>
    expect(dictationShortcut(k("d", { meta: true, repeat: true }), idle)).toBeNull());
  test("⌘⌥D n'est pas un raccourci de dictée", () =>
    expect(dictationShortcut(k("d", { meta: true, alt: true }), idle)).toBeNull());
  test("sur un clavier cyrillique, ⌘D est lu par sa position", () =>
    expect(dictationShortcut(k("в", { meta: true, code: "KeyD" }), idle)).toBe("text"));
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
